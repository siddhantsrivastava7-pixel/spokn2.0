//! Currency and percentage formatting (Indian-first).
//!
//! Runs *after* [`super::numbers::words_to_digits`] has normalised spoken
//! numbers into digits, so we can match plain "500 rupees" patterns.

use once_cell::sync::Lazy;
use regex::Regex;

use super::numbers::format_indian;

/// Convert "<number> rupees" / "rs <number>" variants into "₹<number>" using
/// Indian digit grouping.
pub fn format_rupees(text: &str) -> String {
    static RE_TRAILING: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(\d[\d,]*)\s*(?:rupees|rupee|rs\.?|inr)\b").unwrap());
    static RE_LEADING: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(?:rupees|rs\.?|inr)\s+(\d[\d,]*)").unwrap());

    let replace = |caps: &regex::Captures| {
        let raw = caps[1].replace(',', "");
        match raw.parse::<u64>() {
            Ok(n) => format!("₹{}", format_indian(n)),
            Err(_) => caps[0].to_string(),
        }
    };

    let out = RE_TRAILING.replace_all(text, replace).to_string();
    RE_LEADING.replace_all(&out, replace).to_string()
}

/// "<number> point <number> percent" or "<number> percent" → "<n>%".
/// Runs on digit-normalised input.
pub fn format_percent(text: &str) -> String {
    static RE_POINT: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(\d+)\s+point\s+(\d+)\s+percent\b").unwrap()
    });
    static RE_PLAIN: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s+percent\b").unwrap());

    let out = RE_POINT.replace_all(text, "$1.$2%").to_string();
    RE_PLAIN.replace_all(&out, "$1%").to_string()
}

/// Full currency/percent pass — safe to call on any text.
pub fn apply(text: &str) -> String {
    let s = format_rupees(text);
    format_percent(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_hundred_rupees() {
        let out = format_rupees("500 rupees");
        assert_eq!(out, "₹500");
    }

    #[test]
    fn twenty_five_thousand_rupees() {
        let out = format_rupees("25000 rupees");
        assert_eq!(out, "₹25,000");
    }

    #[test]
    fn one_lakh_rupees_grouped() {
        let out = format_rupees("100000 rupees");
        assert_eq!(out, "₹1,00,000");
    }

    #[test]
    fn two_crore_rupees_grouped() {
        let out = format_rupees("20000000 rupees");
        assert_eq!(out, "₹2,00,00,000");
    }

    #[test]
    fn rs_leading() {
        let out = format_rupees("Rs 250");
        assert_eq!(out, "₹250");
    }

    #[test]
    fn percent_plain() {
        assert_eq!(format_percent("5 percent"), "5%");
    }

    #[test]
    fn percent_with_point() {
        assert_eq!(format_percent("5 point 5 percent"), "5.5%");
    }

    #[test]
    fn preserves_unrelated_text() {
        assert_eq!(format_rupees("hello world"), "hello world");
    }
}
