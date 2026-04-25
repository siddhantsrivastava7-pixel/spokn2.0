import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { LanguagePicker } from "../../ui/LanguagePicker";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { LANGUAGES } from "../../../lib/constants/languages";

export const LanguageSettings: React.FC = () => {
  const { t: _t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  const { models, currentModel, downloadModel, selectModel } = useModelStore();

  const selected = useMemo(
    () => new Set(settings?.transcription_languages ?? []),
    [settings?.transcription_languages],
  );

  const handleChange = (next: Set<string>) => {
    const nextLangs = Array.from(next);
    updateSetting("transcription_languages", nextLangs);
    // Fire-and-forget the auto-switch so the toggle is responsive.
    void autoSwitchModel(nextLangs);
  };

  /**
   * Seamless model switching when the user changes their language list:
   *
   *   English-only Parakeet  →  add Hindi   →  auto-download Whisper
   *                                            (best fit for en+hi +
   *                                             user's hardware tier)
   *   English+Hindi Whisper  →  remove Hindi →  auto-switch back to
   *                                            Parakeet (already
   *                                            installed, no download)
   *
   * The user shouldn't have to know which model supports which
   * language — Spokn picks the right one and handles the download
   * silently with progress feedback.
   */
  const autoSwitchModel = async (nextLangs: string[]) => {
    let recommendedId: string;
    try {
      recommendedId = await commands.recommendModelForLanguages(nextLangs);
    } catch (e) {
      console.warn("recommend_model_for_languages failed:", e);
      // Fallback to the warning-only path so the user is at least informed.
      surfaceCompatibilityWarning(nextLangs);
      return;
    }

    if (!recommendedId || recommendedId === currentModel) {
      // Same model already active — nothing to do.
      return;
    }

    const target = models.find((m) => m.id === recommendedId);
    if (!target) {
      // Unknown id — defensive; surface the warning so the user knows
      // their current model may not cover the new languages.
      surfaceCompatibilityWarning(nextLangs);
      return;
    }

    if (target.is_downloaded) {
      // Already installed — switch silently with a confirmation toast.
      const ok = await selectModel(target.id);
      if (ok) {
        // eslint-disable-next-line i18next/no-literal-string
        toast.success(`Switched to ${target.name} for your languages`);
      }
      return;
    }

    // Need to download. Show progress toast; the model card UI also
    // shows a per-card progress bar.
    const sizeMb = Number(target.size_mb);
    const sizeLabel =
      sizeMb >= 1024
        ? `${(sizeMb / 1024).toFixed(1)} GB`
        : `${sizeMb} MB`;
    // eslint-disable-next-line i18next/no-literal-string
    toast.info(`Downloading ${target.name} (${sizeLabel})…`, {
      // eslint-disable-next-line i18next/no-literal-string
      description:
        "Best model for your selected languages. Spokn will switch to it once ready.",
      duration: 10000,
    });
    const ok = await downloadModel(target.id);
    if (ok) {
      await selectModel(target.id);
      // eslint-disable-next-line i18next/no-literal-string
      toast.success(`${target.name} ready — now active`);
    } else {
      // eslint-disable-next-line i18next/no-literal-string
      toast.error(`Couldn't download ${target.name}`, {
        // eslint-disable-next-line i18next/no-literal-string
        description:
          "Your current model is still active. Try again from Settings → Models, or check your internet connection.",
      });
    }
  };

  /** Last-resort warning when we can't auto-switch (recommendation
   *  failed, or returned a model not in the registry). */
  const surfaceCompatibilityWarning = (nextLangs: string[]) => {
    const active = models.find((m) => m.id === currentModel);
    if (!active) return;
    const missing = nextLangs
      .filter((l) => l && l !== "auto")
      .filter(
        (l) =>
          !active.supported_languages
            .map((s) => s.toLowerCase())
            .includes(l.toLowerCase()),
      )
      .map(
        (code) =>
          LANGUAGES.find((lang) => lang.value.toLowerCase() === code.toLowerCase())
            ?.label || code,
      );
    if (missing.length > 0) {
      // eslint-disable-next-line i18next/no-literal-string
      toast.warning(`${active.name} doesn't support: ${missing.join(", ")}`, {
        // eslint-disable-next-line i18next/no-literal-string
        description:
          "Pick a multilingual model in Settings → Models, or remove the unsupported language(s).",
        duration: 10000,
      });
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title="Languages I dictate in"
        description="Picks a model that covers all of these. Tap to add or remove."
      >
        <div className="p-3 space-y-3">
          <LanguagePicker
            selected={selected}
            onChange={handleChange}
            helpText={
              selected.size === 0
                ? "No languages selected — Spokn will auto-detect."
                : selected.size === 1
                  ? "1 language selected"
                  : `${selected.size} languages selected`
            }
          />
        </div>
      </SettingsGroup>

      <SettingContainer
        title="App interface language"
        description="Language for menus and labels inside Spokn."
        descriptionMode="inline"
      >
        <AppLanguageSelector descriptionMode="tooltip" grouped={false} />
      </SettingContainer>
    </div>
  );
};

export default LanguageSettings;
