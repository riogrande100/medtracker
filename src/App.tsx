import * as Tabs from "@radix-ui/react-tabs";
import { useEffect, useState } from "react";
import { type Api, type AppError, RECOVERABLE_KINDS, errorKind } from "./lib/api";
import { BackupView } from "./components/BackupView";
import { HistoryView } from "./components/HistoryView";
import { MedicationsView } from "./components/MedicationsView";
import { ReportsView } from "./components/ReportsView";
import { TodayView } from "./components/TodayView";

const tab =
  "min-h-14 flex-1 px-2 text-base font-medium text-neutral-700 data-[state=active]:border-b-4 data-[state=active]:border-blue-700 data-[state=active]:text-neutral-900 cursor-pointer";

/**
 * Shown when the data file could not be opened safely (newer version, corrupt, failed upgrade).
 * The app never modifies or deletes the file in these cases, so there is nothing to "reset".
 */
function RecoveryScreen({ error }: { error: AppError }) {
  const title: Record<string, string> = {
    newer_database: "This data needs a newer MedTracker",
    migration_failed: "MedTracker couldn't upgrade your data",
    corrupt_database: "MedTracker couldn't read your data file",
  };
  return (
    <main role="alert" className="mx-auto max-w-xl space-y-4 p-4">
      <h1 className="text-2xl font-semibold">{title[error.kind] ?? "MedTracker can't open your data"}</h1>
      <p>{error.message}</p>
      <p className="rounded-lg border border-neutral-300 bg-white p-4">
        Nothing was deleted or changed. Please don't uninstall MedTracker or clear its data until your file is safe. Closing and
        reopening the app will try again.
      </p>
    </main>
  );
}

export function App({ api }: { api: Api }) {
  const [version, setVersion] = useState(0);
  const [startup, setStartup] = useState<"checking" | "ok" | AppError>("checking");

  useEffect(() => {
    api.startupStatus().then(
      (err) => setStartup(err ?? "ok"),
      // Commands that fail with a recoverable kind mean the same thing.
      (e) => setStartup(RECOVERABLE_KINDS.includes(errorKind(e) as never) ? (e as AppError) : "ok"),
    );
  }, [api]);

  if (startup === "checking") return <p className="p-4">Loading…</p>;
  if (startup !== "ok") return <RecoveryScreen error={startup} />;

  const changed = () => setVersion((v) => v + 1);
  const props = { api, version, onChanged: changed };
  return (
    <Tabs.Root defaultValue="today" className="mx-auto flex min-h-screen max-w-xl flex-col">
      <header className="px-4 pt-4"><h1 className="text-sm font-medium text-neutral-700">MedTracker · stored only on this device</h1></header>
      <Tabs.List aria-label="Sections" className="flex border-b border-neutral-300 bg-neutral-50">
        <Tabs.Trigger value="today" className={tab}>Today</Tabs.Trigger>
        <Tabs.Trigger value="history" className={tab}>History</Tabs.Trigger>
        <Tabs.Trigger value="medications" className={tab}>Medications</Tabs.Trigger>
        <Tabs.Trigger value="backup" className={tab}>Backup</Tabs.Trigger>
        <Tabs.Trigger value="reports" className={tab}>Reports</Tabs.Trigger>
      </Tabs.List>
      <main className="flex-1 p-4">
        <Tabs.Content value="today"><TodayView {...props} /></Tabs.Content>
        <Tabs.Content value="history"><HistoryView {...props} /></Tabs.Content>
        <Tabs.Content value="medications"><MedicationsView {...props} /></Tabs.Content>
        <Tabs.Content value="backup"><BackupView api={api} onChanged={changed} /></Tabs.Content>
        <Tabs.Content value="reports"><ReportsView api={api} version={version} /></Tabs.Content>
      </main>
    </Tabs.Root>
  );
}
