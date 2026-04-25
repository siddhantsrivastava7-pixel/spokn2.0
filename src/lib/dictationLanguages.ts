/* All Whisper-supported dictation languages (99 entries) plus a curated
 * "top" subset that defaults to chip-grid display. Both LanguageOnboarding
 * and LanguageSettings consume this so the two screens never drift. */

export interface DictationLanguage {
  code: string; // ISO-639-1 (with Whisper variants like zh-Hans / zh-Hant)
  native: string;
  english: string;
}

export const TOP_LANGUAGES: DictationLanguage[] = [
  { code: "en", native: "English", english: "English" },
  { code: "es", native: "Español", english: "Spanish" },
  { code: "fr", native: "Français", english: "French" },
  { code: "de", native: "Deutsch", english: "German" },
  { code: "hi", native: "हिन्दी", english: "Hindi" },
  { code: "zh", native: "中文", english: "Chinese" },
  { code: "ja", native: "日本語", english: "Japanese" },
  { code: "ko", native: "한국어", english: "Korean" },
  { code: "ar", native: "العربية", english: "Arabic" },
  { code: "pt", native: "Português", english: "Portuguese" },
  { code: "ru", native: "Русский", english: "Russian" },
  { code: "it", native: "Italiano", english: "Italian" },
  { code: "nl", native: "Nederlands", english: "Dutch" },
  { code: "pl", native: "Polski", english: "Polish" },
  { code: "tr", native: "Türkçe", english: "Turkish" },
  { code: "sv", native: "Svenska", english: "Swedish" },
  { code: "uk", native: "Українська", english: "Ukrainian" },
  { code: "id", native: "Bahasa", english: "Indonesian" },
  { code: "vi", native: "Tiếng Việt", english: "Vietnamese" },
  { code: "th", native: "ภาษาไทย", english: "Thai" },
];

/* Full 99-language list (Whisper's tokenizer set). Sorted by English name
 * for predictable scanning in the search expander. Native names use the
 * dominant script; some entries (Latin, Maori, Hawaiian) just repeat the
 * English label since there's no native-script form in common use. */
