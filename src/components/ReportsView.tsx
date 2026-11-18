import { useCallback, useEffect, useState } from "react";
import { type Api, type ExportResult, type Report, type ReportCounts, errorMessage } from "../lib/api";
import { medLabel } from "../lib/format";
import { Button, Field, inputClass } from "./ui";

const pad = (n: number) => String(n).padStart(2, "0");
const iso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

function defaultRange(): [string, string] {
  const to = new Date();
  const from = new Date(to);
  from.setDate(from.getDate() - 6);
  return [iso(from), iso(to)];
}

const th = "border-b border-neutral-300 px-2 py-2 text-left text-sm font-semibold";
const td = "border-b border-neutral-200 px-2 py-3 align-top";

/** Same columns for every table, so each row reads the same way. */
function CountCells({ c }: { c: ReportCounts }) {
  return (
    <>
      <td className={td}>{c.occurrences}</td>
      <td className={td}>{c.taken}</td>
      <td className={td}>{c.skipped}</td>
      <td className={td}>{c.not_logged}</td>
      <td className={td}>{c.pending}</td>
      <td className={td}>{c.conflicts}</td>
      <td className={td}>{c.unscheduled_taken + c.unscheduled_skipped}</td>
    </>
  );
}

function Head({ first }: { first: string }) {
  return (
    <thead>
      <tr>
        <th scope="col" className={th}>{first}</th>
        <th scope="col" className={th}>Scheduled</th>
        <th scope="col" className={th}>Taken</th>
        <th scope="col" className={th}>Skipped</th>
        <th scope="col" className={th}>Earlier, not logged</th>
        <th scope="col" className={th}>Later, not yet logged</th>
        <th scope="col" className={th}>Conflicting</th>
        <th scope="col" className={th}>Unscheduled entries</th>
      </tr>
    </thead>
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

      <div className="grid grid-cols-2 gap-3">
        <Field label="From">
          <input className={inputClass} type="date" value={from} max={to} onChange={(e) => e.target.value && setRange([e.target.value, to])} />
        </Field>
        <Field label="To">
          <input className={inputClass} type="date" value={to} min={from} onChange={(e) => e.target.value && setRange([from, e.target.value])} />
        </Field>
      </div>

      {error && <p role="alert" className="text-red-800">{error}</p>}

      {report && (
        <>
          <p className="text-sm" data-testid="report-scope">
            {report.from} to {report.to}. {report.grouping}
            {report.reporting_zone ? ` Device time zone: ${report.reporting_zone}.` : ""}
          </p>

          <div className="overflow-x-auto">
            <table className="w-full min-w-max text-base">
              <caption className="pb-2 text-left text-lg font-semibold">Totals for the range</caption>
              <Head first="Range" />
              <tbody>
                <tr>
                  <th scope="row" className={`${td} text-left font-medium`}>All medications</th>
                  <CountCells c={report.totals} />
                </tr>
              </tbody>
            </table>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full min-w-max text-base">
              <caption className="pb-2 text-left text-lg font-semibold">By medication</caption>
              <Head first="Medication" />
              <tbody>
                {report.by_medication.length === 0 && (
                  <tr><td colSpan={8} className={td}>Nothing recorded or scheduled in this range.</td></tr>
                )}
                {report.by_medication.map((m) => (
                  <tr key={m.medication_id}>
                    <th scope="row" className={`${td} text-left font-medium`}>{medLabel(m.name, m.strength)}</th>
                    <CountCells c={m} />
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full min-w-max text-base">
              <caption className="pb-2 text-left text-lg font-semibold">By day</caption>
              <Head first="Day" />
              <tbody>
                {report.by_day.length === 0 && (
                  <tr><td colSpan={8} className={td}>No days to show.</td></tr>
                )}
                {report.by_day.map((d) => (
                  <tr key={d.date}>
                    <th scope="row" className={`${td} text-left font-medium`}>{d.date}</th>
                    <CountCells c={d} />
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

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
          Saves a plain, unencrypted file of the entries in the range above to your Downloads folder. Anyone who can open the file can read it.
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
          <p role="status">
            Saved {exported.rows} {exported.rows === 1 ? "entry" : "entries"} to {exported.path}
          </p>
        )}
      </fieldset>
    </section>
  );
}
