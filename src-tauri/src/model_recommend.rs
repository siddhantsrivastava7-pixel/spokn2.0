//! Picks the best model for a given set of user-selected languages and the
//! detected hardware tier. Keeps the logic in one readable table so it's
//! trivially auditable.

use crate::hardware::HardwareInfo;

const PARAKEET_V3_LANGS: &[&str] = &[
    "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt",
    "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk",
];

/// Given a list of desired language codes (lowercase ISO-639-1 or
/// BCP-47 variants like "zh-Hans") and the detected hardware tier,
/// returns the best matching model ID.
pub fn recommend(languages: &[String], hw: &HardwareInfo) -> String {
    if languages.is_empty() {
        // No language picked — stay safe with the multi-lingual default.
        return "parakeet-tdt-0.6b-v3".into();
    }

    let normalized: Vec<String> = languages
        .iter()
        .map(|l| l.split('-').next().unwrap_or(l).to_lowercase())
        .collect();

    let only_english = normalized.iter().all(|l| l == "en");
    let all_parakeet_v3 = normalized
        .iter()
        .all(|l| PARAKEET_V3_LANGS.contains(&l.as_str()));
    let contains_chinese = normalized.iter().any(|l| l == "zh");

    // English only -> specialist English models
    if only_english {
        return match hw.tier.as_str() {
            "high" => "parakeet-tdt-0.6b-v2".into(), // best English accuracy + fast
            "mid" => "parakeet-tdt-0.6b-v2".into(),
            _ => "moonshine-base".into(), // 55MB fallback for low-end
        };
    }

    // All selections covered by Parakeet V3's 25-language menu
    if all_parakeet_v3 {
        return "parakeet-tdt-0.6b-v3".into();
    }

    // Non-Parakeet-V3 languages → Whisper family, tiered by hardware.
    let contains_hindi = normalized.iter().any(|l| l == "hi");

    if contains_chinese && hw.tier != "low" {
        // Still default Whisper — Breeze ASR is Taiwan-Mandarin-specific
        // and the average user just wants generic Chinese support.
    }

    // Hindi (and by extension Hinglish code-switching) is meaningfully
    // worse on Whisper Turbo — Turbo's speed-tuned decoder dropped some
    // multilingual nuance. Push Hindi-including selections to Large on
    // high-tier hardware where the speed delta is acceptable.
    if contains_hindi && hw.tier == "high" {
        return "large".into();
    }

    match hw.tier.as_str() {
        "high" => "turbo".into(),  // 1.5GB, best general-purpose multilingual
        "mid" => "medium".into(),  // 469MB, solid middle ground
        _ => "small".into(),       // 465MB, safe for low-end hardware
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(tier: &str) -> HardwareInfo {
        HardwareInfo {
            platform: "macos".into(),
            arch: "aarch64".into(),
            ram_gb: 16,
            is_apple_silicon: true,
            tier: tier.into(),
        }
    }

    #[test]
    fn english_only_high_end_picks_parakeet_v2() {
        assert_eq!(
            recommend(&["en".to_string()], &hw("high")),
            "parakeet-tdt-0.6b-v2"
        );
    }

    #[test]
    fn english_only_low_end_picks_moonshine() {
        assert_eq!(
            recommend(&["en".to_string()], &hw("low")),
            "moonshine-base"
        );
    }

    #[test]
    fn european_set_picks_parakeet_v3() {
        assert_eq!(
            recommend(
                &["en".to_string(), "fr".to_string(), "de".to_string()],
                &hw("high")
            ),
            "parakeet-tdt-0.6b-v3"
        );
    }

    #[test]
    fn hindi_plus_english_picks_whisper_large_on_high_tier() {
        // Hinglish code-switching is materially better on Large vs Turbo;
        // we trade some speed for accuracy when Hindi is in the picture.
        assert_eq!(
            recommend(
                &["en".to_string(), "hi".to_string()],
                &hw("high")
            ),
            "large"
        );
    }

    #[test]
    fn non_hindi_high_tier_stays_on_turbo() {
        assert_eq!(
            recommend(&["ja".to_string()], &hw("high")),
            "turbo"
        );
    }

    #[test]
    fn hindi_plus_english_picks_medium_on_mid_hw() {
        assert_eq!(
            recommend(
                &["en".to_string(), "hi".to_string()],
                &hw("mid")
            ),
            "medium"
        );
    }

    #[test]
    fn japanese_low_hw_picks_small() {
        assert_eq!(
            recommend(&["ja".to_string()], &hw("low")),
            "small"
        );
    }

    #[test]
    fn empty_languages_falls_back_to_parakeet_v3() {
        assert_eq!(recommend(&[], &hw("high")), "parakeet-tdt-0.6b-v3");
    }

    #[test]
    fn bcp47_variants_normalise() {
        assert_eq!(
            recommend(&["zh-Hans".to_string()], &hw("high")),
            "turbo"
        );
    }
}
