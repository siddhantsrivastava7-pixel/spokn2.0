//! Post-paste correction capture.
//!
//! After Spokn pastes a transcript into the focused app, this module polls
//! the focused text field via macOS Accessibility API for up to 30s to see
//! if the user edits the text. When a diff is detected, the substituted
//! word(s) are appended to `settings.custom_words` so Whisper's decoder
//! biases toward them on future transcriptions — the vocabulary grows as
//! the user naturally corrects mistakes.
//!
//! Pure diff logic lives here cross-platform. Platform-specific text
//! reading currently supports macOS only (the Accessibility permission
//! is already required for paste to work). Windows/Linux support is a
//! later pass.

use std::collections::HashSet;

/// Words that are too common to learn safely. Learning `from → "the"` would
/// cause catastrophic substitutions on future transcripts.
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from", "have", "he", "i",
    "in", "is", "it", "no", "not", "of", "on", "or", "she", "so", "that", "the", "they", "this",
    "to", "was", "we", "were", "what", "will", "with", "you", "your",
];

/// Minimum word length for learning. Short tokens are usually typos or
/// particles that produce more false positives than value.
const MIN_LEARN_LEN: usize = 3;

/// Hard cap on the custom_words list. Prevents unbounded growth.
pub const CUSTOM_WORDS_CAP: usize = 500;

/// Given the text Spokn originally pasted and the text the user left in
/// the field after editing, extract the RHS tokens that should be learned.
///
/// Uses prefix/suffix token alignment: finds the longest common prefix and
/// suffix of the two token streams, and the middle differing span is the
/// substitution. Returns the edited-side tokens from that span, filtered
/// through safety guards (non-empty, non-stopword, min length).
pub fn extract_substitutions(original: &str, edited: &str) -> Vec<String> {
    let orig_tokens: Vec<&str> = original.split_whitespace().collect();
    let edit_tokens: Vec<&str> = edited.split_whitespace().collect();

    // Trivial: no change, or user deleted everything, etc.
    if orig_tokens == edit_tokens || edit_tokens.is_empty() {
        return Vec::new();
    }

    // Longest common prefix.
    let prefix = orig_tokens
        .iter()
        .zip(edit_tokens.iter())
        .take_while(|(a, b)| tokens_equivalent(a, b))
        .count();

    let orig_rem = &orig_tokens[prefix..];
    let edit_rem = &edit_tokens[prefix..];

    // Longest common suffix (computed on the remaining slices).
    let suffix = orig_rem
        .iter()
        .rev()
        .zip(edit_rem.iter().rev())
        .take_while(|(a, b)| tokens_equivalent(a, b))
        .count();

    let orig_mid = &orig_rem[..orig_rem.len().saturating_sub(suffix)];
    let edit_mid = &edit_rem[..edit_rem.len().saturating_sub(suffix)];

    // Only learn actual substitutions. Pure inserts (orig_mid empty) or
    // pure deletes (edit_mid empty) aren't safe to turn into word hints.
    if orig_mid.is_empty() || edit_mid.is_empty() {
        return Vec::new();
    }

    let stopwords: HashSet<&str> = STOPWORDS.iter().copied().collect();

    edit_mid
        .iter()
        .filter_map(|w| normalize_for_learning(w, &stopwords))
        .collect()
}

/// Case-insensitive + punctuation-insensitive token equality. "Rosary" and
/// "rosary," are treated as equal for alignment purposes, so the diff
/// engine doesn't fire on punctuation-only changes.
fn tokens_equivalent(a: &str, b: &str) -> bool {
    strip_edge_punct(a).eq_ignore_ascii_case(strip_edge_punct(b))
}

fn strip_edge_punct(s: &str) -> &str {
    s.trim_matches(|c: char| !c.is_alphanumeric())
}

fn normalize_for_learning(word: &str, stopwords: &HashSet<&str>) -> Option<String> {
    let stripped = strip_edge_punct(word);
    if stripped.chars().count() < MIN_LEARN_LEN {
        return None;
    }
    let lower = stripped.to_lowercase();
    if stopwords.contains(lower.as_str()) {
        return None;
    }
    // Keep the original casing — proper nouns like "Anthropic" matter.
    Some(stripped.to_string())
}

