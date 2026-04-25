import React from "react";

export interface SegmentedControlOption<T extends string> {
  value: T;
  label: string;
  hint?: string;
}

interface SegmentedControlProps<T extends string> {
  value: T;
  onChange: (value: T) => void;
  options: SegmentedControlOption<T>[];
  disabled?: boolean;
  ariaLabel?: string;
  className?: string;
}

/* Two-or-three option pill toggle. Used for "Mode: Hold to talk / Tap to
 * start-stop" and similar binary choices where a checkbox is too vague
 * about what each state means. */
export function SegmentedControl<T extends string>({
  value,
  onChange,
  options,
  disabled,
  ariaLabel,
  className = "",
}: SegmentedControlProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={`inline-flex items-center gap-0.5 rounded-lg border border-spokn-hairline bg-spokn-surface p-0.5 ${className}`}
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => !disabled && onChange(opt.value)}
            disabled={disabled}
            title={opt.hint}
            className={`px-3 py-1.5 text-[12.5px] font-medium rounded-md transition-all duration-150 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
              active
                ? "bg-spokn-surface-2 text-spokn-text shadow-spokn-sm"
                : "text-spokn-text-2 hover:text-spokn-text"
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

export default SegmentedControl;
