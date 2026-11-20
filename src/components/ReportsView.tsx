import { useCallback, useEffect, useState } from "react";
import { type Api, type ExportResult, type Report, type ReportCounts, errorMessage } from "../lib/api";
import { medLabel } from "../lib/format";
import { SaveCopy } from "./SaveCopy";
import { Button, Field, inputClass } from "./ui";

const pad = (n: number) => String(n).padStart(2, "0");
const iso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

function defaultRange(): [string, string] {
  const to = new Date();
  const from = new Date(to);
  from.setDate(from.getDate() - 6);
  return [iso(from), iso(to)];
}

/** Date controls share the text-field width rules, plus appearance none (iOS). */
const dateClass = `${inputClass} appearance-none`;

const th = "border-b border-neutral-300 px-2 py-2 text-left text-sm font-semibold";
const td = "border-b border-neutral-200 px-2 py-3 align-top";

/** One label per count, in the same order as the values from countValues(). Shared by the table and the cards. */
const COUNT_LABELS = [
  "Scheduled",
  "Taken",
  "Skipped",
  "Earlier, not logged",
  "Later, not yet logged",
  "Conflicting",
  "Unscheduled entries",
] as const;

function countValues(c: ReportCounts): number[] {
  return [c.occurrences, c.taken, c.skipped, c.not_logged, c.pending, c.conflicts, c.unscheduled_taken + c.unscheduled_skipped];
}

/** Same columns for every table, so each row reads the same way. */
function CountCells({ c }: { c: ReportCounts }) {
  return (
    <>
      {countValues(c).map((v, i) => (
        <td key={COUNT_LABELS[i]} className={td}>{v}</td>
      ))}
    </>
  );
}

function Head({ first }: { first: string }) {
  return (
    <thead>
      <tr>
        <th scope="col" className={th}>{first}</th>
        {COUNT_LABELS.map((label) => (
          <th key={label} scope="col" className={th}>{label}</th>
        ))}
      </tr>
    </thead>
  );
}

type Row = { key: string; label: string; counts: ReportCounts };

/**
 * One report section. Below 640px it renders a card per row (heading + definition list); from 640px up it renders
 * the captioned table. Both are in the DOM; the CSS classes pick which one is shown, so only one is in the
 * accessibility tree at a time.
 */
