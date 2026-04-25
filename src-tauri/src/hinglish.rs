//! Hinglish (Hindi written in Roman script) decoder hints.
//!
//! Whisper has very little Roman-Hindi in its training data, so when a
//! user dictates Hinglish it tends to either transliterate to Devanagari
//! (when language=hi) or mishear as English nonsense (when language=en).
//!
//! Two cheap, high-yield interventions:
//!   1. A starter vocabulary of ~150 commonly-used Hindi-Roman words that
//!      Spokn auto-injects into `custom_words` the first time the user
//!      adds Hindi to their language list. Whisper then biases its
//!      decoder toward producing these tokens.
//!   2. A short Hinglish seed phrase prepended to every Whisper
//!      `initial_prompt` when Hindi is in the user's language list,
//!      establishing the expected language register for the decoder.
//!
//! Neither is a true Hinglish solution — for that we'd need a fine-tuned
//! model or a Devanagari→Roman pipeline — but together they noticeably
//! improve transcription on day one with zero extra latency.

/// Most-commonly-used Hindi-Roman tokens. Curated for speed of recall;
/// not exhaustive. Users grow this list further via the regular
/// vocabulary mechanism.
pub const HINGLISH_STARTER_WORDS: &[&str] = &[
    // Greetings / acknowledgements
    "namaste", "namaskar", "salaam", "haan", "haanji", "nahi", "nahin", "theek", "thik",
    "accha", "acha", "sahi", "bilkul", "shukriya", "dhanyavaad",
    // Common pronouns
    "main", "mai", "tum", "tu", "aap", "hum", "woh", "yeh", "kya", "kaun", "kab",
    "kaisa", "kaise", "kaisi", "kahan", "kyun", "kyon", "kyunki",
    // Familial / relational
    "bhai", "behen", "bhaiya", "didi", "yaar", "dost", "mummy", "papa",
    // Time
    "kal", "aaj", "abhi", "subah", "shaam", "raat", "din", "saal",
    "mahina", "hafta", "samay", "waqt",
    // Verbs & connectors
    "karo", "karna", "karta", "karte", "karke", "kar", "raha", "rahi",
    "rahe", "hota", "hoti", "hote", "hoga", "hogi", "honge",
    "tha", "thi", "the", "lekin", "magar", "phir", "fir", "matlab",
    "yaani", "kyonki", "isliye", "isiliye", "agar", "warna", "varna",
    "chaiye", "chahiye", "chaihiye",
    // Day-to-day verbs
    "chalo", "chal", "dekho", "dekh", "suno", "sun", "bolo", "bol",
    "samjho", "samjha", "samjhi", "milte", "mila", "milegi", "milega",
    "khao", "khana", "piyo", "pina", "jao", "ja", "aao", "aa",
    "ruko", "ruk", "baitho", "baith", "utho",
    // Office / casual
    "office", "kaam", "meeting", "call", "saath", "saare", "sara",
    "bahut", "kuch", "thoda", "zyada", "kam", "jaldi", "der",
    "achha", "bura", "buri", "bure", "naya", "purana",
    // Filler / textures
    "wagairah", "vagairah", "matlab", "samjhe", "samjha", "yaar", "boss",
    "jugaad", "scene", "tension", "tention",
    // Polite / requests
    "please", "kripya", "maaf", "sorry", "dhanyavad",
    // Numbers (Roman-Hindi often used colloquially)
    "ek", "do", "teen", "char", "chaar", "paanch", "panch", "che", "chhe",
    "saat", "aath", "nau", "das", "sau", "hazaar", "lakh",
    "crore", "karod",
];

/// Short Hinglish seed phrase prepended to Whisper's `initial_prompt`
/// when Hindi is one of the user's selected dictation languages. The
/// phrase establishes Roman-script Hindi as the expected register and
/// covers the most common syllable patterns Whisper otherwise mangles.
pub const HINGLISH_SEED_PROMPT: &str =
    "Haan bhai, kal subah office chalo phir lunch karenge. Theek hai, matlab agar koi problem nahi to chalo. Yaar yeh kaam jaldi karna hai.";

/// True if the user's transcription_languages list includes Hindi (case
/// insensitive, ISO-639-1 code "hi").
pub fn user_speaks_hindi(transcription_languages: &[String]) -> bool {
    transcription_languages.iter().any(|l| {
        let normalized = l.split('-').next().unwrap_or(l).to_lowercase();
        normalized == "hi"
    })
}

