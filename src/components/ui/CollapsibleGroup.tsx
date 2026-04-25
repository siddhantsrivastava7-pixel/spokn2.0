import React, { useState } from "react";
import { ChevronRight } from "lucide-react";

interface CollapsibleGroupProps {
  title: string;
  description?: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

/* Collapsible variant of SettingsGroup. Used inside the Advanced settings
 * page so the long list of power-user controls is hidden behind one-click
 * disclosures — the page stays scannable. */
export const CollapsibleGroup: React.FC<CollapsibleGroupProps> = ({
  title,
  description,
  defaultOpen = false,
  children,
}) => {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="space-y-2">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="group flex items-center gap-2 w-full px-1 py-1 text-left cursor-pointer"
      >
        <ChevronRight
          size={13}
          strokeWidth={2}
          className={`text-spokn-text-3 group-hover:text-spokn-text-2 transition-transform duration-200 ${open ? "rotate-90" : ""}`}
        />
        <h2 className="text-[10px] font-medium text-spokn-text-3 uppercase tracking-[0.12em] font-mono group-hover:text-spokn-text-2 transition-colors">
          {title}
        </h2>
      </button>
      {open && (
        <>
          {description && (
            <p className="text-xs text-spokn-text-2 mt-1.5 ml-5">
              {description}
            </p>
          )}
          <div className="bg-spokn-surface border border-spokn-hairline rounded-xl overflow-visible backdrop-blur-sm shadow-spokn-sm">
            <div className="divide-y divide-spokn-hairline">{children}</div>
          </div>
        </>
      )}
    </div>
  );
};

export default CollapsibleGroup;
