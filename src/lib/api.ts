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
  reason: "correction" | "undo";
  before: string | null;
  after: string | null;
}

export interface HistoryEntry {
  event_id: string;
  medication_id: string;
  name: string;
  strength: string | null;
  kind: DoseKind;
  undone: boolean;
  occurred_at_utc_us: number;
  entered_at_utc_us: number;
  original_local: string;
  original_offset_seconds: number;
  current_local: string;
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

export interface AppError {
  kind: "validation" | "not_found" | "storage";
  message: string;
}

export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as AppError).message);
  return "Something went wrong. Please try again.";
}

export const api = {
  listToday: () => invoke<TodayItem[]>("list_today"),
  recordDose: (input: RecordDose) => invoke<string>("record_dose", { input }),
  correctDose: (input: CorrectDose) => invoke<void>("correct_dose", { input }),
  undoDose: (eventId: string) => invoke<void>("undo_dose", { eventId }),
  listHistory: () => invoke<HistoryEntry[]>("list_history"),
  createMedication: (input: NewMedication) => invoke<string>("create_medication", { input }),
  listMedications: () => invoke<MedicationSummary[]>("list_medications"),
  archiveMedication: (id: string, archived: boolean) => invoke<void>("archive_medication", { id, archived }),
  deleteMedication: (id: string) => invoke<void>("delete_medication", { id }),
};

export type Api = typeof api;
