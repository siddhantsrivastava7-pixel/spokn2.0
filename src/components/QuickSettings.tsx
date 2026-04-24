import React, { useEffect, useRef, useState } from "react";
import { ChevronRight, Settings as SettingsIcon, X } from "lucide-react";
import { useSettings } from "../hooks/useSettings";
import type { SmartFormattingMode } from "@/bindings";

/* Quick-settings popover ported from the Spokn 2.0 prototype.
 * Wired to the real settings store. */

const MODE_OPTIONS: { value: SmartFormattingMode; label: string }[] = [
  { value: "smart", label: "Smart" },
  { value: "raw", label: "Raw" },
  { value: "email", label: "Email" },
  { value: "message", label: "Message" },
];

const LANGUAGE_OPTIONS = [
  { value: "auto", label: "Auto-detect" },
  { value: "en", label: "English" },
  { value: "hi", label: "हिन्दी" },
  { value: "es", label: "Español" },
  { value: "fr", label: "Français" },
  { value: "de", label: "Deutsch" },
];

interface QuickSettingsProps {
  open: boolean;
  onClose: () => void;
  onOpenAdvanced?: () => void;
  /** Horizontal anchor: either a fixed right offset or centered ('auto'). */
  anchor?: "right" | "center";
}

const QuickSettings: React.FC<QuickSettingsProps> = ({
  open,
  onClose,
  onOpenAdvanced,
  anchor = "right",
}) => {
  const { settings, updateSetting } = useSettings();
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open || !settings) return null;

  const mode: SmartFormattingMode = settings.smart_formatting_mode ?? "smart";
  const smartEnabled: boolean = settings.smart_formatting_enabled ?? true;
  const appAware: boolean = settings.smart_formatting_app_aware ?? true;
  const language: string = settings.selected_language || "auto";
  const shortcut: string =
    settings.bindings?.transcribe?.current_binding || "option+space";

  return (
    <div
      ref={rootRef}
      role="dialog"
      className="spokn"
      style={{
        position: "absolute",
        top: "calc(100% + 8px)",
        ...(anchor === "right"
          ? { right: 0 }
          : { left: "50%", transform: "translateX(-50%)" }),
        width: 300,
        padding: 8,
        borderRadius: 16,
        background: "rgba(16,16,18,0.88)",
        backdropFilter: "blur(28px) saturate(1.35)",
        WebkitBackdropFilter: "blur(28px) saturate(1.35)",
        border: "1px solid rgba(255,255,255,0.08)",
        boxShadow: "var(--shadow-spokn-lg, 0 24px 64px rgba(0,0,0,0.55))",
        animation: "spokn-fade-up 260ms var(--spokn-ease)",
        zIndex: 50,
        color: "#fff",
      }}
    >
      <Header onClose={onClose} />

      <Section label="Mode">
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(4, 1fr)",
            gap: 3,
            padding: 3,
            background: "rgba(255,255,255,0.04)",
            borderRadius: 10,
          }}
        >
          {MODE_OPTIONS.map((m) => {
            const active = mode === m.value;
            return (
              <button
                key={m.value}
                onClick={() => updateSetting("smart_formatting_mode", m.value)}
                style={{
                  padding: "7px 0",
                  borderRadius: 7,
                  background: active
                    ? "rgba(255,255,255,0.08)"
                    : "transparent",
                  border: "none",
                  color: active ? "#fff" : "rgba(255,255,255,0.55)",
                  fontSize: 12,
                  fontWeight: 500,
                  cursor: "pointer",
                  fontFamily: "inherit",
                  transition: "all 160ms var(--spokn-ease)",
                  boxShadow: active
                    ? "0 1px 0 rgba(255,255,255,0.05) inset, 0 1px 2px rgba(0,0,0,0.3)"
                    : "none",
                }}
              >
                {m.label}
              </button>
            );
          })}
        </div>
      </Section>

      <div style={{ padding: "4px 6px" }}>
        <ToggleRow
          label="Smart formatting"
          sub="Punctuation, caps, paragraphs"
          value={smartEnabled}
          onChange={(v) => updateSetting("smart_formatting_enabled", v)}
        />
        <ToggleRow
          label="App-aware mode"
          sub="Force Raw in terminals, Email in Mail"
          value={appAware}
          onChange={(v) => updateSetting("smart_formatting_app_aware", v)}
        />
      </div>

      <div style={{ padding: "4px 6px 2px" }}>
        <SelectRow
          label="Language"
          value={language}
          options={LANGUAGE_OPTIONS}
          onChange={(v) => updateSetting("selected_language", v)}
        />
        <SelectRow
          label="Shortcut"
          value={shortcut}
          options={[
            { value: shortcut, label: prettifyShortcut(shortcut) },
          ]}
          onChange={() => {
            /* no-op — full rebinding lives in Advanced settings */
          }}
          disabled
        />
      </div>

      <div
        style={{
          height: 1,
          background: "rgba(255,255,255,0.05)",
          margin: "6px 8px",
        }}
      />

      <button
        onClick={() => {
          onOpenAdvanced?.();
          onClose();
        }}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 8,
          width: "100%",
          padding: "9px 10px",
          background: "transparent",
          border: "none",
          borderRadius: 8,
          color: "rgba(255,255,255,0.75)",
          fontSize: 12.5,
          textAlign: "left",
          cursor: "pointer",
          fontFamily: "inherit",
          transition: "background 160ms var(--spokn-ease)",
        }}
        onMouseEnter={(e) =>
          (e.currentTarget.style.background = "rgba(255,255,255,0.04)")
        }
        onMouseLeave={(e) =>
          (e.currentTarget.style.background = "transparent")
        }
      >
        <SettingsIcon size={12} strokeWidth={1.7} opacity={0.6} />
        Advanced settings
        <ChevronRight
          size={11}
          strokeWidth={1.7}
          opacity={0.4}
          style={{ marginLeft: "auto" }}
        />
      </button>
    </div>
  );
};

