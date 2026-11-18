import { invoke } from "@tauri-apps/api/core";

export type DoseKind = "take" | "skip";
export type TodayStatus = "upcoming" | "due" | "taken" | "skipped" | "conflict";

export interface TodayItem {
  slot_id: string;
  medication_id: string;
  name: string;
  strength: string | null;
  form: string | null;
  local_date: string;
  local_time: string;
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

export interface MedicationSummary {
  id: string;
  name: string;
  strength: string | null;
  form: string | null;
  archived: boolean;
  times: string[];
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
  slot_id: string | null;
  local_date: string | null;
  occurred_local: string | null;
  note: string | null;
}

export interface CorrectDose {
  event_id: string;
  kind: DoseKind | null;
  occurred_local: string | null;
  note: string | null;
}

export type AppErrorKind =
  | "validation"
  | "not_found"
  | "storage"
  | "stale_date"
  | "newer_database"
  | "migration_failed"
  | "corrupt_database";

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
  listHistory: () => invoke<HistoryEntry[]>("list_history"),
  createMedication: (input: NewMedication) => invoke<string>("create_medication", { input }),
  listMedications: () => invoke<MedicationSummary[]>("list_medications"),
  archiveMedication: (id: string, archived: boolean) => invoke<void>("archive_medication", { id, archived }),
  deleteMedication: (id: string) => invoke<void>("delete_medication", { id }),
};

export type Api = typeof api;
