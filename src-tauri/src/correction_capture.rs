//! Post-paste correction capture (v0.3.9 confidence-based redesign).
//!
//! After Spokn pastes a transcript into the focused app, this module polls
//! the focused text field via macOS Accessibility API for up to 30s to
//! see if the user edits the text. The new pipeline only learns from
//! CLEAN 1:1 word swaps — multi-word edits, insertions, deletions are
//! ignored entirely (too noisy for safe learning).
//!
//! Confidence model:
//!   - Clean swap detected `X → Y` at the same word position:
//!       * Y is added/incremented in `vocab_entries` (+1)
//!       * X, if it was an existing entry, is decremented (-1)
//!   - No edit at all (final text == pasted text):
//!       * Every active vocab word in the pasted text gets +1
//!         (user implicitly accepted what Whisper produced for them)
//!   - Anything messier: ignored. No learning is better than wrong
//!     learning — the v0.3.1 algorithm taught us this.
//!
//! Entries auto-deactivate at confidence ≤ -3 (user keeps reverting
//! us; clearly wrong) and become active at confidence ≥ 3 (used as
//! Whisper `initial_prompt` bias).

use std::collections::HashSet;
use crate::settings::VocabEntry;

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

// ============================================================
// v0.3.9 confidence-based pipeline
// ============================================================

/// One detected swap: user replaced `removed` with `added` at the
/// same word position. Both tokens are pre-stripped of edge
/// punctuation and case-preserved as the user wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordSwap {
    pub removed: String,
    pub added: String,
}

/// Detect a clean 1:1 word swap between `pasted` and `edited`.
///
/// Returns `Some(swap)` only when:
///   - Both texts have the SAME number of whitespace-separated tokens.
///   - EXACTLY ONE token differs (case- and punctuation-insensitive).
///
/// Anything else — multi-word swap, insertion, deletion, or no
/// change — returns `None`. Better to learn nothing than to learn
/// the wrong thing; the v0.3.1 algorithm taught us that.
pub fn detect_one_to_one_swap(pasted: &str, edited: &str) -> Option<WordSwap> {
    let p_tokens: Vec<&str> = pasted.split_whitespace().collect();
    let e_tokens: Vec<&str> = edited.split_whitespace().collect();
    if p_tokens.len() != e_tokens.len() || p_tokens.is_empty() {
        return None;
    }
    let mut diff_index: Option<usize> = None;
    for (i, (p, e)) in p_tokens.iter().zip(e_tokens.iter()).enumerate() {
        if !tokens_equivalent(p, e) {
            if diff_index.is_some() {
                // Second differing position → not a 1:1 swap.
                return None;
            }
            diff_index = Some(i);
        }
    }
    let i = diff_index?;
    let removed = strip_edge_punct(p_tokens[i]).to_string();
    let added = strip_edge_punct(e_tokens[i]).to_string();
    if removed.is_empty() || added.is_empty() {
        return None;
    }
    // Reject swaps where either side is too short or a stopword —
    // these produce false-positive entries (e.g., "I" → "a").
    let stopwords: HashSet<&str> = STOPWORDS.iter().copied().collect();
    if added.chars().count() < MIN_LEARN_LEN
        || stopwords.contains(added.to_lowercase().as_str())
    {
        return None;
    }
    Some(WordSwap { removed, added })
}

