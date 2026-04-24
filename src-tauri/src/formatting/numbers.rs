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
pub fn words_to_digits(text: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        // Match spans of number words (and optional connectors like "and"/"-").
        Regex::new(
            r"(?i)\b(?:zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fourty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|lakh|lac|lakhs|crore|crores|million|billion)(?:[\s-]+(?:and\s+)?(?:zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fourty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|lakh|lac|lakhs|crore|crores|million|billion))*",
        )
        .unwrap()
    });

    RE.replace_all(text, |caps: &regex::Captures| {
        match parse_number_phrase(&caps[0]) {
            Some(n) => n.to_string(),
            None => caps[0].to_string(),
        }
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
