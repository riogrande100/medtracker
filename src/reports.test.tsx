import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { App } from "./App";
import { ReportsView } from "./components/ReportsView";
import type { Api } from "./lib/api";
import { fakeApi } from "./test/fakeApi";

describe("Reports", () => {
  it("shows the fixture counts in accessible, captioned tables", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    const totals = await screen.findByRole("table", { name: "Totals for the range" });
    const cells = within(totals).getAllByRole("cell").map((c) => c.textContent);
    expect(cells).toEqual(["4", "2", "1", "1", "0", "0", "2"]);
    expect(within(totals).getByRole("columnheader", { name: "Earlier, not logged" })).toBeInTheDocument();
    const byMed = screen.getByRole("table", { name: "By medication" });
    expect(within(byMed).getByRole("rowheader", { name: "As-needed liquid B 10 mg/mL" })).toBeInTheDocument();
    expect(within(screen.getByRole("table", { name: "By day" })).getAllByRole("row")).toHaveLength(5);
    expect(screen.getByText(/1 undone entry and 0 deleted entries/)).toBeInTheDocument();
  });

  it("uses neutral wording only", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    await screen.findByRole("table", { name: "By day" });
    expect(document.body.textContent).not.toMatch(/overdue|missed|adherence|compliance/i);
  });

  it("reloads when the date range changes", async () => {
    const reportRange = vi.fn(fakeApi().reportRange);
    const api: Api = { ...fakeApi(), reportRange };
    render(<ReportsView api={api} version={0} />);
    await screen.findByRole("table", { name: "By day" });
    const from = screen.getByLabelText("From");
    fireEvent.change(from, { target: { value: "2026-01-01" } });
    expect(reportRange).toHaveBeenLastCalledWith("2026-01-01", expect.any(String));
  });

  it("exports with notes off by default, and sends the choices when ticked", async () => {
    const user = userEvent.setup();
    const exportLedger = vi.fn(fakeApi().exportLedger);
    render(<ReportsView api={{ ...fakeApi(), exportLedger }} version={0} />);
    await screen.findByRole("table", { name: "By day" });
    expect(screen.getByRole("checkbox", { name: "Include notes" })).not.toBeChecked();
    expect(screen.getByText(/unencrypted/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Export CSV" }));
    expect(exportLedger).toHaveBeenLastCalledWith(expect.objectContaining({ format: "csv", include_notes: false, include_history: false }));
    await user.click(screen.getByRole("checkbox", { name: "Include notes" }));
    await user.click(screen.getByRole("button", { name: "Export JSON" }));
    expect(exportLedger).toHaveBeenLastCalledWith(expect.objectContaining({ format: "json", include_notes: true }));
    expect(await screen.findByRole("status")).toHaveTextContent("/Downloads/medtracker-export.json");
  });

  it("shows an error from the report", async () => {
    const api: Api = { ...fakeApi(), reportRange: async () => { throw { kind: "validation", message: "Choose a date range of up to 400 days." }; } };
    render(<ReportsView api={api} version={0} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("up to 400 days");
  });

  it("is reachable from More", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("tab", { name: "More" }));
    await user.click(screen.getByRole("button", { name: "Reports" }));
    expect(await screen.findByRole("table", { name: "Totals for the range" })).toBeInTheDocument();
  });
});
