import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type as platformType } from "@tauri-apps/plugin-os";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";

interface ProgressPayload {
  collected: number;
  required: number;
}

interface DonePayload {
  threshold: number;
  noise_floor: number;
  avg_tap_peak: number;
}

/* Knock Mode toggle + calibration. Lives inside GeneralSettings (no
 * dedicated settings page). On non-macOS the toggle is shown disabled
 * with a "macOS only" hint so the feature is discoverable but
 * unactionable. */
export const KnockMode: React.FC = () => {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const isMac = platformType() === "macos";

  const enabled = (settings as any)?.knock_mode_enabled ?? false;
  const calibrationCompleted =
    (settings as any)?.knock_calibration_completed ?? false;

  const [busy, setBusy] = useState(false);
  const [calibrating, setCalibrating] = useState(false);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);

  // Listen for calibration progress events from Rust. We always
  // subscribe (cheap) and only render when calibration is active.
  useEffect(() => {
    const unsubProgress = listen<ProgressPayload>(
      "knock-calibration-progress",
      (e) => setProgress(e.payload),
    );
    const unsubDone = listen<DonePayload>("knock-calibration-done", (e) => {
      setCalibrating(false);
      setProgress(null);
      // eslint-disable-next-line i18next/no-literal-string
      toast.success(
        `Calibration done. Threshold ${e.payload.threshold.toFixed(3)}`,
      );
      void refreshSettings();
    });
    return () => {
      void unsubProgress.then((fn) => fn());
      void unsubDone.then((fn) => fn());
    };
  }, [refreshSettings]);

  const toggle = async (v: boolean) => {
    setBusy(true);
    try {
      const r = await commands.setKnockModeEnabled(v);
      if ((r as any).status === "error") {
        toast.error((r as any).error);
      } else {
        await refreshSettings();
      }
    } finally {
      setBusy(false);
    }
  };

  const startCalibration = async () => {
    setCalibrating(true);
    setProgress({ collected: 0, required: 3 });
    const r = await commands.startKnockCalibration();
    if ((r as any).status === "error") {
      setCalibrating(false);
      setProgress(null);
      toast.error((r as any).error);
    }
  };

  const cancelCalibration = async () => {
    await commands.cancelKnockCalibration();
    setCalibrating(false);
    setProgress(null);
  };

  if (!isMac) {
    return (
      <SettingContainer
        title={t("simplified.general.knockTitle")}
        description={t("simplified.general.knockMacOnly")}
        descriptionMode="inline"
        grouped={true}
      >
        <ToggleSwitch
          checked={false}
          onChange={() => {}}
          disabled
          label=""
          description=""
          grouped={true}
        />
      </SettingContainer>
    );
  }

  return (
    <>
      <ToggleSwitch
        checked={enabled}
        onChange={toggle}
        isUpdating={busy}
        label={t("simplified.general.knockTitle")}
        description={t("simplified.general.knockHint")}
        descriptionMode="inline"
        grouped={true}
      />
      {enabled && (
        <SettingContainer
          title={
            calibrationCompleted
              ? t("simplified.general.knockRecalibrate")
              : t("simplified.general.knockCalibrate")
          }
          description={
            calibrating
              ? t("simplified.general.knockCalibratingHint", {
                  collected: progress?.collected ?? 0,
                  required: progress?.required ?? 3,
                })
              : t("simplified.general.knockCalibrateHint")
          }
          descriptionMode="inline"
          grouped={true}
        >
          {calibrating ? (
            <Button
              variant="danger-ghost"
              size="sm"
              onClick={cancelCalibration}
            >
              {t("simplified.general.knockCancel")}
            </Button>
          ) : (
            <Button variant="primary" size="sm" onClick={startCalibration}>
              {calibrationCompleted
                ? t("simplified.general.knockRecalibrateBtn")
                : t("simplified.general.knockCalibrateBtn")}
            </Button>
          )}
        </SettingContainer>
      )}
    </>
  );
};

export default KnockMode;
