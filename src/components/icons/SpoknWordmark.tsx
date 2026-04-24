import React from "react";

interface SpoknWordmarkProps {
  className?: string;
  /** Orb diameter in px. Text size scales from this. */
  size?: number;
  variant?: "sidebar" | "hero";
}

/* Gradient orb + "Spokn" wordmark. Two sizes:
 *   - sidebar  (default 16): the tiny header variant used in the nav
 *   - hero     (default 40): the big onboarding/logo variant */
const SpoknWordmark: React.FC<SpoknWordmarkProps> = ({
  className,
  size,
  variant = "sidebar",
}) => {
  const orbSize = size ?? (variant === "hero" ? 40 : 16);
  const fontSize = variant === "hero" ? 40 : 15;
  const gap = variant === "hero" ? 14 : 8;
  const glow =
    variant === "hero"
      ? "0 0 0 1px rgba(255,255,255,0.08) inset, 0 8px 30px rgba(79,124,255,0.45)"
      : "0 0 0 1px rgba(255,255,255,0.06) inset, 0 4px 12px rgba(79,124,255,0.35)";

  return (
    <div
      className={className}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap,
        color: "var(--color-spokn-text)",
      }}
    >
      <span
        aria-hidden
        style={{
          width: orbSize,
          height: orbSize,
          borderRadius: 999,
          background: "var(--spokn-accent-grad)",
          boxShadow: glow,
        }}
      />
      <span
        style={{
          fontFamily: "var(--spokn-font)",
          fontWeight: variant === "hero" ? 600 : 600,
          fontSize,
          letterSpacing: variant === "hero" ? "-0.02em" : "-0.01em",
        }}
      >
        Spokn
      </span>
    </div>
  );
};

export default SpoknWordmark;
