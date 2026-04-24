import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <div className="space-y-2">
      {title && (
        <div className="px-1">
          <h2 className="text-[10px] font-medium text-spokn-text-3 uppercase tracking-[0.12em] font-mono">
            {title}
          </h2>
          {description && (
            <p className="text-xs text-spokn-text-2 mt-1.5">{description}</p>
          )}
        </div>
      )}
      <div className="bg-spokn-surface border border-spokn-hairline rounded-xl overflow-visible backdrop-blur-sm shadow-spokn-sm">
        <div className="divide-y divide-spokn-hairline">{children}</div>
      </div>
    </div>
  );
};
