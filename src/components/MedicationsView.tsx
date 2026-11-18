import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { useEffect, useState } from "react";
import { type Api, type MedicationSummary, type ScheduleSummary, errorMessage } from "../lib/api";
import { formatInterval, formatLocal, formatOccurrence, formatTime, formatWeekdays, medLabel, nowLocalInput } from "../lib/format";
import { InAppErasureNote, UpgradeCopyNote } from "./PrivacyNotes";
import { ScheduleDialog } from "./ScheduleDialog";
import { Button, Field, Modal, inputClass } from "./ui";

function AddDialog({ api, open, onOpenChange, onSaved }: { api: Api; open: boolean; onOpenChange: (o: boolean) => void; onSaved: () => void }) {
  const [name, setName] = useState("");
  const [strength, setStrength] = useState("");
  const [form, setForm] = useState("");
  const [time, setTime] = useState("08:00");
  const [daily, setDaily] = useState(true);
  const [error, setError] = useState("");

  async function save() {
    try {
      await api.createMedication({ name, strength: strength || null, form: form || null, times: daily ? [time] : [] });
      setName(""); setStrength(""); setForm(""); setError("");
      onOpenChange(false);
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <Modal open={open} onOpenChange={onOpenChange} title="Add medication" description="Type it in yourself. Everything stays on this device.">
      <div className="space-y-3">
        <Field label="Name"><input className={inputClass} value={name} onChange={(e) => setName(e.target.value)} /></Field>
        <Field label="Strength (optional, e.g. 500 mg)"><input className={inputClass} value={strength} onChange={(e) => setStrength(e.target.value)} /></Field>
        <Field label="Form (optional, e.g. tablet)"><input className={inputClass} value={form} onChange={(e) => setForm(e.target.value)} /></Field>
        <label className="flex min-h-12 items-center gap-3">
          <input type="checkbox" className="size-6" checked={daily} onChange={(e) => setDaily(e.target.checked)} />
          <span>Show on Today every day</span>
        </label>
        {daily && <Field label="Time of day"><input className={inputClass} type="time" value={time} onChange={(e) => setTime(e.target.value)} /></Field>}
        {error && <p role="alert" className="text-red-800">{error}</p>}
        <div className="flex gap-2">
          <Button variant="primary" className="flex-1" onClick={save}>Save</Button>
          <Button className="flex-1" onClick={() => onOpenChange(false)}>Cancel</Button>
        </div>
      </div>
    </Modal>
  );
}

function describe(s: ScheduleSummary | null): string {
  if (!s) return "As needed";
  const base =
    s.mode === "elapsed_interval"
      ? `Every ${formatInterval(s.interval_seconds ?? 0)}, counted from ${s.anchor_utc_us ? formatLocal(nowLocalInput(new Date(s.anchor_utc_us / 1000))) : "the first dose"}`
      : `${s.slots.length && s.slots[0].weekdays.length ? `${formatWeekdays(s.slots[0].weekdays)} at` : "Daily at"} ${s.slots.map((x) => formatTime(x.time)).join(", ")}${s.zone_id ? ` (${s.zone_id})` : ""}`;
  return s.state === "paused" ? `${base} — paused` : s.state === "ended" ? `${base} — ended` : base;
}

function EndDialog({ med, onConfirm }: { med: MedicationSummary; onConfirm: () => void }) {
  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger asChild>
        <Button aria-label={`End schedule for ${med.name}`}>End schedule</Button>
      </AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 bg-black/40" />
        <AlertDialog.Content className="fixed inset-x-3 top-6 mx-auto max-w-md rounded-xl bg-white p-5 shadow-lg">
          <AlertDialog.Title className="text-xl font-semibold">End the schedule for {med.name}?</AlertDialog.Title>
          <AlertDialog.Description className="my-3">
            {med.name} stops appearing on Today from now on. Everything already recorded stays in History. You can add a new schedule later.
          </AlertDialog.Description>
          <div className="flex gap-2">
            <AlertDialog.Cancel asChild><Button className="flex-1">Keep schedule</Button></AlertDialog.Cancel>
            <AlertDialog.Action asChild><Button variant="primary" className="flex-1" onClick={onConfirm}>End schedule</Button></AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}

function DeleteDialog({ med, upgradeCopy, onConfirm }: { med: MedicationSummary; upgradeCopy: boolean; onConfirm: () => void }) {
  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger asChild>
        <Button variant="danger">Delete<span className="sr-only"> {med.name}</span></Button>
      </AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 bg-black/40" />
        <AlertDialog.Content className="fixed inset-x-3 top-6 mx-auto max-w-md rounded-xl bg-white p-5 shadow-lg">
          <AlertDialog.Title className="text-xl font-semibold">Delete {med.name}?</AlertDialog.Title>
          <AlertDialog.Description className="my-3 space-y-2">
            <span className="block">
              This permanently removes the medication, its schedule, and every dose and correction recorded for it from MedTracker. It cannot be undone. To keep your history, archive it instead.
            </span>
            <InAppErasureNote />
            <UpgradeCopyNote show={upgradeCopy} />
          </AlertDialog.Description>
          <div className="flex gap-2">
            <AlertDialog.Cancel asChild><Button className="flex-1">Keep it</Button></AlertDialog.Cancel>
            <AlertDialog.Action asChild><Button variant="danger" className="flex-1" onClick={onConfirm}>Delete permanently</Button></AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}

export function MedicationsView({ api, version, onChanged }: { api: Api; version: number; onChanged: () => void }) {
  const [meds, setMeds] = useState<MedicationSummary[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [message, setMessage] = useState("");
  const [scheduling, setScheduling] = useState<MedicationSummary | null>(null);
  const [upgradeCopy, setUpgradeCopy] = useState(false);

  useEffect(() => {
    let live = true;
    api.listMedications().then(setMeds, (e) => setMessage(errorMessage(e)));
    api.upgradeCopyExists().then((v) => live && setUpgradeCopy(v), () => {});
    return () => {
      live = false;
    };
  }, [api, version]);

  async function run(fn: () => Promise<void>) {
    try {
      await fn();
      onChanged();
    } catch (e) {
      setMessage(errorMessage(e));
    }
  }

  return (
    <section aria-labelledby="meds-h" className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <h2 id="meds-h" className="text-2xl font-semibold">Medications</h2>
        <Button variant="primary" onClick={() => setAdding(true)}>Add medication</Button>
      </div>
      {message && <p role="alert" className="text-red-800">{message}</p>}
      {meds === null ? <p>Loading…</p> : meds.length === 0 ? (
        <p className="rounded-lg border border-neutral-300 bg-white p-4">No medications yet.</p>
      ) : (
        <ul className="space-y-3">
          {meds.map((m) => (
            <li key={m.id} className="rounded-xl border border-neutral-300 bg-white p-4">
              <p className="text-lg font-semibold">{medLabel(m.name, m.strength)}{m.archived ? " (archived)" : ""}</p>
              <p className="text-sm text-neutral-700">{[m.form, describe(m.schedule)].filter(Boolean).join(" · ")}</p>
              {m.schedule?.state === "active" && m.schedule.next[0] && (
                <p className="text-sm text-neutral-700">Next: {formatOccurrence(m.schedule.next[0])}</p>
              )}
              <div className="mt-3 flex flex-wrap gap-2">
                {!m.archived && (
                  <>
                    <Button aria-label={`${m.schedule && m.schedule.state !== "ended" ? "Edit schedule" : "Add schedule"} for ${m.name}`} onClick={() => setScheduling(m)}>
                      {m.schedule && m.schedule.state !== "ended" ? "Edit schedule" : "Add schedule"}
                    </Button>
                    {m.schedule?.state === "active" && (
                      <Button aria-label={`Pause schedule for ${m.name}`} onClick={() => run(() => api.pauseSchedule(m.schedule!.id))}>Pause schedule</Button>
                    )}
                    {m.schedule?.state === "paused" && (
                      <Button aria-label={`Resume schedule for ${m.name}`} onClick={() => run(() => api.resumeSchedule(m.schedule!.id))}>Resume schedule</Button>
                    )}
                    {(m.schedule?.state === "active" || m.schedule?.state === "paused") && (
                      <EndDialog med={m} onConfirm={() => run(() => api.endSchedule(m.schedule!.id))} />
                    )}
                  </>
                )}
                <Button onClick={() => run(() => api.archiveMedication(m.id, !m.archived))}>
                  {m.archived ? "Restore" : "Archive"}<span className="sr-only"> {m.name}</span>
                </Button>
                <DeleteDialog med={m} upgradeCopy={upgradeCopy} onConfirm={() => run(() => api.deleteMedication(m.id))} />
              </div>
            </li>
          ))}
        </ul>
      )}
      <AddDialog api={api} open={adding} onOpenChange={setAdding} onSaved={onChanged} />
      {scheduling && (
        <ScheduleDialog
          api={api}
          med={scheduling}
          onClose={() => setScheduling(null)}
          onSaved={() => {
            setScheduling(null);
            onChanged();
          }}
        />
      )}
    </section>
  );
}
