import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  FlaskConical,
  History,
  Info,
  Mic,
  Cpu,
  Zap,
  Headphones,
  Settings as SettingsIcon,
  Sparkles,
} from "lucide-react";
import SpoknWordmark from "./icons/SpoknWordmark";
import { SegmentedControl } from "./ui/SegmentedControl";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
  SnippetsSettings,
  VoiceAudioSettings,
  BehaviorSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
  /** Visible in Simple mode? Defaults to true. */
  simple?: boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: Mic,
    component: GeneralSettings,
    enabled: () => true,
    simple: true,
  },
  voiceAudio: {
    labelKey: "sidebar.voiceAudio",
    icon: Headphones,
    component: VoiceAudioSettings,
    enabled: () => true,
    simple: true,
  },
  behavior: {
    labelKey: "sidebar.behavior",
    icon: Cog,
    component: BehaviorSettings,
    enabled: () => true,
    simple: true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
    simple: true,
  },
  snippets: {
    labelKey: "sidebar.snippets",
    icon: Zap,
    component: SnippetsSettings,
    enabled: () => true,
    simple: true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
    simple: true,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
    simple: true,
  },
  // ---- Advanced-only sections below ----
  advanced: {
    labelKey: "sidebar.advanced",
    icon: SettingsIcon,
    component: AdvancedSettings,
    enabled: () => true,
    simple: false,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
    simple: false,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
    simple: false,
  },
} as const satisfies Record<string, SectionConfig>;

const MODE_STORAGE_KEY = "spokn:sidebar_mode";
type SidebarMode = "simple" | "advanced";

function loadMode(): SidebarMode {
  if (typeof window === "undefined") return "simple";
  const v = window.localStorage.getItem(MODE_STORAGE_KEY);
  return v === "advanced" ? "advanced" : "simple";
}

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const [mode, setMode] = useState<SidebarMode>(loadMode);

  useEffect(() => {
    window.localStorage.setItem(MODE_STORAGE_KEY, mode);
  }, [mode]);

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .filter(([_, config]) => mode === "advanced" || config.simple)
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  // If user flips back to Simple while sitting on an advanced-only section,
  // bounce them to General so they don't see a blank state.
  useEffect(() => {
    const stillVisible = availableSections.some((s) => s.id === activeSection);
    if (!stillVisible) {
      onSectionChange("general");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  return (
    <div className="flex flex-col w-48 h-full border-e border-spokn-hairline items-stretch px-3 bg-spokn-bg-2/40 backdrop-blur-sm">
      <div className="px-2 py-4">
        <SpoknWordmark />
      </div>

      {/* Simple ↔ Advanced toggle */}
      <div className="px-1 pb-3 flex justify-center">
        <SegmentedControl<SidebarMode>
          value={mode}
          onChange={setMode}
          options={[
            { value: "simple", label: t("sidebar.modeSimple") },
            { value: "advanced", label: t("sidebar.modeAdvanced") },
          ]}
          ariaLabel="Sidebar mode"
        />
      </div>

      <div className="flex flex-col w-full gap-0.5 pt-3 border-t border-spokn-hairline">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <button
              key={section.id}
              type="button"
              onClick={() => onSectionChange(section.id)}
              className={`group flex gap-2.5 items-center px-3 py-2 w-full rounded-lg cursor-pointer transition-all duration-200 text-left ${
                isActive
                  ? "bg-spokn-surface-2 text-spokn-text shadow-spokn-sm"
                  : "text-spokn-text-2 hover:text-spokn-text hover:bg-spokn-surface"
              }`}
            >
              <Icon
                width={16}
                height={16}
                className={`shrink-0 transition-colors ${
                  isActive ? "text-spokn-accent-blue" : ""
                }`}
              />
              <span
                className="text-[13px] font-medium truncate tracking-tight"
                title={t(section.labelKey)}
              >
                {t(section.labelKey)}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
};
