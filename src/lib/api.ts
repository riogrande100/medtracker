import { invoke } from "@tauri-apps/api/core";

export type DoseKind = "take" | "skip";
export type TodayStatus = "upcoming" | "due" | "taken" | "skipped" | "conflict" | "earlier_not_logged";
export type ScheduleMode = "home_zone" | "elapsed_interval";
export type ScheduleState = "active" | "paused" | "ended";

export interface TodayItem {
  /** The explicit link target for Taken / Skip. */
  occurrence_id: string;
  schedule_id: string;
  medication_id: string;
  name: string;
  strength: string | null;
  form: string | null;
  mode: ScheduleMode;
  /** The scheduled instant; the UI uses it for timers, never for display. */
  scheduled_at_utc_us: number;
  local_date: string;
  /** "HH:MM" of the scheduled instant in the card's own zone. */
  local_time: string;
  zone_id: string | null;
  zone_abbreviation: string | null;
  offset_seconds: number;
  /** The intended wall time did not exist (clock change) and was moved. */
  adjusted: boolean;
  /** The same instant on this device's clock, only when it reads differently. */
  device_time: string | null;
  /** Derived by the app from schedule + dose events + clock; never stored. */
  status: TodayStatus;
  event_ids: string[];
}

export interface Revision {
  revision_number: number;
  changed_at_utc_us: number;
  reason: "correction" | "undo" | "restore" | "delete";
  before: string | null;
  after: string | null;
}

export interface HistoryEntry {
  event_id: string;
  medication_id: string;
  name: string;
  strength: string | null;
  kind: DoseKind;
  /** Soft-undone; can be restored. */
  undone: boolean;
  /** Privacy-deleted tombstone: details erased, cannot be restored. */
  deleted: boolean;
  occurred_at_utc_us: number;
  entered_at_utc_us: number;
  original_local: string;
  original_offset_seconds: number;
  original_zone_id: string | null;
  current_local: string;
  current_offset_seconds: number;
  note: string | null;
  slot_time: string | null;
  revisions: Revision[];
}

export interface SlotInput {
  /** "HH:MM" */
  time: string;
  /** ISO weekdays, 1 = Monday ... 7 = Sunday. Empty = every day. */
  weekdays: number[];
}

export interface ScheduleInput {
  mode: ScheduleMode;
  /** home_zone: IANA zone id (required). elapsed_interval: null. */
  zone_id: string | null;
  slots: SlotInput[];
  /** elapsed_interval: seconds between doses. */
  interval_seconds: number | null;
  /** elapsed_interval: first dose "YYYY-MM-DDTHH:MM" on the device clock; null = now (create) or keep (edit). */
  anchor_local: string | null;
}

export interface PreviewOccurrence {
  scheduled_at_utc_us: number;
  local_date: string;
  local_time: string;
  offset_seconds: number;
  zone_id: string | null;
  zone_abbreviation: string | null;
  adjusted: boolean;
}

export interface ScheduleSummary {
  id: string;
  state: ScheduleState;
  version_number: number;
  mode: ScheduleMode;
  zone_id: string | null;
  slots: SlotInput[];
  interval_seconds: number | null;
  anchor_utc_us: number | null;
  effective_from_utc_us: number;
  /** Next seven occurrences while active; empty when paused or ended. */
  next: PreviewOccurrence[];
}

export interface NextDue {
  medication_id: string;
  name: string;
  strength: string | null;
  occurrence: PreviewOccurrence;
}

export interface MedicationSummary {
  id: string;
  name: string;
  strength: string | null;
  form: string | null;
  archived: boolean;
  /** Most recent schedule; null = as needed. */
  schedule: ScheduleSummary | null;
}

export interface NewMedication {
  name: string;
  strength: string | null;
  form: string | null;
  times: string[];
}

export interface RecordDose {
  medication_id: string;
  kind: DoseKind;
  /** The occurrence this action is for (the explicit link). null = an unscheduled dose. */
  occurrence_id: string | null;
  occurred_local: string | null;
  note: string | null;
  /** Only for a wall time that happens twice (clocks going back): true picks the later one. Default earlier. */
  second_occurrence?: boolean | null;
}

export interface CorrectDose {
  event_id: string;
  kind: DoseKind | null;
  occurred_local: string | null;
  note: string | null;
  /** As RecordDose. Omit to keep the entry's stored instant when the wall time is unchanged. */
  second_occurrence?: boolean | null;
}

/** Position in History; pass a page's `next_cursor` back as `before` to get older entries. */
export interface HistoryCursor {
  occurred_at_utc_us: number;
  entered_at_utc_us: number;
  event_id: string;
}

export interface HistoryPage {
  entries: HistoryEntry[];
  /** Present when older entries exist. */
  next_cursor: HistoryCursor | null;
}

export interface ReportCounts {
  occurrences: number;
  taken: number;
  skipped: number;
  /** Earlier day, nothing recorded. Descriptive only. */
  not_logged: number;
  /** Today or later, nothing recorded yet. */
  pending: number;
  conflicts: number;
  unscheduled_taken: number;
  unscheduled_skipped: number;
}
export interface ReportMedicationRow extends ReportCounts {
  medication_id: string;
  name: string;
  strength: string | null;
}
export interface ReportDayRow extends ReportCounts {
  date: string;
}
export interface Report {
  from: string;
  to: string;
  grouping: string;
  reporting_zone: string | null;
  totals: ReportCounts;
  by_medication: ReportMedicationRow[];
  by_day: ReportDayRow[];
  voided_entries: number;
  deleted_entries: number;
  definitions: string[];
}
export interface ExportOptions {
  from: string | null;
  to: string | null;
  format: "csv" | "json";
  include_notes: boolean;
  include_history: boolean;
}
export interface ExportResult {
  path: string;
  file_name: string;
  rows: number;
}

