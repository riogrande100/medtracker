import { describe, expect, it, vi } from "vitest";
import {
  ACCENTS,
  ACCENT_ORDER,
  DEFAULT_PREFS,
  STORAGE_KEY,
  TEXT_SIZES,
  applyUiPrefs,
  contrastRatio,
  loadUiPrefs,
  saveUiPrefs,
} from "./uiPrefs";

function memoryStorage(initial: Record<string, string> = {}): Storage {
  const data = new Map(Object.entries(initial));
  return {
    get length() { return data.size; },
    clear: () => data.clear(),
    key: (i: number) => [...data.keys()][i] ?? null,
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => { data.set(k, String(v)); },
    removeItem: (k: string) => { data.delete(k); },
  } as Storage;
}

const throwingStorage = (): Storage => ({
  getItem: () => { throw new DOMException("denied", "SecurityError"); },
  setItem: () => { throw new DOMException("full", "QuotaExceededError"); },
  removeItem: () => { throw new DOMException("denied", "SecurityError"); },
  clear: () => { throw new DOMException("denied", "SecurityError"); },
  key: () => null,
  length: 0,
} as unknown as Storage);

describe("accent contrast (WCAG AA, 4.5:1 for text, light mode)", () => {
  it("computes known reference ratios", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#ffffff", "#ffffff")).toBeCloseTo(1, 5);
  });

  it.each(ACCENT_ORDER)("%s: white button text on the accent passes AA", (accent) => {
    expect(contrastRatio("#ffffff", ACCENTS[accent].base)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(ACCENT_ORDER)("%s: white text on the hover fill passes AA", (accent) => {
    expect(contrastRatio("#ffffff", ACCENTS[accent].hover)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(ACCENT_ORDER)("%s: accent-coloured text on the soft tint passes AA", (accent) => {
    expect(contrastRatio(ACCENTS[accent].ink, ACCENTS[accent].soft)).toBeGreaterThanOrEqual(4.5);
  });
});

describe("preference storage", () => {
  it("defaults when nothing is stored", () => {
    expect(loadUiPrefs(memoryStorage())).toEqual(DEFAULT_PREFS);
  });

  it("round-trips a saved choice", () => {
    const s = memoryStorage();
    expect(saveUiPrefs({ accent: "orange", textSize: "large", motion: "reduce" }, s)).toBe(true);
    expect(loadUiPrefs(s)).toEqual({ accent: "orange", textSize: "large", motion: "reduce" });
  });

  it("ignores corrupt JSON and unknown values field by field", () => {
    expect(loadUiPrefs(memoryStorage({ [STORAGE_KEY]: "{not json" }))).toEqual(DEFAULT_PREFS);
    expect(loadUiPrefs(memoryStorage({ [STORAGE_KEY]: "null" }))).toEqual(DEFAULT_PREFS);
    expect(loadUiPrefs(memoryStorage({ [STORAGE_KEY]: JSON.stringify({ accent: "pink", textSize: "huge", motion: "green" }) })))
      .toEqual(DEFAULT_PREFS);
    expect(loadUiPrefs(memoryStorage({ [STORAGE_KEY]: JSON.stringify({ accent: "red" }) }))).toEqual({ ...DEFAULT_PREFS, accent: "red" });
  });

  it("falls back to defaults and reports no save when storage throws", () => {
    expect(loadUiPrefs(throwingStorage())).toEqual(DEFAULT_PREFS);
    expect(saveUiPrefs({ ...DEFAULT_PREFS, accent: "green" }, throwingStorage())).toBe(false);
    expect(saveUiPrefs(DEFAULT_PREFS, null)).toBe(false);
  });
});

describe("applyUiPrefs", () => {
  it("sets accent variables, root font size and the motion flag", () => {
    const root = document.createElement("html");
    applyUiPrefs({ accent: "purple", textSize: "xlarge", motion: "allow" }, root);
    expect(root.style.getPropertyValue("--accent")).toBe(ACCENTS.purple.base);
    expect(root.style.getPropertyValue("--accent-soft")).toBe(ACCENTS.purple.soft);
    expect(root.style.fontSize).toBe(`${TEXT_SIZES.xlarge.percent}%`);
    expect(root.dataset.motion).toBe("allow");
  });

  it("does not touch storage", () => {
    const spy = vi.spyOn(Storage.prototype, "setItem");
    applyUiPrefs(DEFAULT_PREFS, document.createElement("html"));
    expect(spy).not.toHaveBeenCalled();
    spy.mockRestore();
  });
});
