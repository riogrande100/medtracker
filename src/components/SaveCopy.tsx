import { useState } from "react";
import { type Api, errorMessage } from "../lib/api";
import { Button } from "./ui";

type Outcome = { kind: "saved" } | { kind: "nothing" } | { kind: "error"; message: string };

/**
 * "Save a copy": hands a finished, app-private file to the system save screen (Files / Drive / a
 * USB drive and so on). The private file is never moved or changed, so if the system screen fails
 * or is closed the file is still here and we say where. Use `key={path}` so a new file starts clean.
 */
export function SaveCopy({ api, path, hint }: { api: Api; path: string; hint?: string }) {
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  async function save() {
    setBusy(true);
    setOutcome(null);
    try {
      const r = await api.saveCopy(path);
      setOutcome({ kind: r.saved ? "saved" : "nothing" });
    } catch (e) {
      setOutcome({ kind: "error", message: errorMessage(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-2">
      <p className="text-sm text-neutral-800">
        {hint ?? "Save a copy to Files, a cloud drive or another place you choose."} The file stays inside MedTracker too, at{" "}
        <span className="break-all font-medium">{path}</span>
      </p>
      <Button variant="primary" className="w-full" disabled={busy} onClick={save}>{busy ? "Waiting for you to choose…" : "Save a copy"}</Button>
      {outcome?.kind === "saved" && <p role="status" aria-live="polite" className="font-medium">A copy was saved where you chose.</p>}
      {outcome?.kind === "nothing" && (
        <p role="status" aria-live="polite">
          Nothing was saved, because no place was chosen or the save screen could not open. You can try again.
        </p>
      )}
      {outcome?.kind === "error" && <p role="alert" className="text-red-800">{outcome.message}</p>}
    </div>
  );
}
