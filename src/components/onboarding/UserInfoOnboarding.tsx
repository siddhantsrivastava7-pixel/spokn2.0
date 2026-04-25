import React, { useState } from "react";
import SpoknWordmark from "../icons/SpoknWordmark";
import { commands } from "@/bindings";

interface UserInfoOnboardingProps {
  /** Called once the name is saved (or skipped). */
  onComplete: () => void;
}

/* Welcome-flow name capture. Saved name is used for two things:
 *  1. Seeded into Whisper's custom_words so the spelling stays stable
 *     across transcriptions (no more "Raj" → "Roj" surprises).
 *  2. Auto-signs Email-mode dictations after "Best regards,". */
const UserInfoOnboarding: React.FC<UserInfoOnboardingProps> = ({
  onComplete,
}) => {
  const [first, setFirst] = useState("");
  const [last, setLast] = useState("");
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await commands.setUserName(first.trim(), last.trim());
    } catch (e) {
      console.warn("set_user_name failed", e);
    } finally {
      onComplete();
    }
  };

  const handleSkip = () => onComplete();

  const canContinue = first.trim().length > 0 && !saving;

  return (
    <div
      className="h-screen w-screen flex flex-col items-center justify-center p-6 gap-8 overflow-y-auto"
      style={{ background: "var(--color-spokn-bg)" }}
    >
      {/* eslint-disable i18next/no-literal-string */}
      <div className="flex flex-col items-center gap-3">
        <SpoknWordmark variant="hero" />
        <h1 className="text-spokn-text text-xl font-semibold tracking-tight">
          What should we call you?
        </h1>
        <p className="text-spokn-text-2 text-sm max-w-md text-center">
          Your name keeps spelling consistent across transcriptions and
          auto-signs emails. Stays on this device.
        </p>
      </div>

      <form
        className="flex flex-col gap-3 w-full max-w-sm"
        onSubmit={(e) => {
          e.preventDefault();
          if (canContinue) handleSave();
        }}
      >
        <label className="flex flex-col gap-1.5">
          <span className="text-[11px] font-mono uppercase tracking-wider text-spokn-text-3">
            First name
          </span>
          <input
            type="text"
            autoFocus
            value={first}
            onChange={(e) => setFirst(e.target.value)}
            placeholder="Priya"
            className="bg-spokn-surface border border-spokn-hairline rounded-xl px-4 py-2.5 text-sm text-spokn-text placeholder:text-spokn-text-3 focus:outline-none focus:border-spokn-accent-blue/60 transition-colors"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-[11px] font-mono uppercase tracking-wider text-spokn-text-3">
            Last name (optional)
          </span>
          <input
            type="text"
            value={last}
            onChange={(e) => setLast(e.target.value)}
            placeholder="Sharma"
            className="bg-spokn-surface border border-spokn-hairline rounded-xl px-4 py-2.5 text-sm text-spokn-text placeholder:text-spokn-text-3 focus:outline-none focus:border-spokn-accent-blue/60 transition-colors"
          />
        </label>

        <div className="flex flex-col items-center gap-2 mt-2">
          <button
            type="submit"
            disabled={!canContinue}
            className="w-full px-6 py-2.5 rounded-xl text-sm font-medium text-white transition-all disabled:opacity-40 disabled:cursor-not-allowed"
            style={{ background: "var(--spokn-accent-grad)" }}
          >
            {saving ? "Saving…" : "Continue"}
          </button>
          <button
            type="button"
            onClick={handleSkip}
            disabled={saving}
            className="text-[12px] text-spokn-text-3 hover:text-spokn-text-2 transition-colors"
          >
            Skip for now
          </button>
        </div>
      </form>
      {/* eslint-enable i18next/no-literal-string */}
    </div>
  );
};

export default UserInfoOnboarding;
