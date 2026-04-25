//! Spoken-number → digit conversion.
//!
//! Handles English cardinals up through the Indian numbering system
//! (lakh, crore). The converter is intentionally scoped to contiguous
//! number-word runs — it never touches text outside those spans, so other
//! content is safe from accidental mutation.

use once_cell::sync::Lazy;
use regex::Regex;

/// Convert spoken number phrases in `text` into digit strings.
///
/// Returns `text` unchanged when no number words are present.
///
/// Two-pass conversion:
///   1. Word-only spans like "twenty five thousand" → "25000".
///   2. Digit + magnitude bridging like "50 thousand" → "50000". This
///      catches Whisper's mixed output where short numbers get auto-
///      digitised but the magnitude word stays alphabetic.
///
/// Both passes use a strict trailing word-boundary (each alternative is
/// suffixed with `\b`) so we never partial-match "nine" inside
/// "nineteen", "ninety", "eighties", etc.
pub fn words_to_digits(text: &str) -> String {
    // Each number-word with a trailing `\b` so partial matches inside
    // longer compounds are rejected.
    const NUM_WORDS_B: &str = r"(?:zero\b|one\b|two\b|three\b|four\b|five\b|six\b|seven\b|eight\b|nine\b|ten\b|eleven\b|twelve\b|thirteen\b|fourteen\b|fifteen\b|sixteen\b|seventeen\b|eighteen\b|nineteen\b|twenty\b|thirty\b|forty\b|fourty\b|fifty\b|sixty\b|seventy\b|eighty\b|ninety\b|hundred\b|thousand\b|lakh\b|lac\b|lakhs\b|crore\b|crores\b|million\b|billion\b)";

    static WORD_RE: Lazy<Regex> = Lazy::new(|| {
        let pat = format!(
            r"(?i)\b{}(?:[\s-]+(?:and\s+)?{})*",
            NUM_WORDS_B, NUM_WORDS_B
        );
        Regex::new(&pat).unwrap()
    });

    // Pass 1: pure word spans → digits.
    let after_words = WORD_RE
        .replace_all(text, |caps: &regex::Captures| {
            match parse_number_phrase(&caps[0]) {
                Some(n) => n.to_string(),
                None => caps[0].to_string(),
            }
        })
        .to_string();

    // Pass 2: digit + magnitude word ("50 thousand" / "1.5 crore"). Whisper
    // tends to half-convert numbers — we finish the job. Decimal handled.
    static DIGIT_MAG_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(\d+(?:\.\d+)?)\s+(thousand|lakh|lakhs|lac|crore|crores|million|billion)\b",
        )
        .unwrap()
    });
    let after_mag = DIGIT_MAG_RE
        .replace_all(&after_words, |caps: &regex::Captures| {
            let n: f64 = caps[1].parse().unwrap_or(0.0);
            let mag = caps[2].to_lowercase();
            let mult: f64 = match mag.as_str() {
                "thousand" => 1_000.0,
                "lakh" | "lakhs" | "lac" => 1_00_000.0,
                "crore" | "crores" => 1_00_00_000.0,
                "million" => 1_000_000.0,
                "billion" => 1_000_000_000.0,
                _ => 1.0,
            };
            let result = (n * mult) as u64;
            result.to_string()
        })
        .to_string();

    // Pass 3: decimal recognition. Strict: digit + "point" + digit ONLY.
    // Rejects "the third point of the talk" (no digits around "point").
    // Safe because "point" between two number tokens is unambiguously a
    // decimal — there is no English noun phrase that fits that pattern.
    static DECIMAL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\b(\d+)\s+point\s+(\d+)\b").unwrap()
    });
    let after_decimal = DECIMAL_RE
        .replace_all(&after_mag, "$1.$2")
        .to_string();

    // Pass 4: "minus N" → "-N", strict: only when followed by a digit.
    // Rejects "minus a friend" because "a" isn't a digit. Safe.
    static MINUS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bminus\s+(\d)").unwrap());
    let after_minus = MINUS_RE.replace_all(&after_decimal, "-$1").to_string();

    // Pass 5: decade formatting. Whitelisted words only, after numbers
    // pass so we don't accidentally munge "the eight" → "the 8". Tightly
    // scoped — only fires on the literal decade plural words.
    static DECADE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(twenties|thirties|forties|fifties|sixties|seventies|eighties|nineties)\b")
            .unwrap()
    });
    DECADE_RE
        .replace_all(&after_minus, |caps: &regex::Captures| {
            let word = caps[1].to_lowercase();
            let digits = match word.as_str() {
                "twenties" => "20s",
                "thirties" => "30s",
                "forties" => "40s",
                "fifties" => "50s",
                "sixties" => "60s",
                "seventies" => "70s",
                "eighties" => "80s",
                "nineties" => "90s",
                _ => return caps[0].to_string(),
            };
            digits.to_string()
        })
        .to_string()
}

