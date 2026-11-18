import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { useEffect, useState } from "react";
import { type Api, type BackupCreated, type BackupFolder, type RestoreResult, errorMessage } from "../lib/api";
import { formatLocal, nowLocalInput } from "../lib/format";
import { Button, Field, inputClass } from "./ui";

const MIN_LENGTH = 8;

const when = (us: number) => formatLocal(nowLocalInput(new Date(us / 1000)));
const kb = (n: number) => (n < 1024 * 1024 ? `${Math.max(1, Math.round(n / 1024))} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`);

function CreateSection({ api, folder, onCreated }: { api: Api; folder: string; onCreated: () => void }) {
  const [pass, setPass] = useState("");
  const [again, setAgain] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [done, setDone] = useState<BackupCreated | null>(null);

  const problem = pass.length > 0 && pass.length < MIN_LENGTH ? `Use at least ${MIN_LENGTH} characters.` : again && pass !== again ? "The two passphrases don't match." : "";
  const ready = pass.length >= MIN_LENGTH && pass === again && !busy;

  async function create() {
    setBusy(true);
    setError("");
    setDone(null);
    try {
      setDone(await api.createBackup(pass));
      setPass("");
      setAgain("");
      onCreated();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="create-h" className="space-y-3">
      <h2 id="create-h" className="text-xl font-semibold">Create a backup</h2>
      <p>
        This saves a copy of everything in MedTracker as one locked file on this device. It never goes online. To keep it safe, copy it to
        somewhere else too, such as a USB drive.
      </p>
      <p role="note" className="rounded-lg border-2 border-amber-700 bg-amber-50 p-4 font-medium">
        If you forget this passphrase, your backup cannot be opened. MedTracker doesn't keep your passphrase and nobody, including us, can
        recover it. Write it down and keep it somewhere safe.
      </p>
      <Field label="Passphrase">
        <input className={inputClass} type={show ? "text" : "password"} autoComplete="new-password" value={pass} onChange={(e) => setPass(e.target.value)} />
      </Field>
      <Field label="Type the passphrase again">
        <input className={inputClass} type={show ? "text" : "password"} autoComplete="new-password" value={again} onChange={(e) => setAgain(e.target.value)} />
      </Field>
      <label className="flex min-h-12 items-center gap-3">
        <input type="checkbox" className="size-6" checked={show} onChange={(e) => setShow(e.target.checked)} />
        <span>Show passphrase</span>
      </label>
      {problem && <p role="alert" className="text-red-800">{problem}</p>}
      <p className="text-sm text-neutral-700">The file is saved in: <span className="break-all font-medium">{folder}</span></p>
      <Button variant="primary" className="w-full" disabled={!ready} onClick={create}>{busy ? "Creating…" : "Create backup"}</Button>
      {error && <p role="alert" className="text-red-800">{error}</p>}
      {done && (
        <div role="status" className="space-y-1 rounded-lg border border-brand bg-brand-soft p-4">
          <p className="font-medium">Backup saved and checked.</p>
          <p className="break-all">{done.path}</p>
          <p>
            {done.medications} {done.medications === 1 ? "medication" : "medications"}, {done.doses} {done.doses === 1 ? "entry" : "entries"} ({kb(done.size_bytes)}).
            Anyone who has this file and its passphrase can read your records, so store both carefully.
          </p>
        </div>
      )}
    </section>
  );
}

function RestoreSection({ api, folder, onChanged, onRestored }: { api: Api; folder: BackupFolder; onChanged: () => void; onRestored: () => void }) {
  const [selected, setSelected] = useState("");
  const [typed, setTyped] = useState("");
  const [pass, setPass] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [done, setDone] = useState<RestoreResult | null>(null);

  const path = typed.trim() || selected;
  const ready = path !== "" && pass !== "" && !busy;

  async function restore() {
    setBusy(true);
    setError("");
    setDone(null);
    try {
      setDone(await api.restoreBackup(path, pass));
      setPass("");
      setTyped("");
      setSelected("");
      onChanged();
      onRestored();
    } catch (e) {
      // Whatever went wrong, the data on this device was not touched.
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="restore-h" className="space-y-3">
      <h2 id="restore-h" className="text-xl font-semibold">Restore from a backup</h2>
      <p>
        Restoring replaces what is in MedTracker now with what was in the backup. MedTracker checks the whole file first, and if anything is
        wrong it stops and your current data stays exactly as it is. This moves data between devices once; it does not keep them in sync.
      </p>
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium text-neutral-800">Backups found in {folder.folder}</legend>
        {folder.files.length === 0 && <p className="text-neutral-700">No backups found there yet.</p>}
        {folder.files.map((f) => (
          <label key={f.path} className="flex min-h-12 items-center gap-3 rounded-lg border border-neutral-300 bg-white px-3 py-2">
            <input type="radio" name="backup-file" className="size-6" checked={selected === f.path} onChange={() => setSelected(f.path)} />
            <span>
              <span className="block break-all font-medium">{f.name}</span>
              <span className="block text-sm text-neutral-700">Saved {when(f.modified_utc_us)} · {kb(f.size_bytes)}</span>
            </span>
          </label>
        ))}
      </fieldset>
      <Field label="Or type the full path of a backup file you copied here">
        <input className={inputClass} type="text" autoComplete="off" value={typed} onChange={(e) => setTyped(e.target.value)} />
      </Field>
      <Field label="Backup passphrase">
        <input className={inputClass} type="password" autoComplete="off" value={pass} onChange={(e) => setPass(e.target.value)} />
      </Field>

      <AlertDialog.Root>
        <AlertDialog.Trigger asChild>
          <Button variant="danger" className="w-full" disabled={!ready}>{busy ? "Restoring…" : "Restore this backup"}</Button>
        </AlertDialog.Trigger>
        <AlertDialog.Portal>
          <AlertDialog.Overlay className="fixed inset-0 bg-black/40" />
          <AlertDialog.Content className="fixed inset-x-3 top-6 mx-auto max-w-md rounded-xl bg-white p-5 shadow-lg">
            <AlertDialog.Title className="text-xl font-semibold">Replace your data with this backup?</AlertDialog.Title>
            <AlertDialog.Description className="my-3 space-y-2">
              <span className="block">Everything in MedTracker now will be replaced by what is in the backup, including medications, schedules and history.</span>
              <span className="block">
                MedTracker keeps a copy of your current data until it next starts, then deletes it. After that, the only way back is another backup.
              </span>
            </AlertDialog.Description>
            <div className="flex gap-2">
              <AlertDialog.Cancel asChild><Button className="flex-1">Keep my data</Button></AlertDialog.Cancel>
              <AlertDialog.Action asChild><Button variant="danger" className="flex-1" onClick={restore}>Replace my data</Button></AlertDialog.Action>
            </div>
          </AlertDialog.Content>
        </AlertDialog.Portal>
      </AlertDialog.Root>

      {error && (
        <p role="alert" className="text-red-800">
          {error} <span className="block">Your current data was not changed.</span>
        </p>
      )}
      {done && (
        <div role="status" className="space-y-1 rounded-lg border border-brand bg-brand-soft p-4">
          <p className="font-medium">Restored from the backup made {when(done.backup_created_at_utc_us)}.</p>
          <p>
            {done.medications} {done.medications === 1 ? "medication" : "medications"}, {done.doses} {done.doses === 1 ? "entry" : "entries"}.
            {done.upgraded_from !== null && " The backup was from an older version of MedTracker and was upgraded."}
            {done.safety_copy_kept && " A copy of your previous data is kept until MedTracker next starts."}
          </p>
        </div>
      )}
    </section>
  );
}

export function BackupView({ api, onChanged }: { api: Api; onChanged: () => void }) {
  const [folder, setFolder] = useState<BackupFolder | null>(null);
  const [loadError, setLoadError] = useState("");

  const load = () => api.backupFolder().then(setFolder, (e) => setLoadError(errorMessage(e)));
  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api]);

  if (loadError) return <p role="alert" className="text-red-800">{loadError}</p>;
  if (!folder) return <p>Loading…</p>;
  return (
    <div className="space-y-8">
      <CreateSection api={api} folder={folder.folder} onCreated={() => void load()} />
      <hr className="border-neutral-300" />
      <RestoreSection api={api} folder={folder} onChanged={onChanged} onRestored={() => void load()} />
    </div>
  );
}