/// Apply a single correction to the vocab entries. Returns a list
/// of (word, became_active) tuples for any state changes worth
/// surfacing.
///
/// The bidirectional confidence rules — straight from the user's
/// spec:
///   1. Clean swap detected → `added` gains confidence (+1, new
///      entries land at confidence=1); `removed`, if it's an
///      existing entry, loses confidence (-1).
///   2. No swap and final text == pasted text → every active vocab
///      word that appeared in `pasted` gains confidence (+1)
///      because the user implicitly accepted them.
///   3. Anything else (multi-word edits) is ignored entirely.
///
/// Entries with confidence ≤ VOCAB_REMOVE_THRESHOLD are dropped.
pub fn apply_correction(
    entries: &mut Vec<VocabEntry>,
    pasted: &str,
    edited: &str,
    now_secs: i64,
) -> Vec<String> {
    use crate::settings::{VOCAB_ACTIVE_THRESHOLD, VOCAB_REMOVE_THRESHOLD};

    let mut newly_active: Vec<String> = Vec::new();

    if pasted == edited {
        // Acceptance path: bump every vocab word that appeared in
        // the pasted output. We only count words the user could
        // plausibly have spotted — short stopwords are skipped.
        let pasted_lower: HashSet<String> = pasted
            .split_whitespace()
            .map(|w| strip_edge_punct(w).to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();

        for entry in entries.iter_mut() {
            if pasted_lower.contains(&entry.word.to_lowercase()) {
                let was_active = entry.confidence >= VOCAB_ACTIVE_THRESHOLD;
                entry.confidence = entry.confidence.saturating_add(1);
                entry.samples_seen = entry.samples_seen.saturating_add(1);
                entry.samples_kept = entry.samples_kept.saturating_add(1);
                entry.last_seen_at = now_secs;
                if !was_active && entry.confidence >= VOCAB_ACTIVE_THRESHOLD {
                    newly_active.push(entry.word.clone());
                }
            }
        }
        // No removal pass on accept-only path (confidence only ↑).
        return newly_active;
    }

    let swap = match detect_one_to_one_swap(pasted, edited) {
        Some(s) => s,
        None => return newly_active, // multi-edit; learn nothing
    };

    let added_lower = swap.added.to_lowercase();
    let removed_lower = swap.removed.to_lowercase();

    // Decrement the removed word IF it was already known. User
    // reverted us — clear signal that the previous learning was
    // wrong (or at least wrong here).
    if let Some(rm_entry) = entries
        .iter_mut()
        .find(|e| e.word.to_lowercase() == removed_lower)
    {
        rm_entry.confidence = rm_entry.confidence.saturating_sub(1);
        rm_entry.samples_seen = rm_entry.samples_seen.saturating_add(1);
        rm_entry.last_seen_at = now_secs;
    }

    // Bump (or insert) the added word.
    if let Some(add_entry) = entries
        .iter_mut()
        .find(|e| e.word.to_lowercase() == added_lower)
    {
        let was_active = add_entry.confidence >= VOCAB_ACTIVE_THRESHOLD;
        add_entry.confidence = add_entry.confidence.saturating_add(1);
        add_entry.samples_seen = add_entry.samples_seen.saturating_add(1);
        add_entry.samples_kept = add_entry.samples_kept.saturating_add(1);
        add_entry.last_seen_at = now_secs;
        if !was_active && add_entry.confidence >= VOCAB_ACTIVE_THRESHOLD {
            newly_active.push(add_entry.word.clone());
        }
    } else {
        entries.push(VocabEntry {
            word: swap.added,
            confidence: 1,
            samples_seen: 1,
            samples_kept: 0,
            first_corrected_at: now_secs,
            last_seen_at: now_secs,
        });
    }

    // Sweep removed-threshold entries — keeps the storage clean and
    // prevents a "cursed" word from sticking around polluting the
    // suggestion surface.
    entries.retain(|e| e.confidence > VOCAB_REMOVE_THRESHOLD);

    newly_active
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
    use super::apply_correction;
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

        // Session ended. Run the v0.3.9 confidence-based learner over
        // the pasted-vs-final pair. apply_correction handles all four
        // cases internally (clean swap, no edit, multi-edit, junk).
        let mut settings = get_settings(&app);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut entries = std::mem::take(&mut settings.vocab_entries);
        let newly_active = apply_correction(&mut entries, &pasted, &last_value, now);
        settings.vocab_entries = entries;
        if !newly_active.is_empty() {
            debug!(
                "correction_capture: {} word(s) reached active threshold: {:?}",
                newly_active.len(),
                newly_active
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

// ---------- v0.3.9 confidence-pipeline tests ----------------------------
#[cfg(test)]
mod v3_tests {
    use super::*;
    use crate::settings::{
        VocabEntry, VOCAB_ACTIVE_THRESHOLD, VOCAB_REMOVE_THRESHOLD,
    };

    fn ent(word: &str, conf: i32) -> VocabEntry {
        VocabEntry {
            word: word.into(),
            confidence: conf,
            samples_seen: conf.max(0) as u32,
            samples_kept: conf.max(0) as u32,
            first_corrected_at: 0,
            last_seen_at: 0,
        }
    }

    #[test]
    fn detect_clean_one_to_one_swap() {
        let s = detect_one_to_one_swap("my name is Raj", "my name is Rajesh");
        assert_eq!(
            s,
            Some(WordSwap {
                removed: "Raj".into(),
                added: "Rajesh".into()
            })
        );
    }

    #[test]
    fn detect_no_swap_when_text_unchanged() {
        assert!(detect_one_to_one_swap("hello world", "hello world").is_none());
    }

    #[test]
    fn detect_rejects_insertion() {
        // "hello world" → "hello big world" — extra word, not a swap
        assert!(detect_one_to_one_swap("hello world", "hello big world").is_none());
    }

    #[test]
    fn detect_rejects_deletion() {
        assert!(detect_one_to_one_swap("hello big world", "hello world").is_none());
    }

    #[test]
    fn detect_rejects_two_word_changes() {
        // Two positions differ → too noisy to learn safely
        assert!(detect_one_to_one_swap(
            "send Raj five dollars",
            "send Rajesh ten dollars"
        )
        .is_none());
    }

    #[test]
    fn detect_rejects_short_or_stopword_added() {
        // "I" is too short and a stopword
        assert!(detect_one_to_one_swap("a b", "a I").is_none());
        // "the" is a stopword
        assert!(detect_one_to_one_swap("hello cat", "hello the").is_none());
    }

    #[test]
    fn detect_punctuation_insensitive() {
        let s = detect_one_to_one_swap("hi Raj.", "hi Rajesh.");
        assert_eq!(s.unwrap().added, "Rajesh");
    }

    #[test]
    fn apply_correction_creates_entry_at_confidence_1() {
        let mut entries: Vec<VocabEntry> = Vec::new();
        let _ = apply_correction(
            &mut entries,
            "my name is Raj",
            "my name is Rajesh",
            100,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].word, "Rajesh");
        assert_eq!(entries[0].confidence, 1);
    }

    #[test]
    fn apply_correction_decrements_removed_existing() {
        // User had "Raj" learned with confidence 5; now they're
        // reverting it ("Raj" → "Rajesh"). Raj should drop.
        let mut entries = vec![ent("Raj", 5)];
        let _ = apply_correction(
            &mut entries,
            "my name is Raj",
            "my name is Rajesh",
            100,
        );
        let raj = entries.iter().find(|e| e.word == "Raj").unwrap();
        assert_eq!(raj.confidence, 4);
        let rajesh = entries.iter().find(|e| e.word == "Rajesh").unwrap();
        assert_eq!(rajesh.confidence, 1);
    }

    #[test]
    fn apply_correction_drops_entry_at_remove_threshold() {
        // Already at -2; one more decrement → -3 → removed.
        let mut entries = vec![ent("BadWord", VOCAB_REMOVE_THRESHOLD + 1)];
        let _ = apply_correction(
            &mut entries,
            "BadWord here",
            "Better here",
            100,
        );
        assert!(!entries.iter().any(|e| e.word == "BadWord"));
    }

    #[test]
    fn apply_correction_promotes_at_active_threshold() {
        // Existing entry at confidence 2; one more increment → 3 = active.
        let mut entries = vec![ent("Rajesh", VOCAB_ACTIVE_THRESHOLD - 1)];
        let activated = apply_correction(
            &mut entries,
            "my name is Raj",
            "my name is Rajesh",
            100,
        );
        assert_eq!(activated, vec!["Rajesh".to_string()]);
        let r = entries.iter().find(|e| e.word == "Rajesh").unwrap();
        assert!(r.confidence >= VOCAB_ACTIVE_THRESHOLD);
    }

    #[test]
    fn apply_correction_acceptance_path_bumps_active_words() {
        // No edit at all. Pasted contains "Rajesh" which is active.
        let mut entries = vec![ent("Rajesh", 4)];
        let _ = apply_correction(
            &mut entries,
            "Hi Rajesh",
            "Hi Rajesh",
            100,
        );
        let r = entries.iter().find(|e| e.word == "Rajesh").unwrap();
        assert_eq!(r.confidence, 5);
        assert_eq!(r.samples_kept, 5);
    }

    #[test]
    fn apply_correction_acceptance_does_not_create_new_entries() {
        // No-op when there's no edit and no existing matching entry.
        let mut entries: Vec<VocabEntry> = Vec::new();
        let _ = apply_correction(&mut entries, "Hello world", "Hello world", 100);
        assert!(entries.is_empty());
    }

    #[test]
    fn apply_correction_multi_edit_learns_nothing() {
        // Two-word swap — too noisy. v0.3.9 explicitly skips.
        let mut entries: Vec<VocabEntry> = Vec::new();
        let _ = apply_correction(
            &mut entries,
            "send Raj five dollars",
            "send Rajesh ten dollars",
            100,
        );
        assert!(entries.is_empty());
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
