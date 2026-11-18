import * as Dialog from "@radix-ui/react-dialog";
import type { ButtonHTMLAttributes, ReactNode } from "react";

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

export const inputClass =
  "min-h-12 w-full rounded-lg border border-neutral-400 bg-white px-3 text-base";

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-medium text-neutral-800">{label}</span>
      {children}
    </label>
  );
}

export function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40" />
        <Dialog.Content className="fixed inset-x-3 top-6 mx-auto max-h-[90vh] max-w-md overflow-y-auto rounded-xl bg-white p-5 shadow-lg">
          <Dialog.Title className="text-xl font-semibold">{title}</Dialog.Title>
          <Dialog.Description className="mb-4 text-sm text-neutral-700">{description}</Dialog.Description>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
