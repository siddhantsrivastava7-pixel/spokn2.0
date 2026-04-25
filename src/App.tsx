import { useEffect, useState, useRef } from "react";
import { toast, Toaster } from "sonner";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { ModelStateEvent, RecordingErrorEvent } from "./lib/types/events";
import "./App.css";
import AccessibilityPermissions from "./components/AccessibilityPermissions";
import Footer from "./components/footer";
import Onboarding, { AccessibilityOnboarding } from "./components/onboarding";
import LanguageOnboarding from "./components/onboarding/LanguageOnboarding";
import ModelDownloading from "./components/onboarding/ModelDownloading";
import UserInfoOnboarding from "./components/onboarding/UserInfoOnboarding";
import ConversationStatus from "./components/conversation/ConversationStatus";
import { Sidebar, SidebarSection, SECTIONS_CONFIG } from "./components/Sidebar";
import { RESET_ONBOARDING_EVENT } from "./components/settings/ResetOnboardingButton";
import QuickSettings from "./components/QuickSettings";
import { Zap } from "lucide-react";
import { useSettings } from "./hooks/useSettings";
import { useSettingsStore } from "./stores/settingsStore";
import { useModelStore } from "./stores/modelStore";
import { commands } from "@/bindings";
import { getLanguageDirection, initializeRTL } from "@/lib/utils/rtl";

type OnboardingStep =
  | "accessibility"
  | "language"
  | "user_info"
  | "downloading"
  | "model"
  | "done";

const renderSettingsContent = (section: SidebarSection) => {
  const ActiveComponent =
    SECTIONS_CONFIG[section]?.component || SECTIONS_CONFIG.general.component;
  return <ActiveComponent />;
};

