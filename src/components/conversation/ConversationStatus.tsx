import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useState } from "react";
import { commands } from "@/bindings";

/* Conversation Mode overlay panel.
 *
 * Subscribes to `conversation-state-changed` from the Rust driver.
 * When the state is anything other than `off`, the panel is visible
 * and shows the current step plus the controls relevant to that step.
 *
 * Lives next to the existing recording pill; doesn't interact with
 * the regular dictation flow at all. */

type Label =
  | "off"
  | "paused_unsupported_app"
  | "paused_by_user"
  | "listening"
  | "recording"
  | "transcribing"
  | "ready_to_insert"
  | "sending_in"
  | "error";

interface Payload {
  label: Label;
  transcript?: string | null;
  secs_left?: number | null;
  focused_bundle_id?: string | null;
  reason?: string | null;
}

const STATE_TEXT: Record<Label, string> = {
  off: "",
  paused_unsupported_app: "Paused — open a chat app",
  paused_by_user: "Paused",
  listening: "Listening…",
  recording: "Recording…",
  transcribing: "Transcribing…",
  ready_to_insert: "Ready to insert",
  sending_in: "Sending in",
  error: "Error",
};

const ConversationStatus: React.FC = () => {
  const [payload, setPayload] = useState<Payload>({ label: "off" });

  useEffect(() => {
    const cleanup: Array<() => void> = [];
    (async () => {
      const un = await listen<Payload>("conversation-state-changed", (e) => {
        setPayload(e.payload);
      });
      cleanup.push(un);
    })();
    return () => cleanup.forEach((fn) => fn());
  }, []);

  // Two-tier overlay design: passive states (listening / recording /
  // transcribing) are represented by the existing recording-pill
  // overlay — no panel, no clutter. The panel only surfaces when the
  // user can or must act: countdown to send, focus-lost transcript
  // hold, error, or paused-by-app nudge.
  const ACTION_STATES: Label[] = [
    "sending_in",
    "ready_to_insert",
    "error",
    "paused_unsupported_app",
    "paused_by_user",
  ];
  if (!ACTION_STATES.includes(payload.label)) return null;

  const dot = dotColor(payload.label);

  return (
    <div
      className="spokn-conversation-overlay"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 8,
        background: "rgba(20, 20, 24, 0.88)",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
        border: "1px solid rgba(255, 255, 255, 0.08)",
        borderRadius: 12,
        padding: "10px 14px",
        minWidth: 220,
        color: "#e9e9ea",
        fontSize: 13,
        boxShadow: "0 8px 32px rgba(0, 0, 0, 0.35)",
      }}
    >
      {/* eslint-disable i18next/no-literal-string */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: dot,
            flexShrink: 0,
          }}
        />
        <strong style={{ fontWeight: 600 }}>Conversation Mode</strong>
      </div>
      <div style={{ fontSize: 12, color: "#bdbdc4" }}>
        {STATE_TEXT[payload.label]}
        {payload.label === "sending_in" && payload.secs_left != null && (
          <> {payload.secs_left}…</>
        )}
        {payload.label === "paused_unsupported_app" &&
          payload.focused_bundle_id && (
            <> ({payload.focused_bundle_id})</>
          )}
        {payload.label === "error" && payload.reason && (
          <> — {payload.reason}</>
        )}
      </div>

      {payload.label === "ready_to_insert" && payload.transcript && (
        <div
          style={{
            marginTop: 4,
            padding: 8,
            borderRadius: 8,
            background: "rgba(255, 255, 255, 0.04)",
            fontStyle: "italic",
            color: "#d0d0d6",
            maxWidth: 320,
            wordBreak: "break-word",
          }}
        >
          {payload.transcript.length > 200
            ? payload.transcript.slice(0, 200) + "…"
            : payload.transcript}
        </div>
      )}

      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        {payload.label === "sending_in" && (
          <>
            <Btn
              label="Send now"
              tone="primary"
              onClick={() => commands.conversationForceSend()}
            />
            <Btn
              label="Cancel send"
              onClick={() => commands.conversationCancelSend()}
            />
          </>
        )}
        {payload.label === "ready_to_insert" && (
          <>
            <Btn
              label="Insert"
              tone="primary"
              onClick={() => commands.conversationInsertPending()}
            />
            <Btn
              label="Discard"
              onClick={() => commands.conversationDiscardPending()}
            />
          </>
        )}
        {payload.label === "paused_by_user" && (
          <Btn
            label="Resume"
            tone="primary"
            onClick={() => commands.conversationResume()}
          />
        )}
        <Btn
          label="Stop"
          tone="danger"
          onClick={() => commands.setConversationModeEnabled(false)}
        />
      </div>
      {/* eslint-enable i18next/no-literal-string */}
    </div>
  );
};

const Btn: React.FC<{
  label: string;
  onClick: () => void;
  tone?: "primary" | "danger";
}> = ({ label, onClick, tone }) => (
  <button
    type="button"
    onClick={onClick}
    style={{
      padding: "4px 10px",
      borderRadius: 6,
      fontSize: 12,
      fontWeight: 500,
      border: "1px solid rgba(255, 255, 255, 0.12)",
      background:
        tone === "primary"
          ? "rgba(99, 102, 241, 0.85)"
          : tone === "danger"
            ? "rgba(244, 63, 94, 0.18)"
            : "rgba(255, 255, 255, 0.06)",
      color:
        tone === "danger"
          ? "#fda4af"
          : tone === "primary"
            ? "#fff"
            : "#e9e9ea",
      cursor: "pointer",
    }}
  >
    {label}
  </button>
);

function dotColor(label: Label): string {
  switch (label) {
    case "listening":
      return "#34d399"; // green
    case "recording":
      return "#f87171"; // red
    case "transcribing":
      return "#fbbf24"; // amber
    case "sending_in":
      return "#60a5fa"; // blue
    case "ready_to_insert":
      return "#a78bfa"; // purple
    case "paused_by_user":
    case "paused_unsupported_app":
      return "#9ca3af"; // gray
    case "error":
      return "#ef4444";
    default:
      return "#6b7280";
  }
}

export default ConversationStatus;
