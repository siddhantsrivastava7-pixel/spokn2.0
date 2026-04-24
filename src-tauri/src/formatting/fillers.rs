//! Filler removal and repeated-word cleanup.
//!
//! Rules are conservative — a filler is only stripped when it appears as an
//! obvious discourse marker (surrounded by whitespace / punctuation), never
//! when it could be part of a meaningful phrase. For example, "like" in
//! "I like pizza" is preserved; "I, like, want pizza" strips it.

use once_cell::sync::Lazy;
use regex::Regex;

/// Fillers that are virtually always discourse markers when they stand alone.
/// Conservative list — we intentionally exclude ambiguous words like "so",
/// "right", "well" which carry meaning too often.
pub const DEFAULT_FILLERS: &[&str] = &[
    "um", "umm", "uh", "uhh", "ah", "ahh", "hmm", "hmmm", "er", "erm", "uhm",
];

/// Multi-word discourse fillers. Removed only when conservative (comma-
/// flanked or sentence-leading), see `CONDITIONAL_FILLERS` logic.
pub const CONDITIONAL_FILLERS: &[&str] = &[
    "you know",
    "i mean",
    "kind of",
    "sort of",
    "basically",
    "actually",
    "literally",
    "like",
];

/// Strip fillers from `text`. `custom` is an optional user-supplied list
/// (treated with the same "always" confidence as [`DEFAULT_FILLERS`]).
pub fn strip_fillers(text: &str, custom: &[String]) -> String {
    let mut out = text.to_string();

    // 1. Always-fillers — match as standalone tokens, any case.
    for filler in DEFAULT_FILLERS.iter().copied().chain(custom.iter().map(|s| s.as_str())) {
        out = remove_standalone(&out, filler);
    }

    // 2. Conditional multi-word fillers — only when clearly parenthetical.
    for phrase in CONDITIONAL_FILLERS {
        out = remove_parenthetical(&out, phrase);
    }

    collapse_spaces(&out)
}

/// Collapse `cat cat` → `cat`, but only for identical adjacent words in the
/// same case-insensitive form. Skips short function words where repetition is
/// often intentional emphasis ("no no", "very very").
pub fn dedupe_repeated_words(text: &str) -> String {
    const MIN_LEN: usize = 3;
    let mut out = String::with_capacity(text.len());
    let mut last_word_lower: Option<String> = None;
    let mut current_word = String::new();
    let mut pending_ws = String::new();

    fn flush(
        out: &mut String,
        pending_ws: &mut String,
        last_word_lower: &mut Option<String>,
        word: &str,
        min_len: usize,
    ) {
        if word.is_empty() {
            return;
        }
        let lower = word.to_ascii_lowercase();
        let is_dup = word.chars().count() >= min_len
            && last_word_lower.as_deref() == Some(lower.as_str());
        if !is_dup {
            out.push_str(pending_ws);
            out.push_str(word);
            *last_word_lower = Some(lower);
        }
        // If dup, we drop both the word AND the whitespace leading to it.
        pending_ws.clear();
    }

    for c in text.chars() {
        if c.is_alphanumeric() || c == '\'' || c == '-' {
            current_word.push(c);
        } else {
            flush(&mut out, &mut pending_ws, &mut last_word_lower, &current_word, MIN_LEN);
            current_word.clear();
            if c.is_whitespace() {
                pending_ws.push(c);
            } else {
                // Punctuation breaks the "previous word" context.
                out.push_str(&pending_ws);
                pending_ws.clear();
                out.push(c);
                last_word_lower = None;
            }
        }
    }
    flush(&mut out, &mut pending_ws, &mut last_word_lower, &current_word, MIN_LEN);
    out.push_str(&pending_ws);
    out
}

fn remove_standalone(text: &str, word: &str) -> String {
    let escaped = regex::escape(word);
    // Match the word as a full token including whatever whitespace/punct
    // follows it; then rebuild the surrounding context in a closure so we
    // can keep sentence-ending punctuation but drop the filler's comma.
    //
    // Capture groups:
    //   1: leading boundary (start-of-input or whitespace)
    //   2: optional trailing comma consumed with the filler
    //   3: trailing boundary (whitespace run, end-of-input, or . ! ?)
    let pat = format!(r"(?i)(^|\s)(?:{})(,?)(\s+|$|[.!?])", escaped);
    let re = Regex::new(&pat).expect("valid regex");
    re.replace_all(text, |caps: &regex::Captures| {
        let lead = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let trail = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        // Keep sentence-ending punctuation; otherwise collapse to a single
        // space if either side had whitespace, nothing if we were at EOL.
        if matches!(trail, "." | "!" | "?") {
            format!("{}{}", lead.trim_end(), trail)
        } else if lead.is_empty() && trail.is_empty() {
            String::new()
        } else {
            // Preserve at most one space so later `collapse_spaces` tidies up.
            " ".to_string()
        }
    })
    .to_string()
}

fn remove_parenthetical(text: &str, phrase: &str) -> String {
    let escaped = regex::escape(phrase);
    // Strip the filler along with its flanking commas entirely, letting the
    // downstream `collapse_spaces` pass normalise whitespace. Only fires when
    // the phrase is *clearly* parenthetical (comma-flanked or clause-leading
    // with a trailing comma) so meaning-carrying uses survive.
    let re_flanked = Regex::new(&format!(r"(?i)\s*,\s*{}\s*,\s*", escaped)).expect("valid regex");
    let re_leading = Regex::new(&format!(r"(?i)(^|[.!?]\s+){}\s*,\s*", escaped)).expect("valid regex");

    let step1 = re_flanked.replace_all(text, " ").to_string();
    re_leading
        .replace_all(&step1, |caps: &regex::Captures| {
            caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
        })
        .to_string()
}

pub fn collapse_spaces(text: &str) -> String {
    static RE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+").unwrap());
    static RE_SPACE_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([,.!?;:])").unwrap());
    let s = RE_SPACES.replace_all(text, " ");
    let s = RE_SPACE_PUNCT.replace_all(&s, "$1");
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_um_uh_hmm() {
        let out = strip_fillers("um hello uh world hmm", &[]);
        assert_eq!(out, "hello world");
    }

    #[test]
    fn preserves_meaning_of_like() {
        let out = strip_fillers("I like pizza", &[]);
        assert_eq!(out, "I like pizza");
    }

    #[test]
    fn strips_parenthetical_like() {
        let out = strip_fillers("I, like, want pizza", &[]);
        assert_eq!(out, "I want pizza");
    }

    #[test]
    fn dedupe_repeats_long_words() {
        assert_eq!(dedupe_repeated_words("the the cat sat sat down"), "the cat sat down");
    }

    #[test]
    fn dedupe_keeps_short_emphasis() {
        // "no no" stays — short words often intentional.
        assert_eq!(dedupe_repeated_words("no no please"), "no no please");
    }

    #[test]
    fn custom_fillers_work() {
        let customs = vec!["yaar".to_string(), "na".to_string()];
        let out = strip_fillers("chalo yaar let's go na", &customs);
        assert_eq!(out, "chalo let's go");
    }

    #[test]
    fn leading_filler_gone() {
        let out = strip_fillers("Uh, so I went there", &[]);
        assert!(!out.to_lowercase().starts_with("uh"));
    }
}
