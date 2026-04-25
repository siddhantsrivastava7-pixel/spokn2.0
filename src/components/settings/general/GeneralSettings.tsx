import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { toast } from "sonner";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { SegmentedControl } from "../../ui/SegmentedControl";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { useSettings } from "../../../hooks/useSettings";
import { commands, type SmartFormattingMode } from "@/bindings";
import { KnockMode } from "./KnockMode";
import { ConversationMode } from "./ConversationMode";

const SF_MODES: { value: SmartFormattingMode; label: string }[] = [
  { value: "smart", label: "Smart" },
  { value: "raw", label: "Raw" },
  { value: "email", label: "Email" },
  { value: "message", label: "Message" },
];

/* General — recording shortcut + recording mode + Spokn formatting mode.
 *
 * Combines two intent-related groups:
 *  1. Recording   → shortcut, hold-vs-tap mode, optional cancel shortcut
 *  2. Smart Spokn → which formatting profile, on/off, app-aware override
 *
 * The Smart formatting controls used to live in the Quick Settings popover
 * but consumer users were missing them — surfacing here makes them
 * discoverable. */
export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings, getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const isLinux = type() === "linux";

  const pushToTalk = getSetting("push_to_talk") ?? true;
  const mode: "hold" | "tap" = pushToTalk ? "hold" : "tap";

  const sfMode: SmartFormattingMode = settings?.smart_formatting_mode ?? "smart";
  const sfEnabled = settings?.smart_formatting_enabled ?? true;
  const sfAppAware = settings?.smart_formatting_app_aware ?? true;

  // Local draft for the name fields. We don't push every keystroke to
  // disk — names are paired (first + last) and trigger custom_words
  // seeding, so we save on blur / explicit Save instead.
  const storedFirst = (settings as any)?.user_first_name ?? "";
  const storedLast = (settings as any)?.user_last_name ?? "";
  const [first, setFirst] = useState<string>(storedFirst);
  const [last, setLast] = useState<string>(storedLast);
  const [savingName, setSavingName] = useState(false);
  // Re-sync local draft when stored values change (e.g., after onboarding
  // saved a name and the settings get refreshed).
  useEffect(() => {
    setFirst(storedFirst);
    setLast(storedLast);
  }, [storedFirst, storedLast]);
  const nameDirty = first.trim() !== storedFirst || last.trim() !== storedLast;

  const saveName = async () => {
    setSavingName(true);
    try {
      const r = await commands.setUserName(first.trim(), last.trim());
      if ((r as any).status === "error") {
        toast.error((r as any).error);
      } else {
        toast.success(t("simplified.general.nameSaved"));
        await refreshSettings();
      }
    } finally {
      setSavingName(false);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("simplified.general.nameTitle")}>
        <SettingContainer
          title={t("simplified.general.firstNameLabel")}
          description={t("simplified.general.nameHint")}
          descriptionMode="inline"
          grouped={true}
        >
          <Input
            type="text"
            value={first}
            onChange={(e) => setFirst(e.target.value)}
            placeholder={t("simplified.general.firstNamePlaceholder")}
            className="w-44"
          />
        </SettingContainer>
        <SettingContainer
          title={t("simplified.general.lastNameLabel")}
          description=""
          descriptionMode="inline"
          grouped={true}
        >
          <Input
            type="text"
            value={last}
            onChange={(e) => setLast(e.target.value)}
            placeholder={t("simplified.general.lastNamePlaceholder")}
            className="w-44"
          />
        </SettingContainer>
        {nameDirty && (
          <div className="flex justify-end pt-1">
            <Button
              variant="primary"
              size="sm"
              onClick={saveName}
              disabled={savingName || first.trim().length === 0}
            >
              {savingName
                ? t("simplified.general.saving")
                : t("simplified.general.saveName")}
            </Button>
          </div>
        )}
      </SettingsGroup>

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
        {!isLinux && mode === "tap" && (
          <ShortcutInput
            shortcutId="cancel"
            descriptionMode="inline"
            grouped={true}
          />
        )}
        <KnockMode />
        <ConversationMode />
      </SettingsGroup>

      <SettingsGroup title={t("simplified.general.smartTitle")}>
        <SettingContainer
          title={t("simplified.general.smartModeLabel")}
          description={t("simplified.general.smartModeHint")}
          descriptionMode="inline"
          grouped={true}
        >
          <SegmentedControl<SmartFormattingMode>
            value={sfMode}
            onChange={(v) => updateSetting("smart_formatting_mode", v)}
            options={SF_MODES}
            ariaLabel={t("simplified.general.smartModeLabel")}
            disabled={!sfEnabled}
          />
        </SettingContainer>
        <ToggleSwitch
          checked={sfEnabled}
          onChange={(v) => updateSetting("smart_formatting_enabled", v)}
          isUpdating={isUpdating("smart_formatting_enabled")}
          label={t("simplified.general.smartEnabledLabel")}
          description={t("simplified.general.smartEnabledHint")}
          descriptionMode="inline"
          grouped={true}
        />
        <ToggleSwitch
          checked={sfAppAware}
          onChange={(v) => updateSetting("smart_formatting_app_aware", v)}
          isUpdating={isUpdating("smart_formatting_app_aware")}
          label={t("simplified.general.appAwareLabel")}
          description={t("simplified.general.appAwareHint")}
          descriptionMode="inline"
          grouped={true}
          disabled={!sfEnabled}
        />
      </SettingsGroup>
    </div>
  );
};
