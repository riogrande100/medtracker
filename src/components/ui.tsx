import * as Dialog from "@radix-ui/react-dialog";
import { useEffect, type ButtonHTMLAttributes, type KeyboardEvent, type ReactNode } from "react";
import { trackVisualViewport } from "../lib/visualViewport";

const base =
  "min-h-12 min-w-12 rounded-lg px-4 text-base font-medium disabled:opacity-50 cursor-pointer";

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "danger" }) {
  const styles = {
    // Primary follows the accent. Secondary (Cancel) and danger (Delete) keep fixed neutral/red styling.
    primary: "bg-brand text-white hover:bg-brand-hover",
    secondary: "bg-white text-neutral-900 border border-neutral-400 hover:bg-neutral-100",
    danger: "bg-white text-red-800 border border-red-700 hover:bg-red-50",
  }[variant];
  return <button type="button" className={`${base} ${styles} ${className}`} {...props} />;
}

/** Every text-like input, select and time/date control. min-w-0 stops iOS intrinsic widths from overflowing the dialog. */
export const inputClass =
  "min-h-12 w-full min-w-0 max-w-full box-border rounded-lg border border-neutral-400 bg-white px-3 text-base";

const NON_TEXT_INPUTS = new Set(["checkbox", "radio", "button", "submit", "reset", "file", "range", "color", "hidden"]);

/**
 * Enter in a text-like field moves to the next field; Enter in the last one submits the dialog.
 * Checkboxes, radios and buttons keep their native Enter/Space behaviour.
 */
function advanceOnEnter(e: KeyboardEvent<HTMLFormElement>) {
  const el = e.target;
  if (e.key !== "Enter" || e.shiftKey || e.altKey || e.ctrlKey || e.metaKey || e.nativeEvent.isComposing) return;
  if (!(el instanceof HTMLInputElement) || NON_TEXT_INPUTS.has(el.type)) return;
  e.preventDefault();
  const form = e.currentTarget;
  const fields = Array.from(form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input, select, textarea")).filter(
    (f) => !f.disabled && !(f instanceof HTMLInputElement && NON_TEXT_INPUTS.has(f.type)),
  );
  const next = fields[fields.indexOf(el) + 1];
  if (next) next.focus();
  else form.requestSubmit();
}

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-medium text-neutral-800">{label}</span>
      {children}
    </label>
  );
}

/**
 * Dialog shell. Title stays at the top; the body scrolls; the footer (Save/Cancel) is a solid, sticky row
 * outside the scrolling region. Body and footer share one <form>, so Enter submits and the keyboard never
 * hides the actions. Content is capped to the visual viewport (see `.dialog-content` in index.css).
 */
export function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title: string;
  description: string;
  children: ReactNode;
  footer?: ReactNode;
  onSubmit?: () => void | Promise<void>;
}) {
  useEffect(() => {
    if (!open) return;
    return trackVisualViewport();
  }, [open]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40" />
        <Dialog.Content className="dialog-content fixed inset-x-3 mx-auto flex max-w-md flex-col overflow-hidden rounded-xl bg-white shadow-lg">
          <div className="shrink-0 px-5 pt-5">
            <Dialog.Title className="text-xl font-semibold">{title}</Dialog.Title>
            <Dialog.Description className="mb-4 text-sm text-neutral-700">{description}</Dialog.Description>
          </div>
          <form
            className="flex min-h-0 flex-1 flex-col"
            onKeyDown={advanceOnEnter}
            onSubmit={(e) => {
              e.preventDefault();
              void onSubmit?.();
            }}
          >
            <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 pb-4">{children}</div>
            {footer && (
              <div className="sticky bottom-0 shrink-0 border-t border-neutral-200 bg-white px-5 pt-3 pb-[max(1rem,env(safe-area-inset-bottom))]">
                <div className="flex gap-2">{footer}</div>
              </div>
            )}
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
