//! Intent detection for Smart mode re-dispatch.
//!
//! Given a raw-ish utterance, classify whether the user is dictating an
//! email, a list, or a note. Detection is intentionally shallow and
//! keyword-driven — anything ambiguous maps to [`Intent::None`].

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// No specific intent detected — caller should use the current mode.
    None,
    /// User wants an email. `recipient` is a best-effort name extraction.
    Email { recipient: Option<String>, body_start: usize },
    /// User wants a list (grocery / todo / shopping / enumerated points).
    List { body_start: usize },
    /// User wants a note (headed / bullet-formatted non-email).
    Note { body_start: usize },
}

pub fn detect(text: &str) -> Intent {
    if let Some(intent) = detect_email(text) {
        return intent;
    }
    if let Some(intent) = detect_list(text) {
        return intent;
    }
    if let Some(intent) = detect_note(text) {
        return intent;
    }
    Intent::None
}

fn detect_email(text: &str) -> Option<Intent> {
    // Verb-led with body marker. Matches:
    //   "Write an email to Raj saying ..."
    //   "Send a mail to Priya about ..."
    //   "Compose email to Rohit, ..."
    //   "Email to Raj saying ..."
    //   "Hey can you write an email to Raj saying ..."  (drops the ^ anchor
    //   so a small preamble is fine — humans don't always start clean)
    static RE_TO_BODY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(?:write|send|compose|draft)?\s*(?:an?\s+)?(?:email|mail)\s+to\s+([a-z][a-z\s'\-]{0,40}?)\s*(?:saying|that|with|about|[,:\u{2014}\u{2013}])\s+",
        )
        .unwrap()
    });
    // Verb required, recipient WITHOUT the explicit "to":
    //   "Email Raj saying ..."
    //   "Email Raj about ..."
    //   "Send Priya an email about ..."   (handled by RE_TO_BODY because it
    //   has "an email")
    static RE_NAME_BODY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(?:email|mail)\s+([a-z][a-z\s'\-]{0,40}?)\s*(?:saying|that|with|about|[,:\u{2014}\u{2013}])\s+",
        )
        .unwrap()
    });
    // No recipient at all:
    //   "Write an email about the launch"
    //   "Compose an email saying I'll be late"
    static RE_NO_RECIPIENT: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(?:write|send|compose|draft)\s+(?:an?\s+)?(?:email|mail)\s+(?:saying|about|that|with|[,:\u{2014}\u{2013}])\s+",
        )
        .unwrap()
    });

    for re in [&*RE_TO_BODY, &*RE_NAME_BODY, &*RE_NO_RECIPIENT] {
        if let Some(caps) = re.captures(text) {
            let recipient = caps.get(1).map(|m| m.as_str().trim().to_string());
            let body_start = caps.get(0).map(|m| m.end()).unwrap_or(0);
            return Some(Intent::Email { recipient, body_start });
        }
    }
    None
}

fn detect_list(text: &str) -> Option<Intent> {
    // List intent triggers ONLY at the start of the utterance (±leading ws).
    // Mid-sentence "grocery list" is likely conversational, not an intent.
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)^\s*(grocery\s+list|shopping\s+list|todo\s+list|to\s+do\s+list|points\s+are|items\s+are)\b[:.]?\s*",
        )
        .unwrap()
    });
    // Ordinal-led lists: utterance must BEGIN with "first" (not contain it).
    static RE_ORDINAL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)^\s*first\b.+?\bsecond\b").unwrap()
    });

    if let Some(caps) = RE.captures(text) {
        let body_start = caps.get(0).map(|m| m.end()).unwrap_or(0);
        return Some(Intent::List { body_start });
    }
    if RE_ORDINAL.is_match(text) {
        return Some(Intent::List { body_start: 0 });
    }
    None
}

fn detect_note(text: &str) -> Option<Intent> {
    // Anchored to utterance start. Previously a mid-sentence "action items"
    // would swallow everything before it — breaking "…new paragraph. Action
    // items are next." utterances.
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)^\s*(note\s+that|meeting\s+notes|action\s+items)\b[:.]?\s*").unwrap()
    });
    RE.captures(text).map(|caps| Intent::Note {
        body_start: caps.get(0).map(|m| m.end()).unwrap_or(0),
    })
}

