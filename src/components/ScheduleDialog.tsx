import { useEffect, useRef, useState } from "react";
import { type Api, type MedicationSummary, type PreviewOccurrence, type ScheduleInput, errorMessage } from "../lib/api";
import { WEEKDAYS_LONG, deviceZone, formatOccurrence, nowLocalInput, zoneChoices } from "../lib/format";
import { Button, Field, Modal, inputClass } from "./ui";

type Mode = ScheduleInput["mode"];

/**
 * Create or edit a schedule. Two modes, never silently interchangeable:
 *  - fixed times in one chosen time zone (follows that zone's clock wherever you travel)
 *  - every N hours (absolute time: clock changes and travel never shift it)
 * Always previews the next seven times before saving. Editing changes the future only.
 */
export function ScheduleDialog({ api, med, onClose, onSaved }: { api: Api; med: MedicationSummary; onClose: () => void; onSaved: () => void }) {
  const current = med.schedule && med.schedule.state !== "ended" ? med.schedule : null;
  const editing = current !== null;
  const initialAnchor = current?.anchor_utc_us ? nowLocalInput(new Date(current.anchor_utc_us / 1000)) : null;

  const [mode, setMode] = useState<Mode>(current?.mode ?? "home_zone");
  const [times, setTimes] = useState<string[]>(current?.mode === "home_zone" && current.slots.length ? current.slots.map((s) => s.time) : ["08:00"]);
  const [weekdays, setWeekdays] = useState<number[]>(current?.mode === "home_zone" ? (current.slots[0]?.weekdays ?? []) : []);
  const [zone, setZone] = useState<string>(current?.zone_id ?? deviceZone() ?? "UTC");
  const [hours, setHours] = useState<string>(current?.interval_seconds ? String(current.interval_seconds / 3600) : "8");
  const [firstDose, setFirstDose] = useState<string>(initialAnchor ?? nowLocalInput());
  const [error, setError] = useState("");
  const [preview, setPreview] = useState<PreviewOccurrence[] | null>(null);
  const [previewError, setPreviewError] = useState("");
  const seq = useRef(0);

  const intervalSeconds = Math.round(parseFloat(hours) * 3600);
  const input: ScheduleInput =
    mode === "home_zone"
      ? { mode, zone_id: zone, slots: times.map((time) => ({ time, weekdays })), interval_seconds: null, anchor_local: null }
      : {
          mode,
          zone_id: null,
          slots: [],
          interval_seconds: Number.isFinite(intervalSeconds) ? intervalSeconds : 0,
          // Unchanged first dose while editing: keep the stored instant instead of re-reading it on today's clock.
          anchor_local: editing && current?.mode === "elapsed_interval" && firstDose === initialAnchor ? null : firstDose,
        };
  const inputKey = JSON.stringify(input);

  useEffect(() => {
    const mine = ++seq.current;
    const t = setTimeout(() => {
      api.previewSchedule(JSON.parse(inputKey) as ScheduleInput).then(
        (p) => {
          if (mine === seq.current) {
            setPreview(p);
            setPreviewError("");
          }
        },
        (e) => {
          if (mine === seq.current) {
            setPreview(null);
            setPreviewError(errorMessage(e));
          }
        },
      );
    }, 200);
    return () => clearTimeout(t);
  }, [api, inputKey]);

  const toggleDay = (d: number) => setWeekdays((cur) => (cur.includes(d) ? cur.filter((x) => x !== d) : [...cur, d].sort()));

  async function save() {
    try {
      if (current) await api.editSchedule(current.id, input);
      else await api.createSchedule(med.id, input);
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  const zones = zoneChoices(zone, deviceZone());

  return (
    <Modal
      open
      onOpenChange={(o) => !o && onClose()}
      title={editing ? `Edit schedule: ${med.name}` : `Add schedule: ${med.name}`}
      description={editing ? "Changes apply from now on. Doses you already recorded, and History, are not changed." : "Choose when this medication shows up on Today."}
    >
      <div className="space-y-4">
        <fieldset className="space-y-1">
          <legend className="text-sm font-medium text-neutral-800">How is it scheduled?</legend>
          <label className="flex min-h-12 items-center gap-3">
            <input type="radio" name="mode" className="size-6" checked={mode === "home_zone"} onChange={() => setMode("home_zone")} />
            <span>At set times of day</span>
          </label>
          <label className="flex min-h-12 items-center gap-3">
            <input type="radio" name="mode" className="size-6" checked={mode === "elapsed_interval"} onChange={() => setMode("elapsed_interval")} />
            <span>Every so many hours</span>
          </label>
        </fieldset>

        {mode === "home_zone" ? (
          <>
            <div className="space-y-2">
              <p className="text-sm font-medium text-neutral-800">Times of day</p>
              {times.map((t, i) => (
                <div key={i} className="flex gap-2">
                  <input className={inputClass} type="time" aria-label={`Time ${i + 1}`} value={t} onChange={(e) => setTimes((cur) => cur.map((x, j) => (j === i ? e.target.value : x)))} />
                  <Button disabled={times.length === 1} aria-label={`Remove time ${i + 1}`} onClick={() => setTimes((cur) => cur.filter((_, j) => j !== i))}>
                    Remove
                  </Button>
                </div>
              ))}
              <Button onClick={() => setTimes((cur) => [...cur, "20:00"])} disabled={times.length >= 24}>Add another time</Button>
            </div>
            <fieldset className="space-y-1">
              <legend className="text-sm font-medium text-neutral-800">Days (none selected means every day)</legend>
              <div className="grid grid-cols-2 gap-x-3">
                {WEEKDAYS_LONG.map((name, i) => (
                  <label key={name} className="flex min-h-12 items-center gap-3">
                    <input type="checkbox" className="size-6" checked={weekdays.includes(i + 1)} onChange={() => toggleDay(i + 1)} />
                    <span>{name}</span>
                  </label>
                ))}
              </div>
            </fieldset>
            <Field label="Time zone">
              <select className={inputClass} value={zone} onChange={(e) => setZone(e.target.value)}>
                {zones.map((z) => (
                  <option key={z} value={z}>{z}</option>
                ))}
              </select>
            </Field>
            <p className="text-sm text-neutral-700">These times follow the clock in this time zone, wherever you are. Travelling doesn’t move them.</p>
          </>
        ) : (
          <>
            <Field label="Hours between doses">
              <input className={inputClass} type="number" inputMode="decimal" min="0.25" max="720" step="0.25" value={hours} onChange={(e) => setHours(e.target.value)} />
            </Field>
            <Field label="First dose">
              <input className={inputClass} type="datetime-local" value={firstDose} onChange={(e) => setFirstDose(e.target.value)} />
            </Field>
            <p className="text-sm text-neutral-700">Counted in real elapsed time from the first dose. Clock changes and travel never shift it. A late dose doesn’t move the next one; to start counting again, edit the first dose.</p>
          </>
        )}

        <div aria-live="polite" className="rounded-lg bg-neutral-100 p-3">
          <p className="text-sm font-medium text-neutral-800">Next 7 times</p>
          {previewError ? (
            <p className="text-sm text-neutral-700">{previewError}</p>
          ) : preview === null ? (
            <p className="text-sm text-neutral-700">Checking…</p>
          ) : (
            <ol data-testid="schedule-preview" className="mt-1 space-y-0.5 text-sm">
              {preview.map((p) => (
                <li key={p.scheduled_at_utc_us}>{formatOccurrence(p)}</li>
              ))}
            </ol>
          )}
        </div>

        {error && <p role="alert" className="text-red-800">{error}</p>}
        <div className="flex gap-2">
          <Button variant="primary" className="flex-1" onClick={save}>Save schedule</Button>
          <Button className="flex-1" onClick={onClose}>Cancel</Button>
        </div>
      </div>
    </Modal>
  );
}
