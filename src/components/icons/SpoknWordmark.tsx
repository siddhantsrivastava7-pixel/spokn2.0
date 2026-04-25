import React from "react";
import spoknMark from "@/assets/spokn-mark.png";

interface SpoknWordmarkProps {
  className?: string;
  /** Mark diameter in px. Text size scales from this. */
  size?: number;
  variant?: "sidebar" | "hero";
}

/* Spokn logo (image) + "Spokn" wordmark. Two sizes:
 *   - sidebar  (default 18): the small header variant used in the nav
 *   - hero     (default 44): the big onboarding/about variant */
const SpoknWordmark: React.FC<SpoknWordmarkProps> = ({
  className,
  size,
  variant = "sidebar",
}) => {
  const markSize = size ?? (variant === "hero" ? 44 : 18);
  const fontSize = variant === "hero" ? 40 : 15;
  const gap = variant === "hero" ? 14 : 8;

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
      <img
        src={spoknMark}
        alt=""
        aria-hidden
        width={markSize}
        height={markSize}
        style={{
          width: markSize,
          height: markSize,
          display: "block",
          // Keep the artwork crisp; no smoothing artefacts at small sizes.
          imageRendering: "auto",
        }}
      />
      <span
        style={{
          fontFamily: "var(--spokn-font)",
          fontWeight: 600,
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
