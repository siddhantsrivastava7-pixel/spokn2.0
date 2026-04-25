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
        Lazy::new(|| Regex::new(r"(?i)(\d[\d,]*)\s*(?:rupees|rupee|rs\.?|inr)\b").unwrap());
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

/// Generic Western-currency formatter for USD/EUR/GBP. Uses Western
/// thousands grouping (1,000 / 1,000,000) — different from Indian.
fn format_western(
    text: &str,
    symbol: &str,
    aliases: &[&str],
) -> String {
    use std::fmt::Write;
    let aliases_pat = aliases.join("|");
    // Trailing form: "100 dollars" → "$100"
    let trailing = Regex::new(&format!(
        r"(?i)\b(\d[\d,]*(?:\.\d+)?)\s*(?:{})\b",
        aliases_pat
    ))
    .unwrap();
    // Leading form: "USD 100" → "$100"
    let leading = Regex::new(&format!(
        r"(?i)\b(?:{})\s+(\d[\d,]*(?:\.\d+)?)",
        aliases_pat
    ))
    .unwrap();

    let format_western_grouped = |raw: &str| -> String {
        // Preserve decimals; re-group integer portion in Western 3-digit chunks.
        let cleaned = raw.replace(',', "");
        let (int_part, dec_part) = match cleaned.find('.') {
            Some(i) => (&cleaned[..i], Some(&cleaned[i..])),
            None => (cleaned.as_str(), None),
        };
        let n: u64 = int_part.parse().unwrap_or(0);
        let mut out = String::new();
        let s = n.to_string();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        if let Some(d) = dec_part {
            let _ = write!(out, "{}", d);
        }
        out
    };

    let trailing_replace = |caps: &regex::Captures| {
        format!("{}{}", symbol, format_western_grouped(&caps[1]))
    };
    let leading_replace = |caps: &regex::Captures| {
        format!("{}{}", symbol, format_western_grouped(&caps[1]))
    };

    let out = trailing.replace_all(text, trailing_replace).to_string();
    leading.replace_all(&out, leading_replace).to_string()
}

pub fn format_dollars(text: &str) -> String {
    format_western(text, "$", &["dollars", "dollar", "usd"])
}

pub fn format_euros(text: &str) -> String {
    format_western(text, "€", &["euros", "euro", "eur"])
}

pub fn format_pounds(text: &str) -> String {
    format_western(text, "£", &["pounds", "pound", "gbp"])
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

/// Full currency/percent pass — safe to call on any text. Order matters:
/// rupees first (Indian grouping is incompatible with the Western
/// formatter), then USD/EUR/GBP, then percent.
pub fn apply(text: &str) -> String {
    let s = format_rupees(text);
    let s = format_dollars(&s);
    let s = format_euros(&s);
    let s = format_pounds(&s);
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
