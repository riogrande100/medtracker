import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { useCallback, useEffect, useRef, useState } from "react";
import { type Api, type DoseKind, type HistoryCursor, type HistoryEntry, errorMessage } from "../lib/api";
import { formatLocal, formatOffset, medLabel } from "../lib/format";
import { InAppErasureNote, UpgradeCopyNote } from "./PrivacyNotes";
import { Button, Field, Modal, inputClass } from "./ui";

const PAGE = 50;
const MAX_RELOAD = 1000;

function CorrectDialog({ api, entry, onClose, onSaved }: { api: Api; entry: HistoryEntry; onClose: () => void; onSaved: () => void }) {
  const [kind, setKind] = useState<DoseKind>(entry.kind);
  const [when, setWhen] = useState(entry.current_local);
  const [note, setNote] = useState(entry.note ?? "");
  const [error, setError] = useState("");

  async function save() {
    try {
      await api.correctDose({
        event_id: entry.event_id,
        kind: kind === entry.kind ? null : kind,
        occurred_local: when === entry.current_local ? null : when,
        note: note === (entry.note ?? "") ? null : note,
      });
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <Modal open onOpenChange={(o) => !o && onClose()} title="Correct entry" description="The original entry stays visible in the change history.">
      <div className="space-y-3">
        <Field label="What happened">
          <select className={inputClass} value={kind} onChange={(e) => setKind(e.target.value as DoseKind)}>
            <option value="take">Taken</option>
            <option value="skip">Skipped</option>
          </select>
        </Field>
        <Field label="When">
          <input className={inputClass} type="datetime-local" value={when} onChange={(e) => setWhen(e.target.value)} />
        </Field>
        <Field label="Note (optional)">
          <input className={inputClass} maxLength={500} value={note} onChange={(e) => setNote(e.target.value)} />
        </Field>
        {error && <p role="alert" className="text-red-800">{error}</p>}
        <div className="flex gap-2">
          <Button variant="primary" className="flex-1" onClick={save}>Save correction</Button>
          <Button className="flex-1" onClick={onClose}>Cancel</Button>
        </div>
      </div>
    </Modal>
  );
}

/** Permanent privacy deletion. Deliberately worded and styled differently from Undo. */
function DeleteEntryDialog({ entry, upgradeCopy, onConfirm }: { entry: HistoryEntry; upgradeCopy: boolean; onConfirm: () => void }) {
  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger asChild>
        <Button variant="danger">Delete permanently<span className="sr-only"> {entry.name} entry</span></Button>
      </AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 bg-black/40" />
        <AlertDialog.Content className="fixed inset-x-3 top-6 mx-auto max-w-md rounded-xl bg-white p-5 shadow-lg">
          <AlertDialog.Title className="text-xl font-semibold">Permanently delete this entry's details?</AlertDialog.Title>
          <AlertDialog.Description className="my-3 space-y-2">
            <span className="block">
              This erases the note and the saved before-and-after contents of every change to this {entry.name} entry. It does not erase
              that the entry existed, which medication it was for, its time, or whether it was taken or skipped: those stay in History,
              with a marker that it was deleted. This is not Undo: it can't be reversed or restored.
            </span>
            <InAppErasureNote />
            <UpgradeCopyNote show={upgradeCopy} />
          </AlertDialog.Description>
          <div className="flex gap-2">
            <AlertDialog.Cancel asChild><Button className="flex-1">Keep it</Button></AlertDialog.Cancel>
            <AlertDialog.Action asChild><Button variant="danger" className="flex-1" onClick={onConfirm}>Delete details permanently</Button></AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}

export function HistoryView({ api, version, onChanged }: { api: Api; version: number; onChanged: () => void }) {
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [cursor, setCursor] = useState<HistoryCursor | null>(null);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [upgradeCopy, setUpgradeCopy] = useState(false);
  const [editing, setEditing] = useState<HistoryEntry | null>(null);
  const [message, setMessage] = useState("");
  const loaded = useRef(0);

  // Reload keeps as many entries on screen as were already loaded, so a change near the top
  // doesn't collapse the older pages the user scrolled to.
  useEffect(() => {
    let live = true;
    api.listHistory({ limit: Math.min(MAX_RELOAD, Math.max(PAGE, loaded.current)) }).then(
      (p) => {
        if (!live) return;
        loaded.current = p.entries.length;
        setEntries(p.entries);
        setCursor(p.next_cursor);
      },
      (e) => live && setMessage(errorMessage(e)),
    );
    api.upgradeCopyExists().then((v) => live && setUpgradeCopy(v), () => {});
    return () => {
      live = false;
    };
  }, [api, version]);

  const loadOlder = useCallback(async () => {
    if (!cursor) return;
    setLoadingOlder(true);
    try {
      const p = await api.listHistory({ limit: PAGE, before: cursor });
      setEntries((cur) => {
        const next = [...(cur ?? []), ...p.entries];
        loaded.current = next.length;
        return next;
      });
      setCursor(p.next_cursor);
    } catch (e) {
      setMessage(errorMessage(e));
    } finally {
      setLoadingOlder(false);
    }
  }, [api, cursor]);

  async function run(fn: () => Promise<void>) {
    try {
      await fn();
      setMessage("");
      onChanged();
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  return (
    <section aria-labelledby="history-h" className="space-y-4">
      <h2 id="history-h" className="text-2xl font-semibold">History</h2>
      <p className="text-sm text-neutral-700">Times are what you reported on this device. Nothing here is verified or medical advice.</p>
      {message && <p role="alert" className="text-red-800">{message}</p>}
      {entries === null ? (
        <p>Loading…</p>
      ) : entries.length === 0 ? (
        <p className="rounded-lg border border-neutral-300 bg-white p-4">No doses recorded yet.</p>
      ) : (
        <ul className="space-y-3">
          {entries.map((e) => (
            <li key={e.event_id} className={`rounded-xl border border-neutral-300 p-4 ${e.undone || e.deleted ? "bg-neutral-100" : "bg-white"}`}>
              <p className="text-lg font-semibold">
                {medLabel(e.name, e.strength)} — {e.deleted ? `Deleted (was ${e.kind === "take" ? "taken" : "skipped"})` : e.undone ? "Undone" : e.kind === "take" ? "Taken" : "Skipped"}
              </p>
              <p>{formatLocal(e.current_local)}{e.slot_time ? ` (scheduled ${e.slot_time})` : " (not tied to a schedule)"}</p>
              <p className="text-sm text-neutral-700">
                Originally entered as {formatLocal(e.original_local)} {formatOffset(e.original_offset_seconds)}
                {e.original_zone_id ? ` (${e.original_zone_id})` : ""}
              </p>
              {e.deleted && (
                <p className="text-sm">
                  Note and change details permanently deleted. The medication, time and whether it was taken or skipped stay in History.
                </p>
              )}
              {e.note && <p className="text-sm">Note: {e.note}</p>}
              {e.revisions.length > 0 && (
                <details className="mt-2">
                  <summary className="min-h-12 cursor-pointer py-3 text-sm font-medium">Change history ({e.revisions.length})</summary>
                  <ol className="space-y-1 text-sm">
                    {e.revisions.map((r) => (
                      <li key={r.revision_number}>
                        {{ undo: "Undone", restore: "Restored", delete: "Details permanently deleted", correction: "Corrected" }[r.reason]}
                        {r.before && r.after && (
                          <span className="block font-mono text-xs text-neutral-700">{r.before} → {r.after}</span>
                        )}
                      </li>
                    ))}
                  </ol>
                </details>
              )}
              {!e.deleted && (
                <div className="mt-2 flex flex-wrap gap-2">
                  {e.undone ? (
                    <Button onClick={() => run(() => api.restoreDose(e.event_id))}>Restore<span className="sr-only"> {e.name} entry</span></Button>
                  ) : (
                    <>
                      <Button onClick={() => setEditing(e)}>Correct<span className="sr-only"> {e.name} entry</span></Button>
                      <Button onClick={() => run(() => api.undoDose(e.event_id))}>Undo<span className="sr-only"> {e.name} entry</span></Button>
                    </>
                  )}
                  <DeleteEntryDialog entry={e} upgradeCopy={upgradeCopy} onConfirm={() => run(() => api.deleteDoseEntry(e.event_id))} />
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {cursor && (
        <Button className="w-full" disabled={loadingOlder} onClick={loadOlder}>
          {loadingOlder ? "Loading…" : "Load older entries"}
        </Button>
      )}
      {editing && (
        <CorrectDialog api={api} entry={editing} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); onChanged(); }} />
      )}
    </section>
  );
}
