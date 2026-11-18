-- Schedules, immutable schedule versions, slots and materialized occurrences (spec section 5).
--
-- Phase 1 kept one flat `schedule_slot` per medication and created `scheduled_occurrence` rows only
-- when a dose was logged. This migration rebuilds that subsystem:
--   schedule            one per medication "intent" (status mirrors its current version)
--   schedule_version    immutable definition + effective range [from, until); edits, pause, resume
--                       and end each close the current version and open a new one
--   schedule_slot       wall-clock slots of one version (elapsed_interval versions have none)
--   scheduled_occurrence  materialized expected opportunities; never a dose, never rewritten
--
-- Existing slots become one `home_zone` schedule per medication whose zone is NULL ("not chosen
-- yet"): the app adopts the device's IANA zone for those on first launch after the upgrade.
-- `device_local` is intentionally NOT accepted by the CHECK until its spike passes.
--
-- Rebuilding needs the parent tables dropped while dose links still exist, which RESTRICT
-- forbids, so the dependent rows are parked in temp tables first. Everything runs in the
-- migration transaction; a failure (including the final foreign_key_check) rolls it all back.

CREATE TABLE _legacy_link AS SELECT * FROM dose_event_occurrence;
CREATE TABLE _legacy_occ  AS SELECT * FROM scheduled_occurrence;
CREATE TABLE _legacy_slot AS SELECT * FROM schedule_slot;
DROP TABLE dose_event_occurrence;
DROP TABLE scheduled_occurrence;
DROP TABLE schedule_slot;

CREATE TABLE schedule (
  id TEXT PRIMARY KEY,
  medication_id TEXT NOT NULL REFERENCES user_medication(id) ON DELETE RESTRICT,
  current_version_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('active','paused','ended')),
  archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  created_at_utc INTEGER NOT NULL
);
CREATE INDEX schedule_medication ON schedule (medication_id);

CREATE TABLE schedule_version (
  id TEXT PRIMARY KEY,
  schedule_id TEXT NOT NULL REFERENCES schedule(id) ON DELETE RESTRICT,
  medication_version_id TEXT NOT NULL REFERENCES medication_version(id) ON DELETE RESTRICT,
  version_number INTEGER NOT NULL CHECK (version_number >= 1),
  effective_from_utc INTEGER NOT NULL,
  effective_until_utc INTEGER,
  state TEXT NOT NULL CHECK (state IN ('active','paused','ended')),
  mode TEXT NOT NULL CHECK (mode IN ('home_zone','elapsed_interval')),
  zone_id TEXT,
  recurrence TEXT NOT NULL CHECK (recurrence IN ('weekly_slots','fixed_interval')),
  interval_seconds INTEGER,
  anchor_utc INTEGER,
  dst_policy TEXT NOT NULL DEFAULT 'gap_forward_fold_earlier' CHECK (dst_policy IN ('gap_forward_fold_earlier')),
  tzdb_version TEXT,
  change_reason TEXT NOT NULL CHECK (change_reason IN ('create','edit','pause','resume','end','migrated')),
  created_at_utc INTEGER NOT NULL,
  UNIQUE (schedule_id, version_number),
  CHECK (effective_until_utc IS NULL OR effective_until_utc >= effective_from_utc),
  CHECK ((mode = 'elapsed_interval') = (recurrence = 'fixed_interval')),
  CHECK (mode <> 'elapsed_interval' OR (interval_seconds >= 1 AND anchor_utc IS NOT NULL AND zone_id IS NULL))
);
CREATE INDEX schedule_version_range ON schedule_version (schedule_id, effective_from_utc);

