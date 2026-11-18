import type { Api, AppError, HistoryEntry, TodayItem } from "../lib/api";

/** Minimal in-memory stand-in for the Rust commands; rules are tested in Rust. */
export function fakeApi(opts: { startupError?: AppError } = {}): Api {
  const slot = (): TodayItem => ({
    slot_id: "s1", medication_id: "m1", name: "Metformin", strength: "500 mg", form: "tablet",
    local_date: "2026-03-10", local_time: "08:00", status: "due", event_ids: [],
  });
  let today = slot();
  let history: HistoryEntry[] = [];
  const entry = (kind: "take" | "skip"): HistoryEntry => ({
    event_id: "e1", medication_id: "m1", name: "Metformin", strength: "500 mg", kind, undone: false,
    deleted: false, occurred_at_utc_us: 1, entered_at_utc_us: 2, original_local: "2026-03-10T09:30", original_offset_seconds: -21600,
    original_zone_id: null, current_local: "2026-03-10T09:30", current_offset_seconds: -21600, note: "felt fine", slot_time: "08:00", revisions: [],
  });
  return {
    startupStatus: async () => opts.startupError ?? null,
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
    restoreDose: async () => {
      today = { ...today, status: "taken", event_ids: ["e1"] };
      history = history.map((h) => ({ ...h, undone: false, revisions: [...h.revisions, { revision_number: 2, changed_at_utc_us: 4, reason: "restore", before: null, after: null }] }));
    },
    // Mirrors the Rust tombstone: id kept, note and payloads gone, never counts as a dose.
    deleteDoseEntry: async () => {
      today = slot();
      history = history.map((h) => ({
        ...h, undone: false, deleted: true, note: null,
        revisions: [...h.revisions.map((r) => ({ ...r, before: null, after: null })), { revision_number: 9, changed_at_utc_us: 5, reason: "delete" as const, before: null, after: null }],
      }));
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
