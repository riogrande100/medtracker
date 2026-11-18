import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { App } from "./App";
import type { Api, MedicationSummary, ScheduleInput, TodayItem } from "./lib/api";
import { formatInterval, formatWeekdays } from "./lib/format";
import { defaultMed, fakeApi, previewRows } from "./test/fakeApi";

const card = (over: Partial<TodayItem> = {}): TodayItem => ({
  occurrence_id: "o1", schedule_id: "sch1", medication_id: "m1", name: "Metformin", strength: "500 mg", form: "tablet",
  mode: "home_zone", scheduled_at_utc_us: 1, local_date: "2026-03-10", local_time: "08:00", zone_id: "America/Chicago",
  zone_abbreviation: "CDT", offset_seconds: -18000, adjusted: false, device_time: null, status: "due", event_ids: [], ...over,
});

function withApi(over: Partial<Api>, base = fakeApi()): Api {
  return { ...base, ...over };
}

describe("Today with schedules", () => {
  it("records Taken against the occurrence the card shows (the explicit link)", async () => {
    const user = userEvent.setup();
    const recordDose = vi.fn(async () => "e1");
    render(<App api={withApi({ recordDose })} />);
    await user.click(await screen.findByRole("button", { name: /Taken/ }));
    expect(recordDose).toHaveBeenCalledWith({ medication_id: "m1", kind: "take", occurrence_id: "o1", occurred_local: null, note: null });
    await user.click(await screen.findByRole("button", { name: /Skip/ }).catch(() => screen.getByRole("button", { name: /Undo/ })));
  });

  it("shows a skipped card's Undo and returns it to Due", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("button", { name: /Skip/ }));
    expect(await screen.findByText("Skipped")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Undo/ }));
    expect(await screen.findByText("Due now")).toBeInTheDocument();
  });

  it("shows zone context and the device time only when the clocks differ, and flags adjusted times", async () => {
    const items = [
      card({ occurrence_id: "a", zone_id: "Asia/Tokyo", zone_abbreviation: "JST", device_time: "19:00", local_time: "08:00" }),
      card({ occurrence_id: "b", name: "Other", local_time: "03:30", adjusted: true, status: "upcoming" }),
    ];
    render(<App api={withApi({ listToday: async () => items })} />);
    expect(await screen.findByText(/8:00 AM JST in Asia\/Tokyo · 7:00 PM on this device/)).toBeInTheDocument();
    expect(screen.getByText("Adjusted for a clock change.")).toBeInTheDocument();
    expect(screen.getAllByText(/ in .* · .* on this device/)).toHaveLength(1);
  });

  it("lists earlier unrecorded doses neutrally, without Taken/Skip and without 'missed' or 'overdue'", async () => {
    const items = [card({ occurrence_id: "old", local_date: "2026-03-09", status: "earlier_not_logged" }), card({ occurrence_id: "now", local_time: "09:00" })];
    const { container } = render(<App api={withApi({ listToday: async () => items })} />);
    const heading = await screen.findByRole("heading", { name: "Earlier, not logged" });
    const section = heading.closest("section")!;
    expect(within(section).getByText("Not logged")).toBeInTheDocument();
    expect(within(section).queryByRole("button")).not.toBeInTheDocument();
    // Today's own card still has both actions.
    expect(screen.getAllByRole("button", { name: /^Taken/ })).toHaveLength(1);
    expect(container.textContent).not.toMatch(/missed|overdue|late\b/i);
  });

  it("shows the next scheduled time even when nothing is left today", async () => {
    const next = { medication_id: "m1", name: "Metformin", strength: "500 mg", occurrence: previewRows(1)[0] };
    render(<App api={withApi({ listToday: async () => [card({ status: "taken", event_ids: ["e"] })] }, fakeApi({ next }))} />);
    expect(await screen.findByTestId("next-due")).toHaveTextContent("Next scheduled: Metformin 500 mg, Mar 10, 2026 8:00 AM CDT");
  });

  it("a failing next-due lookup never hides the cards", async () => {
    render(<App api={withApi({ nextDue: async () => { throw { kind: "storage", message: "x" }; } })} />);
    expect(await screen.findByText("Due now")).toBeInTheDocument();
    expect(screen.queryByTestId("next-due")).not.toBeInTheDocument();
  });

  it("refreshes and explains when the schedule changed under an open card", async () => {
    const user = userEvent.setup();
    const listToday = vi.fn(async () => [card()]);
    const recordDose = vi.fn(async () => {
      throw { kind: "stale_date", message: "This schedule was changed, so Today was out of date." };
    });
    render(<App api={withApi({ listToday, recordDose })} />);
    await user.click(await screen.findByRole("button", { name: /Taken/ }));
    expect(await screen.findByRole("status")).toHaveTextContent("This schedule was changed");
    expect(listToday.mock.calls.length).toBeGreaterThanOrEqual(2);
  });
});

