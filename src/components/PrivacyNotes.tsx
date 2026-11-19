/** Shared wording for the permanent-deletion dialogs. Keep claims limited to what the app can do. */

/** Erasure happens inside MedTracker's own data file; the app can't scrub copies it doesn't control. */
export function InAppErasureNote() {
  return (
    <span className="block text-sm text-neutral-700">
      Deleting happens inside MedTracker. It can't reach copies outside the app, such as device or cloud backups. Backup
      (.medbackup) and export files, including any copy you saved elsewhere (such as in Files or a cloud drive), are also outside
      MedTracker's control, and deleting an entry in the app does not delete them.
    </span>
  );
}

/** Shown only while a temporary pre-upgrade copy (made when MedTracker upgraded its data) is on the device. */
export function UpgradeCopyNote({ show }: { show: boolean }) {
  if (!show) return null;
  return (
    <span className="block text-sm font-medium" data-testid="upgrade-copy-note">
      MedTracker also kept a temporary copy of your data from before its latest upgrade, and that copy still contains what you are
      deleting. It is removed automatically the next time MedTracker starts.
    </span>
  );
}