/// Merge new learnings into an existing `custom_words` vec. Returns the
/// number of genuinely new entries added (dedupe is case-insensitive).
/// Enforces [`CUSTOM_WORDS_CAP`] by evicting oldest entries.
///
/// Used for direct promotions and tests. The candidate-aware path lives in
/// [`merge_into_candidates`].
pub fn merge_learnings(existing: &mut Vec<String>, learned: Vec<String>) -> usize {
    let existing_lower: HashSet<String> =
        existing.iter().map(|w| w.to_lowercase()).collect();
    let mut added = 0;
    let mut seen_this_batch: HashSet<String> = HashSet::new();
    for word in learned {
        let lower = word.to_lowercase();
        if existing_lower.contains(&lower) || seen_this_batch.contains(&lower) {
            continue;
        }
        seen_this_batch.insert(lower);
        existing.push(word);
        added += 1;
    }
    if existing.len() > CUSTOM_WORDS_CAP {
        let excess = existing.len() - CUSTOM_WORDS_CAP;
        existing.drain(..excess);
    }
    added
}

/// Confidence-aware merge: bump hits on existing candidates, add new ones,
/// and promote any candidate that has crossed the configured threshold into
/// the live `custom_words` list. Returns the list of newly-promoted words
/// so callers can log/display them.
///
/// `now_secs` is a unix timestamp passed in for testability.
pub fn merge_into_candidates(
    candidates: &mut Vec<crate::settings::VocabCandidate>,
    custom_words: &mut Vec<String>,
    learned: Vec<String>,
    now_secs: i64,
) -> Vec<String> {
    use crate::settings::{VocabCandidate, VOCAB_PROMOTE_THRESHOLD};

    let mut newly_promoted = Vec::new();
    let mut seen_this_batch: HashSet<String> = HashSet::new();

    for word in learned {
        let lower = word.to_lowercase();
        if seen_this_batch.contains(&lower) {
            continue;
        }
        seen_this_batch.insert(lower.clone());

        // Find an existing candidate (case-insensitive on the stored word).
        let existing = candidates
            .iter_mut()
            .find(|c| c.word.to_lowercase() == lower);

        if let Some(c) = existing {
            c.hits = c.hits.saturating_add(1);
            c.last_seen = now_secs;
            // Promote if threshold reached and not already promoted.
            if !c.promoted && c.hits >= VOCAB_PROMOTE_THRESHOLD {
                c.promoted = true;
                if !custom_words
                    .iter()
                    .any(|w| w.to_lowercase() == lower)
                {
                    custom_words.push(c.word.clone());
                    newly_promoted.push(c.word.clone());
                }
            }
        } else {
            candidates.push(VocabCandidate {
                word,
                hits: 1,
                first_seen: now_secs,
                last_seen: now_secs,
                promoted: false,
            });
        }
    }

    // Soft cap on candidates to prevent unbounded growth — drop oldest
    // unpromoted ones first.
    const CANDIDATE_CAP: usize = 1000;
    if candidates.len() > CANDIDATE_CAP {
        // Keep promoted entries even if old; sort everything else by
        // last_seen descending and drop the tail.
        candidates.sort_by(|a, b| {
            // Promoted first, then most-recently-seen first.
            b.promoted
                .cmp(&a.promoted)
                .then_with(|| b.last_seen.cmp(&a.last_seen))
        });
        candidates.truncate(CANDIDATE_CAP);
    }

    // Enforce hard cap on the live word list (pre-existing behaviour).
    if custom_words.len() > CUSTOM_WORDS_CAP {
        let excess = custom_words.len() - CUSTOM_WORDS_CAP;
        custom_words.drain(..excess);
    }

    newly_promoted
}

// ---------- macOS Accessibility polling ---------------------------------

