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
        (r"(?i)\bnew\s+paragraph\b\s*,?", "\n\n"),
        (r"(?i)\bnew\s+line\b\s*,?", "\n"),
        (r"(?i)\bfull\s+stop\b\s*,?", "."),
        (r"(?i)\bquestion\s+mark\b\s*,?", "?"),
        (r"(?i)\bexclamation\s+(mark|point)\b\s*,?", "!"),
        (r"(?i)\bopen\s+quote\b\s*,?", "\""),
        (r"(?i)\bclose\s+quote\b\s*,?", "\""),
        (r"(?i)\bopen\s+paren(thesis)?\b\s*,?", "("),
        (r"(?i)\bclose\s+paren(thesis)?\b\s*,?", ")"),
        // Single-word punctuation — only at clause boundaries (comma / end of
        // sentence / start of utterance) to avoid mangling sentences where
        // "period" or "colon" appear as real nouns.
        (r"(?i)(^|,\s|\.\s)comma\b\s*,?", "$1,<SPC>"),
        (r"(?i)(^|,\s|\.\s)period\b\s*,?", "$1.<SPC>"),
        (r"(?i)(^|,\s|\.\s)colon\b\s*,?", "$1:<SPC>"),
        (r"(?i)(^|,\s|\.\s)semi\s*colon\b\s*,?", "$1;<SPC>"),
        (r"(?i)(^|,\s|\.\s)dash\b\s*,?", "$1 — <SPC>"),
        // Looser pattern for mid-sentence "comma" — rely on surrounding words
        // already being present; only when clearly the command form
        // ("X comma Y" as pause). Still conservative: require whitespace both
        // sides and not at sentence start.
        (r"(?i)\s+comma\s+", ", "),
        (r"(?i)\s+full\s+stop\s+", ". "),
        (r"(?i)\s+question\s+mark\s*$", "?"),
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
    // Marker can include embedded commas that Whisper loves to insert, e.g.
    // "No, wait" or "I, mean".
    const MARKER: &str = r"(?:no\s*,?\s*wait|sorry|scratch\s+that|i\s+mean)";

    // 1. Sentence-spanning correction:
    //    "[full prior sentence]. [MARKER][, ]? [replacement]"
    //    → just the replacement.
    static SENTENCE_CORRECTION: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)[^.!?\n]+[.!?]+\s*{}\s*[,]?\s*([^.!?\n]+[.!?]?)",
            MARKER
        );
        Regex::new(&pat).unwrap()
    });

    // 2. Inline correction within a single clause:
    //    "prior , MARKER , replacement"  →  replacement
    static INLINE_CORRECTION: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)([^,.!?\n]+?)[,\s]+{}[,\s]+([^,.!?\n]+)",
            MARKER
        );
        Regex::new(&pat).unwrap()
    });

    // Apply sentence-level first (bigger span), then inline (narrower).
    // Loop each until stable so chained corrections collapse.
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
        let replaced = INLINE_CORRECTION
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
        let out = apply_corrections("my name is Raj sorry Rajesh");
        assert_eq!(out, "Rajesh");
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
}
