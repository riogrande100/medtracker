//! Small file helpers shared by backup and export: private (0600) files, durable writes,
//! unique names, and an atomic "temp file then rename" save that never leaves a partial file.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Creates a NEW file (fails if it exists) readable and writable by the owner only on Unix.
pub fn create_new_private(path: &Path) -> io::Result<File> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

/// Creates or truncates a file, owner-only on Unix.
pub fn create_private(path: &Path) -> io::Result<File> {
    let mut o = OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

/// Writes `bytes` to `path` (created owner-only, replacing any file), flushes them to disk, and
/// closes the file. A short write or full disk is an error.
pub fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = create_private(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Flushes an existing file's contents to disk.
pub fn sync_file(path: &Path) -> io::Result<()> {
    OpenOptions::new().read(true).write(true).open(path)?.sync_all()
}

/// Makes a rename or creation inside `dir` durable. Only Unix can open a directory to fsync it;
/// elsewhere this does nothing.
#[cfg(unix)]
pub fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}
#[cfg(not(unix))]
pub fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

/// The directory that holds `path` (`.` for a bare file name).
pub fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

/// `<prefix>-YYYYMMDD-HHMMSS-mmm-xxxxxx.<ext>` from a UTC microsecond clock: the millisecond and
/// a random suffix keep two saves in the same second from colliding.
pub fn unique_name(prefix: &str, now_utc_us: i64, ext: &str) -> String {
    let t = jiff::Timestamp::from_microsecond(now_utc_us).map(|t| t.to_string()).unwrap_or_default();
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).take(14).collect();
    let (date, time) = (digits.get(..8).unwrap_or("00000000"), digits.get(8..14).unwrap_or("000000"));
    let millis = now_utc_us.div_euclid(1000).rem_euclid(1000);
    let rand = uuid::Uuid::new_v4().simple().to_string();
    format!("{prefix}-{date}-{time}-{millis:03}-{}.{ext}", &rand[..6])
}

/// Saves `bytes` as `dir/name`: written to a hidden temp file in the same folder (owner-only,
/// flushed), then renamed into place, then the folder is flushed. Never overwrites an existing
/// file. If anything fails the temp file is removed, so no partial copy is left behind.
pub fn save_atomic(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let dest = dir.join(name);
    if dest.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "a file with that name already exists"));
    }
    let tmp = dir.join(format!(".{name}.{}.partial", &uuid::Uuid::new_v4().simple().to_string()[..8]));
    let result = (|| {
        let mut f = create_new_private(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &dest)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let _ = sync_dir(dir); // best effort: the file is complete either way
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("medtracker-fs-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn unique_names_differ_within_one_millisecond_and_carry_ms() {
        let a = unique_name("x", 1_773_133_800_123_456, "csv");
        let b = unique_name("x", 1_773_133_800_123_456, "csv");
        assert_ne!(a, b);
        assert!(a.starts_with("x-20260310-091000-123-") && a.ends_with(".csv"), "{a}");
    }

    #[test]
    fn save_atomic_writes_private_file_and_leaves_no_temp() {
        let d = tmp("atomic");
        let p = save_atomic(&d, "a.csv", b"hello").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"hello");
        let left: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, ["a.csv"]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert!(save_atomic(&d, "a.csv", b"again").is_err(), "never overwrites");
        assert_eq!(std::fs::read(&p).unwrap(), b"hello");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn failed_save_leaves_nothing_behind() {
        let d = tmp("fail");
        let missing = d.join("no-such-folder");
        assert!(save_atomic(&missing, "a.csv", b"x").is_err());
        assert!(!missing.exists());
        std::fs::remove_dir_all(d).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sync_dir_works_on_a_directory() {
        let d = tmp("sync");
        sync_dir(&d).unwrap();
        assert!(sync_dir(&d.join("nope")).is_err());
        std::fs::remove_dir_all(d).unwrap();
    }
}
