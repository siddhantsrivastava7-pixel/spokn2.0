//! Deterministic, local-only Smart Formatting pipeline.
//!
//! Runs after Whisper produces raw transcript text and before the text is
//! injected into the active application. The whole module is pure text
//! transformation with no dependency on Tauri runtime state, so every
//! sub-module is trivially unit-testable.
//!
//! Design principle: *conservative*. If we cannot confidently apply a rule,
//! we leave the text untouched. Never destroy meaning.

pub mod app_context;
pub mod commands;
pub mod currency;
pub mod fillers;
pub mod formatter;
pub mod intent;
pub mod numbers;

#[cfg(test)]
mod test_corpus;

pub use app_context::{detect_app_kind, AppKind};
pub use formatter::format;

/// User-selected formatting profile. The TypeScript-facing mirror lives in
/// [`crate::settings::SmartFormattingMode`]; keep these two enums in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingMode {
    /// No formatting; pass through.
    Raw,
    /// Minimal cleanup: capitalization, fillers, repeats, basic punctuation.
    Clean,
    /// Clean + numbers/currency + intent-aware re-dispatch. Default.
    Smart,
    /// Formal email shape.
    Email,
    /// Casual, concise — no formality.
    Message,
    /// Item-style dictation → bullet list.
    List,
}

impl Default for FormattingMode {
    fn default() -> Self {
        FormattingMode::Smart
    }
}

/// Configuration passed from settings into the formatter.
#[derive(Debug, Clone)]
pub struct FormattingConfig {
    pub enabled: bool,
    pub mode: FormattingMode,
    /// Extra fillers the user wants stripped in addition to the built-in list.
    pub custom_fillers: Vec<String>,
    /// Whether the formatter may override `mode` based on the active app
    /// (e.g. force Raw inside a terminal).
    pub detect_app_context: bool,
    /// User's full name, used as the email-mode signature ("Best regards,
    /// <name>"). Empty string disables auto-signing.
    pub user_full_name: String,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: FormattingMode::Smart,
            custom_fillers: Vec::new(),
            detect_app_context: true,
            user_full_name: String::new(),
        }
    }
}

/// Runtime context captured at the moment of injection.
#[derive(Debug, Clone, Default)]
pub struct FormattingContext {
    pub app_kind: AppKind,
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn cfg(mode: FormattingMode) -> FormattingConfig {
        FormattingConfig {
            enabled: true,
            mode,
            custom_fillers: Vec::new(),
            detect_app_context: false,
            user_full_name: String::new(),
        }
    }

    fn ctx() -> FormattingContext {
        FormattingContext::default()
    }

    #[test]
    fn raw_passthrough_preserves_exactly() {
        let src = "um, so like, whatever.";
        assert_eq!(format(src, &cfg(FormattingMode::Raw), &ctx()), src);
    }

    #[test]
    fn disabled_preserves_exactly() {
        let src = "um, hello world";
        let mut c = cfg(FormattingMode::Smart);
        c.enabled = false;
        assert_eq!(format(src, &c, &ctx()), src);
    }

    #[test]
    fn smart_strips_fillers_and_caps() {
        let out = format("um hello world", &cfg(FormattingMode::Smart), &ctx());
        assert!(out.starts_with("Hello"));
        assert!(out.ends_with('.'));
        assert!(!out.to_lowercase().contains("um"));
    }

    #[test]
    fn smart_converts_indian_currency() {
        let out = format(
            "please send twenty five thousand rupees",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.contains("₹25,000"), "got: {}", out);
    }

    #[test]
    fn smart_converts_lakh() {
        let out = format("transfer one lakh rupees", &cfg(FormattingMode::Smart), &ctx());
        assert!(out.contains("₹1,00,000"), "got: {}", out);
    }

    #[test]
    fn smart_percent_with_point() {
        let out = format("interest is five point five percent",
            &cfg(FormattingMode::Smart), &ctx());
        assert!(out.contains("5.5%"), "got: {}", out);
    }

    #[test]
    fn smart_dispatches_to_list_on_grocery() {
        let out = format(
            "grocery list milk, bread and eggs",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.contains("- Milk"), "got: {}", out);
        assert!(out.contains("- Bread"), "got: {}", out);
        assert!(out.contains("- Eggs"), "got: {}", out);
    }

