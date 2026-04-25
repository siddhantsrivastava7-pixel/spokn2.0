import React, { useState } from "react";
import { type as platformType } from "@tauri-apps/plugin-os";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { SegmentedControl } from "../../ui/SegmentedControl";

/* Conversation Mode + Chat Mode controls.
 *
 * Conversation Mode = the listen→record→transcribe→insert loop inside
 * supported chat apps. Chat Mode (sub-feature) = auto-press Enter
 * after a countdown so the message actually sends.
 *
 * Both are macOS-only in v1 — toggles render disabled with a hint
 * on Windows/Linux. */

const COUNTDOWN_OPTIONS: { value: string; label: string }[] = [
  { value: "1", label: "1s" },
  { value: "2", label: "2s" },
  { value: "3", label: "3s" },
  { value: "5", label: "5s" },
];

export const ConversationMode: React.FC = () => {
  const { settings, refreshSettings } = useSettings();
  const isMac = platformType() === "macos";

  const conversationOn = (settings as any)?.conversation_mode_enabled ?? false;
  const chatOn = (settings as any)?.chat_mode_enabled ?? false;
  const countdown =
    (settings as any)?.chat_mode_countdown_secs ?? 3;

  const [busy, setBusy] = useState(false);

  const setConv = async (v: boolean) => {
    setBusy(true);
    try {
      const r = await commands.setConversationModeEnabled(v);
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const setChat = async (v: boolean) => {
    setBusy(true);
    try {
      const r = await commands.setChatModeEnabled(v);
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const setCountdown = async (n: number) => {
    setBusy(true);
    try {
      const r = await commands.setChatModeCountdownSecs(n);
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  if (!isMac) {
    return (
      <SettingContainer
        // eslint-disable-next-line i18next/no-literal-string
        title="Conversation Mode"
        // eslint-disable-next-line i18next/no-literal-string
        description="Conversation Mode is currently macOS-only."
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

  /* eslint-disable i18next/no-literal-string */
  return (
    <>
      <ToggleSwitch
        checked={conversationOn}
        onChange={setConv}
        isUpdating={busy}
        label="Conversation Mode"
        description="Hands-free dictation loop inside supported chat apps. Listens → transcribes on pause → inserts. Only runs in Messages, WhatsApp, Telegram, Signal, Slack, and Discord."
        descriptionMode="inline"
        grouped={true}
      />
      {conversationOn && (
        <>
          <ToggleSwitch
            checked={chatOn}
            onChange={setChat}
            isUpdating={busy}
            label="Chat Mode (auto-send)"
            description="After each transcription, press Enter automatically following a countdown so the message sends without you touching the keyboard."
            descriptionMode="inline"
            grouped={true}
          />
          {chatOn && (
            <SettingContainer
              title="Send countdown"
              description="Time you have to cancel before Spokn auto-sends the message."
              descriptionMode="inline"
              grouped={true}
            >
              <SegmentedControl<string>
                value={String(countdown)}
                onChange={(v) => setCountdown(parseInt(v, 10))}
                options={COUNTDOWN_OPTIONS}
                ariaLabel="Send countdown"
                disabled={busy}
              />
            </SettingContainer>
          )}
        </>
      )}
    </>
  );
  /* eslint-enable i18next/no-literal-string */
};

export default ConversationMode;
