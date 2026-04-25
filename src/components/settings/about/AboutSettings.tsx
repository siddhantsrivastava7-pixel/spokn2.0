import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Github, Heart } from "lucide-react";
import SpoknWordmark from "../../icons/SpoknWordmark";
import { Button } from "../../ui/Button";

/* About — three things: who we are, how to support, where the code is.
 * Everything else (app data, logs, language, version channel) lives under
 * Settings (Advanced). */
export const AboutSettings: React.FC = () => {
  const { t: _t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(""));
  }, []);

  return (
    <div className="max-w-md w-full mx-auto flex flex-col items-center text-center gap-6 py-8">
      <SpoknWordmark variant="hero" />
      {version && (
        <p className="text-[12px] font-mono tracking-wider uppercase text-spokn-text-3">
          {/* eslint-disable-next-line i18next/no-literal-string */}
          v{version}
        </p>
      )}
      <p className="text-[14px] text-spokn-text-2 max-w-sm leading-relaxed">
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Offline, privacy-focused speech-to-text. Forked from Handy.
      </p>
      <div className="flex items-center gap-2">
        <Button
          variant="primary"
          size="md"
          onClick={() => openUrl("https://handy.computer/donate")}
          className="flex items-center gap-1.5"
        >
          <Heart size={13} strokeWidth={2} />
          {/* eslint-disable-next-line i18next/no-literal-string */}
          Donate
        </Button>
        <Button
          variant="secondary"
          size="md"
          onClick={() =>
            openUrl("https://github.com/siddhantsrivastava7-pixel/spokn2.0")
          }
          className="flex items-center gap-1.5"
        >
          <Github size={13} strokeWidth={2} />
          {/* eslint-disable-next-line i18next/no-literal-string */}
          GitHub
        </Button>
      </div>
    </div>
  );
};