/// Start a capture session in a dedicated thread. After Spokn pastes
/// `pasted`, this polls the focused text field every 2s for up to 30s.
/// If the field's contents diverge from `pasted`, the extracted
/// substitutions are persisted into `settings.custom_words`.
///
/// No-op on non-macOS platforms in this pass.
pub fn start_capture_session(app: tauri::AppHandle, pasted: String) {
    // v0.3.2: gate behind a settings toggle (default OFF). The previous
    // algorithm was over-promoting non-correction tokens (mangled
    // punctuation, URLs, every transcribed word) which poisoned
    // `custom_words`. The redesigned confidence-based learner ships
    // in v0.3.3; until then this path is a no-op unless the user
    // explicitly opts in via Advanced settings.
    let settings = crate::settings::get_settings(&app);
    if !settings.auto_vocab_learning_enabled {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        macos::spawn_session(app, pasted);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, pasted);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{extract_substitutions, merge_into_candidates};
    use crate::settings::{get_settings, write_settings};
    use log::debug;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tauri::AppHandle;

    const POLL_INTERVAL: Duration = Duration::from_millis(2000);
    const MAX_DURATION: Duration = Duration::from_secs(30);
    /// Stop early if the field has been stable and matches `pasted` for
    /// this long — the user probably moved on without editing.
    const STABLE_EXIT: Duration = Duration::from_secs(10);

    pub fn spawn_session(app: AppHandle, pasted: String) {
        thread::spawn(move || {
            run_session(app, pasted);
        });
    }

    fn run_session(app: AppHandle, pasted: String) {
        let started = Instant::now();
        // Give the OS a moment to settle the focused element after paste.
        thread::sleep(Duration::from_millis(500));

        // Capture identity of the initially-focused element. If it changes
        // mid-session we bail — the user navigated away.
        let start_id = focused_element_identity();
        let mut last_value = pasted.clone();
        let mut stable_since = Instant::now();

        while started.elapsed() < MAX_DURATION {
            thread::sleep(POLL_INTERVAL);

            // Focus changed → user moved on. Stop polling.
            if focused_element_identity() != start_id {
                debug!("correction_capture: focus changed, stopping session");
                break;
            }

            let current = match read_focused_value() {
                Some(v) => v,
                None => continue, // transient AX failure; retry next tick
            };

            if current != last_value {
                last_value = current;
                stable_since = Instant::now();
            } else if stable_since.elapsed() >= STABLE_EXIT && last_value == pasted {
                // Value equals what we pasted and it has been stable →
                // user made no edits. Stop early.
                debug!("correction_capture: no edits detected, stopping early");
                return;
            }
        }

        // Session ended. If the final buffer differs from what we pasted,
        // extract substitutions and learn.
        if last_value == pasted {
            return;
        }
        let learned = extract_substitutions(&pasted, &last_value);
        if learned.is_empty() {
            return;
        }
        let mut settings = get_settings(&app);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        // Update candidate hits; promote to custom_words once threshold met.
        // The vec mutations need to happen on a single struct, so we work on
        // separate temporary copies and then assign back.
        let mut candidates = std::mem::take(&mut settings.vocab_candidates);
        let mut custom_words = std::mem::take(&mut settings.custom_words);
        let promoted = merge_into_candidates(
            &mut candidates,
            &mut custom_words,
            learned.clone(),
            now,
        );
        settings.vocab_candidates = candidates;
        settings.custom_words = custom_words;
        if !promoted.is_empty() {
            debug!(
                "correction_capture: promoted {} candidate(s) to vocab: {:?}",
                promoted.len(),
                promoted
            );
        } else {
            debug!(
                "correction_capture: candidate(s) recorded but below promotion threshold: {:?}",
                learned
            );
        }
        write_settings(&app, settings);
    }

    // ---------- raw AX FFI ------------------------------------------------

    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};

    #[allow(non_camel_case_types)]
    type AXUIElementRef = CFTypeRef;
    #[allow(non_camel_case_types)]
    type AXError = i32;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
    }

    /// Read the string value of the currently-focused UI element. Returns
    /// None on any AX error or if the focused element isn't a text field.
    fn read_focused_value() -> Option<String> {
        unsafe {
            let sys_wide = AXUIElementCreateSystemWide();
            if sys_wide.is_null() {
                return None;
            }
            let focused_attr = CFString::from_static_string("AXFocusedUIElement");
            let mut focused: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                sys_wide,
                focused_attr.as_concrete_TypeRef(),
                &mut focused,
            );
            CFRelease(sys_wide);
            if err != 0 || focused.is_null() {
                return None;
            }

            let value_attr = CFString::from_static_string("AXValue");
            let mut value_ref: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                focused,
                value_attr.as_concrete_TypeRef(),
                &mut value_ref,
            );
            CFRelease(focused);
            if err != 0 || value_ref.is_null() {
                return None;
            }

            // Try to interpret as CFString. If the focused element's value
            // isn't a string (slider value etc.), bail gracefully.
            let type_id = core_foundation::base::CFGetTypeID(value_ref);
            if type_id != CFString::type_id() {
                CFRelease(value_ref);
                return None;
            }
            let cf_str = CFString::wrap_under_create_rule(value_ref as CFStringRef);
            Some(cf_str.to_string())
        }
    }

    /// Opaque identity of the focused element — used only to detect focus
    /// changes during a session. We can't compare AXUIElementRefs directly
    /// across polls reliably, so we hash a tuple of role + identifier-ish
    /// attributes as a proxy.
    fn focused_element_identity() -> Option<String> {
        unsafe {
            let sys_wide = AXUIElementCreateSystemWide();
            if sys_wide.is_null() {
                return None;
            }
            let focused_attr = CFString::from_static_string("AXFocusedUIElement");
            let mut focused: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                sys_wide,
                focused_attr.as_concrete_TypeRef(),
                &mut focused,
            );
            CFRelease(sys_wide);
            if err != 0 || focused.is_null() {
                return None;
            }
            // Use AXRole + AXIdentifier (if available) as a coarse identity.
            let id = read_string_attr(focused, "AXRole")
                .into_iter()
                .chain(read_string_attr(focused, "AXIdentifier"))
                .collect::<Vec<_>>()
                .join("|");
            CFRelease(focused);
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        }
    }

    unsafe fn read_string_attr(element: CFTypeRef, attr_name: &str) -> Option<String> {
        let attr = CFString::new(attr_name);
        let mut value: CFTypeRef = std::ptr::null();
        let err =
            AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value);
        if err != 0 || value.is_null() {
            return None;
        }
        if core_foundation::base::CFGetTypeID(value) != CFString::type_id() {
            CFRelease(value);
            return None;
        }
        let cf = CFString::wrap_under_create_rule(value as CFStringRef);
        Some(cf.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_returns_empty() {
        assert!(extract_substitutions("hello world", "hello world").is_empty());
    }

    #[test]
    fn single_word_substitution() {
        let learned = extract_substitutions(
            "I went to the rosary list today",
            "I went to the grocery list today",
        );
        assert_eq!(learned, vec!["grocery"]);
    }

    #[test]
    fn multi_word_substitution() {
        let learned = extract_substitutions(
            "please send five hundred rupees",
            "please send 500 rupees",
        );
        // "500" is 3 chars, qualifies. No stopword.
        assert_eq!(learned, vec!["500"]);
    }

    #[test]
    fn stopword_filtered() {
        // User changed "cat" to "the" — learning "the" would be disaster.
        let learned = extract_substitutions("see the cat sit", "see the the sit");
        assert!(learned.is_empty());
    }

    #[test]
    fn short_word_filtered() {
        let learned = extract_substitutions("the man ran", "the boy ran");
        // "boy" is 3 chars, qualifies.
        assert_eq!(learned, vec!["boy"]);
    }

    #[test]
    fn two_char_word_filtered() {
        let learned = extract_substitutions("hi mom", "hi ma");
        assert!(learned.is_empty());
    }

    #[test]
    fn pure_insertion_not_learned() {
        let learned = extract_substitutions("hello world", "hello brave world");
        // Insertion only — no original token to replace. Skip.
        assert!(learned.is_empty());
    }

    #[test]
    fn pure_deletion_not_learned() {
        let learned = extract_substitutions("hello brave world", "hello world");
        assert!(learned.is_empty());
    }

    #[test]
    fn punctuation_only_change_ignored() {
        let learned = extract_substitutions("hello world", "hello world.");
        assert!(learned.is_empty());
    }

    #[test]
    fn case_preserved_in_learned() {
        let learned = extract_substitutions(
            "meeting with antropic team",
            "meeting with Anthropic team",
        );
        assert_eq!(learned, vec!["Anthropic"]);
    }

    #[test]
    fn merge_dedupes_case_insensitively() {
        let mut existing = vec!["Anthropic".to_string(), "Tauri".to_string()];
        let added = merge_learnings(
            &mut existing,
            vec!["anthropic".to_string(), "Rust".to_string()],
        );
        assert_eq!(added, 1);
        assert_eq!(existing.len(), 3);
        assert!(existing.contains(&"Rust".to_string()));
    }

    #[test]
    fn merge_enforces_cap() {
        let mut existing: Vec<String> = (0..CUSTOM_WORDS_CAP)
            .map(|i| format!("w{i}"))
            .collect();
        let added = merge_learnings(&mut existing, vec!["new_word".to_string()]);
        assert_eq!(added, 1);
        assert_eq!(existing.len(), CUSTOM_WORDS_CAP);
        assert!(existing.contains(&"new_word".to_string()));
        // Oldest (w0) should have been evicted
        assert!(!existing.contains(&"w0".to_string()));
    }

    // ---- candidate-pool tests ----

    use crate::settings::VocabCandidate;

    #[test]
    fn first_correction_only_seeds_candidate_not_custom_words() {
        let mut candidates: Vec<VocabCandidate> = Vec::new();
        let mut custom_words: Vec<String> = Vec::new();
        let promoted = merge_into_candidates(
            &mut candidates,
            &mut custom_words,
            vec!["Anthropic".to_string()],
            100,
        );
        assert!(promoted.is_empty(), "should not promote on first hit");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].hits, 1);
        assert!(!candidates[0].promoted);
        assert!(custom_words.is_empty());
    }

    #[test]
    fn three_corrections_promote_to_custom_words() {
        let mut candidates: Vec<VocabCandidate> = Vec::new();
        let mut custom_words: Vec<String> = Vec::new();
        for i in 0..3 {
            let promoted = merge_into_candidates(
                &mut candidates,
                &mut custom_words,
                vec!["Anthropic".to_string()],
                100 + i,
            );
            if i < 2 {
                assert!(promoted.is_empty(), "iter {i} should not promote");
            } else {
                assert_eq!(promoted, vec!["Anthropic".to_string()]);
            }
        }
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].promoted);
        assert_eq!(candidates[0].hits, 3);
        assert!(custom_words.contains(&"Anthropic".to_string()));
    }

    #[test]
    fn case_insensitive_candidate_match() {
        let mut candidates: Vec<VocabCandidate> = Vec::new();
        let mut custom_words: Vec<String> = Vec::new();
        merge_into_candidates(
            &mut candidates,
            &mut custom_words,
            vec!["Anthropic".to_string()],
            100,
        );
        merge_into_candidates(
            &mut candidates,
            &mut custom_words,
            vec!["anthropic".to_string()],
            101,
        );
        assert_eq!(candidates.len(), 1, "should dedupe case-insensitively");
        assert_eq!(candidates[0].hits, 2);
    }

    #[test]
    fn already_promoted_word_stays_in_custom_words() {
        // A word that was promoted earlier shouldn't get re-added on a
        // subsequent capture (no duplicates in custom_words).
        let mut candidates = vec![VocabCandidate {
            word: "Anthropic".to_string(),
            hits: 5,
            first_seen: 0,
            last_seen: 100,
            promoted: true,
        }];
        let mut custom_words = vec!["Anthropic".to_string()];
        let promoted = merge_into_candidates(
            &mut candidates,
            &mut custom_words,
            vec!["Anthropic".to_string()],
            200,
        );
        assert!(promoted.is_empty());
        assert_eq!(candidates[0].hits, 6);
        assert_eq!(
            custom_words.iter().filter(|w| w.as_str() == "Anthropic").count(),
            1,
            "no duplicate"
        );
    }
}
