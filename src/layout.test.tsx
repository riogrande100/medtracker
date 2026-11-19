import { render, screen, within } from "@testing-library/react";
import { App } from "./App";
import { ReportsView } from "./components/ReportsView";
import { SettingsView } from "./components/SettingsView";
import { DEFAULT_PREFS } from "./lib/uiPrefs";
import { fakeApi } from "./test/fakeApi";

const COUNT_LABELS = [
  "Scheduled",
  "Taken",
  "Skipped",
  "Earlier, not logged",
  "Later, not yet logged",
  "Conflicting",
  "Unscheduled entries",
];

const classes = (el: Element | null | undefined) => (el?.className ?? "").toString().split(/\s+/);

describe("Reports date fields", () => {
  it("give both date inputs the text-field width rules and iOS appearance reset", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    await screen.findByRole("region", { name: "By day" });
    for (const label of ["From", "To"]) {
      const input = screen.getByLabelText(label);
      expect(input).toHaveAttribute("type", "date");
      expect(classes(input)).toEqual(expect.arrayContaining(["w-full", "min-w-0", "max-w-full", "box-border", "min-h-12", "appearance-none"]));
    }
  });

  it("stacks the two dates on one column and only uses two columns from 420px", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    const grid = screen.getByLabelText("From").closest("label")!.parentElement!;
    expect(classes(grid)).toEqual(expect.arrayContaining(["grid-cols-1", "min-[420px]:grid-cols-2", "[&>*]:min-w-0"]));
    expect(classes(grid)).not.toContain("grid-cols-2");
  });
});

describe("Reports narrow layout", () => {
  it("renders a stacked card per row with a definition list, and hides the table below 640px", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    const totals = await screen.findByRole("region", { name: "Totals for the range" });

    const cards = totals.querySelector('[data-layout="cards"]')!;
    expect(classes(cards)).toContain("sm:hidden");
    const card = within(cards as HTMLElement).getByRole("heading", { name: "All medications" }).closest("li")!;
    expect(Array.from(card.querySelectorAll("dt")).map((d) => d.textContent)).toEqual(COUNT_LABELS);
    expect(Array.from(card.querySelectorAll("dd")).map((d) => d.textContent)).toEqual(["4", "2", "1", "1", "0", "0", "2"]);

    const tableWrap = totals.querySelector('[data-layout="table"]')!;
    expect(classes(tableWrap)).toEqual(expect.arrayContaining(["hidden", "sm:block"]));
  });

  it("keeps the captioned table for 640px and up, with the same column labels as the cards", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    const table = await screen.findByRole("table", { name: "By day" });
    const headers = within(table).getAllByRole("columnheader").map((h) => h.textContent);
    expect(headers.slice(1)).toEqual(COUNT_LABELS);
    expect(within(table).getByRole("columnheader", { name: "Earlier, not logged" })).toBeInTheDocument();
  });

  it("keeps the definition-list wording exact for Earlier, not logged", async () => {
    render(<ReportsView api={fakeApi()} version={0} />);
    const byMed = await screen.findByRole("region", { name: "By medication" });
    const dts = Array.from(byMed.querySelectorAll("dt")).map((d) => d.textContent);
    expect(dts).toContain("Earlier, not logged");
    expect(dts).toContain("Later, not yet logged");
    expect(within(byMed).getByRole("heading", { name: "As-needed liquid B 10 mg/mL" })).toBeInTheDocument();
  });
});

describe("Extra large text layout", () => {
  it("puts the tab labels in equal columns that may wrap and hyphenate instead of overlapping", async () => {
    render(<App api={fakeApi()} />);
    const list = await screen.findByRole("tablist", { name: "Sections" });
    expect(classes(list)).toContain("grid-cols-4");
    for (const name of ["Today", "History", "Medications", "More"]) {
      const tab = screen.getByRole("tab", { name });
      expect(classes(tab)).toEqual(expect.arrayContaining(["min-h-12", "min-w-0", "break-words", "hyphens-auto"]));
    }
  });

  it("uses a single column for accent options at Extra large text", () => {
    render(<SettingsView prefs={{ ...DEFAULT_PREFS, textSize: "xlarge" }} onChange={() => {}} saved />);
    const grid = screen.getByTestId("accent-grid");
    expect(classes(grid)).toContain("grid-cols-1");
    expect(classes(grid)).not.toContain("min-[400px]:grid-cols-2");
  });

  it("lets long accent names such as Purple and Orange wrap inside their card", () => {
    render(<SettingsView prefs={{ ...DEFAULT_PREFS, textSize: "xlarge" }} onChange={() => {}} saved />);
    for (const name of ["Purple", "Orange"]) {
      const option = screen.getByRole("radio", { name }).closest("label")!;
      expect(classes(option)).toContain("min-w-0");
      expect(classes(screen.getByText(name))).toEqual(expect.arrayContaining(["min-w-0", "break-words"]));
    }
  });
});
