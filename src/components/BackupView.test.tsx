import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { App } from "../App";
import type { Api, BackupFile } from "../lib/api";
import { fakeApi } from "../test/fakeApi";
import { BackupView } from "./BackupView";

const file = (over: Partial<BackupFile> = {}): BackupFile => ({
  name: "MedTracker-backup-2026-03-10-0930.medbackup",
  path: "/Documents/MedTracker Backups/MedTracker-backup-2026-03-10-0930.medbackup",
  size_bytes: 8192,
  modified_utc_us: 1_773_133_800_000_000,
  ...over,
});

function setup(over: Partial<Api> = {}, backups: BackupFile[] = [file()]) {
  const onChanged = vi.fn();
  const api = { ...fakeApi({ backups }), ...over };
  render(<BackupView api={api} onChanged={onChanged} />);
  return { api, onChanged };
}

describe("Create backup", () => {
  it("says plainly that a forgotten passphrase cannot be recovered and that nothing goes online", async () => {
    setup();
    expect(await screen.findByText(/cannot be opened/)).toHaveTextContent(/nobody, including us, can recover it/);
    expect(screen.getByText(/never goes online/)).toBeInTheDocument();
  });

  it("needs a long enough passphrase typed twice before it will create", async () => {
    const user = userEvent.setup();
    const createBackup = vi.fn(async () => ({ path: "/p/b.medbackup", file_name: "b.medbackup", created_at_utc_us: 1, medications: 1, doses: 2, schedules: 0, size_bytes: 2048 }));
    setup({ createBackup });
    const create = await screen.findByRole("button", { name: "Create backup" });
    expect(create).toBeDisabled();

    await user.type(screen.getByLabelText("Passphrase"), "short");
    expect(screen.getByRole("alert")).toHaveTextContent("at least 8 characters");
    await user.clear(screen.getByLabelText("Passphrase"));
    await user.type(screen.getByLabelText("Passphrase"), "long enough pass");
    await user.type(screen.getByLabelText("Type the passphrase again"), "long enough pasx");
    expect(screen.getByRole("alert")).toHaveTextContent("don't match");
    expect(create).toBeDisabled();

    await user.clear(screen.getByLabelText("Type the passphrase again"));
    await user.type(screen.getByLabelText("Type the passphrase again"), "long enough pass");
    expect(create).toBeEnabled();
    await user.click(create);
    expect(createBackup).toHaveBeenCalledWith("long enough pass");
    expect(await screen.findByRole("status")).toHaveTextContent("Backup saved and checked.");
    expect(screen.getByRole("status")).toHaveTextContent("/p/b.medbackup");
    // The passphrase is cleared once used.
    expect(screen.getByLabelText("Passphrase")).toHaveValue("");
  });

  it("masks the passphrase unless asked to show it", async () => {
    const user = userEvent.setup();
    setup();
    const pass = await screen.findByLabelText("Passphrase");
    expect(pass).toHaveAttribute("type", "password");
    await user.click(screen.getByRole("checkbox", { name: "Show passphrase" }));
    expect(pass).toHaveAttribute("type", "text");
  });

  it("shows the reason when creating fails", async () => {
    const user = userEvent.setup();
    setup({ createBackup: async () => Promise.reject({ kind: "storage", message: "Could not write the backup file (is the disk full?)." }) });
    await user.type(await screen.findByLabelText("Passphrase"), "long enough pass");
    await user.type(screen.getByLabelText("Type the passphrase again"), "long enough pass");
    await user.click(screen.getByRole("button", { name: "Create backup" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("is the disk full");
  });
});

describe("Restore", () => {
  it("lists backups found, asks for confirmation, and only then restores", async () => {
    const user = userEvent.setup();
    const restoreBackup = vi.fn(async () => ({
      backup_created_at_utc_us: 1_773_133_800_000_000, medications: 3, doses: 40, schedules: 2, upgraded_from: 1, safety_copy_kept: true,
    }));
    const { onChanged } = setup({ restoreBackup });
    const restore = await screen.findByRole("button", { name: "Restore this backup" });
    expect(restore).toBeDisabled();

    await user.click(screen.getByRole("radio", { name: /MedTracker-backup-2026-03-10-0930/ }));
    await user.type(screen.getByLabelText("Backup passphrase"), "my pass phrase");
    expect(restore).toBeEnabled();

    await user.click(restore);
    const dialog = await screen.findByRole("alertdialog");
    expect(dialog).toHaveTextContent("Replace your data with this backup?");
    expect(dialog).toHaveTextContent("until it next starts");
    expect(restoreBackup).not.toHaveBeenCalled();

    await user.click(within(dialog).getByRole("button", { name: "Keep my data" }));
    expect(restoreBackup).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Restore this backup" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Replace my data" }));
    expect(restoreBackup).toHaveBeenCalledWith(file().path, "my pass phrase");
    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("Restored from the backup made");
    expect(status).toHaveTextContent("3 medications, 40 entries");
    expect(status).toHaveTextContent("older version");
    expect(onChanged).toHaveBeenCalled();
    expect(screen.getByLabelText("Backup passphrase")).toHaveValue("");
  });

  it("prefers a typed path over the selected file", async () => {
    const user = userEvent.setup();
    const restoreBackup = vi.fn(async () => ({ backup_created_at_utc_us: 1, medications: 0, doses: 0, schedules: 0, upgraded_from: null, safety_copy_kept: true }));
    setup({ restoreBackup });
    await user.click(await screen.findByRole("radio"));
    await user.type(screen.getByLabelText(/type the full path/i), "  /usb/mine.medbackup ");
    await user.type(screen.getByLabelText("Backup passphrase"), "x");
    await user.click(screen.getByRole("button", { name: "Restore this backup" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Replace my data" }));
    expect(restoreBackup).toHaveBeenCalledWith("/usb/mine.medbackup", "x");
  });

  it.each([
    ["backup_wrong_passphrase", "That passphrase does not open this backup."],
    ["backup_truncated", "This backup file is incomplete."],
    ["backup_corrupt", "The backup was changed or damaged."],
    ["backup_newer_schema", "This backup needs a newer MedTracker."],
  ])("shows the %s error and says the current data was not changed", async (kind, message) => {
    const user = userEvent.setup();
    setup({ restoreBackup: async () => Promise.reject({ kind, message }) });
    await user.click(await screen.findByRole("radio"));
    await user.type(screen.getByLabelText("Backup passphrase"), "nope");
    await user.click(screen.getByRole("button", { name: "Restore this backup" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Replace my data" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(message);
    expect(alert).toHaveTextContent("Your current data was not changed.");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("explains an empty backup folder and still lets the user type a path", async () => {
    setup({}, []);
    expect(await screen.findByText("No backups found there yet.")).toBeInTheDocument();
    expect(screen.getByLabelText(/type the full path/i)).toBeInTheDocument();
  });
});

describe("Backup tab", () => {
  it("is reachable from the main navigation", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("tab", { name: "Backup" }));
    expect(await screen.findByRole("heading", { name: "Create a backup" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Restore from a backup" })).toBeInTheDocument();
  });
});
