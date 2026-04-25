//! Pipeline orchestration.
//!
//! [`format`] is the one entry point. It:
//!   1. short-circuits if formatting is disabled,
//!   2. lets app-context optionally override the user's chosen mode,
//!   3. dispatches to the per-mode pipeline,
//!   4. returns a new `String` without mutating the input.

use super::app_context::resolve_mode;
use super::commands::{apply_corrections, apply_spoken_punctuation};
use super::currency;
use super::fillers::{collapse_spaces, dedupe_repeated_words, strip_fillers};
use super::intent::{self, split_list_items, Intent};
use super::numbers::words_to_digits;
use super::{FormattingConfig, FormattingContext, FormattingMode};

pub fn format(text: &str, cfg: &FormattingConfig, ctx: &FormattingContext) -> String {
    if !cfg.enabled || text.trim().is_empty() {
        return text.to_string();
    }

    let mode = if cfg.detect_app_context {
        resolve_mode(cfg.mode, &ctx.app_kind)
    } else {
        cfg.mode
    };

    match mode {
        FormattingMode::Raw => text.to_string(),
        FormattingMode::Clean => clean_pipeline(text, cfg),
        FormattingMode::Smart => smart_pipeline(text, cfg, ctx),
        FormattingMode::Email => email_pipeline(text, cfg, None),
        FormattingMode::Message => message_pipeline(text, cfg),
        FormattingMode::List => list_pipeline(text, cfg),
    }
}

/// Minimal safe cleanup: fillers, repeats, spoken punctuation, basic caps.
fn clean_pipeline(text: &str, cfg: &FormattingConfig) -> String {
    let step1 = apply_corrections(text);
    let step2 = apply_spoken_punctuation(&step1);
    let step3 = strip_fillers(&step2, &cfg.custom_fillers);
    let step4 = dedupe_repeated_words(&step3);
    let step5 = capitalize_sentences(&step4);
    let step6 = ensure_terminal_punctuation(&step5);
    collapse_spaces(&step6)
}

/// Clean + numbers/currency + intent re-dispatch.
///
/// Detects intent on the *raw* text BEFORE applying corrections. Earlier
/// versions ran `apply_corrections` first, which broke utterances like
/// "Email to Raj saying X. No wait, Y" — the sentence-level correction
/// regex would eat "Email to Raj saying X" along with the marker, losing
/// the email envelope. By dispatching first, corrections then run inside
/// the extracted body where only the inline (comma-bounded) variant fires.
fn smart_pipeline(text: &str, cfg: &FormattingConfig, ctx: &FormattingContext) -> String {
    let intent_hit = intent::detect(text);

    match intent_hit {
        Intent::Email { recipient, body_start } => {
            let body = text.get(body_start..).unwrap_or(text).trim();
            email_pipeline(body, cfg, recipient)
        }
        Intent::List { body_start } => {
            let body = text.get(body_start..).unwrap_or(text).trim();
            list_pipeline(body, cfg)
        }
        Intent::Note { body_start } => {
            let body = text.get(body_start..).unwrap_or(text).trim();
            let cleaned = clean_pipeline(body, cfg);
            let digits = words_to_digits(&cleaned);
            currency::apply(&digits)
        }
        Intent::None => {
            // No intent matched — apply corrections globally (sentence-level
            // is fine here because there's no envelope to preserve), then
            // clean + numbers + currency.
            let corrected = apply_corrections(text);
            let cleaned = clean_pipeline(&corrected, cfg);
            let digits = words_to_digits(&cleaned);
            let withccy = currency::apply(&digits);
            let _ = ctx; // app context already baked in via resolve_mode
            withccy
        }
    }
}

/// Casual, concise; numbers + currency allowed but no formalities.
fn message_pipeline(text: &str, cfg: &FormattingConfig) -> String {
    let cleaned = clean_pipeline(text, cfg);
    let digits = words_to_digits(&cleaned);
    currency::apply(&digits)
}

/// Bullet list. Splits the body into items and emits one per line.
fn list_pipeline(text: &str, cfg: &FormattingConfig) -> String {
    let cleaned = clean_pipeline(text, cfg);
    let digits = words_to_digits(&cleaned);
    let withccy = currency::apply(&digits);
    let items = split_list_items(&withccy);
    if items.is_empty() {
        return withccy;
    }
    items
        .into_iter()
        .map(|item| {
            // Strip trailing punctuation that bleeds over from speech cadence —
            // "- Milk." or "- Bread," read noisy. Keep internal punctuation.
            let trimmed = item.trim_end_matches(|c: char| matches!(c, ',' | '.' | ';' | ':'));
            format!("- {}", capitalize_first(trimmed))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formal email shape. Body is cleaned + number/currency-aware.
fn email_pipeline(body: &str, cfg: &FormattingConfig, recipient: Option<String>) -> String {
    let cleaned = clean_pipeline(body, cfg);
    let digits = words_to_digits(&cleaned);
    let withccy = currency::apply(&digits);
    let paragraphs = split_paragraphs(&withccy);

    let greeting = match recipient.as_deref() {
        Some(name) if !name.trim().is_empty() => format!("Hi {},\n\n", title_case(name.trim())),
        _ => "Hi,\n\n".to_string(),
    };
    let body_fmt = paragraphs.join("\n\n");
    format!("{}{}\n\nBest regards,", greeting, body_fmt)
}

// ---------- local helpers ------------------------------------------------

/// Capitalise the first alphabetic letter of each sentence.
fn capitalize_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for c in text.chars() {
        if capitalize_next && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(c);
            if matches!(c, '.' | '!' | '?' | '\n') {
                capitalize_next = true;
            } else if !c.is_whitespace() {
                capitalize_next = false;
            }
        }
    }
    out
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn title_case(name: &str) -> String {
    name.split_whitespace()
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ensure the final sentence ends with `.` / `!` / `?`. Empty input returns
/// empty so callers can short-circuit.
fn ensure_terminal_punctuation(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    let last = trimmed.chars().last().unwrap();
    if matches!(last, '.' | '!' | '?' | ':' | ';') {
        trimmed.to_string()
    } else {
        format!("{}.", trimmed)
    }
}

/// Split on double-newline OR on sentence boundaries with a length heuristic
/// to keep paragraph grouping natural without over-splitting.
fn split_paragraphs(text: &str) -> Vec<String> {
    let explicit: Vec<String> = text
        .split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if explicit.len() > 1 {
        return explicit;
    }
    // One paragraph — just return it whole; never guess additional breaks.
    if explicit.is_empty() {
        Vec::new()
    } else {
        explicit
    }
}