function CountSection({
  caption,
  first,
  rows,
  empty,
}: {
  caption: string;
  first: string;
  rows: Row[];
  empty: string;
}) {
  return (
    <section aria-label={caption} className="space-y-2">
      <h3 className="text-lg font-semibold sm:hidden">{caption}</h3>

      <div className="hidden overflow-x-auto sm:block" data-layout="table">
        <table className="w-full min-w-max text-base">
          <caption className="pb-2 text-left text-lg font-semibold">{caption}</caption>
          <Head first={first} />
          <tbody>
            {rows.length === 0 && (
              <tr><td colSpan={COUNT_LABELS.length + 1} className={td}>{empty}</td></tr>
            )}
            {rows.map((r) => (
              <tr key={r.key}>
                <th scope="row" className={`${td} text-left font-medium`}>{r.label}</th>
                <CountCells c={r.counts} />
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="sm:hidden" data-layout="cards">
        {rows.length === 0 && <p className="text-base">{empty}</p>}
        <ul className="m-0 list-none space-y-3 p-0">
          {rows.map((r) => (
            <li key={r.key} className="min-w-0 rounded-lg border border-neutral-300 bg-white p-3">
              <h4 className="break-words text-base font-semibold">{r.label}</h4>
              <dl className="mt-2 grid grid-cols-[minmax(0,1fr)_auto] gap-x-3 gap-y-1 text-sm">
                {countValues(r.counts).map((v, i) => (
                  <div key={COUNT_LABELS[i]} className="contents">
                    <dt className="min-w-0 break-words text-neutral-800">{COUNT_LABELS[i]}</dt>
                    <dd className="m-0 text-right font-medium tabular-nums">{v}</dd>
                  </div>
                ))}
              </dl>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}

export function ReportsView({ api, version }: { api: Api; version: number }) {
  const [[from, to], setRange] = useState(defaultRange);
  const [report, setReport] = useState<Report | null>(null);
  const [error, setError] = useState("");
  const [notes, setNotes] = useState(false);
  const [history, setHistory] = useState(false);
  const [exported, setExported] = useState<ExportResult | null>(null);

  const load = useCallback(async () => {
    setError("");
    try {
      setReport(await api.reportRange(from, to));
    } catch (e) {
      setReport(null);
      setError(errorMessage(e));
    }
  }, [api, from, to]);

  // `version` re-runs the report after the user changes entries elsewhere.
  useEffect(() => {
    void load();
  }, [load, version]);

  async function save(format: "csv" | "json") {
    setError("");
    setExported(null);
    try {
      setExported(await api.exportLedger({ from, to, format, include_notes: notes, include_history: history }));
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <section aria-labelledby="reports-h" className="space-y-5">
      <h2 id="reports-h" className="text-xl font-semibold">Reports</h2>
      <p className="text-sm text-neutral-700">Plain counts of what you recorded. They are not a score and say nothing about whether any dose was needed or right.</p>

      {/* One column on phones; two side by side once there is room for both dates. */}
      <div className="grid grid-cols-1 gap-3 min-[420px]:grid-cols-2 [&>*]:min-w-0">
        <Field label="From">
          <input className={dateClass} type="date" value={from} max={to} onChange={(e) => e.target.value && setRange([e.target.value, to])} />
        </Field>
        <Field label="To">
          <input className={dateClass} type="date" value={to} min={from} onChange={(e) => e.target.value && setRange([from, e.target.value])} />
        </Field>
      </div>

      {error && <p role="alert" className="text-red-800">{error}</p>}

      {report && (
        <>
          <p className="text-sm" data-testid="report-scope">
            {report.from} to {report.to}. {report.grouping}
            {report.reporting_zone ? ` Device time zone: ${report.reporting_zone}.` : ""}
          </p>

          <CountSection
            caption="Totals for the range"
            first="Range"
            rows={[{ key: "all", label: "All medications", counts: report.totals }]}
            empty="Nothing recorded or scheduled in this range."
          />

          <CountSection
            caption="By medication"
            first="Medication"
            rows={report.by_medication.map((m) => ({ key: m.medication_id, label: medLabel(m.name, m.strength), counts: m }))}
            empty="Nothing recorded or scheduled in this range."
          />

          <CountSection
            caption="By day"
            first="Day"
            rows={report.by_day.map((d) => ({ key: d.date, label: d.date, counts: d }))}
            empty="No days to show."
          />

          <p className="text-sm">
            Not counted above: {report.voided_entries} undone {report.voided_entries === 1 ? "entry" : "entries"} and {report.deleted_entries} deleted{" "}
            {report.deleted_entries === 1 ? "entry" : "entries"} (details erased) in this range.
          </p>

          <details className="rounded-lg border border-neutral-300 bg-white p-3">
            <summary className="min-h-12 cursor-pointer py-3 font-medium">What these counts mean</summary>
            <ul className="list-disc space-y-1 pl-5 text-sm">
              {report.definitions.map((d) => <li key={d}>{d}</li>)}
            </ul>
          </details>
        </>
      )}

      <fieldset className="space-y-3 rounded-lg border border-neutral-300 bg-white p-4">
        <legend className="px-1 text-lg font-semibold">Export entries</legend>
        <p className="text-sm text-neutral-700">
          Creates a plain, unencrypted file of the entries in the range above. It is kept inside MedTracker first; then use Save a copy
          to put it in Files, a cloud drive or another place you choose. Anyone who can open the file can read it.
        </p>
        <label className="flex min-h-12 items-center gap-3">
          <input type="checkbox" className="h-6 w-6" checked={notes} onChange={(e) => setNotes(e.target.checked)} />
          <span>Include notes</span>
        </label>
        <label className="flex min-h-12 items-center gap-3">
          <input type="checkbox" className="h-6 w-6" checked={history} onChange={(e) => setHistory(e.target.checked)} />
          <span>Include undone and deleted entries (marked in a status column)</span>
        </label>
        <div className="flex gap-2">
          <Button variant="primary" className="flex-1" onClick={() => save("csv")}>Export CSV</Button>
          <Button className="flex-1" onClick={() => save("json")}>Export JSON</Button>
        </div>
        {exported && (
          <>
            <p role="status">
              File created with {exported.rows} {exported.rows === 1 ? "entry" : "entries"}: {exported.file_name}
            </p>
            <SaveCopy key={exported.path} api={api} path={exported.path} />
          </>
        )}
      </fieldset>
    </section>
  );
}
