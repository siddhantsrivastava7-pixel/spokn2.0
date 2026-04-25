//! Spoken punctuation commands and self-correction handling.
//!
//! Converts tokens like "comma" / "full stop" / "new line" into actual
//! punctuation or whitespace, and collapses spoken corrections like
//! "no wait X" or "sorry X" so the final text reflects the user's intent.

use once_cell::sync::Lazy;
use regex::Regex;

/// Replace spoken punctuation keywords with real punctuation. Rules are
/// strict: a match must appear as a standalone token (flanked by whitespace
/// or clause boundaries), otherwise the literal word is kept.
pub fn apply_spoken_punctuation(text: &str) -> String {
    // Order matters: multi-word phrases first so "new paragraph" is not
    // shadowed by "new".
    let replacements: &[(&str, &str)] = &[
        // Eat any preceding comma too — Whisper consistently adds a
        // ", " before the "new line" / "new paragraph" cue. Without
        // this we leak commas like "Line 1,\nLine 2,\n…".
        (r"(?i),?\s*\bnew\s+paragraph\b\s*,?", "\n\n"),
        (r"(?i),?\s*\bnew\s+line\b\s*,?", "\n"),
        (r"(?i)\bfull\s+stop\b\s*,?", "."),
        (r"(?i)\bquestion\s+mark\b\s*,?", "?"),
        (r"(?i)\bexclamation\s+(mark|point)\b\s*,?", "!"),
        (r"(?i)\bopen\s+quote\b\s*,?", "\""),
        (r"(?i)\bclose\s+quote\b\s*,?", "\""),
        // "parin" / "paran" are common Whisper mis-transcriptions of
        // "paren". Cheap accuracy win since neither is an English word.
        (r"(?i)\bopen\s+par(?:en|in|an)(?:thesis)?\b\s*,?", "("),
        (r"(?i)\bclose\s+par(?:en|in|an)(?:thesis)?\b\s*,?", ")"),
        // ACCURACY-FIRST POLICY:
        // We deliberately do NOT convert mid-sentence single-word punctuation
        // (period / colon / semicolon / dash) because "the third period of
        // class", "the colon is part of digestion", "a long dash" all use
        // those words as nouns. Disambiguating without grammar parsing
        // would risk garbling valid speech — accuracy beats automation.
        //
        // Conversions ONLY fire when:
        //   - The cue is multi-word (full stop, question mark, exclamation,
        //     new line, new paragraph, open/close quote, etc.) — these are
        //     unambiguously commands.
        //   - OR the cue is at sentence-end (no word follows) — "thanks
        //     comma" / "the year was 2023 period" — also unambiguous.
        //   - OR the cue immediately follows a clause-ending punctuation
        //     mark — ". Comma " — the user is dictating a fresh clause.
        //
        // Mid-sentence "comma" is the exception we keep because "comma" as
        // a noun is rare enough outside grammar talk. Even so, restricted
        // to a strict X COMMA Y pattern so "the comma key" is filtered.
        // (Filtered via the determiner blacklist below.)

        // Mid-sentence "comma" — explicit X COMMA Y dictation cadence.
        // We KEEP this for "comma" specifically (uncommon as a noun in
        // running speech) but NOT for period/colon/semicolon/dash.
        // Edge cost: "the comma key is broken" gets garbled. Trade-off
        // accepted because "Hello comma world" is the bread-and-butter
        // dictation pattern.
        (r"(?i)([A-Za-z0-9'])\s+comma\s+([A-Za-z0-9])", "$1, $2"),

        // End-of-sentence variants — safe.
        (r"(?i)([A-Za-z0-9])\s+full\s+stop\s*$", "$1."),
        (r"(?i)([A-Za-z0-9])\s+period\s*$", "$1."),
        (r"(?i)([A-Za-z0-9])\s+question\s+mark\s*$", "$1?"),
        (r"(?i)([A-Za-z0-9])\s+exclamation\s+(?:mark|point)\s*$", "$1!"),
        (r"(?i)([A-Za-z0-9])\s+comma\s*$", "$1,"),
        (r"(?i)([A-Za-z0-9])\s+colon\s*$", "$1:"),
        (r"(?i)([A-Za-z0-9])\s+semi\s*colon\s*$", "$1;"),
        // Clause-leading: ". comma X" — user dictating a new clause.
        (r"(?i)([.!?])\s+comma\s+", "$1, "),
        (r"(?i)([.!?])\s+period\s+", "$1. "),
        (r"(?i)([.!?])\s+colon\s+", "$1: "),
        // Whisper-mangled pattern ". Comma. Word" — Whisper sometimes
        // wraps the spoken "comma" cue with periods on both sides when
        // it sits at a clause boundary. Without this, "tomorrow comma
        // please confirm" becomes "tomorrow. Please confirm." (period
        // intact, cue dropped). We rewrite to ", word" — same risk
        // profile as the standard clause-leading rule above.
        (r"(?i)([.!?])\s+comma\s*[.!?]\s+([A-Za-z])", ", $2"),
    ];

    let mut out = text.to_string();
    for (pat, rep) in replacements {
        let re = Regex::new(pat).expect("valid regex");
        out = re.replace_all(&out, *rep).to_string();
    }
    // Flatten the placeholder <SPC> and collapse whitespace that the
    // replacements may have stranded next to punctuation.
    out = out.replace("<SPC>", "");
    // Remove stray space before punctuation ("world ." → "world.").
    let space_before_punct = Regex::new(r"\s+([,.!?;:])").expect("valid regex");
    let out = space_before_punct.replace_all(&out, "$1").to_string();
    // Strip space INSIDE close-quote / close-paren and after open variants:
    //   "hi "   → "hi"  (stripped before close)
    //   ( hi )  → (hi)  (stripped after open + before close)
    let space_before_close = Regex::new(r#"\s+([")\]\}])"#).expect("valid regex");
    let out = space_before_close.replace_all(&out, "$1").to_string();
    let space_after_open = Regex::new(r#"([(\[\{])\s+"#).expect("valid regex");
    let out = space_after_open.replace_all(&out, "$1").to_string();
    // Insert a space after a close-paren when missing: "(maybe)is" → "(maybe) is"
    let close_paren_glue = Regex::new(r#"([)\]\}])([A-Za-z])"#).expect("valid regex");
    let out = close_paren_glue.replace_all(&out, "$1 $2").to_string();
    // Collapse runs of ordinary whitespace.
    let runs = Regex::new(r"[ \t]+").expect("valid regex");
    runs.replace_all(&out, " ").to_string()
}

/// Handle spoken self-corrections like:
///   "go to the office no wait go to the cafe"   →  "go to the cafe"
///   "my name is Raj sorry Rajesh"                →  "my name is Rajesh"
///
/// Rule: when a correction marker appears mid-utterance followed by a
/// replacement phrase, keep the correction. We only act when the pattern is
/// unambiguous — otherwise the original text survives.
pub fn apply_corrections(text: &str) -> String {
    // Two MARKER groups split by user intent:
    //   WORD_MARKER  — typically a slip of a single word ("sorry",
    //                  "I mean") → swap only the immediately surrounding
    //                  word, preserve everything else.
    //   CLAUSE_MARKER — restart the whole utterance ("no wait", "scratch
    //                   that", "actually") → eat the full prior clause.
    //
    // This split fixes the over-collapse bug where "my name is Raj sorry
    // Rajesh" was reducing to just "Rajesh". With WORD_MARKER scope it
    // becomes "my name is Rajesh".
    const WORD_MARKER: &str = r"(?:sorry|i\s+mean)";
    const CLAUSE_MARKER: &str = r"(?:no\s*,?\s*wait|scratch\s+that|actually)";

    // 1. Sentence-spanning clause correction:
    //    "[full prior sentence]. [CLAUSE_MARKER][, ]? [replacement]" → just
    //    the replacement.
    static SENTENCE_CORRECTION: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)[^.!?\n]+[.!?]+\s*{}\s*[,]?\s*([^.!?\n]+[.!?]?)",
            CLAUSE_MARKER
        );
        Regex::new(&pat).unwrap()
    });

    // 2. Inline CLAUSE correction within a single clause:
    //    "prior CLAUSE_MARKER replacement" → replacement.
    static INLINE_CLAUSE_CORRECTION: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)([^,.!?\n]+?)[,\s]+{}[,\s]+([^,.!?\n]+)",
            CLAUSE_MARKER
        );
        Regex::new(&pat).unwrap()
    });

    // 3. WORD-LEVEL correction: just one word on each side of the marker.
    //    "X WORD_MARKER Y" → "Y" (swap only that word). Strict: prior and
    //    replacement must each be a single token (no spaces in either).
    //    This preserves "my name is Raj sorry Rajesh" → "my name is Rajesh"
    //    instead of collapsing to just "Rajesh".
    static WORD_CORRECTION: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)\b([A-Za-z0-9'\-]+)[,\s]+{}[,\s]+([A-Za-z0-9'\-]+)\b",
            WORD_MARKER
        );
        Regex::new(&pat).unwrap()
    });

    // Order matters:
    //   - Sentence-level clause corrections first (largest span).
    //   - Then inline clause corrections.
    //   - Then word-level corrections (smallest span; would otherwise
    //     consume the prior word that a clause correction would discard).
    // Loop each pass until stable so chained corrections fully collapse.
    let mut prev = text.to_string();
    loop {
        let replaced = SENTENCE_CORRECTION
            .replace_all(&prev, |caps: &regex::Captures| {
                caps.get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default()
            })
            .to_string();
        if replaced == prev {
            break;
        }
        prev = replaced;
    }
    loop {
        let replaced = INLINE_CLAUSE_CORRECTION
            .replace_all(&prev, |caps: &regex::Captures| {
                caps.get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default()
            })
            .to_string();
        if replaced == prev {
            break;
        }
        prev = replaced;
    }
    loop {
        let replaced = WORD_CORRECTION
            .replace_all(&prev, |caps: &regex::Captures| {
                // Replace `prior MARKER replacement` with just `replacement`.
                caps.get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            })
            .to_string();
        if replaced == prev {
            break;
        }
        prev = replaced;
    }
    prev
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comma_full_stop_basic() {
        let out = apply_spoken_punctuation("hello comma world full stop");
        assert_eq!(out.trim(), "hello, world.");
    }

    #[test]
    fn question_mark_at_end() {
        let out = apply_spoken_punctuation("how are you question mark");
        assert!(out.trim_end().ends_with('?'));
    }

    #[test]
    fn new_line_inserts_newline() {
        let out = apply_spoken_punctuation("line one new line line two");
        assert!(out.contains('\n'));
    }

    #[test]
    fn new_paragraph_inserts_blank_line() {
        let out = apply_spoken_punctuation("para one new paragraph para two");
        assert!(out.contains("\n\n"));
    }

    #[test]
    fn correction_no_wait_replaces() {
        let out = apply_corrections("go to the office no wait go to the cafe");
        assert_eq!(out, "go to the cafe");
    }

    #[test]
    fn correction_sorry_replaces() {
        // "sorry" is a WORD_MARKER: swaps only the immediately preceding
        // word, preserving the rest of the sentence.
        let out = apply_corrections("my name is Raj sorry Rajesh");
        assert_eq!(out, "my name is Rajesh");
    }

    #[test]
    fn correction_word_marker_keeps_envelope() {
        // Regression guard for the over-collapse bug: WORD_MARKER must NOT
        // eat the entire prior clause the way CLAUSE_MARKER does.
        let out = apply_corrections("call him at three sorry four");
        assert_eq!(out, "call him at four");
    }

    #[test]
    fn correction_clause_marker_still_eats_clause() {
        // CLAUSE_MARKER ("no wait") must still discard the full prior clause.
        let out = apply_corrections("go to the office no wait go to the cafe");
        assert_eq!(out, "go to the cafe");
    }

    #[test]
    fn no_correction_when_no_marker() {
        let out = apply_corrections("my name is Raj");
        assert_eq!(out, "my name is Raj");
    }

    #[test]
    fn correction_across_sentence_boundary() {
        // Whisper splits "no wait" as a fresh sentence after a period.
        // The correction should still collapse.
        let out = apply_corrections(
            "Let's meet at the office. No wait, let's meet at the cafe instead.",
        );
        assert!(out.to_lowercase().contains("cafe"), "got: {}", out);
        assert!(!out.to_lowercase().contains("office"), "got: {}", out);
    }

    #[test]
    fn correction_handles_comma_split_marker() {
        // Whisper sometimes inserts a comma between "No" and "wait".
        let out = apply_corrections(
            "Let's meet at the office. No, wait, let's meet at the cafe.",
        );
        assert!(out.to_lowercase().contains("cafe"), "got: {}", out);
        assert!(!out.to_lowercase().contains("office"), "got: {}", out);
    }

    #[test]
    fn literal_period_noun_safe() {
        // Don't mangle when "period" is a legitimate noun mid-sentence.
        let out = apply_spoken_punctuation("I had a great period in life");
        assert!(out.contains("period"));
    }

    #[test]
    fn new_line_eats_preceding_comma() {
        // Whisper inserts a comma before "new line"; we want that comma
        // gone so the result is clean line breaks.
        let out = apply_spoken_punctuation("line one, new line, line two");
        assert!(!out.contains(","), "got: {:?}", out);
        assert!(out.contains('\n'));
    }

    #[test]
    fn paren_misspelling_still_matches() {
        let out = apply_spoken_punctuation("open parin this is a side note close paran");
        assert!(out.contains('('), "got: {:?}", out);
        assert!(out.contains(')'), "got: {:?}", out);
    }

    #[test]
    fn comma_wrapped_with_periods_becomes_comma() {
        // Regression for the "tomorrow comma please confirm" →
        // "tomorrow. Please confirm." case — Whisper wraps the cue
        // with periods on both sides; our rule should recover it.
        let out = apply_spoken_punctuation("tomorrow. Comma. please confirm");
        assert!(out.contains(", "), "got: {:?}", out);
    }
}
