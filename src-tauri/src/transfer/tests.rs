use super::*;
use crate::application::medication;
use crate::backup::format::KdfParams;
use crate::backup::{commit_restore, create_backup, prepare_restore};
use crate::clock::TimeCtx;
use crate::dto::NewMedication;
use crate::storage::{self, migrations::MIGRATIONS};
use rusqlite::Connection;

const FAST: KdfParams = KdfParams { m_kib: 8, t: 1, p: 1 };
const PASS: &str = "correct horse battery";
const NOW: i64 = 1_773_133_800_000_000;

struct Env(PathBuf);
impl Env {
    fn new(tag: &str) -> Env {
        let d = std::env::temp_dir().join(format!("medtracker-tr-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        Env(d)
    }
    fn sub(&self, name: &str) -> PathBuf {
        let d = self.0.join(name);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn db_with(dir: &Path, med: &str) -> (PathBuf, Connection) {
    let path = dir.join("user-data.sqlite");
    let mut c = storage::open(&path).unwrap();
    medication::create(&mut c, TimeCtx::fixed(NOW, 0), NewMedication { name: med.into(), strength: None, form: None, times: vec![] }).unwrap();
    (path, c)
}

fn med_names(c: &Connection) -> Vec<String> {
    let mut s = c.prepare("SELECT name FROM medication_version ORDER BY name").unwrap();
    s.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn kind(e: &AppError) -> String {
    serde_json::to_value(e).unwrap()["kind"].as_str().unwrap().to_string()
}

/// A backup made on "another device", sitting where a document provider would hand it from.
fn foreign_backup(env: &Env) -> PathBuf {
    let src_dir = env.sub("other-device");
    let (path, conn) = db_with(&src_dir, "Metformin");
    let made = create_backup(&conn, &path, &env.sub("provider"), "mine.medbackup", PASS, NOW, FAST).unwrap();
    PathBuf::from(made.path)
}

#[test]
fn picked_file_is_copied_into_private_staging_and_then_restored_through_the_normal_flow() {
    let env = Env::new("happy");
    let picked = foreign_backup(&env);
    let (live, conn) = db_with(&env.sub("device"), "Lisinopril");
    let staging = env.sub("device").join("import-staging");

    // What `pick_backup_file` does after the system picker returns: open it, stream it in.
    let (staged, size) = stage_import(&staging, File::open(&picked).unwrap(), MAX_IMPORT_BYTES, NOW).unwrap();
    assert!(staged.starts_with(&staging));
    assert_eq!(std::fs::read(&staged).unwrap(), std::fs::read(&picked).unwrap(), "byte-identical copy");
    assert_eq!(size, std::fs::metadata(&picked).unwrap().len());
    assert_eq!(names(&staging), [staged.file_name().unwrap().to_string_lossy().into_owned()], "no temp file left");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&staged).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // The command only accepts paths inside the app's own folders.
    let ok = ensure_inside(staged.to_str().unwrap(), &[env.sub("backups"), staging.clone()]).unwrap();
    assert_eq!(ok, std::fs::canonicalize(&staged).unwrap());

    // The unchanged, fully verified restore, now reading the staged copy.
    let verified = prepare_restore(&live, &ok, PASS, MIGRATIONS).unwrap();
    let (conn, result) = commit_restore(&live, conn, verified);
    result.unwrap();
    assert_eq!(med_names(&conn.unwrap()), ["Metformin"]);
    clear_staging(&staging);
    assert!(names(&staging).is_empty());
    assert!(picked.exists(), "the user's original is never touched");
}

#[test]
fn a_wrong_file_is_staged_but_rejected_by_restore_and_the_live_data_is_untouched() {
    let env = Env::new("wrong");
    let (live, _conn) = db_with(&env.sub("device"), "Lisinopril");
    let before = std::fs::read(&live).unwrap();
    let dir_before = names(live.parent().unwrap());
    let staging = env.sub("device").join("import-staging");

    // A photo, a spreadsheet, a plain SQLite file: anything can be picked.
    for (i, content) in [b"not a backup at all".to_vec(), std::fs::read(&live).unwrap(), vec![0u8; 3]].into_iter().enumerate() {
        let wrong = env.sub("provider").join(format!("wrong{i}.bin"));
        std::fs::write(&wrong, &content).unwrap();
        let (staged, _) = stage_import(&staging, File::open(&wrong).unwrap(), MAX_IMPORT_BYTES, NOW + i as i64).unwrap();
        let err = prepare_restore(&live, &staged, PASS, MIGRATIONS).map(|_| ()).unwrap_err();
        assert!(["backup_not_a_backup", "backup_truncated"].contains(&kind(&err).as_str()), "{err:?}");
        assert_eq!(std::fs::read(&live).unwrap(), before, "live database bytes changed");
        assert_eq!(names(live.parent().unwrap()).iter().filter(|n| *n != "import-staging").count(), dir_before.len(), "files left next to the live database");
    }
}

#[test]
fn a_wrong_passphrase_on_a_staged_copy_changes_nothing_and_keeps_the_copy_for_a_retry() {
    let env = Env::new("pass");
    let picked = foreign_backup(&env);
    let (live, _conn) = db_with(&env.sub("device"), "Lisinopril");
    let before = std::fs::read(&live).unwrap();
    let staging = env.sub("device").join("import-staging");
    let (staged, _) = stage_import(&staging, File::open(&picked).unwrap(), MAX_IMPORT_BYTES, NOW).unwrap();

    let err = prepare_restore(&live, &staged, "wrong passphrase!", MIGRATIONS).map(|_| ()).unwrap_err();
    assert_eq!(kind(&err), "backup_wrong_passphrase");
    assert_eq!(std::fs::read(&live).unwrap(), before);
    assert!(staged.exists(), "kept so the user can retry without picking again");
    prepare_restore(&live, &staged, PASS, MIGRATIONS).expect("the right passphrase still works on the same copy");
}

#[test]
fn an_oversize_or_failing_source_leaves_no_partial_file_and_a_second_pick_replaces_the_first() {
    let env = Env::new("limits");
    let staging = env.sub("staging");

    // Over the limit: refused as soon as it is exceeded, nothing left behind.
    let err = stage_import(&staging, io::repeat(7).take(1_000), 999, NOW).unwrap_err();
    assert_eq!(kind(&err), "backup_corrupt");
    assert!(names(&staging).is_empty(), "{:?}", names(&staging));
    // Exactly at the limit is fine.
    let (p, n) = stage_import(&staging, io::repeat(7).take(999), 999, NOW).unwrap();
    assert_eq!((n, std::fs::metadata(&p).unwrap().len()), (999, 999));

    // The source breaks halfway (a provider that goes away): typed storage error, no partial.
    struct Broken(usize);
    impl Read for Broken {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0 == 0 {
                return Err(io::Error::other("provider went away"));
            }
            let n = buf.len().min(self.0);
            buf[..n].fill(1);
            self.0 -= n;
            Ok(n)
        }
    }
    let err = stage_import(&staging, Broken(100_000), MAX_IMPORT_BYTES, NOW + 1).unwrap_err();
    assert_eq!(kind(&err), "storage");
    assert!(names(&staging).is_empty(), "failed import removed the earlier copy and wrote no partial");

    // A later pick replaces an earlier staged copy.
    let (a, _) = stage_import(&staging, &b"one"[..], 10, NOW + 2).unwrap();
    let (b, _) = stage_import(&staging, &b"two"[..], 10, NOW + 3).unwrap();
    assert!(!a.exists() && b.exists());
    assert_eq!(names(&staging).len(), 1);
}

#[test]
fn only_plain_files_directly_inside_the_apps_own_folders_may_be_shared_or_restored() {
    let env = Env::new("inside");
    let backups = env.sub("backups");
    let ok = backups.join("a.medbackup");
    std::fs::write(&ok, b"x").unwrap();
    let outside = env.sub("elsewhere").join("user-data.sqlite");
    std::fs::write(&outside, b"x").unwrap();
    std::fs::create_dir_all(backups.join("sub")).unwrap();
    let allowed = [backups.clone()];

    assert!(ensure_inside(ok.to_str().unwrap(), &allowed).is_ok());
    assert!(ensure_inside(&format!(" {} ", ok.display()), &allowed).is_ok(), "surrounding spaces are tolerated");
    assert_eq!(kind(&ensure_inside(outside.to_str().unwrap(), &allowed).unwrap_err()), "validation");
    // `..` cannot climb out.
    let climb = backups.join("..").join("elsewhere").join("user-data.sqlite");
    assert_eq!(kind(&ensure_inside(climb.to_str().unwrap(), &allowed).unwrap_err()), "validation");
    // A folder, and a file that does not exist.
    assert_eq!(kind(&ensure_inside(backups.join("sub").to_str().unwrap(), &allowed).unwrap_err()), "not_found");
    assert_eq!(kind(&ensure_inside(backups.join("gone").to_str().unwrap(), &allowed).unwrap_err()), "not_found");
    // A symlink inside the folder that points outside resolves to its target and is refused.
    #[cfg(unix)]
    {
        let link = backups.join("link.medbackup");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert_eq!(kind(&ensure_inside(link.to_str().unwrap(), &allowed).unwrap_err()), "validation");
    }
}

#[test]
fn copy_out_writes_every_byte_and_leaves_the_private_file_alone() {
    let env = Env::new("out");
    let src = env.sub("exports").join("e.csv");
    let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(&src, &data).unwrap();
    let mut dest = Vec::new();
    assert_eq!(copy_out(&src, &mut dest).unwrap(), data.len() as u64);
    assert_eq!(dest, data);
    assert_eq!(std::fs::read(&src).unwrap(), data);

    // A destination that stops accepting data is an error, not a silent short copy.
    struct Full(usize);
    impl Write for Full {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.0 == 0 {
                return Err(io::Error::other("no space left"));
            }
            let n = b.len().min(self.0);
            self.0 -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert!(copy_out(&src, &mut Full(1000)).is_err());
    assert!(copy_out(&env.0.join("missing"), &mut Vec::new()).is_err());
}

#[test]
fn copy_limited_stops_at_the_limit() {
    let mut out = Vec::new();
    assert_eq!(copy_limited(&mut &b"12345"[..], &mut out, 5).unwrap(), 5);
    let e = copy_limited(&mut &b"123456"[..], &mut Vec::new(), 5).unwrap_err();
    assert_eq!(e.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn dialog_filter_is_a_mime_type_on_mobile_and_an_extension_on_desktop() {
    assert_eq!(filter_for("x.csv", true), "text/csv");
    assert_eq!(filter_for("x.JSON", true), "application/json");
    assert_eq!(filter_for("x.medbackup", true), "application/octet-stream");
    assert_eq!(filter_for("x.medbackup", false), "medbackup");
    assert_eq!(filter_for("noext", true), "application/octet-stream");
}
