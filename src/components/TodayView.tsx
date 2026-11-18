import { useCallback, useEffect, useState } from "react";
import { type Api, type NextDue, type TodayItem, errorKind, errorMessage } from "../lib/api";
import { useTodayRefresh } from "../lib/useTodayRefresh";
import { formatOccurrence, formatTime, medLabel } from "../lib/format";
import { Button } from "./ui";
import { LogEarlierDialog } from "./LogEarlierDialog";

const statusText: Record<TodayItem["status"], string> = {
  upcoming: "Later today",
  due: "Due now",
  taken: "Taken",
  skipped: "Skipped",
  conflict: "Conflicting entries — review in History",
  // Neutral on purpose: the app never says "missed" or "overdue".
  earlier_not_logged: "Not logged",
};

function ZoneContext({ item }: { item: TodayItem }) {
  // Only when this card's clock differs from the device's, so the same instant is never ambiguous.
  if (item.device_time === null) return null;
  return (
    <p className="text-sm text-neutral-700">
      {formatTime(item.local_time)} {item.zone_abbreviation ?? ""} in {item.zone_id ?? "its own zone"} · {formatTime(item.device_time)} on this device
    </p>
  );
}

export function TodayView({ api, version, onChanged }: { api: Api; version: number; onChanged: () => void }) {
  const [items, setItems] = useState<TodayItem[] | null>(null);
  const [next, setNext] = useState<NextDue | null>(null);
  const [message, setMessage] = useState("");
  const [logOpen, setLogOpen] = useState(false);

  const load = useCallback(async () => {
    try {
      setItems(await api.listToday());
    } catch (e) {
      setMessage(errorMessage(e));
    }
    // The next-due line is a convenience: its failure must never hide the cards.
    api.nextDue().then(setNext, () => setNext(null));
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
      // The day (or the schedule) changed under an open screen: show the new state instead.
      if (errorKind(e) === "stale_date") void load();
    }
  }

  const record = (i: TodayItem, kind: "take" | "skip") =>
    act(
      () =>
        api.recordDose({
          medication_id: i.medication_id,
          kind,
          occurrence_id: i.occurrence_id,
          occurred_local: null,
          note: null,
        }),
      `${i.name} marked as ${kind === "take" ? "taken" : "skipped"}. You can undo it.`,
    );

  const card = (i: TodayItem) => {
    const open = i.status === "due" || i.status === "upcoming";
    return (
      <li key={i.occurrence_id} className="rounded-xl border border-neutral-300 bg-white p-4" aria-label={`${medLabel(i.name, i.strength)} at ${formatTime(i.local_time)}`}>
        <div className="flex items-baseline justify-between gap-2">
          <p className="text-lg font-semibold">{medLabel(i.name, i.strength)}</p>
          <p className="text-base">{formatTime(i.local_time)}</p>
        </div>
        {i.form && <p className="text-sm text-neutral-700">{i.form}</p>}
        <ZoneContext item={i} />
        {i.adjusted && <p className="text-sm text-neutral-700">Adjusted for a clock change.</p>}
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
  };

  const earlier = (items ?? []).filter((i) => i.status === "earlier_not_logged");
  const todays = (items ?? []).filter((i) => i.status !== "earlier_not_logged");

  return (
    <section aria-labelledby="today-h" className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <h2 id="today-h" className="text-2xl font-semibold">Today</h2>
        <Button onClick={() => setLogOpen(true)}>Log earlier dose</Button>
      </div>
      {next && (
        <p data-testid="next-due" className="rounded-lg bg-neutral-100 p-3 text-sm">
          <span className="font-medium">Next scheduled:</span> {medLabel(next.name, next.strength)}, {formatOccurrence(next.occurrence)}
        </p>
      )}
      <p role="status" className="min-h-6 text-sm text-neutral-800">{message}</p>
      {items === null ? (
        <p>Loading…</p>
      ) : items.length === 0 ? (
        <p className="rounded-lg border border-neutral-300 bg-white p-4">
          Nothing scheduled today. Add a schedule to a medication in the Medications tab.
        </p>
      ) : (
        <>
          {todays.length === 0 ? (
            <p className="rounded-lg border border-neutral-300 bg-white p-4">Nothing else is scheduled today.</p>
          ) : (
            <ul className="space-y-3">{todays.map(card)}</ul>
          )}
          {earlier.length > 0 && (
            <section aria-labelledby="earlier-h" className="space-y-3">
              <h3 id="earlier-h" className="text-lg font-semibold">Earlier, not logged</h3>
              <p className="text-sm text-neutral-700">
                These were scheduled on an earlier day and have nothing recorded. Use “Log earlier dose” if you took or skipped one.
              </p>
              <ul className="space-y-3">{earlier.map(card)}</ul>
            </section>
          )}
        </>
      )}
      <LogEarlierDialog api={api} open={logOpen} onOpenChange={setLogOpen} onSaved={() => { setMessage("Dose saved."); onChanged(); }} />
    </section>
  );
}
