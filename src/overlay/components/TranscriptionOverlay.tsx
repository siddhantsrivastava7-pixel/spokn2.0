import React from "react";

export type OverlayTextState = "hidden" | "listening" | "smart-formatting" | "settled";

interface TranscriptionOverlayProps {
  text: string;
  state: OverlayTextState;
  mode?: string;
}

/* Streaming transcription chip that floats above the pill.
 * Listens for an `overlay-text` event in a future pass; this component is
 * mounted but invisible until that event arrives. Keeps the visual shell
 * identical to the prototype so wiring becomes trivial later. */
const TranscriptionOverlay: React.FC<TranscriptionOverlayProps> = ({
  text,
  state,
  mode,
}) => {
  if (state === "hidden" || !text) return null;

  const listening = state === "listening";
  const settled = state === "settled";

  return (
    <div
      className="spokn"
      style={{
        position: "relative",
        maxWidth: 420,
        padding: "10px 14px",
        borderRadius: 14,
        background: "rgba(12,12,14,0.82)",
        backdropFilter: "blur(18px) saturate(1.3)",
        WebkitBackdropFilter: "blur(18px) saturate(1.3)",
        border: "1px solid rgba(255,255,255,0.07)",
        boxShadow: "var(--spokn-shadow-md)",
        color: settled ? "#fff" : "rgba(255,255,255,0.92)",
        fontSize: 13,
        lineHeight: 1.45,
        letterSpacing: "-0.005em",
        animation:
          "spokn-fade-up var(--spokn-dur-slow) var(--spokn-ease) both",
      }}
    >
      <span>{text}</span>
      {listening && (
        <span
          aria-hidden
          style={{
            display: "inline-block",
            width: 2,
            height: 14,
            marginLeft: 3,
            verticalAlign: "-2px",
            background: "rgba(255,255,255,0.7)",
            animation: "spokn-breathe-soft 1.1s ease-in-out infinite",
          }}
        />
      )}
      {mode && settled && (
        <span
          style={{
            marginLeft: 10,
            fontSize: 11,
            color: "rgba(255,255,255,0.45)",
            fontFamily: "var(--spokn-font-mono)",
            letterSpacing: "0.04em",
            textTransform: "uppercase",
          }}
        >
          · {mode}
        </span>
      )}
    </div>
  );
};

export default TranscriptionOverlay;
