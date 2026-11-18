CREATE TABLE app_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  created_at_utc INTEGER NOT NULL
);
INSERT INTO app_meta (id, created_at_utc) VALUES (1, CAST(strftime('%s','now') AS INTEGER) * 1000000);

CREATE TABLE user_medication (
  id TEXT PRIMARY KEY,
  current_version_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('active','archived')),
  created_at_utc INTEGER NOT NULL,
  updated_at_utc INTEGER NOT NULL
);

CREATE TABLE medication_version (
  id TEXT PRIMARY KEY,
  medication_id TEXT NOT NULL REFERENCES user_medication(id) ON DELETE RESTRICT,
  version_number INTEGER NOT NULL,
  effective_from_utc INTEGER NOT NULL,
  name TEXT NOT NULL CHECK (length(name) > 0),
  strength TEXT,
  form TEXT,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('manual','catalog')),
  created_at_utc INTEGER NOT NULL,
  UNIQUE (medication_id, version_number)
);
CREATE TRIGGER medication_version_immutable BEFORE UPDATE ON medication_version
BEGIN SELECT RAISE(ABORT, 'medication_version is immutable'); END;

-- Daily local wall-clock slot, resolved in the device's zone at use time.
CREATE TABLE schedule_slot (
  id TEXT PRIMARY KEY,
  medication_id TEXT NOT NULL REFERENCES user_medication(id) ON DELETE RESTRICT,
  medication_version_id TEXT NOT NULL REFERENCES medication_version(id) ON DELETE RESTRICT,
  local_time TEXT NOT NULL CHECK (local_time GLOB '[0-2][0-9]:[0-5][0-9]'),
  created_at_utc INTEGER NOT NULL
);

-- Materialized only when the user logs against a slot; never implies a dose.
CREATE TABLE scheduled_occurrence (
  id TEXT PRIMARY KEY,
  slot_id TEXT NOT NULL REFERENCES schedule_slot(id) ON DELETE RESTRICT,
  local_date TEXT NOT NULL,
  local_time TEXT NOT NULL,
  offset_seconds INTEGER NOT NULL,
  due_at_utc INTEGER NOT NULL,
  UNIQUE (slot_id, local_date)
);

CREATE TABLE dose_event (
  id TEXT PRIMARY KEY,
  medication_id TEXT NOT NULL REFERENCES user_medication(id) ON DELETE RESTRICT,
  medication_version_id TEXT NOT NULL REFERENCES medication_version(id) ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('take','skip')),
  occurred_at_utc INTEGER NOT NULL,
  entered_at_utc INTEGER NOT NULL,
  original_local TEXT NOT NULL,
  original_offset_seconds INTEGER NOT NULL,
  note TEXT,
  current_revision INTEGER NOT NULL DEFAULT 0,
  deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0,1))
);
CREATE INDEX dose_event_med_time ON dose_event (medication_id, occurred_at_utc);
CREATE INDEX dose_event_time ON dose_event (occurred_at_utc);

CREATE TABLE dose_event_occurrence (
  dose_event_id TEXT NOT NULL REFERENCES dose_event(id) ON DELETE RESTRICT,
  occurrence_id TEXT NOT NULL REFERENCES scheduled_occurrence(id) ON DELETE RESTRICT,
  linked_at_utc INTEGER NOT NULL,
  PRIMARY KEY (dose_event_id, occurrence_id)
);
CREATE INDEX dose_event_occurrence_occ ON dose_event_occurrence (occurrence_id);

CREATE TABLE dose_event_revision (
  id TEXT PRIMARY KEY,
  dose_event_id TEXT NOT NULL REFERENCES dose_event(id) ON DELETE RESTRICT,
  revision_number INTEGER NOT NULL,
  changed_at_utc INTEGER NOT NULL,
  reason TEXT NOT NULL CHECK (reason IN ('correction','undo')),
  before_value TEXT,
  after_value TEXT,
  UNIQUE (dose_event_id, revision_number)
);
