import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
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
  const { models, currentModel } = useModelStore();

  const selected = useMemo(
    () => new Set(settings?.transcription_languages ?? []),
    [settings?.transcription_languages],
  );

  const handleChange = (next: Set<string>) => {
    const nextLangs = Array.from(next);
    updateSetting("transcription_languages", nextLangs);

    // Compatibility nudge: if the user's currently active model
    // doesn't support all of their newly-selected languages, surface
    // a one-shot toast so they don't silently get bad transcription.
    const active = models.find((m) => m.id === currentModel);
    if (active) {
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
        toast.warning(
          `${active.name} doesn't support: ${missing.join(", ")}`,
          {
            // eslint-disable-next-line i18next/no-literal-string
            description:
              "Pick a multilingual model in Settings → Models, or remove the unsupported language(s) from your list.",
            duration: 10000,
          },
        );
      }
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
