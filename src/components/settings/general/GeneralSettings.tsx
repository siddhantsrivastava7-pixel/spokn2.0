import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { SegmentedControl } from "../../ui/SegmentedControl";
import { useSettings } from "../../../hooks/useSettings";

/* Simplified General — just two controls.
 *
 * "Mode" replaces the engineer-y "Push To Talk" toggle with a clear
 * binary segmented control. Cancel shortcut still surfaces inline when
 * the user picks Tap mode (since release-to-stop no longer applies). */
export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const isLinux = type() === "linux";
  const pushToTalk = getSetting("push_to_talk") ?? true;
  const mode: "hold" | "tap" = pushToTalk ? "hold" : "tap";

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("simplified.general.title")}>
        <ShortcutInput
          shortcutId="transcribe"
          descriptionMode="inline"
          grouped={true}
        />
        <SettingContainer
          title={t("simplified.general.modeLabel")}
          description={
            mode === "hold"
              ? t("simplified.general.modeHoldHint")
              : t("simplified.general.modeTapHint")
          }
          descriptionMode="inline"
          grouped={true}
        >
          <SegmentedControl<"hold" | "tap">
            value={mode}
            onChange={(v) => updateSetting("push_to_talk", v === "hold")}
            options={[
              {
                value: "hold",
                label: t("simplified.general.modeHold"),
              },
              {
                value: "tap",
                label: t("simplified.general.modeTap"),
              },
            ]}
            ariaLabel={t("simplified.general.modeLabel")}
          />
        </SettingContainer>
        {/* Cancel shortcut only matters in Tap mode — in Hold mode, releasing
         * the shortcut key already cancels. Linux suppressed because dynamic
         * shortcut updates are unstable there. */}
        {!isLinux && mode === "tap" && (
          <ShortcutInput
            shortcutId="cancel"
            descriptionMode="inline"
            grouped={true}
          />
        )}
      </SettingsGroup>
    </div>
  );
};
