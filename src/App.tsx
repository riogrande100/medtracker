import * as Tabs from "@radix-ui/react-tabs";
import { useState } from "react";
import type { Api } from "./lib/api";
import { HistoryView } from "./components/HistoryView";
import { MedicationsView } from "./components/MedicationsView";
import { TodayView } from "./components/TodayView";

const tab =
  "min-h-14 flex-1 px-2 text-base font-medium text-neutral-700 data-[state=active]:border-b-4 data-[state=active]:border-blue-700 data-[state=active]:text-neutral-900 cursor-pointer";

export function App({ api }: { api: Api }) {
  const [version, setVersion] = useState(0);
  const changed = () => setVersion((v) => v + 1);
  const props = { api, version, onChanged: changed };
  return (
    <Tabs.Root defaultValue="today" className="mx-auto flex min-h-screen max-w-xl flex-col">
      <header className="px-4 pt-4"><h1 className="text-sm font-medium text-neutral-700">MedTracker · stored only on this device</h1></header>
      <Tabs.List aria-label="Sections" className="flex border-b border-neutral-300 bg-neutral-50">
        <Tabs.Trigger value="today" className={tab}>Today</Tabs.Trigger>
        <Tabs.Trigger value="history" className={tab}>History</Tabs.Trigger>
        <Tabs.Trigger value="medications" className={tab}>Medications</Tabs.Trigger>
      </Tabs.List>
      <main className="flex-1 p-4">
        <Tabs.Content value="today"><TodayView {...props} /></Tabs.Content>
        <Tabs.Content value="history"><HistoryView {...props} /></Tabs.Content>
        <Tabs.Content value="medications"><MedicationsView {...props} /></Tabs.Content>
      </main>
    </Tabs.Root>
  );
}
