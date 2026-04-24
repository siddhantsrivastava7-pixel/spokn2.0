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
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)^\s*(?:write|send|compose|draft)?\s*(?:an?\s+)?(?:email|mail)\s+to\s+([a-z][a-z\s]{0,40}?)\s+(?:saying|that|with|about)\b",
        )
        .unwrap()
    });
    static RE_SIMPLE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)^\s*(?:write|send|compose)\s+(?:an?\s+)?(?:email|mail)\s+to\s+([a-z][a-z\s]{0,40}?)\s*[,.:]")
            .unwrap()
    });

    if let Some(caps) = RE.captures(text) {
        let recipient = caps.get(1).map(|m| m.as_str().trim().to_string());
        let body_start = caps.get(0).map(|m| m.end()).unwrap_or(0);
        return Some(Intent::Email { recipient, body_start });
    }
    if let Some(caps) = RE_SIMPLE.captures(text) {
        let recipient = caps.get(1).map(|m| m.as_str().trim().to_string());
        let body_start = caps.get(0).map(|m| m.end()).unwrap_or(0);
        return Some(Intent::Email { recipient, body_start });
    }
    None
}

fn detect_list(text: &str) -> Option<Intent> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(grocery\s+list|shopping\s+list|todo\s+list|to\s+do\s+list|points\s+are|items\s+are)\b[:.]?\s*",
        )
        .unwrap()
    });
    // Ordinal-led lists: "first X second Y third Z" — need at least two ordinals.
    static RE_ORDINAL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\bfirst\b.+?\bsecond\b").unwrap()
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
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(note\s+that|meeting\s+notes|action\s+items)\b[:.]?\s*").unwrap()
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