-- A version's definition never changes. Only: closing it (effective_until_utc NULL -> value) and,
-- for versions migrated without a zone, adopting one (zone_id NULL -> value).
CREATE TRIGGER schedule_version_immutable BEFORE UPDATE ON schedule_version
WHEN NEW.id IS NOT OLD.id OR NEW.schedule_id IS NOT OLD.schedule_id
  OR NEW.medication_version_id IS NOT OLD.medication_version_id
  OR NEW.version_number IS NOT OLD.version_number
  OR NEW.effective_from_utc IS NOT OLD.effective_from_utc
  OR NEW.state IS NOT OLD.state OR NEW.mode IS NOT OLD.mode OR NEW.recurrence IS NOT OLD.recurrence
  OR NEW.interval_seconds IS NOT OLD.interval_seconds OR NEW.anchor_utc IS NOT OLD.anchor_utc
  OR NEW.dst_policy IS NOT OLD.dst_policy OR NEW.tzdb_version IS NOT OLD.tzdb_version
  OR NEW.change_reason IS NOT OLD.change_reason OR NEW.created_at_utc IS NOT OLD.created_at_utc
  OR (NEW.zone_id IS NOT OLD.zone_id AND OLD.zone_id IS NOT NULL)
  OR (NEW.effective_until_utc IS NOT OLD.effective_until_utc AND OLD.effective_until_utc IS NOT NULL)
BEGIN SELECT RAISE(ABORT, 'schedule_version is immutable'); END;

-- Effective ranges [from, until) of one schedule never overlap (NULL until = open ended).
CREATE TRIGGER schedule_version_no_overlap BEFORE INSERT ON schedule_version
WHEN EXISTS (
  SELECT 1 FROM schedule_version o
  WHERE o.schedule_id = NEW.schedule_id
    AND o.effective_from_utc < COALESCE(NEW.effective_until_utc, 9223372036854775807)
    AND COALESCE(o.effective_until_utc, 9223372036854775807) > NEW.effective_from_utc
)
BEGIN SELECT RAISE(ABORT, 'schedule versions must not overlap'); END;

CREATE TABLE schedule_slot (
  id TEXT PRIMARY KEY,
  schedule_version_id TEXT NOT NULL REFERENCES schedule_version(id) ON DELETE RESTRICT,
  -- Stable across versions: the wall time itself ("08:00"), so an unchanged slot is recognised.
  slot_key TEXT NOT NULL,
  local_time TEXT NOT NULL CHECK (local_time GLOB '[0-2][0-9]:[0-5][0-9]'),
  -- Bit 0 = Monday ... bit 6 = Sunday. 0 means every day. No implicit time zone.
  weekday_mask INTEGER NOT NULL DEFAULT 0 CHECK (weekday_mask BETWEEN 0 AND 127),
  created_at_utc INTEGER NOT NULL,
  UNIQUE (schedule_version_id, slot_key)
);

