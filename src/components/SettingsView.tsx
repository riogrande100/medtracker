import { useId } from "react";
import {
  ACCENTS,
  ACCENT_ORDER,
  TEXT_SIZES,
  TEXT_SIZE_ORDER,
  type Accent,
  type MotionPref,
  type TextSize,
  type UiPrefs,
} from "../lib/uiPrefs";

const option =
  "flex min-h-12 cursor-pointer items-center gap-3 rounded-lg border border-neutral-400 bg-white px-3 text-base has-[:checked]:border-brand has-[:checked]:bg-brand-soft";
const radio = "h-5 w-5 shrink-0 accent-brand";

const MOTION: { value: MotionPref; label: string }[] = [
  { value: "system", label: "Follow my device setting" },
  { value: "reduce", label: "Reduce motion" },
  { value: "allow", label: "Allow motion" },
];

export function SettingsView({
  prefs,
  onChange,
  saved,
}: {
  prefs: UiPrefs;
  onChange: (next: UiPrefs) => void;
  /** False when this device refused to store the choice; it then lasts only until MedTracker closes. */
  saved: boolean;
}) {
  const id = useId();
  return (
    <div className="space-y-8">
      <h2 className="text-2xl font-semibold">Settings</h2>
      <p className="text-sm text-neutral-700">
        These are display choices only. They are saved on this device and contain no health information.
      </p>
      {!saved && (
        <p role="status" className="rounded-lg border border-neutral-400 bg-white p-3 text-sm">
          This device wouldn't save your choice. It will stay in place until you close MedTracker.
        </p>
      )}

      <fieldset className="space-y-3">
        <legend className="text-lg font-semibold">Accent color</legend>
        <p className="text-sm text-neutral-700">
          Sets the main buttons, focus outlines, selected choices and success messages. Cancel and Delete keep their own styling.
        </p>
        <div className="grid grid-cols-1 gap-2 min-[400px]:grid-cols-2">
          {ACCENT_ORDER.map((key: Accent) => (
            <label key={key} className={option}>
              <input
                type="radio"
                name={`${id}-accent`}
                value={key}
                checked={prefs.accent === key}
                onChange={() => onChange({ ...prefs, accent: key })}
                className={radio}
              />
              <span aria-hidden="true" className="h-5 w-5 shrink-0 rounded-full border border-neutral-400" style={{ backgroundColor: ACCENTS[key].base }} />
              <span>{ACCENTS[key].label}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <fieldset className="space-y-3">
        <legend className="text-lg font-semibold">Text size</legend>
        <div className="space-y-2">
          {TEXT_SIZE_ORDER.map((key: TextSize) => (
            <label key={key} className={option}>
              <input
                type="radio"
                name={`${id}-size`}
                value={key}
                checked={prefs.textSize === key}
                onChange={() => onChange({ ...prefs, textSize: key })}
                className={radio}
              />
              <span>{TEXT_SIZES[key].label}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <fieldset className="space-y-3">
        <legend className="text-lg font-semibold">Motion</legend>
        <div className="space-y-2">
          {MOTION.map((m) => (
            <label key={m.value} className={option}>
              <input
                type="radio"
                name={`${id}-motion`}
                value={m.value}
                checked={prefs.motion === m.value}
                onChange={() => onChange({ ...prefs, motion: m.value })}
                className={radio}
              />
              <span>{m.label}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <section aria-labelledby={`${id}-privacy`} className="space-y-3 rounded-xl border border-neutral-300 bg-white p-4">
        <h3 id={`${id}-privacy`} className="text-lg font-semibold">About and privacy</h3>
        <ul className="list-disc space-y-2 pl-5 text-sm">
          <li>Your data stays on this device.</li>
          <li>MedTracker has no accounts. You don't create or sign in to anything.</li>
          <li>MedTracker has no analytics.</li>
          <li>The network allowlist is currently empty.</li>
        </ul>
      </section>
    </div>
  );
}
