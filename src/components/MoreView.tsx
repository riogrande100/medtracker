import { useId, useState } from "react";
import type { Api } from "../lib/api";
import type { UiPrefs } from "../lib/uiPrefs";
import { BackupView } from "./BackupView";
import { ReportsView } from "./ReportsView";
import { SettingsView } from "./SettingsView";
import { Button } from "./ui";

type Page = "home" | "reports" | "backup" | "settings";

/** One large, full-width row. The title is the accessible name; the description is linked separately. */
function NavRow({ title, description, onClick }: { title: string; description: string; onClick: () => void }) {
  const descId = useId();
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={title}
      aria-describedby={descId}
      className="flex min-h-14 w-full cursor-pointer items-center justify-between gap-3 rounded-xl border border-neutral-300 bg-white p-4 text-left hover:bg-neutral-100"
    >
      <span className="block">
        <span className="block text-lg font-semibold">{title}</span>
        <span id={descId} className="block text-sm text-neutral-700">{description}</span>
      </span>
      <span aria-hidden="true" className="text-2xl text-neutral-600">›</span>
    </button>
  );
}

/** The More tab: Reports, Backup and Settings as large rows, each opening its own screen. */
export function MoreView({
  api,
  version,
  onChanged,
  prefs,
  onPrefsChange,
  prefsSaved,
}: {
  api: Api;
  version: number;
  onChanged: () => void;
  prefs: UiPrefs;
  onPrefsChange: (next: UiPrefs) => void;
  prefsSaved: boolean;
}) {
  const [page, setPage] = useState<Page>("home");

  if (page === "home") {
    return (
      <div className="space-y-3">
        <h2 className="text-2xl font-semibold">More</h2>
        <NavRow title="Reports" description="Counts for a date range, and export" onClick={() => setPage("reports")} />
        <NavRow title="Backup" description="Save a backup file, or restore from one" onClick={() => setPage("backup")} />
        <NavRow title="Settings" description="Colors, text size, motion, and privacy" onClick={() => setPage("settings")} />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Button onClick={() => setPage("home")}>Back to More</Button>
      {page === "reports" && <ReportsView api={api} version={version} />}
      {page === "backup" && <BackupView api={api} onChanged={onChanged} />}
      {page === "settings" && <SettingsView prefs={prefs} onChange={onPrefsChange} saved={prefsSaved} />}
    </div>
  );
}
