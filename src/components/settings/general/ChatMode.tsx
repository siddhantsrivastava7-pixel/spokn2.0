import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { SegmentedControl } from "../../ui/SegmentedControl";

/* Chat Mode = "after every transcription, press Enter for me — but
 * give me a 1–5s countdown to abort if I'm pasting somewhere I
 * shouldn't auto-send."
 *
 * v0.3.7: promoted out of Conversation Mode into a top-level General
 * setting. Applies to ALL transcription triggers — hotkey,
 * Conversation Mode, Knock Mode. The countdown shows as a toast with
 * a Cancel button so a misfired transcription never silently sends. */

const COUNTDOWN_OPTIONS: { value: string; label: string }[] = [
  { value: "1", label: "1s" },
  { value: "2", label: "2s" },
  { value: "3", label: "3s" },
  { value: "5", label: "5s" },
];

export const ChatMode: React.FC = () => {
  const { settings, refreshSettings } = useSettings();
  const chatOn = (settings as any)?.chat_mode_enabled ?? false;
  const countdown = (settings as any)?.chat_mode_countdown_secs ?? 3;
  const [busy, setBusy] = useState(false);

  // Live countdown toast wired to the backend events emitted by
  // clipboard::schedule_chat_mode_send. Single shared toast id so
  // each tick replaces the previous one rather than stacking.
  useEffect(() => {
    const TOAST_ID = "chat-mode-countdown";
    const cleanup: Array<() => void> = [];
    (async () => {
      const a = await listen<{ secs_left: number; total: number }>(
        "chat-mode-countdown",
        (e) => {
          const { secs_left } = e.payload;
          // eslint-disable-next-line i18next/no-literal-string
          toast.info(`Sending in ${secs_left}…`, {
            id: TOAST_ID,
            duration: 1500,
            action: {
              // eslint-disable-next-line i18next/no-literal-string
              label: "Cancel",
              onClick: () => commands.chatModeCancelSend(),
            },
          });
        },
      );
      const b = await listen("chat-mode-countdown-cancelled", () => {
        toast.dismiss(TOAST_ID);
      });
      const c = await listen("chat-mode-countdown-sent", () => {
        toast.dismiss(TOAST_ID);
      });
      cleanup.push(a, b, c);
    })();
    return () => cleanup.forEach((fn) => fn());
  }, []);

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

  /* eslint-disable i18next/no-literal-string */
  return (
    <>
      <ToggleSwitch
        checked={chatOn}
        onChange={setChat}
        isUpdating={busy}
        label="Chat Mode (auto-send)"
        description="After each transcription, press Enter for you with a short countdown so you can cancel if it's about to land in the wrong place. Works for hotkey, Conversation Mode, and Knock Mode triggers."
        descriptionMode="inline"
        grouped={true}
      />
      {chatOn && (
        <SettingContainer
          title="Send countdown"
          description="Time you have to cancel before Spokn auto-sends."
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
  );
  /* eslint-enable i18next/no-literal-string */
};

export default ChatMode;