function App() {
  const { t, i18n } = useTranslation();
  const [onboardingStep, setOnboardingStep] = useState<OnboardingStep | null>(
    null,
  );
  // Track if this is a returning user who just needs to grant permissions
  // (vs a new user who needs full onboarding including model selection)
  const [isReturningUser, setIsReturningUser] = useState(false);
  const [currentSection, setCurrentSection] =
    useState<SidebarSection>("general");
  const [quickSettingsOpen, setQuickSettingsOpen] = useState(false);
  const [recommendedModelId, setRecommendedModelId] = useState<string | null>(
    null,
  );
  const { settings, updateSetting } = useSettings();
  const direction = getLanguageDirection(i18n.language);
  const refreshAudioDevices = useSettingsStore(
    (state) => state.refreshAudioDevices,
  );
  const refreshOutputDevices = useSettingsStore(
    (state) => state.refreshOutputDevices,
  );
  const hasCompletedPostOnboardingInit = useRef(false);

  useEffect(() => {
    checkOnboardingStatus();
  }, []);

  // Dev/QA hook: AdvancedSettings → Replay welcome flow dispatches this
  // event. We rewind the state machine to the start so the entire
  // language picker → permissions → model selection sequence plays out
  // again. Models stay downloaded; settings stay intact.
  useEffect(() => {
    const handler = () => {
      hasCompletedPostOnboardingInit.current = false;
      setIsReturningUser(false);
      setRecommendedModelId(null);
      setOnboardingStep("language");
    };
    window.addEventListener(RESET_ONBOARDING_EVENT, handler);
    return () => window.removeEventListener(RESET_ONBOARDING_EVENT, handler);
  }, []);

  // Initialize RTL direction when language changes
  useEffect(() => {
    initializeRTL(i18n.language);
  }, [i18n.language]);

  // Initialize Enigo, shortcuts, and refresh audio devices when main app loads
  useEffect(() => {
    if (onboardingStep === "done" && !hasCompletedPostOnboardingInit.current) {
      hasCompletedPostOnboardingInit.current = true;
      Promise.all([
        commands.initializeEnigo(),
        commands.initializeShortcuts(),
      ]).catch((e) => {
        console.warn("Failed to initialize:", e);
      });
      refreshAudioDevices();
      refreshOutputDevices();
    }
  }, [onboardingStep, refreshAudioDevices, refreshOutputDevices]);

  // Handle keyboard shortcuts for debug mode toggle
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Check for Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
      const isDebugShortcut =
        event.shiftKey &&
        event.key.toLowerCase() === "d" &&
        (event.ctrlKey || event.metaKey);

      if (isDebugShortcut) {
        event.preventDefault();
        const currentDebugMode = settings?.debug_mode ?? false;
        updateSetting("debug_mode", !currentDebugMode);
      }
    };

    // Add event listener when component mounts
    document.addEventListener("keydown", handleKeyDown);

    // Cleanup event listener when component unmounts
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [settings?.debug_mode, updateSetting]);

  // Listen for recording errors from the backend and show a toast
  useEffect(() => {
    const unlisten = listen<RecordingErrorEvent>("recording-error", (event) => {
      const { error_type, detail } = event.payload;

      if (error_type === "microphone_permission_denied") {
        const currentPlatform = platform();
        const platformKey = `errors.micPermissionDenied.${currentPlatform}`;
        const description = t(platformKey, {
          defaultValue: t("errors.micPermissionDenied.generic"),
        });
        toast.error(t("errors.micPermissionDeniedTitle"), { description });
      } else if (error_type === "no_input_device") {
        toast.error(t("errors.noInputDeviceTitle"), {
          description: t("errors.noInputDevice"),
        });
      } else {
        toast.error(
          t("errors.recordingFailed", { error: detail ?? "Unknown error" }),
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  // Listen for the auto-detect "this looks like Hinglish" event from the
  // Rust pipeline. Fires at most once per app install (Rust persists the
  // shown flag), so no debouncing needed here. Offers a one-tap action to
  // add Hindi to the user's transcription_languages.
  useEffect(() => {
    const unlisten = listen("hinglish-detected", () => {
      const current = (settings as any)?.transcription_languages ?? [];
      if (current.includes("hi")) return; // double-safety: already enabled
      // eslint-disable-next-line i18next/no-literal-string
      toast("Looks like you dictate in Hinglish", {
        // eslint-disable-next-line i18next/no-literal-string
        description:
          "Enable Hindi in your languages for noticeably better accuracy.",
        duration: 12000,
        action: {
          // eslint-disable-next-line i18next/no-literal-string
          label: "Enable",
          onClick: () => {
            const next = Array.from(new Set([...current, "hi"]));
            (updateSetting as any)("transcription_languages", next);
            // eslint-disable-next-line i18next/no-literal-string
            toast.success("Hindi enabled. Try a new dictation.");
          },
        },
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [settings, updateSetting]);

  // Listen for paste failures and show a toast.
  // The technical error detail is logged to handy.log on the Rust side
  // (see actions.rs `error!("Failed to paste transcription: ...")`),
  // so we show a localized, user-friendly message here instead of the raw error.
  useEffect(() => {
    const unlisten = listen("paste-error", () => {
      toast.error(t("errors.pasteFailedTitle"), {
        description: t("errors.pasteFailed"),
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  // Listen for model loading failures and show a toast
  useEffect(() => {
    const unlisten = listen<ModelStateEvent>("model-state-changed", (event) => {
      if (event.payload.event_type === "loading_failed") {
        toast.error(
          t("errors.modelLoadFailed", {
            model:
              event.payload.model_name || t("errors.modelLoadFailedUnknown"),
          }),
          {
            description: event.payload.error,
          },
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  // Listen for the stale-selected_model fallback. Emitted when the
  // saved model id is no longer in the registry (renames between
  // versions) or no longer marked as downloaded — the backend
  // auto-switches to the first installed model and tells the user.
  useEffect(() => {
    const unlisten = listen<{
      stale_id: string;
      new_id: string | null;
    }>("model-fallback", (event) => {
      const { stale_id, new_id } = event.payload;
      if (new_id) {
        // eslint-disable-next-line i18next/no-literal-string
        toast.info(`Switched model: '${stale_id || "(none)"}' → '${new_id}'`, {
          // eslint-disable-next-line i18next/no-literal-string
          description:
            "Your saved model wasn't available; using a downloaded one instead. Spokn picks the best model for your languages automatically.",
          duration: 8000,
        });
      } else {
        // eslint-disable-next-line i18next/no-literal-string
        toast.error("No transcription model installed", {
          // eslint-disable-next-line i18next/no-literal-string
          description:
            "Add a language in Settings → Language and Spokn will download the right model automatically.",
          duration: 12000,
        });
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const revealMainWindowForPermissions = async () => {
    try {
      await commands.showMainWindowCommand();
    } catch (e) {
      console.warn("Failed to show main window for permission onboarding:", e);
    }
  };

  const checkOnboardingStatus = async () => {
    try {
      // Check if they have any models available
      const result = await commands.hasAnyModelsAvailable();
      const hasModels = result.status === "ok" && result.data;
      const currentPlatform = platform();

      if (hasModels) {
        // Returning user - check if they need to grant permissions first
        setIsReturningUser(true);

        if (currentPlatform === "macos") {
          try {
            const [hasAccessibility, hasMicrophone] = await Promise.all([
              checkAccessibilityPermission(),
              checkMicrophonePermission(),
            ]);
            if (!hasAccessibility || !hasMicrophone) {
              await revealMainWindowForPermissions();
              setOnboardingStep("accessibility");
              return;
            }
          } catch (e) {
            console.warn("Failed to check macOS permissions:", e);
            // If we can't check, proceed to main app and let them fix it there
          }
        }

        if (currentPlatform === "windows") {
          try {
            const microphoneStatus =
              await commands.getWindowsMicrophonePermissionStatus();
            if (
              microphoneStatus.supported &&
              microphoneStatus.overall_access === "denied"
            ) {
              await revealMainWindowForPermissions();
              setOnboardingStep("accessibility");
              return;
            }
          } catch (e) {
            console.warn("Failed to check Windows microphone permissions:", e);
            // If we can't check, proceed to main app and let them fix it there
          }
        }

        setOnboardingStep("done");
      } else {
        // New user — flow is Language → Accessibility → Download → Done.
        // Lead with the friendly "what do you speak?" picker so users can
        // confirm Spokn supports them BEFORE granting OS-level permissions.
        setIsReturningUser(false);
        setOnboardingStep("language");
      }
    } catch (error) {
      console.error("Failed to check onboarding status:", error);
      setOnboardingStep("language");
    }
  };

  const handleAccessibilityComplete = () => {
    // Returning users already have models, skip to main app.
    // New users have just granted permissions; proceed to model download
    // (recommendedModelId was set in handleLanguagesPicked above).
    if (isReturningUser) {
      setOnboardingStep("done");
    } else if (recommendedModelId) {
      setOnboardingStep("downloading");
    } else {
      // Edge case: somehow we're at accessibility without a recommended
      // model. Fall back to manual model picker.
      setOnboardingStep("model");
    }
  };

  const handleLanguagesPicked = (modelId: string) => {
    setRecommendedModelId(modelId);
    // Pre-warm: kick off the model download NOW, in the background, while
    // the user moves through the next steps. By the time they reach the
    // download screen the model may already be largely downloaded.
    // ModelDownloading on mount checks `isDownloading` before starting,
    // so this won't double-fire.
    void useModelStore
      .getState()
      .downloadModel(modelId)
      .catch((e) => console.warn("Background pre-download failed:", e));
    setOnboardingStep("user_info");
  };

  const handleUserInfoComplete = () => setOnboardingStep("accessibility");

  const handleDownloadComplete = () => {
    setOnboardingStep("done");
  };

  const handleModelSelected = () => {
    // Legacy manual flow (Skip-for-now on the old model screen)
    setOnboardingStep("done");
  };

  // Still checking onboarding status
  if (onboardingStep === null) {
    return null;
  }

  if (onboardingStep === "accessibility") {
    return <AccessibilityOnboarding onComplete={handleAccessibilityComplete} />;
  }

  if (onboardingStep === "language") {
    return <LanguageOnboarding onComplete={handleLanguagesPicked} />;
  }

  if (onboardingStep === "user_info") {
    return <UserInfoOnboarding onComplete={handleUserInfoComplete} />;
  }

  if (onboardingStep === "downloading" && recommendedModelId) {
    return (
      <ModelDownloading
        modelId={recommendedModelId}
        onComplete={handleDownloadComplete}
      />
    );
  }

  if (onboardingStep === "model") {
    return <Onboarding onModelSelected={handleModelSelected} />;
  }

  return (
    <div
      dir={direction}
      className="h-screen flex flex-col select-none cursor-default bg-spokn-bg text-spokn-text"
    >
      <div
        style={{
          position: "fixed",
          top: 12,
          right: 12,
          zIndex: 50,
        }}
      >
        <ConversationStatus />
      </div>
      <Toaster
        theme="dark"
        toastOptions={{
          unstyled: true,
          classNames: {
            toast:
              "bg-spokn-bg-2 border border-spokn-hairline rounded-xl shadow-spokn-md px-4 py-3 flex items-center gap-3 text-sm text-spokn-text backdrop-blur-xl",
            title: "font-medium",
            description: "text-spokn-text-2",
          },
        }}
      />
      <div className="flex-1 flex overflow-hidden">
        <Sidebar
          activeSection={currentSection}
          onSectionChange={setCurrentSection}
        />
        <div className="flex-1 flex flex-col overflow-hidden relative">
          <div className="absolute top-4 right-5 z-30">
            <button
              type="button"
              aria-label="Quick settings"
              onClick={() => setQuickSettingsOpen((v) => !v)}
              className={`h-8 w-8 rounded-lg border border-spokn-hairline flex items-center justify-center transition-all duration-200 ${
                quickSettingsOpen
                  ? "bg-spokn-surface-2 text-spokn-accent-blue border-spokn-hairline-2"
                  : "bg-spokn-surface text-spokn-text-2 hover:text-spokn-text hover:bg-spokn-surface-2"
              }`}
            >
              <Zap size={14} strokeWidth={1.7} />
            </button>
            <QuickSettings
              open={quickSettingsOpen}
              onClose={() => setQuickSettingsOpen(false)}
              onOpenAdvanced={() => setCurrentSection("advanced")}
            />
          </div>
          <div className="flex-1 overflow-y-auto">
            <div className="flex flex-col items-stretch max-w-3xl mx-auto px-6 py-8 gap-6">
              <AccessibilityPermissions />
              {renderSettingsContent(currentSection)}
            </div>
          </div>
        </div>
      </div>
      <Footer />
    </div>
  );
}

export default App;
