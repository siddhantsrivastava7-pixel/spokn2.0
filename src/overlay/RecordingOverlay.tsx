import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useRef, useState } from "react";
import VoicePillCapsule, { PillState } from "./components/VoicePillCapsule";
import TranscriptionOverlay, {
  OverlayTextState,
} from "./components/TranscriptionOverlay";
import "./RecordingOverlay.css";
import { commands } from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import { getLanguageDirection } from "@/lib/utils/rtl";

type BackendState = "recording" | "transcribing" | "processing";

interface OverlayTextPayload {
  text: string;
  state?: OverlayTextState;
  mode?: string;
}

const mapToPillState = (s: BackendState): PillState =>
  s === "recording" ? "listening" : "processing";

const RecordingOverlay: React.FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [backendState, setBackendState] = useState<BackendState>("recording");
  const [levels, setLevels] = useState<number[]>(Array(9).fill(0));
  const smoothedLevelsRef = useRef<number[]>(Array(9).fill(0));
  const [overlayText, setOverlayText] = useState("");
  const [textState, setTextState] = useState<OverlayTextState>("hidden");
  const [textMode, setTextMode] = useState<string | undefined>(undefined);
  const direction = getLanguageDirection(i18n.language);

  useEffect(() => {
    const cleanupFns: Array<() => void> = [];

    (async () => {
      const unlistenShow = await listen("show-overlay", async (event) => {
        await syncLanguageFromSettings();
        const next = event.payload as BackendState;
        setBackendState(next);
        setIsVisible(true);
      });

      const unlistenHide = await listen("hide-overlay", () => {
        setIsVisible(false);
        // Reset text chip when overlay closes
        setOverlayText("");
        setTextState("hidden");
      });

      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const incoming = event.payload as number[];
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = incoming[i] ?? 0;
          return prev * 0.7 + target * 0.3;
        });
        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, 9));
      });

      /* Future: a backend stream event that feeds streaming text into the
       * TranscriptionOverlay. Harmless if never emitted. */
      const unlistenText = await listen<OverlayTextPayload>(
        "overlay-text",
        (event) => {
          const { text, state, mode } = event.payload;
          setOverlayText(text ?? "");
          setTextState(state ?? (text ? "listening" : "hidden"));
          setTextMode(mode);
        },
      );

      cleanupFns.push(unlistenShow, unlistenHide, unlistenLevel, unlistenText);
    })();

    return () => cleanupFns.forEach((fn) => fn());
  }, []);

  const pillState: PillState = isVisible
    ? mapToPillState(backendState)
    : "done";

  return (
    <div
      dir={direction}
      className={`spokn-overlay-root ${isVisible ? "fade-in" : ""}`}
    >
      {textState !== "hidden" && overlayText && (
        <div className="spokn-overlay-text">
          <TranscriptionOverlay
            text={overlayText}
            state={textState}
            mode={textMode}
          />
        </div>
      )}
      <div className="spokn-overlay-pill">
        <VoicePillCapsule
          state={pillState}
          levels={levels}
          onCancel={() => commands.cancelOperation()}
        />
      </div>
    </div>
  );
};

export default RecordingOverlay;
