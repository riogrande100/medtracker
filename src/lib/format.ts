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
