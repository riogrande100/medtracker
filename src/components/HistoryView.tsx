import { useEffect, useState } from "react";
import { type Api, type DoseKind, type HistoryEntry, errorMessage } from "../lib/api";
import { formatLocal, formatOffset, medLabel } from "../lib/format";
import { Button, Field, Modal, inputClass } from "./ui";

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

export function HistoryView({ api, version, onChanged }: { api: Api; version: number; onChanged: () => void }) {
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [editing, setEditing] = useState<HistoryEntry | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    api.listHistory().then(setEntries, (e) => setMessage(errorMessage(e)));
  }, [api, version]);

  async function undo(e: HistoryEntry) {
    try {
      await api.undoDose(e.event_id);
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
            <li key={e.event_id} className={`rounded-xl border border-neutral-300 p-4 ${e.undone ? "bg-neutral-100" : "bg-white"}`}>
              <p className="text-lg font-semibold">
                {medLabel(e.name, e.strength)} — {e.undone ? "Undone" : e.kind === "take" ? "Taken" : "Skipped"}
              </p>
              <p>{formatLocal(e.current_local)}{e.slot_time ? ` (scheduled ${e.slot_time})` : " (not tied to a schedule)"}</p>
              <p className="text-sm text-neutral-700">
                Originally entered as {formatLocal(e.original_local)} {formatOffset(e.original_offset_seconds)}
              </p>
              {e.note && <p className="text-sm">Note: {e.note}</p>}
              {e.revisions.length > 0 && (
                <details className="mt-2">
                  <summary className="min-h-12 cursor-pointer py-3 text-sm font-medium">Change history ({e.revisions.length})</summary>
                  <ol className="space-y-1 text-sm">
                    {e.revisions.map((r) => (
                      <li key={r.revision_number}>
                        {r.reason === "undo" ? "Undone" : "Corrected"}
                        {r.before && r.after && (
                          <span className="block font-mono text-xs text-neutral-700">{r.before} → {r.after}</span>
                        )}
                      </li>
                    ))}
                  </ol>
                </details>
              )}
              {!e.undone && (
                <div className="mt-2 flex gap-2">
                  <Button onClick={() => setEditing(e)}>Correct<span className="sr-only"> {e.name} entry</span></Button>
                  <Button onClick={() => undo(e)}>Undo<span className="sr-only"> {e.name} entry</span></Button>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {editing && (
        <CorrectDialog api={api} entry={editing} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); onChanged(); }} />
      )}
    </section>
  );
}
