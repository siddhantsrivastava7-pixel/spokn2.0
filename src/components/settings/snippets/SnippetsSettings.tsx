import React, { useEffect, useState } from "react";
import { Plus, Trash2, Pencil, Check, X, Zap } from "lucide-react";
import { toast } from "sonner";
import { commands, type Snippet } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";

/* Snippets UI — create, list, edit, delete. When the user speaks a
 * snippet's trigger during dictation, the backend expands it to the
 * snippet's expansion before injecting the text. */

type EditState = {
  id: string | "new";
  trigger: string;
  expansion: string;
};

export const SnippetsSettings: React.FC = () => {
  const { settings, refreshSettings } = useSettings();
  const [editing, setEditing] = useState<EditState | null>(null);
  const [busy, setBusy] = useState(false);

  const snippets = (settings?.snippets ?? []) as Snippet[];

  const byMostUsed = [...snippets].sort(
    (a, b) => (b.hits ?? 0) - (a.hits ?? 0),
  );

  const resetEdit = () => setEditing(null);

  const save = async () => {
    if (!editing) return;
    const trigger = editing.trigger.trim();
    const expansion = editing.expansion;
    if (!trigger || !expansion) {
      toast.error("Trigger and expansion can't be empty");
      return;
    }
    setBusy(true);
    try {
      if (editing.id === "new") {
        const res = await commands.addSnippet(trigger, expansion);
        if (res.status === "error") {
          toast.error(res.error);
        } else {
          toast.success("Snippet added");
          resetEdit();
        }
      } else {
        const res = await commands.updateSnippet(editing.id, trigger, expansion);
        if (res.status === "error") {
          toast.error(res.error);
        } else {
          toast.success("Snippet updated");
          resetEdit();
        }
      }
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    if (!confirm("Delete this snippet?")) return;
    setBusy(true);
    try {
      const res = await commands.deleteSnippet(id);
      if (res.status === "error") {
        toast.error(res.error);
      } else {
        toast.success("Snippet deleted");
      }
      await refreshSettings();
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    // Reset edit state if the snippet being edited disappeared (e.g. from
    // another window).
    if (editing && editing.id !== "new") {
      if (!snippets.find((s) => s.id === editing.id)) resetEdit();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snippets.length]);

  return (
    <div className="w-full space-y-6">
      {/* Header with intro + add button */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-spokn-text tracking-tight">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            Snippets
          </h2>
          <p className="mt-1 text-[13px] text-spokn-text-2 max-w-lg leading-relaxed">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            Say a short trigger like <em>"my youtube link"</em> and Spokn
            swaps it for your full expansion before typing. Case-insensitive
            word-boundary match.
          </p>
        </div>
        <Button
          variant="primary"
          size="md"
          onClick={() =>
            setEditing({ id: "new", trigger: "", expansion: "" })
          }
          className="flex items-center gap-1.5 shrink-0"
          disabled={busy || editing?.id === "new"}
        >
          <Plus size={14} strokeWidth={2.2} />
          {/* eslint-disable-next-line i18next/no-literal-string */}
          Add snippet
        </Button>
      </div>

      {/* Inline editor for new entry */}
      {editing?.id === "new" && (
        <EditForm
          editing={editing}
          setEditing={setEditing}
          onSave={save}
          onCancel={resetEdit}
          busy={busy}
        />
      )}

      {/* List */}
      {snippets.length === 0 && editing?.id !== "new" ? (
        <div className="rounded-xl border border-spokn-hairline bg-spokn-surface p-8 text-center">
          <Zap
            size={22}
            strokeWidth={1.4}
            className="mx-auto text-spokn-text-3 mb-3"
          />
          <p className="text-[13px] text-spokn-text-2">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            No snippets yet. Add one above to get started.
          </p>
        </div>
      ) : (
        <div className="rounded-xl border border-spokn-hairline bg-spokn-surface overflow-hidden backdrop-blur-sm shadow-spokn-sm">
          <div className="divide-y divide-spokn-hairline">
            {byMostUsed.map((snip) =>
              editing?.id === snip.id ? (
                <EditForm
                  key={snip.id}
                  editing={editing}
                  setEditing={setEditing}
                  onSave={save}
                  onCancel={resetEdit}
                  busy={busy}
                />
              ) : (
                <SnippetRow
                  key={snip.id}
                  snippet={snip}
                  onEdit={() =>
                    setEditing({
                      id: snip.id,
                      trigger: snip.trigger,
                      expansion: snip.expansion,
                    })
                  }
                  onDelete={() => remove(snip.id)}
                  disabled={busy}
                />
              ),
            )}
          </div>
        </div>
      )}
    </div>
  );
};

const SnippetRow: React.FC<{
  snippet: Snippet;
  onEdit: () => void;
  onDelete: () => void;
  disabled?: boolean;
}> = ({ snippet, onEdit, onDelete, disabled }) => (
  <div className="group flex items-start gap-4 px-4 py-3 hover:bg-spokn-surface-2/60 transition-colors">
    <div className="flex-1 min-w-0 space-y-1">
      <div className="flex items-center gap-2">
        <span
          className="inline-block font-mono text-[12px] px-1.5 py-0.5 rounded bg-spokn-accent-blue/15 text-spokn-accent-blue border border-spokn-accent-blue/25"
          style={{ letterSpacing: 0 }}
        >
          {snippet.trigger}
        </span>
        {snippet.hits > 0 && (
          <span className="text-[10px] font-mono tracking-wider uppercase text-spokn-text-3">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            {snippet.hits}× used
          </span>
        )}
      </div>
      <p className="text-[13px] text-spokn-text-2 break-words whitespace-pre-wrap">
        {snippet.expansion}
      </p>
    </div>
    <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
      <IconButton onClick={onEdit} disabled={disabled} title="Edit">
        <Pencil size={13} strokeWidth={1.8} />
      </IconButton>
      <IconButton
        onClick={onDelete}
        disabled={disabled}
        title="Delete"
        danger
      >
        <Trash2 size={13} strokeWidth={1.8} />
      </IconButton>
    </div>
  </div>
);

const EditForm: React.FC<{
  editing: EditState;
  setEditing: (s: EditState) => void;
  onSave: () => void;
  onCancel: () => void;
  busy: boolean;
}> = ({ editing, setEditing, onSave, onCancel, busy }) => (
  <div className="flex flex-col gap-3 px-4 py-4 bg-spokn-surface-2/60">
    <div className="flex flex-col gap-1">
      <label className="text-[10px] font-mono tracking-[0.12em] uppercase text-spokn-text-3">
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Trigger
      </label>
      <Input
        value={editing.trigger}
        onChange={(e) =>
          setEditing({ ...editing, trigger: e.target.value })
        }
        placeholder="e.g. my youtube link"
        className="w-full"
        disabled={busy}
      />
    </div>
    <div className="flex flex-col gap-1">
      <label className="text-[10px] font-mono tracking-[0.12em] uppercase text-spokn-text-3">
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Expansion
      </label>
      <Textarea
        value={editing.expansion}
        onChange={(e) =>
          setEditing({ ...editing, expansion: e.target.value })
        }
        placeholder="https://youtube.com/encryptictv"
        className="w-full min-h-[72px]"
        disabled={busy}
      />
    </div>
    <div className="flex items-center justify-end gap-2">
      <Button
        variant="ghost"
        size="sm"
        onClick={onCancel}
        disabled={busy}
        className="flex items-center gap-1"
      >
        <X size={13} strokeWidth={2} />
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Cancel
      </Button>
      <Button
        variant="primary"
        size="sm"
        onClick={onSave}
        disabled={busy || !editing.trigger.trim() || !editing.expansion}
        className="flex items-center gap-1"
      >
        <Check size={13} strokeWidth={2} />
        {/* eslint-disable-next-line i18next/no-literal-string */}
        Save
      </Button>
    </div>
  </div>
);

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  danger?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, danger, children }) => (
  <button
    type="button"
    onClick={onClick}
    title={title}
    disabled={disabled}
    className={`h-7 w-7 rounded-md flex items-center justify-center transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
      danger
        ? "text-spokn-text-3 hover:text-spokn-danger hover:bg-spokn-danger/10"
        : "text-spokn-text-3 hover:text-spokn-text hover:bg-spokn-surface"
    }`}
  >
    {children}
  </button>
);

export default SnippetsSettings;