export const ALL_LANGUAGES: DictationLanguage[] = [
  { code: "af", native: "Afrikaans", english: "Afrikaans" },
  { code: "sq", native: "Shqip", english: "Albanian" },
  { code: "am", native: "አማርኛ", english: "Amharic" },
  { code: "ar", native: "العربية", english: "Arabic" },
  { code: "hy", native: "Հայերեն", english: "Armenian" },
  { code: "as", native: "অসমীয়া", english: "Assamese" },
  { code: "az", native: "Azərbaycan", english: "Azerbaijani" },
  { code: "ba", native: "Башҡортса", english: "Bashkir" },
  { code: "eu", native: "Euskara", english: "Basque" },
  { code: "be", native: "Беларуская", english: "Belarusian" },
  { code: "bn", native: "বাংলা", english: "Bengali" },
  { code: "bs", native: "Bosanski", english: "Bosnian" },
  { code: "br", native: "Brezhoneg", english: "Breton" },
  { code: "bg", native: "Български", english: "Bulgarian" },
  { code: "my", native: "မြန်မာဘာသာ", english: "Burmese" },
  { code: "ca", native: "Català", english: "Catalan" },
  { code: "zh", native: "中文", english: "Chinese" },
  { code: "zh-Hans", native: "简体中文", english: "Chinese (Simplified)" },
  { code: "zh-Hant", native: "繁體中文", english: "Chinese (Traditional)" },
  { code: "yue", native: "粵語", english: "Cantonese" },
  { code: "hr", native: "Hrvatski", english: "Croatian" },
  { code: "cs", native: "Čeština", english: "Czech" },
  { code: "da", native: "Dansk", english: "Danish" },
  { code: "nl", native: "Nederlands", english: "Dutch" },
  { code: "en", native: "English", english: "English" },
  { code: "et", native: "Eesti", english: "Estonian" },
  { code: "fo", native: "Føroyskt", english: "Faroese" },
  { code: "fi", native: "Suomi", english: "Finnish" },
  { code: "fr", native: "Français", english: "French" },
  { code: "gl", native: "Galego", english: "Galician" },
  { code: "ka", native: "ქართული", english: "Georgian" },
  { code: "de", native: "Deutsch", english: "German" },
  { code: "el", native: "Ελληνικά", english: "Greek" },
  { code: "gu", native: "ગુજરાતી", english: "Gujarati" },
  { code: "ht", native: "Kreyòl Ayisyen", english: "Haitian Creole" },
  { code: "ha", native: "Hausa", english: "Hausa" },
  { code: "haw", native: "ʻŌlelo Hawaiʻi", english: "Hawaiian" },
  { code: "he", native: "עברית", english: "Hebrew" },
  { code: "hi", native: "हिन्दी", english: "Hindi" },
  { code: "hu", native: "Magyar", english: "Hungarian" },
  { code: "is", native: "Íslenska", english: "Icelandic" },
  { code: "id", native: "Bahasa Indonesia", english: "Indonesian" },
  { code: "it", native: "Italiano", english: "Italian" },
  { code: "ja", native: "日本語", english: "Japanese" },
  { code: "jw", native: "Basa Jawa", english: "Javanese" },
  { code: "kn", native: "ಕನ್ನಡ", english: "Kannada" },
  { code: "kk", native: "Қазақша", english: "Kazakh" },
  { code: "km", native: "ខ្មែរ", english: "Khmer" },
  { code: "ko", native: "한국어", english: "Korean" },
  { code: "lo", native: "ລາວ", english: "Lao" },
  { code: "la", native: "Latina", english: "Latin" },
  { code: "lv", native: "Latviešu", english: "Latvian" },
  { code: "ln", native: "Lingála", english: "Lingala" },
  { code: "lt", native: "Lietuvių", english: "Lithuanian" },
  { code: "lb", native: "Lëtzebuergesch", english: "Luxembourgish" },
  { code: "mk", native: "Македонски", english: "Macedonian" },
  { code: "mg", native: "Malagasy", english: "Malagasy" },
  { code: "ms", native: "Bahasa Melayu", english: "Malay" },
  { code: "ml", native: "മലയാളം", english: "Malayalam" },
  { code: "mt", native: "Malti", english: "Maltese" },
  { code: "mi", native: "Te Reo Māori", english: "Maori" },
  { code: "mr", native: "मराठी", english: "Marathi" },
  { code: "mn", native: "Монгол", english: "Mongolian" },
  { code: "ne", native: "नेपाली", english: "Nepali" },
  { code: "no", native: "Norsk", english: "Norwegian" },
  { code: "nn", native: "Nynorsk", english: "Norwegian Nynorsk" },
  { code: "oc", native: "Occitan", english: "Occitan" },
  { code: "ps", native: "پښتو", english: "Pashto" },
  { code: "fa", native: "فارسی", english: "Persian" },
  { code: "pl", native: "Polski", english: "Polish" },
  { code: "pt", native: "Português", english: "Portuguese" },
  { code: "pa", native: "ਪੰਜਾਬੀ", english: "Punjabi" },
  { code: "ro", native: "Română", english: "Romanian" },
  { code: "ru", native: "Русский", english: "Russian" },
  { code: "sa", native: "संस्कृतम्", english: "Sanskrit" },
  { code: "sr", native: "Српски", english: "Serbian" },
  { code: "sn", native: "ChiShona", english: "Shona" },
  { code: "sd", native: "سنڌي", english: "Sindhi" },
  { code: "si", native: "සිංහල", english: "Sinhala" },
  { code: "sk", native: "Slovenčina", english: "Slovak" },
  { code: "sl", native: "Slovenščina", english: "Slovenian" },
  { code: "so", native: "Soomaali", english: "Somali" },
  { code: "es", native: "Español", english: "Spanish" },
  { code: "su", native: "Basa Sunda", english: "Sundanese" },
  { code: "sw", native: "Kiswahili", english: "Swahili" },
  { code: "sv", native: "Svenska", english: "Swedish" },
  { code: "tl", native: "Tagalog", english: "Tagalog" },
  { code: "tg", native: "Тоҷикӣ", english: "Tajik" },
  { code: "ta", native: "தமிழ்", english: "Tamil" },
  { code: "tt", native: "Татарча", english: "Tatar" },
  { code: "te", native: "తెలుగు", english: "Telugu" },
  { code: "th", native: "ภาษาไทย", english: "Thai" },
  { code: "bo", native: "བོད་ཡིག", english: "Tibetan" },
  { code: "tr", native: "Türkçe", english: "Turkish" },
  { code: "tk", native: "Türkmen", english: "Turkmen" },
  { code: "uk", native: "Українська", english: "Ukrainian" },
  { code: "ur", native: "اردو", english: "Urdu" },
  { code: "uz", native: "Oʻzbek", english: "Uzbek" },
  { code: "vi", native: "Tiếng Việt", english: "Vietnamese" },
  { code: "cy", native: "Cymraeg", english: "Welsh" },
  { code: "yi", native: "ייִדיש", english: "Yiddish" },
  { code: "yo", native: "Yorùbá", english: "Yoruba" },
];

/** Lookup a language entry by code (case-insensitive, BCP47-friendly). */
export function findLanguage(code: string): DictationLanguage | undefined {
  const lower = code.toLowerCase();
  return ALL_LANGUAGES.find((l) => l.code.toLowerCase() === lower);
}

/** True if `code` is in the curated chip-grid set. */
export function isTopLanguage(code: string): boolean {
  const lower = code.toLowerCase();
  return TOP_LANGUAGES.some((l) => l.code.toLowerCase() === lower);
}
