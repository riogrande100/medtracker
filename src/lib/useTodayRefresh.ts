import { useEffect, useState } from "react";
import type { TodayItem } from "./api";

/** Milliseconds from `now` until the next local midnight (DST-safe: uses the calendar). */
export function msUntilLocalMidnight(now: Date): number {
  return new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() - now.getTime();
}

/** Milliseconds until a scheduled instant (negative/zero when already past). */
function msUntilInstant(nowMs: number, utcUs: number): number {
  return utcUs / 1000 - nowMs;
}

const MAX_TIMEOUT = 2 ** 31 - 1;

/**
 * Keeps Today from going stale. Calls `refresh` when the window becomes visible or focused
 * (resume from sleep / app switch), at the next local midnight, and when the next upcoming item
 * becomes due. The Rust side independently rejects slot-linked requests for a past day.
 */
export function useTodayRefresh(items: TodayItem[] | null, refresh: () => unknown) {
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") refresh();
    };
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", refresh);
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", refresh);
    };
  }, [refresh]);

  useEffect(() => {
    const now = new Date();
    let next = msUntilLocalMidnight(now);
    for (const i of items ?? []) {
      if (i.status !== "upcoming") continue;
      const d = msUntilInstant(now.getTime(), i.scheduled_at_utc_us);
      if (d > 0) next = Math.min(next, d);
    }
    // A small cushion so the clock has definitely crossed the boundary when we re-query.
    const timer = setTimeout(() => {
      refresh();
      setTick((t) => t + 1);
    }, Math.min(next + 250, MAX_TIMEOUT));
    return () => clearTimeout(timer);
  }, [items, refresh, tick]);
}
