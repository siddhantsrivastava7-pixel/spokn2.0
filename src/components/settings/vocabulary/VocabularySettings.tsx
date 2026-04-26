import React, { useMemo, useState } from "react";
import { Trash2, BookOpen, Plus } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

/* Vocabulary — auto-learned words via the v0.3.9 confidence pipeline.
 *
 * Two sections: Active (confidence >= ACTIVE_THRESHOLD, currently
 * biasing Whisper) and Learning (1 <= confidence < ACTIVE_THRESHOLD,
 * not yet trusted). User can delete any entry. Confidence rises on
 * acceptance / further correction toward this word, falls when the
 * user reverts it. */
const ACTIVE_THRESHOLD = 3;

interface VocabEntry {
  word: string;
  confidence: number;
  samples_seen: number;
  samples_kept: number;
  first_corrected_at: number;
  last_seen_at: number;
}

export const VocabularySettings: React.FC = () => {
  const { settings, refreshSettings } = useSettings();
  const [busy, setBusy] = useState(false);

  const entries: VocabEntry[] = ((settings as any)?.vocab_entries ?? []) as
    VocabEntry[];
  const active = useMemo(
    () =>
      entries
        .filter((e) => e.confidence >= ACTIVE_THRESHOLD)
        .sort((a, b) => b.confidence - a.confidence),
    [entries],
  );
  const learning = useMemo(
    () =>
      entries
        .filter((e) => e.confidence >= 1 && e.confidence < ACTIVE_THRESHOLD)
        .sort((a, b) => b.confidence - a.confidence || b.last_seen_at - a.last_seen_at),
    [entries],
  );

  // Names = full names of OTHER people the user dictates about. Distinct
  // from auto-learned vocabulary so the UI can edit them as a clean list.
  const knownNames: string[] = (settings as any)?.known_names ?? [];
  const [newName, setNewName] = useState("");

  const persistNames = async (next: string[]) => {
    setBusy(true);
    try {
      const r = await commands.setKnownNames(next);
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const addName = async () => {
    const trimmed = newName.trim();
    if (!trimmed) return;
    if (
      knownNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())
    ) {
      toast.error(`"${trimmed}" is already in your names list`);
      return;
    }
    const next = [...knownNames, trimmed];
    setNewName("");
    await persistNames(next);
  };

  const removeName = async (name: string) => {
    const next = knownNames.filter((n) => n !== name);
    await persistNames(next);
  };

  const remove = async (word: string) => {
    if (!confirm(`Remove "${word}" from vocabulary?`)) return;
    setBusy(true);
    try {
      const r = await (commands as any).deleteVocabEntry(word);
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const clearAll = async () => {
    if (!confirm("Clear all auto-learned vocabulary? This cannot be undone."))
      return;
    setBusy(true);
    try {
      const r = await (commands as any).clearVocabEntries();
      if ((r as any).status === "error") toast.error((r as any).error);
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  // v0.3.2: nuke EVERYTHING auto-learned (custom_words + candidates)
  // and re-seed only the user's intentional name + known names. The
  // rebuild-from-scratch escape hatch when transcription quality has
  // degraded over time.
  const resetEverything = async () => {
    if (
      !confirm(
        "Reset ALL learned vocabulary?\n\nThis clears every auto-learned word and starts fresh. Your name and Names list are preserved.",
      )
    )
      return;
    setBusy(true);
    try {
      const r = await commands.resetLearnedVocabulary();
      if ((r as any).status === "error") toast.error((r as any).error);
      // eslint-disable-next-line i18next/no-literal-string
      else toast.success("Vocabulary reset");
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="w-full space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-spokn-text tracking-tight">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            Vocabulary
          </h2>
          <p className="mt-1 text-[13px] text-spokn-text-2 max-w-lg leading-relaxed">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            Spokn learns words you actually correct. Each clean
            correction nudges a word's confidence up; reverting it
            nudges it down. Words reach Active at {ACTIVE_THRESHOLD}+
            and start biasing transcription.
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <Button
            variant="danger-ghost"
            size="md"
            onClick={resetEverything}
            disabled={busy}
            title="Wipe all auto-learned words and start over (keeps your name and Names list)"
          >
            Reset all
          </Button>
        </div>
        {entries.length > 0 && (
          <Button
            variant="danger-ghost"
            size="md"
            onClick={clearAll}
            disabled={busy}
            className="shrink-0"
          >
            {/* eslint-disable-next-line i18next/no-literal-string */}
            Clear all
          </Button>
        )}
      </div>

      {/* Names — user-curated list of common names (partner, team, contacts). */}
      {/* eslint-disable i18next/no-literal-string */}
      <Section title="Names — people you dictate about">
        <div className="rounded-xl border border-spokn-hairline bg-spokn-surface overflow-hidden backdrop-blur-sm shadow-spokn-sm">
          <div className="flex items-center gap-2 px-3 py-2 border-b border-spokn-hairline">
            <Input
              type="text"
              variant="compact"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addName();
                }
              }}
              placeholder="e.g. Priya Sharma"
              className="flex-1"
              disabled={busy}
            />
            <Button
              variant="primary"
              size="sm"
              onClick={addName}
              disabled={busy || newName.trim().length === 0}
            >
              <Plus size={13} strokeWidth={2} />
              <span className="ml-1">Add</span>
            </Button>
          </div>
          {knownNames.length === 0 ? (
            <div className="px-4 py-5 text-center">
              <p className="text-[12px] text-spokn-text-2">
                Add names Spokn struggles with — partner, kids, team-mates,
                clients. Each one nudges Whisper toward the correct spelling.
              </p>
            </div>
          ) : (
            <div className="divide-y divide-spokn-hairline">
              {knownNames.map((name) => (
                <Row
                  key={name}
                  label={name}
                  meta="name"
                  onDelete={() => removeName(name)}
                  disabled={busy}
                  accent
                />
              ))}
            </div>
          )}
        </div>
      </Section>
      {/* eslint-enable i18next/no-literal-string */}

      {/* Active — currently biasing Whisper */}
      {/* eslint-disable i18next/no-literal-string */}
      <Section title="Active — biasing transcription">
        {active.length === 0 ? (
          <Empty>
            No active words yet. Edit a transcript with a clean 1-word swap
            (e.g. "Raj" → "Rajesh") to start teaching Spokn.
          </Empty>
        ) : (
          <List>
            {active.map((e) => (
              <Row
                key={e.word}
                label={e.word}
                meta={`confidence ${e.confidence}`}
                onDelete={() => remove(e.word)}
                disabled={busy}
                accent
              />
            ))}
          </List>
        )}
      </Section>

      {/* Learning — confidence 1 or 2, not yet active */}
      <Section title={`Learning — needs confidence ${ACTIVE_THRESHOLD}+`}>
        {learning.length === 0 ? (
          <Empty>Nothing in flight. New corrections land here first.</Empty>
        ) : (
          <List>
            {learning.map((e) => (
              <Row
                key={e.word}
                label={e.word}
                meta={`${e.confidence}/${ACTIVE_THRESHOLD}`}
                onDelete={() => remove(e.word)}
                disabled={busy}
              />
            ))}
          </List>
        )}
      </Section>
      {/* eslint-enable i18next/no-literal-string */}
    </div>
  );
};

const Section: React.FC<{ title: string; children: React.ReactNode }> = ({
  title,
  children,
}) => (
  <div className="space-y-2">
    <h3 className="text-[10px] font-medium text-spokn-text-3 uppercase tracking-[0.12em] font-mono px-1">
      {title}
    </h3>
    {children}
  </div>
);

const List: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="rounded-xl border border-spokn-hairline bg-spokn-surface overflow-hidden backdrop-blur-sm shadow-spokn-sm divide-y divide-spokn-hairline">
    {children}
  </div>
);

const Empty: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="rounded-xl border border-spokn-hairline bg-spokn-surface px-4 py-6 text-center">
    <BookOpen
      size={18}
      strokeWidth={1.4}
      className="mx-auto text-spokn-text-3 mb-2"
    />
    <p className="text-[12px] text-spokn-text-2">{children}</p>
  </div>
);

const Row: React.FC<{
  label: string;
  meta: string;
  onDelete: () => void;
  disabled?: boolean;
  accent?: boolean;
}> = ({ label, meta, onDelete, disabled, accent }) => (
  <div className="group flex items-center justify-between px-4 py-2.5 hover:bg-spokn-surface-2/60 transition-colors">
    <div className="flex items-center gap-3 min-w-0">
      <span
        className={`font-mono text-[13px] px-1.5 py-0.5 rounded ${
          accent
            ? "bg-spokn-accent-blue/15 text-spokn-accent-blue border border-spokn-accent-blue/25"
            : "bg-spokn-surface-2 text-spokn-text border border-spokn-hairline"
        }`}
        style={{ letterSpacing: 0 }}
      >
        {label}
      </span>
      <span className="text-[10px] font-mono tracking-wider uppercase text-spokn-text-3">
        {meta}
      </span>
    </div>
    <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
      <button
        onClick={onDelete}
        disabled={disabled}
        title="Delete"
        className="h-7 w-7 rounded-md flex items-center justify-center text-spokn-text-3 hover:text-spokn-danger hover:bg-spokn-danger/10 transition-colors disabled:opacity-40"
      >
        <Trash2 size={13} strokeWidth={2} />
      </button>
    </div>
  </div>
);

export default VocabularySettings;
