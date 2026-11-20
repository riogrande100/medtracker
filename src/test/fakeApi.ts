import type { Api, AppError, BackupFile, Report, ReportCounts, HistoryEntry, MedicationSummary, NextDue, PreviewOccurrence, TodayItem } from "../lib/api";

/** Minimal in-memory stand-in for the Rust commands; rules are tested in Rust. */
export const previewRows = (n = 7): PreviewOccurrence[] =>
  Array.from({ length: n }, (_, i) => ({
    scheduled_at_utc_us: 1_773_151_200_000_000 + i * 86_400_000_000, local_date: `2026-03-${String(10 + i).padStart(2, "0")}`,
    local_time: "08:00", offset_seconds: -18000, zone_id: "America/Chicago", zone_abbreviation: "CDT", adjusted: false,
  }));

export const defaultMed = (): MedicationSummary => ({
  id: "m1", name: "Metformin", strength: "500 mg", form: "tablet", archived: false,
  schedule: {
    id: "sch1", state: "active", version_number: 1, mode: "home_zone", zone_id: "America/Chicago",
    slots: [{ time: "08:00", weekdays: [] }], interval_seconds: null, anchor_utc_us: null, effective_from_utc_us: 1, next: previewRows(),
  },
});

export function fakeApi(opts: { startupError?: AppError; upgradeCopy?: boolean; olderEntries?: number; meds?: MedicationSummary[]; next?: NextDue | null; backups?: BackupFile[]; report?: Report } = {}): Api {
  const slot = (): TodayItem => ({
    occurrence_id: "o1", schedule_id: "sch1", medication_id: "m1", name: "Metformin", strength: "500 mg", form: "tablet",
    mode: "home_zone", scheduled_at_utc_us: 1_773_151_200_000_000, local_date: "2026-03-10", local_time: "08:00",
    zone_id: "America/Chicago", zone_abbreviation: "CDT", offset_seconds: -18000, adjusted: false, device_time: null,
    status: "due", event_ids: [],
  });
  let today = slot();
  let history: HistoryEntry[] = [];
  const entry = (kind: "take" | "skip"): HistoryEntry => ({
    event_id: "e1", medication_id: "m1", name: "Metformin", strength: "500 mg", kind, undone: false,
    deleted: false, occurred_at_utc_us: 1, entered_at_utc_us: 2, original_local: "2026-03-10T09:30", original_offset_seconds: -21600,
    original_zone_id: null, current_local: "2026-03-10T09:30", current_offset_seconds: -21600, note: "felt fine", slot_time: "08:00", revisions: [],
  });
  const older: HistoryEntry[] = Array.from({ length: opts.olderEntries ?? 0 }, (_, i) => ({
    ...entry("take"), event_id: `o${i}`, name: `Older${i}`, strength: null, note: null, current_local: "2026-01-01T09:00", original_local: "2026-01-01T09:00",
  }));
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
    // Cursor is just "how many entries have been served"; the real paging rules are tested in Rust.
    listHistory: async ({ limit = 100, before = null } = {}) => {
      const all = [...history, ...older];
      const start = before ? Number(before.event_id.replace("old-", "")) + 1 : 0;
      const entries = all.slice(start, start + limit);
      const end = start + entries.length;
      return { entries, next_cursor: end < all.length ? { occurred_at_utc_us: 0, entered_at_utc_us: 0, event_id: `old-${end - 1}` } : null };
    },
    upgradeCopyExists: async () => opts.upgradeCopy ?? false,
    createMedication: async () => "m1",
    listMedications: async () => opts.meds ?? [defaultMed()],
    archiveMedication: async () => {},
    deleteMedication: async () => {},
    createSchedule: async () => "sch1",
    editSchedule: async () => {},
    pauseSchedule: async () => {},
    resumeSchedule: async () => {},
    endSchedule: async () => {},
    previewSchedule: async () => previewRows(),
    nextDue: async () => opts.next ?? null,
    backupFolder: async () => ({ folder: "/data/MedTracker/backups", files: opts.backups ?? [] }),
    createBackup: async () => ({
      path: "/data/MedTracker/backups/MedTracker-backup-2026-03-10-0930.medbackup", file_name: "MedTracker-backup-2026-03-10-0930.medbackup",
      created_at_utc_us: 1_773_133_800_000_000, medications: 1, doses: 2, schedules: 1, size_bytes: 4096,
    }),
    restoreBackup: async () => ({ backup_created_at_utc_us: 1_773_133_800_000_000, medications: 1, doses: 2, schedules: 1, upgraded_from: null, safety_copy_kept: true }),
    reportRange: async (from, to) => opts.report ?? sampleReport(from, to),
    saveCopy: async () => ({ saved: true, bytes: 4096 }),
    pickBackupFile: async () => ({ path: "/data/MedTracker/import-staging/picked.medbackup", name: "my-backup.medbackup", size_bytes: 4096 }),
    discardStagedBackup: async () => {},
    exportLedger: async (input) => ({ path: `/data/MedTracker/exports/medtracker-export.${input.format}`, file_name: `medtracker-export.${input.format}`, rows: 6 }),
  };
}

const zero: ReportCounts = { occurrences: 0, taken: 0, skipped: 0, not_logged: 0, pending: 0, conflicts: 0, unscheduled_taken: 0, unscheduled_skipped: 0 };

/** The spec section 10 fixture numbers. */
export const sampleReport = (from = "2026-03-07", to = "2026-03-10"): Report => ({
  from, to, grouping: "Scheduled times are grouped by their own schedule-local date.", reporting_zone: "America/New_York",
  totals: { ...zero, occurrences: 4, taken: 2, skipped: 1, not_logged: 1, unscheduled_taken: 2 },
  by_medication: [
    { medication_id: "a", name: "Daily tablet A", strength: "5 mg", ...zero, occurrences: 4, taken: 2, skipped: 1, not_logged: 1 },
    { medication_id: "b", name: "As-needed liquid B", strength: "10 mg/mL", ...zero, unscheduled_taken: 2 },
  ],
  by_day: [
    { date: "2026-03-07", ...zero, occurrences: 1, taken: 1 },
    { date: "2026-03-08", ...zero, occurrences: 1, skipped: 1 },
    { date: "2026-03-09", ...zero, occurrences: 1, not_logged: 1 },
    { date: "2026-03-10", ...zero, occurrences: 1, taken: 1, unscheduled_taken: 2 },
  ],
  voided_entries: 1, deleted_entries: 0, definitions: ["Counts only; this is not a score."],
});