/// Tighter list used by the auto-detector. Subset of the starter pack —
/// only words with no English homograph that would falsely trigger the
/// "looks like Hinglish?" prompt on regular English text. Excludes
/// loan words like "office", "call", "please", "do", numbers like
/// "saat" (= "seven", but visually equal to English "sat").
const HINGLISH_DETECTION_MARKERS: &[&str] = &[
    "haan", "haanji", "nahin", "theek", "thik", "accha", "acha", "sahi",
    "bilkul", "shukriya", "dhanyavaad", "dhanyavad", "namaste", "namaskar",
    "salaam", "main", "mai", "tum", "hum", "woh", "yeh", "kya", "kaun",
    "kab", "kaisa", "kaise", "kaisi", "kahan", "kyun", "kyon", "kyunki",
    "kyonki", "bhai", "behen", "bhaiya", "didi", "yaar",
    "kal", "aaj", "abhi", "subah", "shaam", "raat", "mahina", "hafta",
    "waqt", "karo", "karna", "karta", "karte", "karke", "raha", "rahi",
    "rahe", "hota", "hoti", "hote", "hoga", "hogi", "honge", "tha", "thi",
    "lekin", "magar", "phir", "matlab", "yaani", "isliye", "isiliye",
    "agar", "warna", "varna", "chahiye", "chaihiye", "chalo", "chal",
    "dekho", "suno", "bolo", "samjho", "samjha", "samjhi", "milte",
    "milegi", "milega", "khao", "khana", "piyo", "pina", "jao", "aao",
    "ruko", "baitho", "saath", "saare", "bahut", "kuch", "thoda",
    "zyada", "jaldi", "achha", "bura", "buri", "bure", "naya", "purana",
    "wagairah", "vagairah", "samjhe", "jugaad", "kripya", "maaf",
    "hazaar", "crore", "karod", "lakh",
];

/// Heuristic: does this transcript look like Hinglish?
///
/// Tokenizes into ASCII words and counts how many appear in
/// [`HINGLISH_DETECTION_MARKERS`] (a curated subset that excludes
/// English-loan words). Returns true when at least 2 absolute matches AND
/// they account for ≥15% of the total word count.
///
/// Used by the auto-detect path that suggests "enable Hindi for better
/// accuracy?" the first time a user dictates Hinglish without having
/// selected Hindi at onboarding.
pub fn looks_like_hinglish(text: &str) -> bool {
    use std::collections::HashSet;
    let starter: HashSet<&str> = HINGLISH_DETECTION_MARKERS.iter().copied().collect();

    let words: Vec<String> = text
        .split_whitespace()
        .filter_map(|raw| {
            let cleaned: String = raw
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect();

    // Don't fire on very short utterances — too noisy.
    if words.len() < 4 {
        return false;
    }

    let matches = words
        .iter()
        .filter(|w| starter.contains(w.as_str()))
        .count();

    matches >= 2 && (matches as f32 / words.len() as f32) >= 0.15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hindi_in_list() {
        assert!(user_speaks_hindi(&["hi".to_string()]));
        assert!(user_speaks_hindi(&[
            "en".to_string(),
            "hi".to_string()
        ]));
        assert!(user_speaks_hindi(&["HI".to_string()]));
        assert!(user_speaks_hindi(&["hi-IN".to_string()]));
    }

    #[test]
    fn ignores_unrelated_languages() {
        assert!(!user_speaks_hindi(&["en".to_string()]));
        assert!(!user_speaks_hindi(&["hi_FR".to_string()])); // weird shape
    }

    #[test]
    fn empty_list_is_not_hindi() {
        assert!(!user_speaks_hindi(&[]));
    }

    // ---- Hinglish detector ----

    #[test]
    fn detects_obvious_hinglish() {
        assert!(looks_like_hinglish(
            "Haan bhai kal subah office chalo phir lunch karenge"
        ));
    }

    #[test]
    fn detects_short_hinglish_above_threshold() {
        // "main aaj office jaa raha hu" — 6 words, 4 matches → 67% ratio
        assert!(looks_like_hinglish("main aaj office jaa raha hu"));
    }

    #[test]
    fn ignores_pure_english() {
        assert!(!looks_like_hinglish(
            "I will meet you tomorrow at the cafe near the office"
        ));
    }

    #[test]
    fn one_word_does_not_trigger() {
        // Single "yaar" in English — common code-switching but not enough
        // signal to interrupt the user.
        assert!(!looks_like_hinglish("yaar can you send the report please"));
    }

    #[test]
    fn very_short_utterance_does_not_trigger() {
        // <4 words guard against noise on tiny transcripts
        assert!(!looks_like_hinglish("haan bhai"));
    }

    #[test]
    fn punctuation_does_not_break_match() {
        assert!(looks_like_hinglish(
            "haan, bhai. kal subah office chalo!"
        ));
    }
}
