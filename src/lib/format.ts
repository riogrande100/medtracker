const pad = (n: number) => String(n).padStart(2, "0");

/** "2026-03-10T09:30" -> "Mar 10, 9:30 AM" (no timezone conversion; text is preserved). */
export function formatLocal(s: string): string {
  const [d, t] = s.split("T");
  const [y, m, day] = d.split("-").map(Number);
  const [h, min] = t.split(":").map(Number);
  const month = new Date(2000, m - 1, 1).toLocaleString("en-US", { month: "short" });
  return `${month} ${day}, ${y} ${formatTime(`${pad(h)}:${pad(min)}`)}`;
}

export function formatTime(hhmm: string): string {
  const [h, m] = hhmm.split(":").map(Number);
  const suffix = h >= 12 ? "PM" : "AM";
  return `${h % 12 || 12}:${pad(m)} ${suffix}`;
}

export function formatOffset(seconds: number): string {
  const sign = seconds < 0 ? "-" : "+";
  const a = Math.abs(seconds);
  return `UTC${sign}${pad(Math.floor(a / 3600))}:${pad(Math.floor((a % 3600) / 60))}`;
}

export function nowLocalInput(d = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function medLabel(name: string, strength: string | null): string {
  return strength ? `${name} ${strength}` : name;
}

export const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;
export const WEEKDAYS_LONG = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"] as const;

/** "Mon, Wed" / "Every day" / "Weekdays". `days` are ISO numbers 1..7. */
export function formatWeekdays(days: number[]): string {
  if (days.length === 0 || days.length === 7) return "every day";
  if (days.length === 5 && [1, 2, 3, 4, 5].every((d) => days.includes(d))) return "weekdays";
  return days.map((d) => WEEKDAYS[d - 1]).join(", ");
}

/** 28800 -> "8 hours"; 5400 -> "90 minutes"; 172800 -> "2 days". */
export function formatInterval(seconds: number): string {
  const unit = (n: number, u: string) => `${n} ${u}${n === 1 ? "" : "s"}`;
  if (seconds % 86400 === 0) return unit(seconds / 86400, "day");
  if (seconds % 3600 === 0) return unit(seconds / 3600, "hour");
  if (seconds % 60 === 0) return unit(seconds / 60, "minute");
  return unit(seconds, "second");
}

/** The device's IANA zone id, or null where the platform can't say. */
export function deviceZone(): string | null {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || null;
  } catch {
    return null;
  }
}

/** IANA zone ids to choose from, offline (from the platform), always including `extra`. */
export function zoneChoices(...extra: (string | null | undefined)[]): string[] {
  let all: string[] = [];
  try {
    all = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf?.("timeZone") ?? [];
  } catch {
    all = [];
  }
  const set = new Set([...all, ...extra.filter((z): z is string => !!z)]);
  if (set.size === 0) set.add("UTC");
  return [...set].sort();
}

/** "Mar 10, 2026 8:00 AM EDT" from a preview/next-due row. */
export function formatOccurrence(o: { local_date: string; local_time: string; zone_abbreviation: string | null; offset_seconds: number; adjusted: boolean }): string {
  const zone = o.zone_abbreviation ?? formatOffset(o.offset_seconds);
  return `${formatLocal(`${o.local_date}T${o.local_time}`)} ${zone}${o.adjusted ? " (adjusted for a clock change)" : ""}`;
}
