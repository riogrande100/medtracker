use super::format::{self, KdfParams, HEADER_LEN};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::application::{ledger, medication};
use crate::clock::TimeCtx;
use crate::dto::{DoseKind, NewMedication, RecordDose};
use crate::storage::{self, migrations::MIGRATIONS};

/// Cheap Argon2 settings so the suite is fast; the format carries them in the header, so restore
/// handles them exactly as it handles the production defaults.
const FAST: KdfParams = KdfParams { m_kib: 8, t: 1, p: 1 };
const PASS: &str = "correct horse battery";
const SECRET: &str = "NOTE-the-quick-brown-fox-7731";
const NOW: i64 = 1_773_133_800_000_000;

struct Env {
    dir: PathBuf,
}
impl Env {
    fn new(tag: &str) -> Env {
        let dir = std::env::temp_dir().join(format!("medtracker-bk-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Env { dir }
    }
    fn db(&self, name: &str) -> PathBuf {
        let d = self.dir.join(name);
        std::fs::create_dir_all(&d).unwrap();
        d.join("user-data.sqlite")
    }
    fn backups(&self) -> PathBuf {
        self.dir.join("backups")
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A current-version database with one medication and one dose carrying SECRET in its note.
fn live_with_data(path: &Path, med: &str, note: &str) -> Connection {
    let mut c = storage::open(path).unwrap();
    let ctx = TimeCtx::fixed(NOW, 0);
    let id = medication::create(&mut c, ctx.clone(), NewMedication { name: med.into(), strength: Some("500 mg".into()), form: None, times: vec![] }).unwrap();
    ledger::record(
        &mut c,
        ctx,
        RecordDose { medication_id: id, kind: DoseKind::Take, occurrence_id: None, occurred_local: Some("2026-03-10T09:00".into()), note: Some(note.into()), second_occurrence: None },
    )
    .unwrap();
    c
}

fn make_backup(env: &Env, conn: &Connection, live: &Path) -> PathBuf {
    let made = create_backup(conn, live, &env.backups(), "b.medbackup", PASS, NOW, FAST).unwrap();
    PathBuf::from(made.path)
}

/// Builds a backup file from arbitrary parts, the way a different or hostile writer could.
fn forge(env: &Env, name: &str, manifest: &[u8], db: &[u8], header_schema: u32) -> PathBuf {
    let mut plain = (manifest.len() as u32).to_be_bytes().to_vec();
    plain.extend_from_slice(manifest);
    plain.extend_from_slice(db);
    let sealed = format::seal(&plain, PASS, header_schema, FAST).unwrap();
    std::fs::create_dir_all(env.backups()).unwrap();
    let p = env.backups().join(name);
    std::fs::write(&p, sealed).unwrap();
    p
}

fn manifest_for(schema: u32) -> Vec<u8> {
    serde_json::to_vec(&Manifest {
        format_version: 1,
        app_schema_version: schema,
        app_version: "test".into(),
        created_at_utc_us: NOW,
        medications: 1,
        doses: 1,
        schedules: 0,
    })
    .unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn note_of(conn: &Connection) -> Option<String> {
    conn.query_row("SELECT note FROM dose_event LIMIT 1", [], |r| r.get(0)).ok()
}

fn med_names(conn: &Connection) -> Vec<String> {
    let mut s = conn.prepare("SELECT name FROM medication_version ORDER BY name").unwrap();
    s.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
}

/// Runs the whole restore as the command does and returns the error. Then proves the live file is
/// byte-for-byte what it was, still opens with its data, and no staging file is left.
fn restore_must_fail(live: &Path, file: &Path, pass: &str, migrations: &[&str]) -> AppError {
    let before = std::fs::read(live).unwrap();
    let dir_before = names(live.parent().unwrap());
    let err = match prepare_restore(live, file, pass, migrations) {
        Ok(_) => panic!("restore unexpectedly verified"),
        Err(e) => e,
    };
    assert_eq!(std::fs::read(live).unwrap(), before, "live database bytes changed");
    assert_eq!(names(live.parent().unwrap()), dir_before, "files left behind next to the live database");
    err
}

fn is(e: &AppError, kind: &str) -> bool {
    serde_json::to_value(e).unwrap()["kind"] == kind
}

// ---- happy path -----------------------------------------------------------------------------

#[test]
fn round_trip_restores_data_and_keeps_a_pre_restore_copy() {
    let env = Env::new("rt");
    let a = env.db("a");
    let conn_a = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn_a, &a);

    let b = env.db("b");
    let conn_b = live_with_data(&b, "Lisinopril", "other note");
    let staged = prepare_restore(&b, &file, PASS, MIGRATIONS).unwrap();
    assert_eq!((staged.medications, staged.doses, staged.manifest.created_at_utc_us), (1, 1, NOW));
    let (conn, result) = commit_restore(&b, conn_b, staged);
    let result = result.unwrap();
    let conn = conn.unwrap();
    assert_eq!(result.upgraded_from, None);
    assert!(result.safety_copy_kept);
    assert_eq!(med_names(&conn), ["Metformin"]);
    assert_eq!(note_of(&conn).as_deref(), Some(SECRET));
    assert_eq!(conn.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0)).unwrap(), 1, "restored connection is configured");

    // The previous data survives in the pre-restore copy, under the existing snapshot lifecycle.
    let copies = storage::snapshot_files(&b);
    assert_eq!(copies.len(), 1, "{copies:?}");
    let old = Connection::open(&copies[0]).unwrap();
    assert_eq!(med_names(&old), ["Lisinopril"]);
    assert_eq!(names(b.parent().unwrap()).len(), 2, "only the database and the one pre-restore copy remain");
    drop(old);
    storage::purge_snapshots(&b, None); // what the next clean launch does
    assert_eq!(names(b.parent().unwrap()), ["user-data.sqlite"]);
}

#[test]
fn later_restores_keep_the_first_safety_copy_and_rotate_one_more() {
    let env = Env::new("two");
    let a = env.db("a");
    let conn_a = live_with_data(&a, "Metformin", SECRET);
    let file_a = make_backup(&env, &conn_a, &a);
    let c = env.db("c");
    let conn_c = live_with_data(&c, "Atorvastatin", "third");
    let file_c = create_backup(&conn_c, &c, &env.backups(), "c.medbackup", PASS, NOW, FAST).map(|m| PathBuf::from(m.path)).unwrap();

    // The device's ORIGINAL data is Lisinopril; restore A (Metformin), then C (Atorvastatin), then A again.
    let b = env.db("b");
    let mut conn = live_with_data(&b, "Lisinopril", "original");
    for f in [&file_a, &file_c, &file_a] {
        let staged = prepare_restore(&b, f, PASS, MIGRATIONS).unwrap();
        let (new_conn, r) = commit_restore(&b, conn, staged);
        assert!(r.unwrap().safety_copy_kept);
        conn = new_conn.unwrap();
    }
    assert_eq!(med_names(&conn), ["Metformin"]);

    let copies = storage::snapshot_files(&b);
    assert_eq!(copies.len(), 2, "never more than two safety copies: {copies:?}");
    let first = Connection::open(pre_restore_copy(&b)).unwrap();
    assert_eq!(med_names(&first), ["Lisinopril"], "the original data is still recoverable");
    assert_eq!(note_of(&first).as_deref(), Some("original"));
    let later = Connection::open(pre_restore_copy_later(&b)).unwrap();
    assert_eq!(med_names(&later), ["Atorvastatin"], "the later copy holds the most recent pre-restore data");
    drop((first, later));

    storage::purge_snapshots(&b, None); // next clean launch / privacy deletion removes both
    assert_eq!(names(b.parent().unwrap()), ["user-data.sqlite"]);
}

#[test]
fn backup_is_a_consistent_snapshot_and_leaves_no_plaintext_scratch() {
    let env = Env::new("scratch");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let made = create_backup(&conn, &a, &env.backups(), "x.medbackup", PASS, NOW, FAST).unwrap();
    assert_eq!((made.medications, made.doses, made.schedules), (1, 1, 0));
    assert_eq!(names(a.parent().unwrap()), ["user-data.sqlite"]);
    assert_eq!(names(&env.backups()), ["x.medbackup"]);
    // A second backup with the same name never overwrites the first.
    let again = create_backup(&conn, &a, &env.backups(), "x.medbackup", PASS, NOW, FAST).unwrap();
    assert_eq!(names(&env.backups()), ["x-2.medbackup", "x.medbackup"]);
    assert_ne!(made.path, again.path);
}

#[test]
fn backup_bytes_do_not_contain_the_plaintext_note_or_any_sqlite_marker() {
    let env = Env::new("plain");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let bytes = std::fs::read(&file).unwrap();
    let has = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w == needle);
    assert!(!has(&bytes, SECRET.as_bytes()));
    assert!(!has(&bytes, b"Metformin"));
    assert!(!has(&bytes, b"SQLite format 3"));
    assert!(!has(&bytes, b"medication"), "not even table names");
    // Sanity: the note really was in the database, so the checks above are meaningful.
    assert!(has(&std::fs::read(&a).unwrap(), SECRET.as_bytes()));
    assert!(bytes.starts_with(b"MEDTRBAK"));
}

#[test]
fn two_backups_of_the_same_data_differ_completely() {
    let env = Env::new("salt");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let one = std::fs::read(make_backup(&env, &conn, &a)).unwrap();
    std::fs::remove_file(env.backups().join("b.medbackup")).unwrap();
    let two = std::fs::read(make_backup(&env, &conn, &a)).unwrap();
    assert_ne!(one[27..67], two[27..67], "salt and nonce are fresh per backup");
    assert_ne!(one[HEADER_LEN..], two[HEADER_LEN..]);
}

#[test]
fn short_passphrase_is_refused_and_writes_nothing() {
    let env = Env::new("short");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let e = create_backup(&conn, &a, &env.backups(), "x.medbackup", "short", NOW, FAST).unwrap_err();
    assert!(is(&e, "validation"));
    assert!(!env.backups().exists() || names(&env.backups()).is_empty());
}

#[test]
fn list_backups_finds_only_backup_files_newest_first() {
    let env = Env::new("list");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    create_backup(&conn, &a, &env.backups(), "old.medbackup", PASS, NOW, FAST).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    create_backup(&conn, &a, &env.backups(), "new.medbackup", PASS, NOW, FAST).unwrap();
    std::fs::write(env.backups().join("notes.txt"), "x").unwrap();
    let l = list_backups(&env.backups());
    assert_eq!(l.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["new.medbackup", "old.medbackup"]);
    assert!(list_backups(&env.dir.join("missing")).is_empty());
}

// ---- failures leave the live database untouched ---------------------------------------------

#[test]
fn wrong_passphrase_is_its_own_error_and_changes_nothing() {
    let env = Env::new("wrongpass");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    let e = restore_must_fail(&b, &file, "not the passphrase", MIGRATIONS);
    assert!(is(&e, "backup_wrong_passphrase"), "{e:?}");
}

#[test]
fn flipping_any_ciphertext_byte_is_detected_as_tampering_not_a_wrong_passphrase() {
    let env = Env::new("tamper");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let original = std::fs::read(&file).unwrap();
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    let step = (original.len() - HEADER_LEN) / 40 + 1;
    for i in (HEADER_LEN..original.len()).step_by(step).chain([original.len() - 1]) {
        let mut bytes = original.clone();
        bytes[i] ^= 0x01;
        std::fs::write(&file, &bytes).unwrap();
        let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
        assert!(is(&e, "backup_corrupt"), "byte {i}: {e:?}");
    }
}

#[test]
fn flipping_any_header_byte_is_refused_with_a_typed_error() {
    let env = Env::new("hdr");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let original = std::fs::read(&file).unwrap();
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    for i in 0..HEADER_LEN {
        let mut bytes = original.clone();
        bytes[i] ^= 0x01;
        std::fs::write(&file, &bytes).unwrap();
        let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
        let expected: &[&str] = match i {
            0..=7 => &["backup_not_a_backup"],
            8..=9 => &["backup_unsupported_format", "backup_corrupt"],
            _ => &["backup_corrupt"],
        };
        let kind = serde_json::to_value(&e).unwrap()["kind"].as_str().unwrap().to_string();
        assert!(expected.contains(&kind.as_str()), "header byte {i}: {kind}");
        assert!(!is(&e, "backup_wrong_passphrase"), "a damaged header must not read as a wrong passphrase (byte {i})");
    }
}

#[test]
fn truncation_at_every_boundary_is_reported_as_truncated() {
    let env = Env::new("trunc");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let original = std::fs::read(&file).unwrap();
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    for len in [0, 3, 8, 50, HEADER_LEN - 1, HEADER_LEN, HEADER_LEN + 1, original.len() / 2, original.len() - 17, original.len() - 1] {
        std::fs::write(&file, &original[..len]).unwrap();
        let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
        assert!(is(&e, "backup_truncated"), "length {len}: {e:?}");
    }
    // Extra bytes after the data are not truncation; they are damage.
    let mut longer = original.clone();
    longer.push(0);
    std::fs::write(&file, &longer).unwrap();
    assert!(is(&restore_must_fail(&b, &file, PASS, MIGRATIONS), "backup_corrupt"));
}

#[test]
fn wrong_magic_is_not_a_backup() {
    let env = Env::new("magic");
    let b = env.db("b");
    let conn = live_with_data(&b, "Lisinopril", "x");
    drop(conn);
    std::fs::create_dir_all(env.backups()).unwrap();
    // A plain SQLite file (someone picked the live database), a text file, and a prefix-only file.
    let sqlite_copy = env.backups().join("copy.medbackup");
    std::fs::copy(&b, &sqlite_copy).unwrap();
    let text = env.backups().join("text.medbackup");
    std::fs::write(&text, "hello this is not a backup at all, just words").unwrap();
    for f in [&sqlite_copy, &text] {
        let e = restore_must_fail(&b, f, PASS, MIGRATIONS);
        assert!(is(&e, "backup_not_a_backup"), "{e:?}");
    }
    let missing = restore_must_fail(&b, &env.backups().join("nope.medbackup"), PASS, MIGRATIONS);
    assert!(is(&missing, "not_found"));
}

#[test]
fn newer_backup_format_is_refused_before_any_passphrase_work() {
    let env = Env::new("newfmt");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn, &a);
    let mut bytes = std::fs::read(&file).unwrap();
    bytes[9] = 2; // format version 2
    std::fs::write(&file, &bytes).unwrap();
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    // Even a wrong passphrase gets the format message, because the header is read first.
    let e = restore_must_fail(&b, &file, "wrong wrong wrong", MIGRATIONS);
    assert!(is(&e, "backup_unsupported_format"), "{e:?}");
}

