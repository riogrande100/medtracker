import { useCallback, useEffect, useState } from "react";
import { type Api, type TodayItem, errorKind, errorMessage } from "../lib/api";
import { useTodayRefresh } from "../lib/useTodayRefresh";
import { formatTime, medLabel } from "../lib/format";
import { Button } from "./ui";
import { LogEarlierDialog } from "./LogEarlierDialog";

const statusText: Record<TodayItem["status"], string> = {
  upcoming: "Later today",
  due: "Due now",
  taken: "Taken",
  skipped: "Skipped",
  conflict: "Conflicting entries — review in History",
};

export function TodayView({ api, version, onChanged }: { api: Api; version: number; onChanged: () => void }) {
  const [items, setItems] = useState<TodayItem[] | null>(null);
  const [message, setMessage] = useState("");
  const [logOpen, setLogOpen] = useState(false);

  const load = useCallback(async () => {
    try {
      setItems(await api.listToday());
    } catch (e) {
      setMessage(errorMessage(e));
    }
  }, [api]);

  useEffect(() => {
    void load();
  }, [load, version]);

  useTodayRefresh(items, load);

  async function act(run: () => Promise<unknown>, done: string) {
    try {
      await run();
      setMessage(done);
      onChanged();
    } catch (e) {
      setMessage(errorMessage(e));
      // The day changed under an open screen: show the new day instead of acting on the old one.
      if (errorKind(e) === "stale_date") void load();
    }
  }

  const record = (i: TodayItem, kind: "take" | "skip") =>
    act(
      () =>
        api.recordDose({
          medication_id: i.medication_id,
          kind,
          slot_id: i.slot_id,
          local_date: i.local_date,
          occurred_local: null,
          note: null,
        }),
      `${i.name} marked as ${kind === "take" ? "taken" : "skipped"}. You can undo it.`,
    );

  return (
    <section aria-labelledby="today-h" className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <h2 id="today-h" className="text-2xl font-semibold">Today</h2>
        <Button onClick={() => setLogOpen(true)}>Log earlier dose</Button>
      </div>
      <p role="status" className="min-h-6 text-sm text-neutral-800">{message}</p>
      {items === null ? (
        <p>Loading…</p>
      ) : items.length === 0 ? (
        <p className="rounded-lg border border-neutral-300 bg-white p-4">
          Nothing scheduled today. Add a medication with a daily time in the Medications tab.
        </p>
      ) : (
        <ul className="space-y-3">
          {items.map((i) => {
            const open = i.status === "due" || i.status === "upcoming";
            return (
              <li key={i.slot_id} className="rounded-xl border border-neutral-300 bg-white p-4" aria-label={`${medLabel(i.name, i.strength)} at ${formatTime(i.local_time)}`}>
                <div className="flex items-baseline justify-between gap-2">
                  <p className="text-lg font-semibold">{medLabel(i.name, i.strength)}</p>
                  <p className="text-base">{formatTime(i.local_time)}</p>
                </div>
                {i.form && <p className="text-sm text-neutral-700">{i.form}</p>}
                <p className="mt-1 font-medium">{statusText[i.status]}</p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {open ? (
                    <>
                      <Button variant="primary" className="flex-1" onClick={() => record(i, "take")}>
                        Taken<span className="sr-only">: {i.name}</span>
                      </Button>
                      <Button className="flex-1" onClick={() => record(i, "skip")}>
                        Skip<span className="sr-only">: {i.name}</span>
                      </Button>
                    </>
                  ) : (
                    i.event_ids.map((id) => (
                      <Button key={id} onClick={() => act(() => api.undoDose(id), `Undone. ${i.name} is back to due.`)}>
                        Undo<span className="sr-only"> for {i.name}</span>
                      </Button>
                    ))
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      )}
      <LogEarlierDialog api={api} open={logOpen} onOpenChange={setLogOpen} onSaved={() => { setMessage("Dose saved."); onChanged(); }} />
    </section>
  );
}
