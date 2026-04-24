import React, { useEffect, useMemo, useState } from "react";
import SpoknWordmark from "../icons/SpoknWordmark";
import { commands } from "@/bindings";

interface LanguageOption {
  code: string;
  native: string;
  english: string;
}

/* Curated list — the 20 most-commonly-requested languages. Users with
 * something more obscure can still pick multiple neighbours or change in
 * Advanced settings later. Intentionally shorter than Whisper's 99 to keep
 * the screen scannable. */
const LANGUAGES: LanguageOption[] = [
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

interface LanguageOnboardingProps {
  /** Called with the resolved model ID once languages are picked. */
  onComplete: (modelId: string) => void;
}

const LanguageOnboarding: React.FC<LanguageOnboardingProps> = ({ onComplete }) => {
  const [selected, setSelected] = useState<Set<string>>(new Set(["en"]));
  const [resolving, setResolving] = useState(false);

  // Prefetch hardware info so we can tease the tier in the footer.
  const [tier, setTier] = useState<string | null>(null);
  useEffect(() => {
    commands.detectHardware().then((hw) => setTier(hw.tier)).catch(() => {});
  }, []);

  const canContinue = selected.size > 0 && !resolving;

  const toggle = (code: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(code)) next.delete(code);
      else next.add(code);
      return next;
    });
  };

  const handleContinue = async () => {
    if (!canContinue) return;
    setResolving(true);
    try {
      const modelId = await commands.recommendModelForLanguages(
        Array.from(selected),
      );
      onComplete(modelId);
    } catch (e) {
      console.error("recommend_model_for_languages failed", e);
      setResolving(false);
    }
  };

  const summary = useMemo(() => {
    if (selected.size === 0) return "Pick at least one language";
    if (selected.size === 1) return "1 language selected";
    return `${selected.size} languages selected`;
  }, [selected]);

  return (
    <div
      className="h-screen w-screen flex flex-col items-center justify-center p-6 gap-8"
      style={{ background: "var(--color-spokn-bg)" }}
    >
      {/* eslint-disable i18next/no-literal-string */}
      <div className="flex flex-col items-center gap-3">
        <SpoknWordmark variant="hero" />
        <p className="text-spokn-text-2 text-sm max-w-md text-center">
          Which languages do you dictate in? Pick all that apply — we'll
          choose the right model for your hardware.
        </p>
      </div>

      <div className="grid grid-cols-4 gap-2 max-w-2xl w-full">
        {LANGUAGES.map((lang) => {
          const isSelected = selected.has(lang.code);
          return (
            <button
              key={lang.code}
              type="button"
              onClick={() => toggle(lang.code)}
              className={`group relative flex flex-col items-start gap-0.5 rounded-xl border px-3 py-2.5 text-left transition-all duration-150 cursor-pointer ${
                isSelected
                  ? "border-spokn-accent-blue/60 bg-spokn-accent-blue/10"
                  : "border-spokn-hairline bg-spokn-surface hover:bg-spokn-surface-2 hover:border-spokn-hairline-2"
              }`}
            >
              <span
                className={`text-sm font-medium ${
                  isSelected ? "text-spokn-text" : "text-spokn-text"
                }`}
              >
                {lang.native}
              </span>
              <span className="text-[11px] text-spokn-text-3 uppercase tracking-wider font-mono">
                {lang.english}
              </span>
              {isSelected && (
                <span
                  aria-hidden
                  className="absolute top-2 right-2 w-1.5 h-1.5 rounded-full"
                  style={{ background: "var(--spokn-accent-grad)" }}
                />
              )}
            </button>
          );
        })}
      </div>

      <div className="flex flex-col items-center gap-3">
        <button
          type="button"
          onClick={handleContinue}
          disabled={!canContinue}
          className="px-6 py-2.5 rounded-xl text-sm font-medium text-white transition-all disabled:opacity-40 disabled:cursor-not-allowed"
          style={{ background: "var(--spokn-accent-grad)" }}
        >
          {resolving ? "Choosing model…" : "Continue"}
        </button>
        <p className="text-[11px] text-spokn-text-3 font-mono tracking-wider uppercase">
          {summary}
          {tier && (
            <>
              {" · "}
              {tier === "high" ? "Apple Silicon" : tier} tier
            </>
          )}
        </p>
      </div>
      {/* eslint-enable i18next/no-literal-string */}
    </div>
  );
};

export default LanguageOnboarding;