#[test]
fn backup_from_a_newer_data_version_is_refused() {
    let env = Env::new("newschema");
    // A "newer app": the current migrations plus one more.
    let mut newer: Vec<&str> = MIGRATIONS.to_vec();
    newer.push("CREATE TABLE future_thing (id TEXT PRIMARY KEY);");
    let a = env.db("a");
    let conn_new = storage::open_with(&a, &newer).unwrap();
    let file = make_backup(&env, &conn_new, &a);
    assert_eq!(u32::from_be_bytes(std::fs::read(&file).unwrap()[10..14].try_into().unwrap()), newer.len() as u32);

    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
    assert!(is(&e, "backup_newer_schema"), "{e:?}");

    // A header that LIES about being old, around a newer database, is caught by the database's own
    // version once it is authenticated and staged.
    let db_bytes = std::fs::read(&a).unwrap();
    drop(conn_new);
    let lying = forge(&env, "lie.medbackup", &manifest_for(MIGRATIONS.len() as u32), &db_bytes, MIGRATIONS.len() as u32);
    let e = restore_must_fail(&b, &lying, PASS, MIGRATIONS);
    assert!(is(&e, "backup_newer_schema"), "{e:?}");
}

#[test]
fn older_version_backup_restores_and_is_migrated() {
    let env = Env::new("older");
    // A real version-1 database, as an older app would have backed up.
    let v1 = env.db("v1");
    {
        let c = Connection::open(&v1).unwrap();
        c.execute_batch(MIGRATIONS[0]).unwrap();
        c.execute_batch(
            "PRAGMA user_version = 1;
             INSERT INTO user_medication (id, current_version_id, status, created_at_utc, updated_at_utc) VALUES ('m1', NULL, 'active', 1, 1);
             INSERT INTO medication_version (id, medication_id, version_number, effective_from_utc, name, strength, form, source_kind, created_at_utc)
               VALUES ('v1', 'm1', 1, 1, 'Metformin', '500 mg', 'tablet', 'manual', 1);
             UPDATE user_medication SET current_version_id = 'v1';
             INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, note, current_revision, deleted)
               VALUES ('e1', 'm1', 'v1', 'take', 1773133800000000, 1773133800000000, '2026-03-10T03:30', -21600, 'old note', 0, 0);",
        )
        .unwrap();
    }
    let file = forge(&env, "v1.medbackup", &manifest_for(1), &std::fs::read(&v1).unwrap(), 1);

    let b = env.db("b");
    let conn_b = live_with_data(&b, "Lisinopril", "x");
    let staged = prepare_restore(&b, &file, PASS, MIGRATIONS).unwrap();
    assert_eq!(staged.upgraded_from, Some(1));
    let (conn, result) = commit_restore(&b, conn_b, staged);
    let result = result.unwrap();
    assert_eq!(result.upgraded_from, Some(1));
    let conn = conn.unwrap();
    assert_eq!(crate::storage::migrations::user_version(&conn).unwrap(), MIGRATIONS.len() as i64);
    assert_eq!(med_names(&conn), ["Metformin"]);
    assert_eq!(note_of(&conn).as_deref(), Some("old note"));
    // No staging, no migration scratch from the candidate: only the db and the pre-restore copy.
    assert_eq!(names(b.parent().unwrap()).len(), 2, "{:?}", names(b.parent().unwrap()));
}

