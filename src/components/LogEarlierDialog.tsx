import { useEffect, useState } from "react";
import { type Api, type DoseKind, type MedicationSummary, errorMessage } from "../lib/api";
import { nowLocalInput, medLabel } from "../lib/format";
import { Button, Field, Modal, inputClass } from "./ui";

export function LogEarlierDialog({
  api,
  open,
  onOpenChange,
  onSaved,
}: {
  api: Api;
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onSaved: () => void;
}) {
  const [meds, setMeds] = useState<MedicationSummary[]>([]);
  const [medId, setMedId] = useState("");
  const [kind, setKind] = useState<DoseKind>("take");
  const [when, setWhen] = useState(nowLocalInput());
  const [note, setNote] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    setWhen(nowLocalInput());
    setError("");
    api.listMedications().then((all) => {
      const active = all.filter((m) => !m.archived);
      setMeds(active);
      setMedId((cur) => cur || active[0]?.id || "");
    });
  }, [open, api]);

  async function save() {
    try {
      await api.recordDose({ medication_id: medId, kind, slot_id: null, local_date: null, occurred_local: when, note: note || null });
      setNote("");
      onOpenChange(false);
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <Modal open={open} onOpenChange={onOpenChange} title="Log an earlier dose" description="Record a dose that already happened, at the time you say it did.">
      <div className="space-y-3">
        <Field label="Medication">
          <select className={inputClass} value={medId} onChange={(e) => setMedId(e.target.value)}>
            {meds.map((m) => (
              <option key={m.id} value={m.id}>{medLabel(m.name, m.strength)}</option>
            ))}
          </select>
        </Field>
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
          <Button variant="primary" className="flex-1" disabled={!medId} onClick={save}>Save</Button>
          <Button className="flex-1" onClick={() => onOpenChange(false)}>Cancel</Button>
        </div>
      </div>
    </Modal>
  );
}