/** Each way a backup can fail is its own kind; all of them leave the data on this device untouched. */
export type BackupErrorKind =
  | "backup_not_a_backup"
  | "backup_truncated"
  | "backup_unsupported_format"
  | "backup_newer_schema"
  | "backup_wrong_passphrase"
  | "backup_corrupt"
  | "busy";

export interface BackupFile {
  name: string;
  path: string;
  size_bytes: number;
  modified_utc_us: number;
}

export interface BackupFolder {
  folder: string;
  files: BackupFile[];
}

export interface BackupCreated {
  path: string;
  file_name: string;
  created_at_utc_us: number;
  medications: number;
  doses: number;
  schedules: number;
  size_bytes: number;
}

export interface RestoreResult {
  backup_created_at_utc_us: number;
  medications: number;
  doses: number;
  schedules: number;
  /** Set when the backup was made by an older version and had to be upgraded. */
  upgraded_from: number | null;
  /** A copy of what was on this device before the restore is kept until MedTracker next starts. */
  safety_copy_kept: boolean;
}

export type AppErrorKind =
  | "validation"
  | "not_found"
  | "storage"
  | "stale_date"
  | "newer_database"
  | "migration_failed"
  | "corrupt_database"
  | BackupErrorKind;

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

/** Errors where the data file was left untouched and the app cannot continue normally. */
export const RECOVERABLE_KINDS: AppErrorKind[] = ["newer_database", "migration_failed", "corrupt_database"];

export function errorKind(e: unknown): AppErrorKind | null {
  return e && typeof e === "object" && "kind" in e ? ((e as AppError).kind ?? null) : null;
}

export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as AppError).message);
  return "Something went wrong. Please try again.";
}

export const api = {
  /** null when the database opened normally. */
  startupStatus: () => invoke<AppError | null>("startup_status"),
  listToday: () => invoke<TodayItem[]>("list_today"),
  recordDose: (input: RecordDose) => invoke<string>("record_dose", { input }),
  correctDose: (input: CorrectDose) => invoke<void>("correct_dose", { input }),
  undoDose: (eventId: string) => invoke<void>("undo_dose", { eventId }),
  /** Reverses an Undo. */
  restoreDose: (eventId: string) => invoke<void>("restore_dose", { eventId }),
  /** Permanent privacy deletion of one entry's details. Not the same as Undo. */
  deleteDoseEntry: (eventId: string) => invoke<void>("delete_dose_entry", { eventId }),
  /** Newest first. `before` is the previous page's `next_cursor`. */
  listHistory: (opts: { limit?: number; before?: HistoryCursor | null } = {}) =>
    invoke<HistoryPage>("list_history", { limit: opts.limit ?? null, before: opts.before ?? null }),
  /** True while a temporary pre-upgrade copy of the data (made by this launch) still exists on the device. */
  upgradeCopyExists: () => invoke<boolean>("upgrade_copy_exists"),
  createMedication: (input: NewMedication) => invoke<string>("create_medication", { input }),
  listMedications: () => invoke<MedicationSummary[]>("list_medications"),
  archiveMedication: (id: string, archived: boolean) => invoke<void>("archive_medication", { id, archived }),
  deleteMedication: (id: string) => invoke<void>("delete_medication", { id }),
  createSchedule: (medicationId: string, definition: ScheduleInput) =>
    invoke<string>("create_schedule", { input: { medication_id: medicationId, definition } }),
  /** Saves a new schedule version effective now; past doses and History are not rewritten. */
  editSchedule: (scheduleId: string, definition: ScheduleInput) =>
    invoke<void>("edit_schedule", { input: { schedule_id: scheduleId, definition } }),
  pauseSchedule: (scheduleId: string) => invoke<void>("pause_schedule", { scheduleId }),
  resumeSchedule: (scheduleId: string) => invoke<void>("resume_schedule", { scheduleId }),
  endSchedule: (scheduleId: string) => invoke<void>("end_schedule", { scheduleId }),
  /** The next seven occurrences of a schedule that has not been saved. Writes nothing. */
  previewSchedule: (input: ScheduleInput) => invoke<PreviewOccurrence[]>("preview_schedule", { input }),
  /** The next occurrence that hasn't been recorded, which may be on a later day. */
  nextDue: () => invoke<NextDue | null>("next_due"),
  /** The folder backups are saved to and the backups found there. Nothing is opened. */
  backupFolder: () => invoke<BackupFolder>("backup_folder"),
  /** Encrypts a consistent snapshot with the passphrase. The passphrase is never stored; it cannot be recovered. */
  createBackup: (passphrase: string) => invoke<BackupCreated>("create_backup", { passphrase }),
  /** Verifies the file in a staging copy first; the current data is replaced only if everything checks out. */
  restoreBackup: (path: string, passphrase: string) => invoke<RestoreResult>("restore_backup", { path, passphrase }),
  /** Descriptive counts for an inclusive local date range. */
  reportRange: (from: string, to: string) => invoke<Report>("report_range", { from, to }),
  /** Writes a plain CSV/JSON file to the Downloads folder (or Documents / app folder) and says where. */
  exportLedger: (input: ExportOptions) => invoke<ExportResult>("export_ledger", { input }),
};

export type Api = typeof api;