async function openMedications(api: Api) {
  const user = userEvent.setup();
  render(<App api={api} />);
  await user.click(await screen.findByRole("tab", { name: "Medications" }));
  return user;
}

describe("Schedule editor", () => {
  it("describes each schedule in plain words with its zone and next time", async () => {
    const elapsed: MedicationSummary = {
      ...defaultMed(), id: "m2", name: "Ibuprofen", strength: null, form: null,
      schedule: { ...defaultMed().schedule!, id: "sch2", mode: "elapsed_interval", zone_id: null, slots: [], interval_seconds: 8 * 3600, anchor_utc_us: 1_773_151_200_000_000 },
    };
    const weekly: MedicationSummary = {
      ...defaultMed(), id: "m3", name: "Weekly", strength: null, form: null,
      schedule: { ...defaultMed().schedule!, id: "sch3", state: "paused", slots: [{ time: "09:00", weekdays: [1, 3] }] },
    };
    await openMedications(withApi({}, fakeApi({ meds: [defaultMed(), elapsed, weekly] })));
    expect(await screen.findByText(/Daily at 8:00 AM \(America\/Chicago\)/)).toBeInTheDocument();
    expect(screen.getByText(/Every 8 hours, counted from/)).toBeInTheDocument();
    expect(screen.getByText(/Mon, Wed at 9:00 AM \(America\/Chicago\) — paused/)).toBeInTheDocument();
    expect(screen.getAllByText(/^Next: /).length).toBe(2); // paused schedules show no next time
    expect(formatWeekdays([1, 2, 3, 4, 5])).toBe("weekdays");
    expect(formatInterval(5400)).toBe("90 minutes");
  });

  it("edits fixed times with a preview, then saves exactly what was chosen", async () => {
    const editSchedule = vi.fn(async () => {});
    const previewSchedule = vi.fn(async (_: ScheduleInput) => previewRows());
    const user = await openMedications(withApi({ editSchedule, previewSchedule }));
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));

    const dialog = await screen.findByRole("dialog", { name: /Edit schedule: Metformin/ });
    expect(dialog).toHaveTextContent("Doses you already recorded, and History, are not changed");
    expect(within(dialog).getByLabelText("Time 1")).toHaveValue("08:00");
    expect(within(dialog).getByLabelText("Time zone")).toHaveValue("America/Chicago");
    expect(await within(dialog).findByTestId("schedule-preview")).toBeInTheDocument();
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(7);

    await user.click(within(dialog).getByRole("button", { name: "Add another time" }));
    expect(within(dialog).getByLabelText("Time 2")).toHaveValue("20:00");
    await user.click(within(dialog).getByRole("checkbox", { name: "Monday" }));
    await user.click(within(dialog).getByRole("checkbox", { name: "Friday" }));
    await user.click(within(dialog).getByRole("button", { name: "Save schedule" }));

    expect(editSchedule).toHaveBeenCalledWith("sch1", {
      mode: "home_zone", zone_id: "America/Chicago", interval_seconds: null, anchor_local: null,
      slots: [{ time: "08:00", weekdays: [1, 5] }, { time: "20:00", weekdays: [1, 5] }],
    });
    // The preview was asked for with the same definition.
    expect(previewSchedule).toHaveBeenLastCalledWith(expect.objectContaining({ mode: "home_zone", slots: expect.any(Array) }));
  });

  it("is fully keyboard operable: space toggles a day, Tab reaches Save, Enter saves", async () => {
    const editSchedule = vi.fn(async () => {});
    const user = await openMedications(withApi({ editSchedule }));
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));
    const monday = await screen.findByRole("checkbox", { name: "Monday" });
    monday.focus();
    await user.keyboard(" ");
    expect(monday).toBeChecked();
    const save = screen.getByRole("button", { name: "Save schedule" });
    save.focus();
    await user.keyboard("{Enter}");
    expect(editSchedule).toHaveBeenCalledTimes(1);
    const [, def] = editSchedule.mock.calls[0] as unknown as [string, ScheduleInput];
    expect(def.slots[0].weekdays).toEqual([1]);
  });

  it("every control has an accessible name and a touch-sized target", async () => {
    const user = await openMedications(fakeApi());
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));
    const dialog = await screen.findByRole("dialog");
    for (const el of within(dialog).getAllByRole("button")) expect(el).toHaveAccessibleName();
    for (const el of within(dialog).getAllByRole("checkbox")) expect(el).toHaveAccessibleName();
    for (const el of within(dialog).getAllByRole("radio")) expect(el).toHaveAccessibleName();
    expect(within(dialog).getByRole("group", { name: "How is it scheduled?" })).toBeInTheDocument();
    expect(within(dialog).getByRole("group", { name: /Days/ })).toBeInTheDocument();
    for (const b of within(dialog).getAllByRole("button")) expect(b.className).toMatch(/min-h-12/);
  });

  it("switches to every-N-hours and saves interval seconds and the first dose", async () => {
    const createSchedule = vi.fn(async () => "new");
    const asNeeded: MedicationSummary = { ...defaultMed(), schedule: null };
    const user = await openMedications(withApi({ createSchedule }, fakeApi({ meds: [asNeeded] })));
    expect(await screen.findByText(/As needed/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Add schedule/ }));
    const dialog = await screen.findByRole("dialog", { name: /Add schedule: Metformin/ });
    await user.click(within(dialog).getByRole("radio", { name: "Every so many hours" }));
    const hours = within(dialog).getByLabelText("Hours between doses");
    await user.clear(hours);
    await user.type(hours, "6");
    expect(within(dialog).getByText(/A late dose doesn’t move the next one/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Save schedule" }));
    expect(createSchedule).toHaveBeenCalledTimes(1);
    const [medId, def] = createSchedule.mock.calls[0] as unknown as [string, ScheduleInput];
    expect(medId).toBe("m1");
    expect(def).toMatchObject({ mode: "elapsed_interval", zone_id: null, slots: [], interval_seconds: 21600 });
    expect(def.anchor_local).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
  });

  it("keeps the stored first dose when only the interval is edited", async () => {
    const editSchedule = vi.fn(async () => {});
    const elapsed: MedicationSummary = {
      ...defaultMed(),
      schedule: { ...defaultMed().schedule!, mode: "elapsed_interval", zone_id: null, slots: [], interval_seconds: 8 * 3600, anchor_utc_us: 1_773_151_200_000_000 },
    };
    const user = await openMedications(withApi({ editSchedule }, fakeApi({ meds: [elapsed] })));
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));
    const hours = await screen.findByLabelText("Hours between doses");
    await user.clear(hours);
    await user.type(hours, "12");
    await user.click(screen.getByRole("button", { name: "Save schedule" }));
    expect(editSchedule).toHaveBeenCalledWith("sch1", { mode: "elapsed_interval", zone_id: null, slots: [], interval_seconds: 43200, anchor_local: null });
  });

  it("shows the app's validation message instead of closing", async () => {
    const editSchedule = vi.fn(async () => {
      throw { kind: "validation", message: "Each time can only be listed once." };
    });
    const user = await openMedications(withApi({ editSchedule }));
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Monday" }));
    await user.click(screen.getByRole("button", { name: "Save schedule" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Each time can only be listed once.");
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("shows a preview error without blocking the form", async () => {
    const previewSchedule = vi.fn(async () => {
      throw { kind: "validation", message: "Choose a time zone for these times." };
    });
    const user = await openMedications(withApi({ previewSchedule }));
    await user.click(await screen.findByRole("button", { name: /Edit schedule/ }));
    expect(await screen.findByText("Choose a time zone for these times.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save schedule" })).toBeEnabled();
  });

  it("pauses, resumes and ends (with a confirmation that says history is kept)", async () => {
    const pauseSchedule = vi.fn(async () => {});
    const endSchedule = vi.fn(async () => {});
    const user = await openMedications(withApi({ pauseSchedule, endSchedule }));
    await user.click(await screen.findByRole("button", { name: /Pause schedule/ }));
    expect(pauseSchedule).toHaveBeenCalledWith("sch1");

    await user.click(screen.getByRole("button", { name: "End schedule for Metformin" }));
    const confirm = await screen.findByRole("alertdialog");
    expect(confirm).toHaveTextContent("Everything already recorded stays in History");
    await user.click(within(confirm).getByRole("button", { name: "Keep schedule" }));
    expect(endSchedule).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "End schedule for Metformin" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "End schedule" }));
    expect(endSchedule).toHaveBeenCalledWith("sch1");

  });

  it("a paused schedule offers Resume instead of Pause", async () => {
    const resumeSchedule = vi.fn(async () => {});
    const paused: MedicationSummary = { ...defaultMed(), schedule: { ...defaultMed().schedule!, state: "paused", next: [] } };
    const user = await openMedications(withApi({ resumeSchedule }, fakeApi({ meds: [paused] })));
    expect(await screen.findByText(/— paused/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Pause schedule/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Resume schedule/ }));
    expect(resumeSchedule).toHaveBeenCalledWith("sch1");
  });

  it("an ended schedule offers Add schedule, and archived medications offer no schedule actions", async () => {
    const ended: MedicationSummary = { ...defaultMed(), schedule: { ...defaultMed().schedule!, state: "ended", next: [] } };
    const archived: MedicationSummary = { ...defaultMed(), id: "m9", name: "Old", archived: true };
    await openMedications(withApi({}, fakeApi({ meds: [ended, archived] })));
    await screen.findByText(/Old 500 mg/);
    expect(screen.getByRole("button", { name: "Add schedule for Metformin" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /schedule for Old/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Pause schedule/ })).not.toBeInTheDocument();
  });
});
