//! Active-app detection and mode override policy.
//!
//! Platform detection is a deliberate stub in this first pass — cross-
//! platform active-window APIs add non-trivial dependencies and varied
//! failure modes. Today every platform returns [`AppKind::Unknown`], which
//! causes the formatter to fall back to the user-selected mode. The shape
//! of the enum and the override policy are already final so a follow-up
//! platform module can plug detection in without touching callers.

use super::FormattingMode;

/// Coarse bucket describing the currently focused application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppKind {
    #[default]
    Unknown,
    Email,
    Messaging,
    Chat,   // Slack / Teams — work messaging
    Notes,  // Notes / Notion / Docs
    Search, // Browser URL bar / search field
    Terminal,
    Code,
    Other(String),
}

/// Detect the currently focused app. Returns [`AppKind::Unknown`] on all
/// platforms in this first pass.
pub fn detect_app_kind() -> AppKind {
    AppKind::Unknown
}

/// Given a user-selected mode and the active app, return the mode the
/// pipeline should actually run. Safety-critical overrides (like Terminal
/// forcing Raw) always win; otherwise the user's selection is preserved.
pub fn resolve_mode(user_mode: FormattingMode, app: &AppKind) -> FormattingMode {
    match app {
        // Safety-first: never inject punctuation or auto-capitalise into
        // shells or editors. User can still opt in by typing their request
        // through a dedicated mode.
        AppKind::Terminal | AppKind::Code => FormattingMode::Raw,
        AppKind::Search => FormattingMode::Raw,
        AppKind::Email if user_mode != FormattingMode::Raw => FormattingMode::Email,
        AppKind::Messaging if user_mode == FormattingMode::Smart => FormattingMode::Message,
        AppKind::Chat if user_mode == FormattingMode::Smart => FormattingMode::Message,
        AppKind::Notes if user_mode == FormattingMode::Smart => FormattingMode::Clean,
        _ => user_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting::FormattingMode;

    #[test]
    fn unknown_preserves_user_mode() {
        assert_eq!(resolve_mode(FormattingMode::Smart, &AppKind::Unknown), FormattingMode::Smart);
        assert_eq!(resolve_mode(FormattingMode::Clean, &AppKind::Unknown), FormattingMode::Clean);
    }

    #[test]
    fn terminal_forces_raw() {
        assert_eq!(resolve_mode(FormattingMode::Smart, &AppKind::Terminal), FormattingMode::Raw);
    }

    #[test]
    fn email_app_upgrades_to_email_mode() {
        assert_eq!(resolve_mode(FormattingMode::Smart, &AppKind::Email), FormattingMode::Email);
    }

    #[test]
    fn email_app_respects_explicit_raw() {
        assert_eq!(resolve_mode(FormattingMode::Raw, &AppKind::Email), FormattingMode::Raw);
    }

    #[test]
    fn messaging_drops_smart_to_message() {
        assert_eq!(resolve_mode(FormattingMode::Smart, &AppKind::Messaging), FormattingMode::Message);
    }

    #[test]
    fn detect_returns_unknown_in_first_pass() {
        assert_eq!(detect_app_kind(), AppKind::Unknown);
    }
}
