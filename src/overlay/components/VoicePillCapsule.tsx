import React from "react";
import Waveform from "./Waveform";

export type PillState = "idle" | "listening" | "processing" | "done";

interface VoicePillCapsuleProps {
  state: PillState;
  levels: number[];
  onCancel?: () => void;
}

const ACCENT_GRAD =
  "linear-gradient(135deg, #4F7CFF 0%, #8B5CF6 100%)";

/* Morphing capsule pill.
 *   idle       36×36 orb, dim inner dot (rarely visible; overlay normally hides)
 *   listening  80×36 capsule, accent inner disk, live waveform
 *   processing 72×36 capsule, pulsing accent dot
 *   done       fades out
 *
 * All transitions are 220ms on the prototype's cubic-bezier(0.22,1,0.36,1)
 * via the --spokn-ease var loaded by spokn-tokens.css. */
const VoicePillCapsule: React.FC<VoicePillCapsuleProps> = ({
  state,
  levels,
  onCancel,
}) => {
  const listening = state === "listening";
  const processing = state === "processing";
  const done = state === "done";
  const idle = state === "idle";

  const H = 36;
  const width = idle
    ? 36
    : listening
      ? 96
      : processing
        ? 72
        : 36;

  const radius = idle ? 999 : 22;

  return (
    <div
      className="spokn group"
      style={{
        position: "relative",
        height: H,
        width,
        borderRadius: radius,
        background: "rgba(12,12,14,0.78)",
        backdropFilter: "blur(14px) saturate(1.2)",
        WebkitBackdropFilter: "blur(14px) saturate(1.2)",
        border: "1px solid rgba(255,255,255,0.07)",
        boxShadow: "0 4px 14px rgba(0,0,0,0.32)",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "flex-start",
        overflow: "hidden",
        opacity: done ? 0 : 1,
        transition:
          "width 220ms var(--spokn-ease), border-radius 220ms var(--spokn-ease), opacity 320ms var(--spokn-ease), background 220ms var(--spokn-ease)",
      }}
    >
      {/* Breathing glow behind the pill */}
      <span
        aria-hidden
        style={{
          position: "absolute",
          inset: -14,
          borderRadius: 999,
          background: ACCENT_GRAD,
          filter: "blur(26px)",
          opacity: listening ? 0.2 : 0,
          animation: listening
            ? "spokn-breathe 3.2s ease-in-out infinite"
            : "none",
          transition: "opacity var(--spokn-dur-slow) var(--spokn-ease)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />

      {/* Inner accent disk */}
      <span
        aria-hidden
        style={{
          position: "absolute",
          left: idle ? "50%" : 11,
          top: "50%",
          width: idle ? 8 : 20,
          height: idle ? 8 : 20,
          transform: "translate(-50%, -50%)",
          marginLeft: idle ? 0 : 10,
          borderRadius: 999,
          background:
            listening || processing ? ACCENT_GRAD : "rgba(255,255,255,0.55)",
          opacity: idle ? 0.4 : 1,
          transition:
            "width 220ms var(--spokn-ease), height 220ms var(--spokn-ease), left 220ms var(--spokn-ease), margin 220ms var(--spokn-ease), background 220ms var(--spokn-ease), opacity 220ms var(--spokn-ease)",
          animation: processing
            ? "spokn-pulse-processing 1.4s ease-in-out infinite"
            : "none",
          zIndex: 1,
        }}
      />

      {/* Content — waveform or dots, right of the inner disk */}
      <span
        style={{
          position: "relative",
          zIndex: 2,
          marginLeft: idle ? 0 : 34,
          opacity: idle ? 0 : 1,
          transition:
            "opacity 200ms var(--spokn-ease) 60ms, margin 220ms var(--spokn-ease)",
          display: "inline-flex",
          alignItems: "center",
          pointerEvents: idle ? "none" : "auto",
        }}
      >
        {listening && <Waveform levels={levels} bars={5} height={18} />}
        {processing && <ProcessingDots />}
      </span>

      {/* Invisible cancel hit-area overlay — only during listening */}
      {listening && onCancel && (
        <button
          aria-label="Cancel"
          onClick={onCancel}
          style={{
            position: "absolute",
            inset: 0,
            background: "transparent",
            border: "none",
            cursor: "pointer",
            zIndex: 3,
          }}
        />
      )}
    </div>
  );
};

const ProcessingDots: React.FC = () => (
  <span
    style={{ display: "inline-flex", gap: 3, alignItems: "center" }}
    aria-hidden
  >
    {[0, 1, 2].map((i) => (
      <span
        key={i}
        style={{
          width: 3,
          height: 3,
          borderRadius: 999,
          background: "rgba(255,255,255,0.8)",
          animation: `spokn-dot-pulse 1.3s ${i * 0.16}s ease-in-out infinite`,
        }}
      />
    ))}
  </span>
);

export default VoicePillCapsule;
