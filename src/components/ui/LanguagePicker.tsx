import React, { useMemo, useState } from "react";
import { ChevronDown, ChevronUp, Search } from "lucide-react";
import {
  ALL_LANGUAGES,
  TOP_LANGUAGES,
  findLanguage,
  isTopLanguage,
  type DictationLanguage,
} from "@/lib/dictationLanguages";

interface LanguagePickerProps {
  selected: Set<string>;
  onChange: (next: Set<string>) => void;
  /** Optional copy under the chip grid. Empty string suppresses it. */
  helpText?: string;
}

/* Curated 20-chip grid for fast picking + an expandable search section
 * covering all 99 Whisper-supported languages. Languages selected from
 * the search are pinned to the top of the chip grid so the user can see
 * everything they've picked at a glance. */
export const LanguagePicker: React.FC<LanguagePickerProps> = ({
  selected,
  onChange,
  helpText,
}) => {
  const [showAll, setShowAll] = useState(false);
  const [query, setQuery] = useState("");

  // Build the chip grid: curated top 20 + any selected non-curated
  // entries (so a Bengali-speaker still sees their pick alongside the
  // common 20 without having to re-open "Show all").
  const chips: DictationLanguage[] = useMemo(() => {
    const extras: DictationLanguage[] = [];
    selected.forEach((code) => {
      if (!isTopLanguage(code)) {
        const found = findLanguage(code);
        if (found) extras.push(found);
      }
    });
    return [...extras, ...TOP_LANGUAGES];
  }, [selected]);

  const filteredAll = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return ALL_LANGUAGES;
    return ALL_LANGUAGES.filter(
      (l) =>
        l.code.toLowerCase().includes(q) ||
        l.native.toLowerCase().includes(q) ||
        l.english.toLowerCase().includes(q),
    );
  }, [query]);

  const toggle = (code: string) => {
    const next = new Set(selected);
    if (next.has(code)) next.delete(code);
    else next.add(code);
    onChange(next);
  };

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-4 gap-2">
        {chips.map((lang) => {
          const isSelected = selected.has(lang.code);
          return (
            <button
              key={lang.code}
              type="button"
              onClick={() => toggle(lang.code)}
              className={`group relative flex flex-col items-start gap-0.5 rounded-xl border px-3 py-2.5 text-left transition-all duration-150 cursor-pointer ${
                isSelected
                  ? "border-spokn-accent-blue/60 bg-spokn-accent-blue/10"
                  : "border-spokn-hairline bg-spokn-surface hover:bg-spokn-surface-2 hover:border-spokn-hairline-2"
              }`}
            >
              <span className="text-sm font-medium text-spokn-text">
                {lang.native}
              </span>
              <span className="text-[11px] text-spokn-text-3 uppercase tracking-wider font-mono">
                {lang.english}
              </span>
              {isSelected && (
                <span
                  aria-hidden
                  className="absolute top-2 right-2 w-1.5 h-1.5 rounded-full"
                  style={{ background: "var(--spokn-accent-grad)" }}
                />
              )}
            </button>
          );
        })}
      </div>

      {helpText && (
        <p className="text-[12px] text-spokn-text-3 px-1">{helpText}</p>
      )}

      <button
        type="button"
        onClick={() => setShowAll((v) => !v)}
        className="group flex items-center gap-1.5 text-[12px] font-medium text-spokn-text-2 hover:text-spokn-text transition-colors px-1 cursor-pointer"
      >
        {showAll ? (
          <ChevronUp size={14} strokeWidth={2} />
        ) : (
          <ChevronDown size={14} strokeWidth={2} />
        )}
        {/* eslint-disable-next-line i18next/no-literal-string */}
        {showAll
          ? "Hide all languages"
          : `Show all ${ALL_LANGUAGES.length} languages`}
      </button>

      {showAll && (
        <div className="rounded-xl border border-spokn-hairline bg-spokn-surface overflow-hidden">
          <div className="flex items-center gap-2 px-3 py-2 border-b border-spokn-hairline">
            <Search size={14} strokeWidth={2} className="text-spokn-text-3" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search by name…"
              className="flex-1 bg-transparent outline-none text-[13px] text-spokn-text placeholder:text-spokn-text-3"
              autoFocus
            />
            {query && (
              <span className="text-[11px] font-mono text-spokn-text-3">
                {filteredAll.length}
              </span>
            )}
          </div>
          <div className="max-h-72 overflow-y-auto divide-y divide-spokn-hairline">
            {filteredAll.length === 0 ? (
              <div className="px-3 py-4 text-center text-[12px] text-spokn-text-3">
                {/* eslint-disable-next-line i18next/no-literal-string */}
                No matches.
              </div>
            ) : (
              filteredAll.map((lang) => {
                const isSelected = selected.has(lang.code);
                return (
                  <button
                    key={lang.code}
                    type="button"
                    onClick={() => toggle(lang.code)}
                    className={`flex items-center justify-between w-full px-3 py-2 text-left transition-colors cursor-pointer ${
                      isSelected
                        ? "bg-spokn-accent-blue/10"
                        : "hover:bg-spokn-surface-2"
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="text-[13px] font-medium text-spokn-text truncate">
                        {lang.native}
                      </span>
                      <span className="text-[11px] text-spokn-text-3 uppercase tracking-wider font-mono truncate">
                        {lang.english}
                      </span>
                    </div>
                    {isSelected && (
                      <span
                        aria-hidden
                        className="w-1.5 h-1.5 rounded-full shrink-0"
                        style={{ background: "var(--spokn-accent-grad)" }}
                      />
                    )}
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default LanguagePicker;