/// Split a list body into items. Recognises explicit delimiters first, then
/// falls back to ordinal markers, then to commas.
pub fn split_list_items(body: &str) -> Vec<String> {
    // 1. Ordinal split: "first X second Y third Z".
    let ord = split_on_ordinals(body);
    if ord.len() >= 2 {
        return ord;
    }
    // 2. "and"/comma split — classic dictation: "milk, bread and eggs".
    let parts: Vec<String> = body
        .split(|c: char| c == ',' || c == '\n')
        .flat_map(|chunk| {
            chunk.split(" and ").map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect();
    parts
}

fn split_on_ordinals(body: &str) -> Vec<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth)\b[,:\s]+")
            .unwrap()
    });
    let mut items = Vec::new();
    let mut last = 0usize;
    let matches: Vec<_> = RE.find_iter(body).collect();
    if matches.is_empty() {
        return items;
    }
    for (i, m) in matches.iter().enumerate() {
        if i == 0 && m.start() > 0 {
            let pre = body[..m.start()].trim();
            if !pre.is_empty() {
                items.push(pre.to_string());
            }
        }
        let next_start = matches.get(i + 1).map(|nm| nm.start()).unwrap_or(body.len());
        let item = body[m.end()..next_start].trim();
        if !item.is_empty() {
            items.push(item.to_string());
        }
        last = next_start;
    }
    if last < body.len() {
        // Tail already captured above.
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_email_to_raj() {
        let intent = detect("email to Raj saying meeting at 5");
        match intent {
            Intent::Email { recipient, .. } => {
                assert_eq!(recipient.as_deref().map(|s| s.to_lowercase()), Some("raj".to_string()));
            }
            _ => panic!("expected email intent"),
        }
    }

    #[test]
    fn detects_write_email() {
        match detect("write an email to Priya, thanks for your help") {
            Intent::Email { recipient, .. } => assert!(recipient.is_some()),
            other => panic!("expected email, got {:?}", other),
        }
    }

    #[test]
    fn detects_email_without_to() {
        // "Email Raj about meeting" — no "to" keyword.
        match detect("email Raj about the meeting tomorrow") {
            Intent::Email { recipient, .. } => {
                assert_eq!(recipient.as_deref(), Some("Raj"));
            }
            other => panic!("expected email, got {:?}", other),
        }
    }

    #[test]
    fn detects_email_no_recipient() {
        // "Compose an email about the launch" — no recipient at all.
        match detect("compose an email about the upcoming launch") {
            Intent::Email { recipient, .. } => {
                assert!(recipient.is_none() || recipient.as_deref() == Some(""));
            }
            other => panic!("expected email, got {:?}", other),
        }
    }

    #[test]
    fn detects_email_with_preamble() {
        // Common conversational lead-in.
        match detect("hey, write an email to Raj saying we're delayed") {
            Intent::Email { recipient, .. } => {
                assert_eq!(recipient.as_deref(), Some("Raj"));
            }
            other => panic!("expected email, got {:?}", other),
        }
    }

    #[test]
    fn detects_email_with_colon() {
        // Colon as body marker, not "saying".
        match detect("email to Priya: please review the proposal") {
            Intent::Email { recipient, .. } => {
                assert_eq!(recipient.as_deref(), Some("Priya"));
            }
            other => panic!("expected email, got {:?}", other),
        }
    }

    #[test]
    fn detects_grocery_list() {
        match detect("grocery list: milk, bread and eggs") {
            Intent::List { .. } => {}
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn detects_ordinal_list() {
        match detect("first buy milk second pick up laundry") {
            Intent::List { .. } => {}
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn detects_action_items_note() {
        match detect("action items: follow up with team") {
            Intent::Note { .. } => {}
            other => panic!("expected note, got {:?}", other),
        }
    }

    #[test]
    fn no_intent_on_plain_text() {
        assert_eq!(detect("just a regular sentence"), Intent::None);
    }

    #[test]
    fn note_intent_not_greedy_mid_utterance() {
        // Previously "action items" anywhere triggered Note intent and
        // swallowed everything before it. Must anchor to utterance start.
        assert_eq!(
            detect("Meeting went well. We agreed on Q2 targets. Action items are next."),
            Intent::None
        );
    }

    #[test]
    fn list_intent_requires_utterance_start() {
        // "grocery list" mid-sentence should not trigger list mode.
        assert_eq!(detect("I put the grocery list on the counter"), Intent::None);
    }

    #[test]
    fn ordinal_list_requires_leading_first() {
        // "first" mid-sentence shouldn't trigger ordinal list mode.
        assert_eq!(
            detect("we won the first prize, second place, and third try"),
            Intent::None
        );
    }

    #[test]
    fn splits_list_on_commas_and_and() {
        let items = split_list_items("milk, bread and eggs");
        assert_eq!(items, vec!["milk", "bread", "eggs"]);
    }

    #[test]
    fn splits_ordinal_list() {
        let items = split_list_items("first buy milk second pick up laundry third call mom");
        assert_eq!(items.len(), 3);
    }
}
