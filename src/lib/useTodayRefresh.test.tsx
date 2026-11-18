import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TodayView } from "../components/TodayView";
import { type Api, type TodayItem } from "./api";
import { fakeApi } from "../test/fakeApi";
import { msUntilLocalMidnight } from "./useTodayRefresh";

function setup(items: Partial<TodayItem>[] = [{}]) {
  const base: TodayItem = {
    occurrence_id: "o1", schedule_id: "sch1", medication_id: "m1", name: "Metformin", strength: "500 mg", form: "tablet",
    mode: "home_zone", scheduled_at_utc_us: new Date(2026, 2, 10, 8, 0).getTime() * 1000, local_date: "2026-03-10", local_time: "08:00",
    zone_id: null, zone_abbreviation: null, offset_seconds: 0, adjusted: false, device_time: null, status: "upcoming", event_ids: [],
  };
  const listToday = vi.fn(async () => items.map((i) => ({ ...base, ...i })));
  const api: Api = { ...fakeApi(), listToday };
  render(<TodayView api={api} version={0} onChanged={() => {}} />);
  return { api, listToday };
}

const advance = (ms: number) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });

describe("Today stays fresh", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("computes the time to local midnight", () => {
    expect(msUntilLocalMidnight(new Date(2026, 2, 10, 23, 59, 30))).toBe(30_000);
    expect(msUntilLocalMidnight(new Date(2026, 2, 10, 0, 0, 0))).toBe(24 * 3600_000);
  });

  it("reloads at local midnight, then again the next midnight", async () => {
    vi.setSystemTime(new Date(2026, 2, 10, 23, 59, 30));
    const { listToday } = setup([{ status: "taken", local_time: "08:00" }]);
    await advance(0);
    expect(listToday).toHaveBeenCalledTimes(1);

    await advance(29_000);
    expect(listToday).toHaveBeenCalledTimes(1); // not yet midnight
    await advance(2_000);
    expect(listToday).toHaveBeenCalledTimes(2); // crossed midnight

    await advance(24 * 3600_000);
    expect(listToday).toHaveBeenCalledTimes(3); // re-armed for the following midnight
  });

  it("reloads when the next upcoming item becomes due", async () => {
    vi.setSystemTime(new Date(2026, 2, 10, 7, 58, 0));
    const { listToday } = setup([{ status: "upcoming", local_time: "08:00" }]);
    await advance(0);
    expect(screen.getByText("Later today")).toBeInTheDocument();
    expect(listToday).toHaveBeenCalledTimes(1);

    await advance(110_000); // 07:59:50
    expect(listToday).toHaveBeenCalledTimes(1);
    await advance(11_000); // past 08:00:00
    expect(listToday).toHaveBeenCalledTimes(2);
  });

  it("reloads when the window becomes visible again or regains focus", async () => {
    vi.setSystemTime(new Date(2026, 2, 10, 12, 0, 0));
    const { listToday } = setup([{ status: "taken" }]);
    await advance(0);
    expect(listToday).toHaveBeenCalledTimes(1);

    await act(async () => { document.dispatchEvent(new Event("visibilitychange")); });
    expect(listToday).toHaveBeenCalledTimes(2);
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(listToday).toHaveBeenCalledTimes(3);

    // Hidden is not a resume.
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    await act(async () => { document.dispatchEvent(new Event("visibilitychange")); });
    expect(listToday).toHaveBeenCalledTimes(3);
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
  });

  it("refreshes Today and explains when the app reports the day has changed", async () => {
    vi.setSystemTime(new Date(2026, 2, 10, 23, 58, 0));
    const base = fakeApi();
    const listToday = vi.fn(async () => [
      { occurrence_id: "o1", schedule_id: "sch1", medication_id: "m1", name: "Metformin", strength: null, form: null, mode: "home_zone" as const, scheduled_at_utc_us: 0, local_date: "2026-03-10", local_time: "08:00", zone_id: null, zone_abbreviation: null, offset_seconds: 0, adjusted: false, device_time: null, status: "due" as const, event_ids: [] },
    ]);
    const recordDose = vi.fn(async () => {
      throw { kind: "stale_date", message: "The day has changed, so Today was out of date." };
    });
    render(<TodayView api={{ ...base, listToday, recordDose }} version={0} onChanged={() => {}} />);
    await advance(0);
    expect(listToday).toHaveBeenCalledTimes(1);
    await act(async () => { screen.getByRole("button", { name: /Taken/ }).click(); });
    await advance(0);
    expect(recordDose).toHaveBeenCalledTimes(1);
    expect(listToday).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("status")).toHaveTextContent("The day has changed");
  });
});
