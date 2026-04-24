import React from "react";

interface WaveformProps {
  levels: number[];
  bars?: number;
  height?: number;
}

/* Live mic-level bars. Fed from the `mic-level` Tauri event upstream.
 * Visual parity with the Spokn capsule waveform: 5–9 bars, white-ish,
 * thin rounded lines, softened via pow() so quiet input doesn't look dead. */
const Waveform: React.FC<WaveformProps> = ({ levels, bars = 5, height = 20 }) => {
  const slice = levels.slice(0, bars);
  return (
    <span
      aria-hidden
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 3,
        height,
      }}
    >
      {Array.from({ length: bars }).map((_, i) => {
        const v = slice[i] ?? 0;
        const h = Math.max(3, Math.min(height, 3 + Math.pow(v, 0.7) * (height - 3)));
        return (
          <span
            key={i}
            style={{
              width: 2.5,
              height: h,
              borderRadius: 2,
              background: "rgba(255,255,255,0.85)",
              opacity: Math.max(0.35, v * 1.6),
              transition: "height 60ms linear, opacity 120ms ease-out",
            }}
          />
        );
      })}
    </span>
  );
};

export default Waveform;