    #[test]
    fn smart_dispatches_to_email_on_email_to() {
        let out = format(
            "email to Raj saying please review the proposal",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.starts_with("Hi Raj"), "got: {}", out);
        assert!(out.contains("Best regards"), "got: {}", out);
    }

    #[test]
    fn email_mode_direct() {
        let out = format(
            "thank you for your time",
            &cfg(FormattingMode::Email),
            &ctx(),
        );
        assert!(out.starts_with("Hi"));
        assert!(out.contains("Best regards"));
    }

    #[test]
    fn email_mode_signs_with_user_name() {
        let mut c = cfg(FormattingMode::Email);
        c.user_full_name = "Siddhant Srivastava".to_string();
        let out = format("thank you for your time", &c, &ctx());
        assert!(
            out.contains("Best regards,\nSiddhant Srivastava"),
            "got: {}",
            out
        );
    }

    #[test]
    fn email_mode_unsigned_when_name_blank() {
        // No name set → no trailing newline + name; signature line stays bare.
        let out = format(
            "thank you for your time",
            &cfg(FormattingMode::Email),
            &ctx(),
        );
        assert!(out.trim_end().ends_with("Best regards,"), "got: {}", out);
    }

    #[test]
    fn hinglish_custom_fillers() {
        let mut c = cfg(FormattingMode::Clean);
        c.custom_fillers = vec!["yaar".to_string(), "matlab".to_string()];
        let out = format("matlab bhai yaar let's go", &c, &ctx());
        assert!(!out.to_lowercase().contains("yaar"), "got: {}", out);
        assert!(!out.to_lowercase().contains("matlab"), "got: {}", out);
    }

    #[test]
    fn hinglish_currency_inline() {
        let out = format(
            "bhai send five hundred rupees",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.contains("₹500"), "got: {}", out);
    }

    #[test]
    fn spoken_punctuation_in_smart() {
        let out = format(
            "hello comma how are you question mark",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.contains(','), "got: {}", out);
        assert!(out.trim_end().ends_with('?'), "got: {}", out);
    }

    #[test]
    fn correction_collapses_in_smart() {
        let out = format(
            "meet at the office no wait meet at the cafe",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.to_lowercase().contains("cafe"), "got: {}", out);
        assert!(!out.to_lowercase().contains("office"), "got: {}", out);
    }

    #[test]
    fn email_with_correction_in_body_keeps_envelope() {
        // Regression: "Email to Raj saying X. No wait, Y." used to be
        // mauled by sentence-level correction running BEFORE intent
        // detection, eating "Email to Raj saying X" along with the
        // marker. Intent detection now runs first; corrections inside
        // the body are inline-only.
        let out = format(
            "Email to Raj saying send fifty rupees no wait, send five hundred rupees",
            &cfg(FormattingMode::Smart),
            &ctx(),
        );
        assert!(out.starts_with("Hi Raj"), "envelope lost: {}", out);
        assert!(out.contains("Best regards"), "sign-off lost: {}", out);
        assert!(out.contains("\u{20B9}500"), "replacement currency lost: {}", out);
        // The discarded prior amount (50) should not appear as currency.
        assert!(
            !out.contains("\u{20B9}50."),
            "discarded prior survived: {}",
            out
        );
    }

    #[test]
    fn list_mode_ordinal() {
        let out = format(
            "first buy milk second pick up laundry third call mom",
            &cfg(FormattingMode::List),
            &ctx(),
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "got: {:?}", lines);
        assert!(lines.iter().all(|l| l.starts_with("- ")));
    }

    #[test]
    fn empty_input_is_empty_output() {
        assert_eq!(format("", &cfg(FormattingMode::Smart), &ctx()), "");
        assert_eq!(format("   ", &cfg(FormattingMode::Smart), &ctx()), "   ");
    }

    #[test]
    fn app_context_terminal_forces_raw() {
        let mut c = cfg(FormattingMode::Smart);
        c.detect_app_context = true;
        let mut ctx = FormattingContext::default();
        ctx.app_kind = AppKind::Terminal;
        let src = "um hello world";
        assert_eq!(format(src, &c, &ctx), src);
    }
}