CREATE TABLE scheduled_occurrence (
  id TEXT PRIMARY KEY,
  schedule_id TEXT NOT NULL REFERENCES schedule(id) ON DELETE RESTRICT,
  schedule_version_id TEXT NOT NULL REFERENCES schedule_version(id) ON DELETE RESTRICT,
  slot_id TEXT REFERENCES schedule_slot(id) ON DELETE RESTRICT,
  slot_key TEXT,
  -- elapsed_interval: the n in anchor + n * interval. NULL for wall-clock slots.
  seq INTEGER,
  -- Intended local date/time (the slot's wall time). The resolved instant may differ if adjusted.
  local_date TEXT NOT NULL,
  local_time TEXT NOT NULL,
  zone_id TEXT,
  offset_seconds INTEGER NOT NULL,
  fold INTEGER NOT NULL DEFAULT 0 CHECK (fold IN (0,1)),
  adjusted INTEGER NOT NULL DEFAULT 0 CHECK (adjusted IN (0,1)),
  scheduled_at_utc INTEGER NOT NULL,
  tzdb_version TEXT,
  generated_at_utc INTEGER NOT NULL,
  state TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active','superseded','cancelled')),
  superseded_at_utc INTEGER
);
-- At most one occurrence per version, slot and local date (and fold / elapsed index).
CREATE UNIQUE INDEX scheduled_occurrence_identity
  ON scheduled_occurrence (schedule_version_id, ifnull(slot_id, ''), local_date, fold, ifnull(seq, -1));
CREATE INDEX scheduled_occurrence_time ON scheduled_occurrence (schedule_version_id, scheduled_at_utc);
CREATE INDEX scheduled_occurrence_schedule_time ON scheduled_occurrence (schedule_id, scheduled_at_utc);

-- An occurrence is a snapshot: only its state may change, and only away from 'active'.
CREATE TRIGGER scheduled_occurrence_immutable BEFORE UPDATE ON scheduled_occurrence
WHEN NEW.id IS NOT OLD.id OR NEW.schedule_id IS NOT OLD.schedule_id
  OR NEW.schedule_version_id IS NOT OLD.schedule_version_id OR NEW.slot_id IS NOT OLD.slot_id
  OR NEW.slot_key IS NOT OLD.slot_key OR NEW.seq IS NOT OLD.seq
  OR NEW.local_date IS NOT OLD.local_date OR NEW.local_time IS NOT OLD.local_time
  OR NEW.zone_id IS NOT OLD.zone_id OR NEW.offset_seconds IS NOT OLD.offset_seconds
  OR NEW.fold IS NOT OLD.fold OR NEW.adjusted IS NOT OLD.adjusted
  OR NEW.scheduled_at_utc IS NOT OLD.scheduled_at_utc OR NEW.tzdb_version IS NOT OLD.tzdb_version
  OR NEW.generated_at_utc IS NOT OLD.generated_at_utc
  OR (OLD.state <> 'active' AND NEW.state IS NOT OLD.state)
BEGIN SELECT RAISE(ABORT, 'scheduled_occurrence is a snapshot'); END;

CREATE TABLE dose_event_occurrence (
  dose_event_id TEXT NOT NULL REFERENCES dose_event(id) ON DELETE RESTRICT,
  occurrence_id TEXT NOT NULL REFERENCES scheduled_occurrence(id) ON DELETE RESTRICT,
  linked_at_utc INTEGER NOT NULL,
  PRIMARY KEY (dose_event_id, occurrence_id)
);
CREATE INDEX dose_event_occurrence_occ ON dose_event_occurrence (occurrence_id);

-- Carry Phase 1 data over. Ids are deterministic so the statements below can refer to each other.
INSERT INTO schedule (id, medication_id, current_version_id, status, archived, created_at_utc)
  SELECT 'sched-' || medication_id, medication_id, 'schedv-' || medication_id || '-1', 'active', 0, min(created_at_utc)
  FROM _legacy_slot GROUP BY medication_id;

INSERT INTO schedule_version (id, schedule_id, medication_version_id, version_number, effective_from_utc, state, mode,
                              zone_id, recurrence, change_reason, created_at_utc)
  SELECT 'schedv-' || s.medication_id || '-1', 'sched-' || s.medication_id,
         (SELECT x.medication_version_id FROM _legacy_slot x WHERE x.medication_id = s.medication_id ORDER BY x.created_at_utc, x.id LIMIT 1),
         1,
         min(min(s.created_at_utc),
             COALESCE((SELECT min(o.due_at_utc) FROM _legacy_occ o JOIN _legacy_slot y ON y.id = o.slot_id WHERE y.medication_id = s.medication_id),
                      min(s.created_at_utc))),
         'active', 'home_zone', NULL, 'weekly_slots', 'migrated', min(s.created_at_utc)
  FROM _legacy_slot s GROUP BY s.medication_id;

INSERT INTO schedule_slot (id, schedule_version_id, slot_key, local_time, weekday_mask, created_at_utc)
  SELECT id, 'schedv-' || medication_id || '-1', local_time, local_time, 0, created_at_utc FROM _legacy_slot;

INSERT INTO scheduled_occurrence (id, schedule_id, schedule_version_id, slot_id, slot_key, seq, local_date, local_time,
                                  zone_id, offset_seconds, fold, adjusted, scheduled_at_utc, tzdb_version, generated_at_utc, state)
  SELECT o.id, 'sched-' || s.medication_id, 'schedv-' || s.medication_id || '-1', o.slot_id, s.local_time, NULL,
         o.local_date, o.local_time, o.zone_id, o.offset_seconds, 0, 0, o.due_at_utc, NULL, o.due_at_utc, 'active'
  FROM _legacy_occ o JOIN _legacy_slot s ON s.id = o.slot_id;

INSERT INTO dose_event_occurrence (dose_event_id, occurrence_id, linked_at_utc)
  SELECT dose_event_id, occurrence_id, linked_at_utc FROM _legacy_link;

DROP TABLE _legacy_link;
DROP TABLE _legacy_occ;
DROP TABLE _legacy_slot;
