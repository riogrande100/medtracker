//! Hand-off between MedTracker's private files and the places the user chooses: the system "save"
//! screen for a finished export or backup, and the system file picker for restore.
//!
//! The rule that keeps this small and safe: **the app-private file is always the source of truth.**
//! Exports and backups are still created in the app's own folders (owner-only, written atomically,
//! see `fsutil`) and verified there. Saving a copy only copies those bytes to wherever the user
//! points; restoring first copies the picked file into app-private staging and then runs the
//! unchanged verified restore on that copy. If a system screen is unavailable or fails, the private
//! file is still there and the UI shows where it is.
//!
//! ## Why these plugins, and why no capability permissions
//!
//! `tauri-plugin-dialog` (save / open on iOS and Android) and `tauri-plugin-fs` (opens the
//! `content://` URIs Android returns, and iOS security-scoped URLs) are used ONLY from Rust,
//! through their Rust APIs, inside the three commands below. Capabilities gate what the web view
//! may call; Rust code is not gated by them. So `capabilities/default.json` stays `core:default`:
//! the web view gets no `dialog:*` and no `fs:*` permission, no file scope, and no network.
//! The only new surface is the three commands in this module, each of which accepts either nothing
//! or a path that must resolve to a plain file inside one of the app's own folders.
//!
//! ## What is documented vs. unverified (no device was available)
//!
//! Documented by the plugin sources and Tauri docs (read in this repo's cargo registry):
//! * Android save = `ACTION_CREATE_DOCUMENT`, open = `ACTION_GET_CONTENT`; both return `content://`
//!   URIs that `tauri_plugin_fs::Fs::open` turns into a file descriptor (modes `r` / `wt`). Neither
//!   intent needs a manifest permission.
//! * iOS open = `UIDocumentPickerViewController(forOpeningContentTypes:, asCopy: true)` when
//!   `FileAccessMode::Copy` is requested, i.e. a copy inside the app sandbox.
//! * iOS save is NOT a native "write here" call: the plugin creates an EMPTY placeholder named
//!   `fileName` in the app's Documents folder, shows the export-to-service picker for it, and
//!   returns the chosen location; the app must then write the content itself.
//! * A cancelled or failed pick/save is indistinguishable to Rust (`None`), so both are reported
//!   as "nothing was saved".
//!
//! UNVERIFIED on a device: that writing to the location the iOS picker returns succeeds (it relies
//! on security-scoped access that `Fs::open` starts); that the empty placeholder the plugin leaves
//! in the iOS app's Documents folder is harmless; how each Android document provider (Drive, SD
//! card, Downloads) treats `wt` mode; that Android's picker shows `.medbackup` files without a
//! content-type filter (none is set on purpose: the extension is not a registered type, and the
//! full verification below rejects anything that is not a backup). There is also no native share
//! sheet here: that needs a community plugin or small native code and was not added.

use crate::backup::{self, format};
use crate::clock::TimeCtx;
use crate::error::{AppError, AppResult};
use crate::fsutil;
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, FileAccessMode, PickerMode};
use tauri_plugin_fs::{FsExt, OpenOptions};

// ---- folders --------------------------------------------------------------------------------

fn app_data(app: &AppHandle) -> AppResult<PathBuf> {
    app.path().app_data_dir().map_err(|e| AppError::Storage(e.to_string()))
}

/// Where finished backups are created (app-private).
pub fn backups_dir(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_data(app)?.join("backups"))
}

/// Where finished exports are created (app-private).
pub fn exports_dir(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_data(app)?.join("exports"))
}

/// Where a picked backup waits, copied, until it is restored or discarded (app-private).
pub fn staging_dir(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(staging_dir_in(&app_data(app)?))
}

pub fn staging_dir_in(app_data: &Path) -> PathBuf {
    app_data.join("import-staging")
}

// ---- pure helpers (everything below is unit-tested without a device) ------------------------

/// The largest file a restore will look at: the same ceiling `prepare_restore` enforces.
pub const MAX_IMPORT_BYTES: u64 = format::MAX_PAYLOAD + format::HEADER_LEN as u64;