#[test]
fn malformed_manifest_or_payload_is_corrupt_and_changes_nothing() {
    let env = Env::new("manifest");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let db = {
        let f = make_backup(&env, &conn, &a);
        std::fs::remove_file(f).unwrap();
        // Use a clean database file for the payload.
        let v = env.db("v");
        drop(live_with_data(&v, "Metformin", SECRET));
        std::fs::read(v).unwrap()
    };
    let cur = MIGRATIONS.len() as u32;
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");

    let cases: Vec<(&str, PathBuf)> = vec![
        ("garbage manifest", forge(&env, "m1.medbackup", b"{not json", &db, cur)),
        ("wrong shape", forge(&env, "m2.medbackup", br#"{"hello":1}"#, &db, cur)),
        ("manifest disagrees with header", forge(&env, "m3.medbackup", &manifest_for(cur - 1), &db, cur)),
        ("payload is not a database", forge(&env, "m4.medbackup", &manifest_for(cur), b"this is definitely not sqlite", cur)),
        ("empty payload", forge(&env, "m5.medbackup", &manifest_for(cur), b"", cur)),
    ];
    for (what, f) in &cases {
        let e = restore_must_fail(&b, f, PASS, MIGRATIONS);
        assert!(is(&e, "backup_corrupt"), "{what}: {e:?}");
    }
    // Manifest length field larger than the payload.
    let mut plain = 9_000u32.to_be_bytes().to_vec();
    plain.extend_from_slice(b"{}");
    let p = env.backups().join("m6.medbackup");
    std::fs::write(&p, format::seal(&plain, PASS, cur, FAST).unwrap()).unwrap();
    assert!(is(&restore_must_fail(&b, &p, PASS, MIGRATIONS), "backup_corrupt"));
}

#[test]
fn a_damaged_database_inside_a_valid_backup_is_corrupt() {
    let env = Env::new("dbdamage");
    let v = env.db("v");
    drop(live_with_data(&v, "Metformin", SECRET));
    let mut db = std::fs::read(&v).unwrap();
    for b in &mut db[100..300] {
        *b = 0xFF; // scribble over the schema page's b-tree header and cells
    }
    let cur = MIGRATIONS.len() as u32;
    let file = forge(&env, "d.medbackup", &manifest_for(cur), &db, cur);
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
    assert!(is(&e, "backup_corrupt"), "{e:?}");
}

/// Rewrites the KDF fields of a backup header (and its CRC, so only the bounds check can object).
fn set_kdf(file: &Path, m_kib: u32, t: u32, p: u32) {
    let mut bytes = std::fs::read(file).unwrap();
    bytes[15..19].copy_from_slice(&m_kib.to_be_bytes());
    bytes[19..23].copy_from_slice(&t.to_be_bytes());
    bytes[23..27].copy_from_slice(&p.to_be_bytes());
    let mut c = 0xFFFF_FFFFu32;
    for &b in &bytes[..91] {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    bytes[91..95].copy_from_slice(&(!c).to_be_bytes());
    std::fs::write(file, &bytes).unwrap();
}

#[test]
fn hostile_kdf_settings_are_refused_without_allocating() {
    let env = Env::new("kdf");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    // (memory KiB, passes, lanes): 4 TiB; just over 128 MiB; 7 passes; 5 lanes; zero; memory below 8 KiB per lane.
    for (m, t, p) in [(u32::MAX, 1, 1), (128 * 1024 + 1, 1, 1), (8, 7, 1), (8, 1, 5), (8, 0, 1), (8, 1, 0), (7, 1, 1), (u32::MAX, u32::MAX, u32::MAX)] {
        let file = make_backup(&env, &conn, &a);
        set_kdf(&file, m, t, p);
        let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
        assert!(is(&e, "backup_corrupt"), "{m}/{t}/{p}: {e:?}");
        std::fs::remove_file(&file).unwrap();
    }
}

#[test]
fn kdf_bounds_admit_the_default_and_the_ceiling_and_seal_refuses_beyond_it() {
    assert!(format::DEFAULT_KDF.in_bounds());
    assert!(KdfParams { m_kib: 128 * 1024, t: 6, p: 4 }.in_bounds());
    assert!(!KdfParams { m_kib: 128 * 1024 + 1, t: 6, p: 4 }.in_bounds());
    // Derivation itself re-checks, so an out-of-range request never reaches the allocator.
    let e = format::seal(b"x", PASS, 1, KdfParams { m_kib: u32::MAX, t: 3, p: 1 }).unwrap_err();
    assert!(is(&e, "backup_corrupt"), "{e:?}");
}

#[test]
fn nfc_and_nfd_forms_of_a_passphrase_open_the_same_backup() {
    let env = Env::new("nfc");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let nfc = "caf\u{e9}-pass\u{e4}-w\u{f6}rd"; // precomposed
    let nfd = "cafe\u{301}-pass\u{61}\u{308}-wo\u{308}rd"; // letter + combining mark
    assert_ne!(nfc, nfd);
    let b = env.db("b");
    for (make, open) in [(nfc, nfd), (nfd, nfc)] {
        let file = create_backup(&conn, &a, &env.backups(), "n.medbackup", make, NOW, FAST).map(|m| PathBuf::from(m.path)).unwrap();
        let live = live_with_data(&b, "Lisinopril", "x");
        let staged = prepare_restore(&b, &file, open, MIGRATIONS).unwrap();
        let (conn2, r) = commit_restore(&b, live, staged);
        r.unwrap();
        assert_eq!(med_names(&conn2.unwrap()), ["Metformin"]);
        std::fs::remove_file(&file).unwrap();
        for f in std::fs::read_dir(b.parent().unwrap()).unwrap() {
            std::fs::remove_file(f.unwrap().path()).unwrap();
        }
    }
    // A genuinely different passphrase still fails, and nothing else (case, spaces) is folded.
    let file = make_backup(&env, &conn, &a);
    let _live = live_with_data(&b, "Lisinopril", "x");
    let e = restore_must_fail(&b, &file, &PASS.to_uppercase(), MIGRATIONS);
    assert!(is(&e, "backup_wrong_passphrase"), "{e:?}");
    let e = restore_must_fail(&b, &file, &format!("{PASS} "), MIGRATIONS);
    assert!(is(&e, "backup_wrong_passphrase"), "{e:?}");
}

#[test]
fn only_one_backup_or_restore_may_run_at_a_time() {
    static FLAG: AtomicBool = AtomicBool::new(false);
    let first = OpGuard::acquire_flag(&FLAG).unwrap();
    let e = OpGuard::acquire_flag(&FLAG).err().expect("second is refused");
    assert!(is(&e, "busy"), "{e:?}");
    drop(first);
    let again = OpGuard::acquire_flag(&FLAG).expect("released on drop");
    drop(again);
    assert!(!FLAG.load(Ordering::SeqCst));
}

#[test]
fn a_restored_database_missing_a_required_trigger_or_table_is_refused() {
    let env = Env::new("schema");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let trigger: String = conn.query_row("SELECT name FROM sqlite_master WHERE type = 'trigger' LIMIT 1", [], |r| r.get(0)).unwrap();
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");

    conn.execute_batch(&format!("DROP TRIGGER {trigger}")).unwrap();
    let file = make_backup(&env, &conn, &a);
    let e = restore_must_fail(&b, &file, PASS, MIGRATIONS);
    assert!(is(&e, "backup_corrupt"), "{e:?}");
    assert!(format!("{e:?}").contains(&trigger), "{e:?}");
    std::fs::remove_file(&file).unwrap();

    // A table with nothing pointing at it.
    let a2 = env.db("a2");
    let conn2 = live_with_data(&a2, "Metformin", SECRET);
    conn2.execute_batch("DROP TABLE app_meta").unwrap();
    let file2 = make_backup(&env, &conn2, &a2);
    let e = restore_must_fail(&b, &file2, PASS, MIGRATIONS);
    assert!(is(&e, "backup_corrupt"), "{e:?}");
    assert!(format!("{e:?}").contains("app_meta"), "{e:?}");
}

#[test]
fn backup_falls_back_to_the_next_folder_and_files_are_private_and_complete() {
    let env = Env::new("fallback");
    let a = env.db("a");
    let conn = live_with_data(&a, "Metformin", SECRET);
    let blocker = env.dir.join("a-file");
    std::fs::write(&blocker, b"x").unwrap();
    let dirs = [blocker.join("MedTracker Backups"), env.backups()];
    let made = create_backup_in(&conn, &a, &dirs, "f.medbackup", PASS, NOW, FAST).unwrap();
    assert!(Path::new(&made.path).starts_with(env.backups()), "{}", made.path);
    assert_eq!(names(&env.backups()), ["f.medbackup"], "no partial or temp file left");
    assert_eq!(std::fs::metadata(&made.path).unwrap().len(), made.size_bytes);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&made.path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // Nowhere writable: a typed storage error and no leftovers.
    let e = create_backup_in(&conn, &a, &[blocker.join("x")], "g.medbackup", PASS, NOW, FAST).unwrap_err();
    assert!(is(&e, "storage"), "{e:?}");
    assert_eq!(names(&env.backups()), ["f.medbackup"]);
}

#[cfg(unix)]
#[test]
fn staged_restore_file_is_private_while_it_waits() {
    use std::os::unix::fs::PermissionsExt;
    let env = Env::new("perm");
    let a = env.db("a");
    let conn_a = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn_a, &a);
    let b = env.db("b");
    let _live = live_with_data(&b, "Lisinopril", "x");
    let staged = prepare_restore(&b, &file, PASS, MIGRATIONS).unwrap();
    assert_eq!(std::fs::metadata(staging_path(&b)).unwrap().permissions().mode() & 0o777, 0o600);
    drop(staged);
    assert!(!staging_path(&b).exists());
}

#[test]
fn failed_swap_keeps_the_original_database_and_cleans_up() {
    let env = Env::new("swap");
    let a = env.db("a");
    let conn_a = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn_a, &a);
    let b = env.db("b");
    let conn_b = live_with_data(&b, "Lisinopril", "keep me");
    let before = std::fs::read(&b).unwrap();

    let staged = prepare_restore(&b, &file, PASS, MIGRATIONS).unwrap();
    let (conn, result) = commit_restore_with(&b, conn_b, staged, &|_, _| Err(std::io::Error::other("disk full")));
    assert!(matches!(result, Err(AppError::Storage(_))), "{result:?}");
    let conn = conn.expect("the original is reopened");
    assert_eq!(med_names(&conn), ["Lisinopril"]);
    assert_eq!(note_of(&conn).as_deref(), Some("keep me"));
    drop(conn);
    assert_eq!(std::fs::read(&b).unwrap(), before, "live file is byte-identical");
    let left = names(b.parent().unwrap());
    assert!(!left.iter().any(|n| n.contains("staging")), "staging file removed: {left:?}");
}

#[test]
fn stale_staging_file_from_a_crash_is_replaced_not_trusted() {
    let env = Env::new("stale");
    let a = env.db("a");
    let conn_a = live_with_data(&a, "Metformin", SECRET);
    let file = make_backup(&env, &conn_a, &a);
    let b = env.db("b");
    let conn_b = live_with_data(&b, "Lisinopril", "x");
    std::fs::write(staging_path(&b), b"left over from a crash").unwrap();
    let staged = prepare_restore(&b, &file, PASS, MIGRATIONS).unwrap();
    let (conn, r) = commit_restore(&b, conn_b, staged);
    r.unwrap();
    assert_eq!(med_names(&conn.unwrap()), ["Metformin"]);
}
