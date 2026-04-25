import React, { useEffect, useMemo, useState } from "react";
import SpoknWordmark from "../icons/SpoknWordmark";
import { LanguagePicker } from "../ui/LanguagePicker";
import { commands } from "@/bindings";

interface LanguageOnboardingProps {
  /** Called with the resolved model ID once languages are picked. */
  onComplete: (modelId: string) => void;
}

const LanguageOnboarding: React.FC<LanguageOnboardingProps> = ({ onComplete }) => {
  // Seed the selection with English plus any languages we can infer from
  // the user's system locale. Hinglish is the obvious example: someone on
  // an Indian English (en-IN) or Hindi locale almost certainly speaks
  // some Hindi too, so pre-checking it removes a discovery step.
  const [selected, setSelected] = useState<Set<string>>(() => {
    const initial = new Set<string>(["en"]);
    if (typeof navigator !== "undefined") {
      const loc = (navigator.language || "").toLowerCase();
      if (loc.startsWith("hi") || loc === "en-in") {
        initial.add("hi");
      }
    }
    return initial;
  });
  const [resolving, setResolving] = useState(false);

  // Prefetch hardware info so we can tease the tier in the footer.
  const [tier, setTier] = useState<string | null>(null);
  useEffect(() => {
    commands.detectHardware().then((hw) => setTier(hw.tier)).catch(() => {});
  }, []);

  const canContinue = selected.size > 0 && !resolving;

  const handleContinue = async () => {
    if (!canContinue) return;
    setResolving(true);
    try {
      const langs = Array.from(selected);
      // Persist the user's choice so the Language settings page can show
      // and edit it later. Fire-and-forget — model recommendation doesn't
      // need to wait on the store write.
      commands.changeTranscriptionLanguagesSetting(langs).catch(() => {});
      const modelId = await commands.recommendModelForLanguages(langs);
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
      className="h-screen w-screen flex flex-col items-center justify-center p-6 gap-8 overflow-y-auto"
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

      <div className="max-w-2xl w-full">
        <LanguagePicker selected={selected} onChange={setSelected} />
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