const TOO_BIG: &str = "This file is far larger than any MedTracker backup, so it was not opened. Nothing was changed.";

/// Copies `src` to `dst`, failing (with `InvalidData`) as soon as more than `max` bytes arrive, so a
/// huge or endless source can never fill the disk.
pub fn copy_limited<R: Read, W: Write>(src: &mut R, dst: &mut W, max: u64) -> io::Result<u64> {
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        total += n as u64;
        if total > max {
            return Err(io::Error::new(io::ErrorKind::InvalidData, TOO_BIG));
        }
        dst.write_all(&buf[..n])?;
    }
}

/// Removes everything in the staging folder (it holds nothing but copies of picked files).
pub fn clear_staging(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Copies a picked file's bytes into a NEW owner-only file in `dir` and returns its path and size.
/// Any earlier staged copy is removed first. The copy is written under a hidden temp name, flushed,
/// then renamed, so a failed or oversize import leaves nothing behind. No assumption is made about
/// the contents: the restore that follows is what decides whether it is a backup.
pub fn stage_import<R: Read>(dir: &Path, mut src: R, max: u64, now_utc_us: i64) -> AppResult<(PathBuf, u64)> {
    let storage = |what: &str, e: io::Error| AppError::Storage(format!("{what} ({e}). Your data was not changed."));
    std::fs::create_dir_all(dir).map_err(|e| storage("Could not prepare a place for the chosen file", e))?;
    clear_staging(dir);
    let name = fsutil::unique_name("picked", now_utc_us, backup::EXTENSION);
    let dest = dir.join(&name);
    let tmp = dir.join(format!(".{name}.partial"));
    let result = (|| {
        let mut f = fsutil::create_new_private(&tmp)?;
        let n = copy_limited(&mut src, &mut f, max)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &dest)?;
        Ok::<u64, io::Error>(n)
    })();
    match result {
        Ok(n) => {
            let _ = fsutil::sync_dir(dir);
            Ok((dest, n))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(if e.kind() == io::ErrorKind::InvalidData { AppError::BackupCorrupt(TOO_BIG.into()) } else { storage("Could not copy the chosen file", e) })
        }
    }
}

/// `path` as a canonical path, but only if it is a regular file sitting directly inside one of
/// `allowed` (after resolving `..` and symlinks). This keeps the web view from asking the app to
/// copy out, or restore from, anything but its own backups, exports and staged picks.
pub fn ensure_inside(path: &str, allowed: &[PathBuf]) -> AppResult<PathBuf> {
    let not_found = || AppError::NotFound("That file was not found. Create it again or choose another.".into());
    let real = std::fs::canonicalize(path.trim()).map_err(|_| not_found())?;
    if !real.is_file() {
        return Err(not_found());
    }
    let parent = real.parent().ok_or_else(not_found)?;
    if allowed.iter().filter_map(|d| std::fs::canonicalize(d).ok()).any(|d| d == parent) {
        Ok(real)
    } else {
        Err(AppError::Validation("MedTracker can only share or restore its own backup and export files, or one you just chose.".into()))
    }
}