const Header: React.FC<{ onClose: () => void }> = ({ onClose }) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      padding: "4px 6px 0",
    }}
  >
    <span
      style={{
        fontSize: 10,
        color: "rgba(255,255,255,0.4)",
        letterSpacing: "0.08em",
        textTransform: "uppercase",
        fontFamily: "var(--spokn-font-mono)",
      }}
    >
      Spokn
    </span>
    <button
      onClick={onClose}
      aria-label="Close"
      style={{
        width: 22,
        height: 22,
        borderRadius: 6,
        background: "transparent",
        border: "none",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: "pointer",
        color: "rgba(255,255,255,0.5)",
      }}
    >
      <X size={11} strokeWidth={1.8} />
    </button>
  </div>
);

const Section: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div style={{ padding: "8px 6px 4px" }}>
    <div
      style={{
        fontSize: 10,
        color: "rgba(255,255,255,0.38)",
        padding: "0 4px 6px",
        letterSpacing: "0.08em",
        textTransform: "uppercase",
        fontFamily: "var(--spokn-font-mono)",
      }}
    >
      {label}
    </div>
    {children}
  </div>
);

const ToggleRow: React.FC<{
  label: string;
  sub: string;
  value: boolean;
  onChange: (v: boolean) => void;
}> = ({ label, sub, value, onChange }) => (
  <button
    onClick={() => onChange(!value)}
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      width: "100%",
      padding: "10px 8px",
      background: "transparent",
      border: "none",
      borderRadius: 8,
      color: "#fff",
      textAlign: "left",
      cursor: "pointer",
      transition: "background 160ms var(--spokn-ease)",
    }}
    onMouseEnter={(e) =>
      (e.currentTarget.style.background = "rgba(255,255,255,0.03)")
    }
    onMouseLeave={(e) =>
      (e.currentTarget.style.background = "transparent")
    }
  >
    <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <span style={{ fontSize: 13, fontWeight: 500 }}>{label}</span>
      <span style={{ fontSize: 11.5, color: "rgba(255,255,255,0.42)" }}>
        {sub}
      </span>
    </span>
    <span
      aria-hidden
      style={{
        position: "relative",
        width: 30,
        height: 18,
        borderRadius: 999,
        background: value
          ? "linear-gradient(135deg, #4F7CFF 0%, #8B5CF6 100%)"
          : "rgba(255,255,255,0.1)",
        transition: "background 260ms var(--spokn-ease)",
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 2,
          left: value ? 14 : 2,
          width: 14,
          height: 14,
          borderRadius: 999,
          background: "#fff",
          transition: "left 260ms var(--spokn-ease)",
          boxShadow: "0 1px 3px rgba(0,0,0,0.4)",
        }}
      />
    </span>
  </button>
);

const SelectRow: React.FC<{
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
  disabled?: boolean;
}> = ({ label, value, options, onChange, disabled }) => (
  <label
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      padding: "8px 8px",
      opacity: disabled ? 0.6 : 1,
    }}
  >
    <span style={{ fontSize: 13, color: "rgba(255,255,255,0.85)" }}>
      {label}
    </span>
    <span style={{ position: "relative", display: "inline-flex", alignItems: "center" }}>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        style={{
          appearance: "none",
          WebkitAppearance: "none",
          background: "rgba(255,255,255,0.05)",
          color: "#fff",
          border: "1px solid rgba(255,255,255,0.06)",
          borderRadius: 7,
          padding: "4px 22px 4px 8px",
          fontSize: 12,
          fontFamily: "inherit",
          cursor: disabled ? "not-allowed" : "pointer",
          outline: "none",
        }}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value} style={{ background: "#141416" }}>
            {o.label}
          </option>
        ))}
      </select>
      <ChevronRight
        size={10}
        strokeWidth={1.6}
        opacity={0.4}
        style={{ transform: "rotate(90deg)", position: "absolute", right: 7, pointerEvents: "none" }}
      />
    </span>
  </label>
);

function prettifyShortcut(s: string): string {
  return s
    .replace(/\+/g, " ")
    .replace(/option/gi, "⌥")
    .replace(/command/gi, "⌘")
    .replace(/cmd/gi, "⌘")
    .replace(/shift/gi, "⇧")
    .replace(/ctrl/gi, "⌃")
    .replace(/alt/gi, "⌥")
    .replace(/\s+/g, " ")
    .trim();
}

export default QuickSettings;
