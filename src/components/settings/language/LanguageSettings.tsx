import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { LanguagePicker } from "../../ui/LanguagePicker";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { useSettings } from "../../../hooks/useSettings";

export const LanguageSettings: React.FC = () => {
  const { t: _t } = useTranslation();
  const { settings, updateSetting } = useSettings();

  const selected = useMemo(
    () => new Set(settings?.transcription_languages ?? []),
    [settings?.transcription_languages],
  );

  const handleChange = (next: Set<string>) => {
    updateSetting("transcription_languages", Array.from(next));
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