/// The content type to ask the system for on mobile (these are passed as MIME types by the dialog
/// plugin), and the plain extension on desktop where the dialog expects extensions.
pub fn filter_for(file_name: &str, mobile: bool) -> String {
    let ext = Path::new(file_name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !mobile {
        return ext;
    }
    match ext.as_str() {
        "csv" => "text/csv",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Streams the private file `src` into an already-open destination and makes sure every byte was
/// written. Returns the byte count.
pub fn copy_out<W: Write>(src: &Path, dst: &mut W) -> io::Result<u64> {
    let mut f = File::open(src)?;
    let expected = f.metadata()?.len();
    let n = io::copy(&mut f, dst)?;
    dst.flush()?;
    if n != expected {
        return Err(io::Error::new(io::ErrorKind::WriteZero, "fewer bytes were written than the file holds"));
    }
    Ok(n)
}

fn display_name(p: &tauri_plugin_fs::FilePath) -> Option<String> {
    use tauri_plugin_fs::FilePath;
    let path = match p {
        FilePath::Path(p) => Some(p.clone()),
        // iOS returns file:// URLs; Android returns opaque content:// URIs with no usable name.
        FilePath::Url(u) if u.scheme() == "file" => u.to_file_path().ok(),
        FilePath::Url(_) => None,
    }?;
    path.file_name().map(|n| n.to_string_lossy().chars().take(80).collect())
}

// ---- commands -------------------------------------------------------------------------------

fn join_err(e: tauri::Error) -> AppError {
    AppError::Storage(format!("The operation did not finish ({e}). Nothing was changed."))
}

#[derive(Debug, Serialize)]
pub struct SaveCopyResult {
    /// False when the user closed the system screen (or it could not open): nothing was saved.
    pub saved: bool,
    pub bytes: u64,
}

/// Opens the system "save" screen for one of the app's own backup or export files and copies its
/// bytes to the place the user picks. The private file is left exactly as it was.
#[tauri::command]
pub async fn save_copy(app: AppHandle, path: String) -> AppResult<SaveCopyResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let src = ensure_inside(&path, &[backups_dir(&app)?, exports_dir(&app)?])?;
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "MedTracker-file".into());
        let picked = app
            .dialog()
            .file()
            .set_title("Save a copy")
            .set_file_name(name.clone())
            .add_filter("MedTracker file", &[filter_for(&name, cfg!(mobile)).as_str()])
            .blocking_save_file();
        let Some(dest) = picked else { return Ok(SaveCopyResult { saved: false, bytes: 0 }) };

        let mut opts = OpenOptions::new();
        opts.write(true).truncate(true);
        let mut out = app
            .fs()
            .open(dest, opts)
            .map_err(|e| AppError::Storage(format!("Could not write to the place you chose ({e}). Your file is still saved inside MedTracker.")))?;
        let bytes = copy_out(&src, &mut out)
            .map_err(|e| AppError::Storage(format!("Could not finish saving the copy ({e}). Your file is still saved inside MedTracker.")))?;
        let _ = out.sync_all(); // not every provider's descriptor can be synced; the byte count above is the check
        Ok(SaveCopyResult { saved: true, bytes })
    })
    .await
    .map_err(join_err)?
}

#[derive(Debug, Serialize)]
pub struct PickedBackup {
    /// Path of the app-private copy; pass it to `restore_backup`.
    pub path: String,
    /// The chosen file's name when the system tells us one (not on Android).
    pub name: Option<String>,
    pub size_bytes: u64,
}

/// Opens the system file picker, copies the chosen file into app-private staging and returns that
/// copy. `None` means nothing was chosen (or the picker could not open). Nothing is verified here:
/// `restore_backup` does all of that on the copy, exactly as it does for any file.
#[tauri::command]
pub async fn pick_backup_file(app: AppHandle) -> AppResult<Option<PickedBackup>> {
    let guard = backup::OpGuard::acquire()?; // a restore may be reading the old staged copy
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let picked = app
            .dialog()
            .file()
            .set_title("Choose a backup file")
            .set_picker_mode(PickerMode::Document)
            .set_file_access_mode(FileAccessMode::Copy) // iOS: a copy inside the app, not the original
            .blocking_pick_file();
        let Some(file) = picked else { return Ok(None) };
        let name = display_name(&file);

        let mut opts = OpenOptions::new();
        opts.read(true);
        let src = app
            .fs()
            .open(file, opts)
            .map_err(|e| AppError::Storage(format!("Could not read the file you chose ({e}). Your data was not changed.")))?;
        let (staged, size) = stage_import(&staging_dir(&app)?, src, MAX_IMPORT_BYTES, TimeCtx::system().now_utc_us)?;
        Ok(Some(PickedBackup { path: staged.to_string_lossy().into_owned(), name, size_bytes: size }))
    })
    .await
    .map_err(join_err)?
}

/// Deletes the staged copy of a picked file (the user changed their mind).
#[tauri::command]
pub fn discard_staged_backup(app: AppHandle) -> AppResult<()> {
    let _guard = backup::OpGuard::acquire()?;
    clear_staging(&staging_dir(&app)?);
    Ok(())
}

#[cfg(test)]
mod tests;
