import type { Api, HistoryEntry, TodayItem } from "../lib/api";

/** Minimal in-memory stand-in for the Rust commands; rules are tested in Rust. */
export function fakeApi(): Api {
  const slot = (): TodayItem => ({
    slot_id: "s1", medication_id: "m1", name: "Metformin", strength: "500 mg", form: "tablet",
    local_date: "2026-03-10", local_time: "08:00", status: "due", event_ids: [],
  });
  let today = slot();
  let history: HistoryEntry[] = [];
  const entry = (kind: "take" | "skip"): HistoryEntry => ({
    event_id: "e1", medication_id: "m1", name: "Metformin", strength: "500 mg", kind, undone: false,
    occurred_at_utc_us: 1, entered_at_utc_us: 2, original_local: "2026-03-10T09:30", original_offset_seconds: -21600,
    current_local: "2026-03-10T09:30", note: null, slot_time: "08:00", revisions: [],
  });
  return {
    listToday: async () => [today],
    recordDose: async (i) => {
      today = { ...today, status: i.kind === "take" ? "taken" : "skipped", event_ids: ["e1"] };
      history = [entry(i.kind)];
      return "e1";
    },
    undoDose: async () => {
      today = slot();
      history = history.map((h) => ({ ...h, undone: true, revisions: [{ revision_number: 1, changed_at_utc_us: 3, reason: "undo", before: null, after: null }] }));
    },
    correctDose: async (i) => {
      const kind = i.kind ?? history[0].kind;
      history = history.map((h) => ({
        ...h, kind, revisions: [{ revision_number: 1, changed_at_utc_us: 3, reason: "correction", before: '{"kind":"take"}', after: `{"kind":"${kind}"}` }],
      }));
    },
    listHistory: async () => history,
    createMedication: async () => "m1",
    listMedications: async () => [{ id: "m1", name: "Metformin", strength: "500 mg", form: "tablet", archived: false, times: ["08:00"] }],
    archiveMedication: async () => {},
    deleteMedication: async () => {},
  };
}
