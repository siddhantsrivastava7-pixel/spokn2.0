import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { useSettings } from "../../../hooks/useSettings";

/* Voice & Audio — the absolute basics most users want to confirm:
 *   - which mic
 *   - whether Spokn beeps when recording starts/stops
 * Output device + volume + sound theme live under Settings (Advanced). */
export const VoiceAudioSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const audioFeedback = getSetting("audio_feedback") ?? false;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("simplified.voiceAudio.title")}>
        <MicrophoneSelector descriptionMode="inline" grouped={true} />
        <ToggleSwitch
          checked={audioFeedback}
          onChange={(v) => updateSetting("audio_feedback", v)}
          isUpdating={isUpdating("audio_feedback")}
          label={t("simplified.voiceAudio.playSound")}
          description={t("simplified.voiceAudio.playSoundHint")}
          descriptionMode="inline"
          grouped={true}
        />
      </SettingsGroup>
    </div>
  );
};
