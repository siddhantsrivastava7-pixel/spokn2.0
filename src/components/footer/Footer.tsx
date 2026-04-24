import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";

import ModelSelector from "../model-selector";
import UpdateChecker from "../update-checker";

const Footer: React.FC = () => {
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  return (
    <div className="w-full border-t border-spokn-hairline bg-spokn-bg-2/50 backdrop-blur-sm">
      <div className="flex justify-between items-center text-[11px] px-4 py-2 text-spokn-text-3 font-mono tracking-wide">
        <div className="flex items-center gap-4">
          <ModelSelector />
        </div>
        <div className="flex items-center gap-2">
          <UpdateChecker />
          <span className="text-spokn-hairline">·</span>
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span>v{version}</span>
        </div>
      </div>
    </div>
  );
};

export default Footer;