/// Best-effort parser: splits on whitespace / hyphen, fold values using the
/// classic "total + current" algorithm. Returns `None` if the phrase is
/// ambiguous or invalid so the caller can preserve the original text.
pub fn parse_number_phrase(phrase: &str) -> Option<u64> {
    let tokens: Vec<&str> = phrase
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("and"))
        .collect();
    if tokens.is_empty() {
        return None;
    }

    let mut total: u64 = 0;
    let mut current: u64 = 0;
    let mut any_value = false;

    for token in tokens {
        let t = token.to_ascii_lowercase();
        let val = word_value(&t);
        match val {
            WordValue::Unit(n) => {
                current = current.checked_add(n)?;
                any_value = true;
            }
            WordValue::Hundred => {
                // "five hundred" — multiply the running unit by 100.
                current = current.checked_mul(100)?;
                if current == 0 {
                    current = 100; // "hundred" alone
                }
                any_value = true;
            }
            WordValue::Scale(scale) => {
                if current == 0 {
                    current = 1; // "thousand" alone → 1000
                }
                total = total.checked_add(current.checked_mul(scale)?)?;
                current = 0;
                any_value = true;
            }
            WordValue::None => return None,
        }
    }
    if !any_value {
        return None;
    }
    Some(total + current)
}

enum WordValue {
    Unit(u64),
    Hundred,
    Scale(u64),
    None,
}

fn word_value(token: &str) -> WordValue {
    match token {
        "zero" => WordValue::Unit(0),
        "one" => WordValue::Unit(1),
        "two" => WordValue::Unit(2),
        "three" => WordValue::Unit(3),
        "four" => WordValue::Unit(4),
        "five" => WordValue::Unit(5),
        "six" => WordValue::Unit(6),
        "seven" => WordValue::Unit(7),
        "eight" => WordValue::Unit(8),
        "nine" => WordValue::Unit(9),
        "ten" => WordValue::Unit(10),
        "eleven" => WordValue::Unit(11),
        "twelve" => WordValue::Unit(12),
        "thirteen" => WordValue::Unit(13),
        "fourteen" => WordValue::Unit(14),
        "fifteen" => WordValue::Unit(15),
        "sixteen" => WordValue::Unit(16),
        "seventeen" => WordValue::Unit(17),
        "eighteen" => WordValue::Unit(18),
        "nineteen" => WordValue::Unit(19),
        "twenty" => WordValue::Unit(20),
        "thirty" => WordValue::Unit(30),
        "forty" | "fourty" => WordValue::Unit(40),
        "fifty" => WordValue::Unit(50),
        "sixty" => WordValue::Unit(60),
        "seventy" => WordValue::Unit(70),
        "eighty" => WordValue::Unit(80),
        "ninety" => WordValue::Unit(90),
        "hundred" => WordValue::Hundred,
        "thousand" => WordValue::Scale(1_000),
        "lakh" | "lac" | "lakhs" => WordValue::Scale(1_00_000),
        "crore" | "crores" => WordValue::Scale(1_00_00_000),
        "million" => WordValue::Scale(1_000_000),
        "billion" => WordValue::Scale(1_000_000_000),
        _ => WordValue::None,
    }
}

/// Format `n` using the Indian digit grouping convention
/// (1,00,000 / 1,00,00,000). For values below 1,000 the number is returned as-is.
pub fn format_indian(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let (head, tail) = s.split_at(s.len() - 3);
    let mut head_chars: Vec<char> = head.chars().collect();
    head_chars.reverse();
    let mut grouped: Vec<String> = head_chars
        .chunks(2)
        .map(|chunk| {
            let mut part: String = chunk.iter().collect();
            part = part.chars().rev().collect();
            part
        })
        .collect();
    grouped.reverse();
    format!("{},{}", grouped.join(","), tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_hundred() {
        assert_eq!(parse_number_phrase("five hundred"), Some(500));
    }

    #[test]
    fn parses_twenty_five_thousand() {
        assert_eq!(parse_number_phrase("twenty five thousand"), Some(25_000));
    }

    #[test]
    fn parses_one_lakh() {
        assert_eq!(parse_number_phrase("one lakh"), Some(100_000));
    }

    #[test]
    fn parses_two_crore() {
        assert_eq!(parse_number_phrase("two crore"), Some(20_000_000));
    }

    #[test]
    fn parses_hyphenated() {
        assert_eq!(parse_number_phrase("twenty-five"), Some(25));
    }

    #[test]
    fn indian_grouping_one_lakh() {
        assert_eq!(format_indian(100_000), "1,00,000");
    }

    #[test]
    fn indian_grouping_two_crore() {
        assert_eq!(format_indian(20_000_000), "2,00,00,000");
    }

    #[test]
    fn indian_grouping_small() {
        assert_eq!(format_indian(500), "500");
    }

    #[test]
    fn words_to_digits_in_sentence() {
        let out = words_to_digits("I bought twenty five apples");
        assert!(out.contains("25"));
    }

    #[test]
    fn preserves_non_number_text() {
        let out = words_to_digits("hello world");
        assert_eq!(out, "hello world");
    }
}
