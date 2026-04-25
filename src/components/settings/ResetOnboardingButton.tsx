import React from "react";
import { RotateCcw } from "lucide-react";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

export const RESET_ONBOARDING_EVENT = "spokn:reset-onboarding";

interface ResetOnboardingButtonProps {
  grouped?: boolean;
}

/* Dev/QA convenience: trigger the welcome flow on demand without
 * uninstalling the app. Dispatches a window CustomEvent that App.tsx
 * listens for; App resets `onboardingStep` to "accessibility" so the
 * full sequence (permissions → language picker → model selection) plays
 * out again. Models stay downloaded; settings stay intact — only the
 * UI flow re-runs. */
export const ResetOnboardingButton: React.FC<ResetOnboardingButtonProps> = ({
  grouped = true,
}) => {
  const handleClick = () => {
    if (
      !confirm(
        "Restart the welcome flow? Your settings, models, history and snippets are kept — only the onboarding screens re-appear.",
      )
    ) {
      return;
    }
    window.dispatchEvent(new CustomEvent(RESET_ONBOARDING_EVENT));
  };

  return (
    <SettingContainer
      title="Replay welcome flow"
      description="Useful for testing or re-picking your dictation languages."
      descriptionMode="tooltip"
      grouped={grouped}
    >
      <Button
        variant="secondary"
        size="md"
        onClick={handleClick}
        className="flex items-center gap-1.5"
      >
        <RotateCcw size={13} strokeWidth={2} />
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Replay
      </Button>
    </SettingContainer>
  );
};

export default ResetOnboardingButton;
