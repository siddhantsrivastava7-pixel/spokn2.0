import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { useSettings } from "../../../hooks/useSettings";

/* Behavior — three lifestyle toggles. Each maps to an existing backend
 * setting but uses plain-English labels:
 *
 *   Start on login              ↔ autostart_enabled
 *   Keep running in background  ↔ start_hidden  (also forces tray icon)
 *   Auto-send messages          ↔ auto_submit
 */
export const BehaviorSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const autostart = getSetting("autostart_enabled") ?? false;
  const runInBackground = getSetting("start_hidden") ?? false;
  const autoSend = getSetting("auto_submit") ?? false;

  const handleRunInBackground = (next: boolean) => {
    updateSetting("start_hidden", next);
    // If user wants Spokn to live in the background, the tray icon is the
    // only way back into the app — force it on too.
    if (next && !(getSetting("show_tray_icon") ?? true)) {
      updateSetting("show_tray_icon", true);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("simplified.behavior.title")}>
        <ToggleSwitch
          checked={autostart}
          onChange={(v) => updateSetting("autostart_enabled", v)}
          isUpdating={isUpdating("autostart_enabled")}
          label={t("simplified.behavior.startOnLogin")}
          description={t("simplified.behavior.startOnLoginHint")}
          descriptionMode="inline"
          grouped={true}
        />
        <ToggleSwitch
          checked={runInBackground}
          onChange={handleRunInBackground}
          isUpdating={isUpdating("start_hidden")}
          label={t("simplified.behavior.runInBackground")}
          description={t("simplified.behavior.runInBackgroundHint")}
          descriptionMode="inline"
          grouped={true}
        />
        <ToggleSwitch
          checked={autoSend}
          onChange={(v) => updateSetting("auto_submit", v)}
          isUpdating={isUpdating("auto_submit")}
          label={t("simplified.behavior.autoSend")}
          description={t("simplified.behavior.autoSendHint")}
          descriptionMode="inline"
          grouped={true}
        />
      </SettingsGroup>
    </div>
  );
};
