import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { PushToTalk } from "../PushToTalk";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { ModelSettingsCard } from "./ModelSettingsCard";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const pushToTalk = getSetting("push_to_talk");
  const isLinux = type() === "linux";
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.general.title")}>
        <ShortcutInput
          shortcutId="transcribe"
          descriptionMode="inline"
          grouped={true}
        />
        <PushToTalk descriptionMode="inline" grouped={true} />
        {/* Cancel shortcut is hidden with push-to-talk (release key cancels) and on Linux (dynamic shortcut instability) */}
        {!isLinux && !pushToTalk && (
          <ShortcutInput
            shortcutId="cancel"
            descriptionMode="inline"
            grouped={true}
          />
        )}
      </SettingsGroup>
      <ModelSettingsCard />
      <SettingsGroup title={t("settings.sound.title")}>
        <MicrophoneSelector descriptionMode="inline" grouped={true} />
        <MuteWhileRecording descriptionMode="inline" grouped={true} />
        <AudioFeedback descriptionMode="inline" grouped={true} />
        <OutputDeviceSelector
          descriptionMode="inline"
          grouped={true}
          disabled={!audioFeedbackEnabled}
        />
        <VolumeSlider
          descriptionMode="inline"
          disabled={!audioFeedbackEnabled}
        />
      </SettingsGroup>
    </div>
  );
};
