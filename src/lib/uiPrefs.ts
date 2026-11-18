/**
 * Display preferences only (accent, text size, motion). Stored on this device in localStorage.
 * Holds no health information. Every storage access is guarded so the app works when storage throws.
 */

export type Accent = "blue" | "green" | "purple" | "orange" | "red";
export type TextSize = "normal" | "large" | "xlarge";
export type MotionPref = "system" | "reduce" | "allow";

export interface UiPrefs {
  accent: Accent;
  textSize: TextSize;
  motion: MotionPref;
}

export const DEFAULT_PREFS: UiPrefs = { accent: "blue", textSize: "normal", motion: "system" };
export const STORAGE_KEY = "medtracker.uiPrefs.v1";

/**
 * Each accent has a 700 base (buttons, focus, selected controls), an 800 hover, and a 50 soft tint.
 * `ink` is the 800 shade used for text on the soft tint. Base is white-text safe (see uiPrefs.test.ts).
 */
export const ACCENTS: Record<Accent, { label: string; base: string; hover: string; soft: string; ink: string }> = {
  blue: { label: "Blue", base: "#1d4ed8", hover: "#1e40af", soft: "#eff6ff", ink: "#1e40af" },
  green: { label: "Green", base: "#15803d", hover: "#166534", soft: "#f0fdf4", ink: "#166534" },
  purple: { label: "Purple", base: "#7e22ce", hover: "#6b21a8", soft: "#faf5ff", ink: "#6b21a8" },
  orange: { label: "Orange", base: "#c2410c", hover: "#9a3412", soft: "#fff7ed", ink: "#9a3412" },
  red: { label: "Red", base: "#b91c1c", hover: "#991b1b", soft: "#fef2f2", ink: "#991b1b" },
};

export const TEXT_SIZES: Record<TextSize, { label: string; percent: number }> = {
  normal: { label: "Normal", percent: 100 },
  large: { label: "Large", percent: 120 },
  xlarge: { label: "Extra large", percent: 140 },
};

export const ACCENT_ORDER = Object.keys(ACCENTS) as Accent[];
export const TEXT_SIZE_ORDER = Object.keys(TEXT_SIZES) as TextSize[];
export const MOTION_ORDER: MotionPref[] = ["system", "reduce", "allow"];

/** Returns localStorage, or null when the browser refuses access (private mode, blocked site data). */
export function readStorage(): Storage | null {
  try {
    return typeof window !== "undefined" && window.localStorage ? window.localStorage : null;
  } catch {
    return null;
  }
}

export function loadUiPrefs(storage: Storage | null = readStorage()): UiPrefs {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_PREFS };
    const v = JSON.parse(raw) as Partial<Record<keyof UiPrefs, unknown>>;
    return {
      accent: ACCENT_ORDER.includes(v.accent as Accent) ? (v.accent as Accent) : DEFAULT_PREFS.accent,
      textSize: TEXT_SIZE_ORDER.includes(v.textSize as TextSize) ? (v.textSize as TextSize) : DEFAULT_PREFS.textSize,
      motion: MOTION_ORDER.includes(v.motion as MotionPref) ? (v.motion as MotionPref) : DEFAULT_PREFS.motion,
    };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

/** Returns true only when the choice was written to storage. */
export function saveUiPrefs(prefs: UiPrefs, storage: Storage | null = readStorage()): boolean {
  try {
    if (!storage) return false;
    storage.setItem(STORAGE_KEY, JSON.stringify(prefs));
    return true;
  } catch {
    return false;
  }
}

/** Writes the accent tokens, root text scale and motion flag. Pure DOM work, never touches storage. */
export function applyUiPrefs(prefs: UiPrefs, root: HTMLElement = document.documentElement): void {
  const a = ACCENTS[prefs.accent];
  root.style.setProperty("--accent", a.base);
  root.style.setProperty("--accent-hover", a.hover);
  root.style.setProperty("--accent-soft", a.soft);
  root.style.setProperty("--accent-ink", a.ink);
  root.style.fontSize = `${TEXT_SIZES[prefs.textSize].percent}%`;
  root.dataset.motion = prefs.motion;
}

/** WCAG 2.x relative luminance of a #rrggbb colour. */
export function luminance(hex: string): number {
  const n = parseInt(hex.replace("#", ""), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio between two #rrggbb colours, from 1 to 21. */
export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
