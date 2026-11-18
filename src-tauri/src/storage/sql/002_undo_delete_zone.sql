-- Undo (soft, reversible) is now separate from privacy deletion (permanent tombstone), and
-- each event keeps the IANA zone plus the offset actually in force at its instant.
--
--   undone = 1  : hidden from Today and totals; can be restored. (was `deleted` in v1)
--   deleted = 1 : privacy deletion tombstone; note and revision payloads are redacted; cannot be
--                 restored. Tombstones also have undone = 1 so they never count as doses.
ALTER TABLE dose_event ADD COLUMN undone INTEGER NOT NULL DEFAULT 0 CHECK (undone IN (0,1));
ALTER TABLE dose_event ADD COLUMN original_zone_id TEXT;
-- Offset in force at occurred_at_utc; changes if a correction moves the event across a DST change.
ALTER TABLE dose_event ADD COLUMN occurred_offset_seconds INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scheduled_occurrence ADD COLUMN zone_id TEXT;
UPDATE dose_event SET undone = deleted, deleted = 0, occurred_offset_seconds = original_offset_seconds;

CREATE TABLE dose_event_revision_new (
  id TEXT PRIMARY KEY,
  dose_event_id TEXT NOT NULL REFERENCES dose_event(id) ON DELETE RESTRICT,
  revision_number INTEGER NOT NULL,
  changed_at_utc INTEGER NOT NULL,
  reason TEXT NOT NULL CHECK (reason IN ('correction','undo','restore','delete')),
  before_value TEXT,
  after_value TEXT,
  UNIQUE (dose_event_id, revision_number)
);
INSERT INTO dose_event_revision_new (id, dose_event_id, revision_number, changed_at_utc, reason, before_value, after_value)
  SELECT id, dose_event_id, revision_number, changed_at_utc, reason, before_value, after_value FROM dose_event_revision;
DROP TABLE dose_event_revision;
ALTER TABLE dose_event_revision_new RENAME TO dose_event_revision;
