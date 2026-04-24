//! Text-snippet expansion. When the user speaks a trigger phrase that
//! matches one of their saved snippets, the phrase is replaced with the
//! snippet's expansion before the text is injected.
//!
//! Design notes:
//! - Case-insensitive, whole-word match (`\b…\b`).
//! - Longer triggers are applied first so "my youtube link" wins over
//!   "youtube".
//! - `replace_all` does not re-scan already-replaced content, so expansions
//!   that happen to contain another trigger don't trigger recursive match.
//! - Runs *after* Smart Formatting and *before* paste so URLs, emails and
//!   signatures inside expansions survive unchanged.

use regex::Regex;

use crate::settings::Snippet;

/// Apply all `snippets` to `text`, returning the expanded result. Counts
/// how many snippets matched at least once, returned as the second tuple
/// element — callers can use it to bump hit counts in storage.
pub fn apply(text: &str, snippets: &[Snippet]) -> (String, Vec<String>) {
    if snippets.is_empty() || text.is_empty() {
        return (text.to_string(), Vec::new());
    }

    // Longest triggers first. Regex alternation is tried in order, so by
    // putting longer triggers earlier we guarantee the longest match wins
    // at any given position. Single-pass replace_all doesn't re-scan the
    // substituted text, so later snippets can't corrupt earlier matches
    // (e.g. "youtube" won't re-match inside an already-expanded URL).
    let mut ordered: Vec<&Snippet> = snippets
        .iter()
        .filter(|s| !s.trigger.trim().is_empty())
        .collect();
    ordered.sort_by(|a, b| b.trigger.trim().len().cmp(&a.trigger.trim().len()));

    // Build a single alternation regex. Escape each trigger and join with |.
    let alternation = ordered
        .iter()
        .map(|s| regex::escape(s.trigger.trim()))
        .collect::<Vec<_>>()
        .join("|");
    let pat = format!(r"(?i)\b(?:{})\b", alternation);
    let re = match Regex::new(&pat) {
        Ok(r) => r,
        Err(_) => return (text.to_string(), Vec::new()),
    };

    let mut hit_ids: Vec<String> = Vec::new();
    let replaced = re
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if let Some(snip) = ordered
                .iter()
                .find(|s| s.trigger.trim().eq_ignore_ascii_case(matched))
            {
                if !hit_ids.contains(&snip.id) {
                    hit_ids.push(snip.id.clone());
                }
                return snip.expansion.clone();
            }
            matched.to_string()
        })
        .to_string();

    (replaced, hit_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(id: &str, trigger: &str, expansion: &str) -> Snippet {
        Snippet {
            id: id.to_string(),
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
            hits: 0,
            created_at: 0,
        }
    }

    #[test]
    fn single_word_expansion() {
        let s = vec![snippet("1", "youtube", "youtube.com/encryptictv")];
        let (out, hits) = apply("check out my youtube channel", &s);
        assert_eq!(out, "check out my youtube.com/encryptictv channel");
        assert_eq!(hits, vec!["1".to_string()]);
    }

    #[test]
    fn multi_word_trigger() {
        let s = vec![snippet(
            "1",
            "my youtube link",
            "youtube.com/encryptictv",
        )];
        let (out, _) = apply("here's my youtube link for reference", &s);
        assert_eq!(out, "here's youtube.com/encryptictv for reference");
    }

    #[test]
    fn longer_trigger_wins() {
        let snips = vec![
            snippet("s", "youtube", "yt.com"),
            snippet("l", "my youtube link", "youtube.com/encryptictv"),
        ];
        let (out, hits) = apply("send my youtube link please", &snips);
        assert!(out.contains("youtube.com/encryptictv"));
        assert!(!out.contains("yt.com"));
        assert_eq!(hits, vec!["l".to_string()]);
    }

    #[test]
    fn case_insensitive() {
        let s = vec![snippet("1", "YouTube", "yt.com")];
        let (out, _) = apply("visit youtube now", &s);
        assert!(out.contains("yt.com"));
    }

    #[test]
    fn word_boundary_respected() {
        // "cat" shouldn't match inside "category".
        let s = vec![snippet("1", "cat", "🐱")];
        let (out, hits) = apply("this category is nice", &s);
        assert_eq!(out, "this category is nice");
        assert!(hits.is_empty());
    }

    #[test]
    fn expansion_containing_trigger_does_not_loop() {
        // Trigger "y" expands to "say y hi". If we re-scanned, we'd loop.
        let s = vec![snippet("1", "y", "say y hi")];
        let (out, _) = apply("hey y there", &s);
        // regex::replace_all does not re-scan the substituted text
        assert_eq!(out, "hey say y hi there");
    }

    #[test]
    fn empty_trigger_ignored() {
        let s = vec![snippet("1", "", "nothing")];
        let (out, hits) = apply("hello world", &s);
        assert_eq!(out, "hello world");
        assert!(hits.is_empty());
    }

    #[test]
    fn multiple_triggers_one_utterance() {
        let snips = vec![
            snippet("yt", "youtube", "yt.com/me"),
            snippet("tw", "twitter", "x.com/me"),
        ];
        let (out, hits) = apply("my youtube and twitter links", &snips);
        assert!(out.contains("yt.com/me"));
        assert!(out.contains("x.com/me"));
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn empty_snippets_returns_unchanged() {
        let (out, hits) = apply("hello world", &[]);
        assert_eq!(out, "hello world");
        assert!(hits.is_empty());
    }

    #[test]
    fn trigger_with_trailing_whitespace_trimmed() {
        let s = vec![snippet("1", "  youtube  ", "yt.com")];
        let (out, _) = apply("my youtube page", &s);
        assert!(out.contains("yt.com"));
    }
}
