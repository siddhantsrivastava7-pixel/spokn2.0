import React, { useState } from "react";
import { type as platformType } from "@tauri-apps/plugin-os";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";

/* Conversation Mode toggle. Chat Mode (auto-send) is now a separate
 * top-level control that applies to every transcription, not just
 * Conversation Mode ones — see the ChatMode component. */

export const ConversationMode: React.FC = () => {
  const { settings, refreshSettings } = useSettings();
  const isMac = platformType() === "macos";

  const conversationOn = (settings as any)?.conversation_mode_enabled ?? false;
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
      {/* Chat Mode is now a top-level toggle (General → Chat Mode)
          since it applies to every transcription, not just
          Conversation Mode ones. Kept this component focused on the
          Conversation Mode loop alone. */}
    </>
  );
  /* eslint-enable i18next/no-literal-string */
};

export default ConversationMode;
