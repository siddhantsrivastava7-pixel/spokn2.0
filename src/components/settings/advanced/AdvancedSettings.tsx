import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { CollapsibleGroup } from "../../ui/CollapsibleGroup";
import { useSettings } from "../../../hooks/useSettings";

// Recording / output / audio
import { ShortcutInput } from "../ShortcutInput";
import { AppendTrailingSpace } from "../AppendTrailingSpace";
import { CustomWords } from "../CustomWords";
import { PasteMethodSetting } from "../PasteMethod";
import { TypingToolSetting } from "../TypingTool";
import { ClipboardHandlingSetting } from "../ClipboardHandling";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { VolumeSlider } from "../VolumeSlider";
import { SoundPicker } from "../SoundPicker";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { AlwaysOnMicrophone } from "../AlwaysOnMicrophone";
import { ClamshellMicrophoneSelector } from "../ClamshellMicrophoneSelector";

// Memory / overlay / files
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { ShowOverlay } from "../ShowOverlay";
import { ShowTrayIcon } from "../ShowTrayIcon";
import { AppDataDirectory } from "../AppDataDirectory";
import { LogDirectory, LogLevelSelector, PasteDelay, RecordingBuffer, WordCorrectionThreshold } from "../debug";
import { RecordingRetentionPeriodSelector } from "../RecordingRetentionPeriod";
import { HistoryLimit } from "../HistoryLimit";

// Updates / language / experimental
import { UpdateChecksToggle } from "../UpdateChecksToggle";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { TranslateToEnglish } from "../TranslateToEnglish";
import { PostProcessingToggle } from "../PostProcessingToggle";
import { ExperimentalToggle } from "../ExperimentalToggle";
import { KeyboardImplementationSelector } from "../debug/KeyboardImplementationSelector";
import { AccelerationSelector } from "../AccelerationSelector";
import { LazyStreamClose } from "../LazyStreamClose";

/* The Advanced ("Settings") page — a single scrollable list of collapsible
 * groups. Each group is closed by default; users open only what they care
 * about. Nothing here is required for the app to work — these are tuning
 * knobs that the simplified Simple-mode sidebar deliberately hides. */
export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const isLinux = type() === "linux";
  const experimentalEnabled = getSetting("experimental_enabled") || false;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-4">
      <div className="px-1">
        <h1 className="text-lg font-semibold text-spokn-text tracking-tight">
          {/* eslint-disable-next-line i18next/no-literal-string */}
          {t("simplified.advancedPage.title")}
        </h1>
        <p className="mt-1 text-[13px] text-spokn-text-2 max-w-lg">
          {/* eslint-disable-next-line i18next/no-literal-string */}
          {t("simplified.advancedPage.intro")}
        </p>
      </div>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.recording")}>
        <ShortcutInput shortcutId="cancel" descriptionMode="tooltip" grouped />
        <AppendTrailingSpace descriptionMode="tooltip" grouped />
        <CustomWords descriptionMode="tooltip" grouped />
        <WordCorrectionThreshold descriptionMode="tooltip" grouped />
        <RecordingBuffer descriptionMode="tooltip" grouped />
        <TranslateToEnglish descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.output")}>
        <PasteMethodSetting descriptionMode="tooltip" grouped />
        {isLinux && <TypingToolSetting descriptionMode="tooltip" grouped />}
        <ClipboardHandlingSetting descriptionMode="tooltip" grouped />
        <PasteDelay descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.audio")}>
        <MuteWhileRecording descriptionMode="tooltip" grouped />
        <SoundPicker
          label={t("settings.debug.soundTheme.label")}
          description={t("settings.debug.soundTheme.description")}
        />
        <OutputDeviceSelector descriptionMode="tooltip" grouped />
        <VolumeSlider descriptionMode="tooltip" />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.microphone")}>
        <AlwaysOnMicrophone descriptionMode="tooltip" grouped />
        <ClamshellMicrophoneSelector descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.memory")}>
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.overlay")}>
        <ShowOverlay descriptionMode="tooltip" grouped />
        <ShowTrayIcon descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.files")}>
        <HistoryLimit descriptionMode="tooltip" grouped />
        <RecordingRetentionPeriodSelector descriptionMode="tooltip" grouped />
        <AppDataDirectory descriptionMode="tooltip" grouped />
        <LogDirectory grouped />
        <LogLevelSelector grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.ai")}>
        <PostProcessingToggle descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.language")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped />
      </CollapsibleGroup>

      <CollapsibleGroup title={t("simplified.advancedPage.groups.experimental")}>
        <UpdateChecksToggle descriptionMode="tooltip" grouped />
        <ExperimentalToggle descriptionMode="tooltip" grouped />
        {experimentalEnabled && (
          <>
            <KeyboardImplementationSelector descriptionMode="tooltip" grouped />
            <AccelerationSelector descriptionMode="tooltip" grouped />
            <LazyStreamClose descriptionMode="tooltip" grouped />
          </>
        )}
      </CollapsibleGroup>
    </div>
  );
};
