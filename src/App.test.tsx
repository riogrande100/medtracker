import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { fakeApi } from "./test/fakeApi";

describe("Today flow", () => {
  it("shows a due medication, records Taken, and undoes it", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    expect(await screen.findByText("Due now")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Taken/ }));
    expect(await screen.findByText("Taken")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("marked as taken");

    await user.click(screen.getByRole("button", { name: /Undo/ }));
    expect(await screen.findByText("Due now")).toBeInTheDocument();
  });

  it("records Skip as a distinct outcome from Taken", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("button", { name: /Skip/ }));
    expect(await screen.findByText("Skipped")).toBeInTheDocument();
    expect(screen.queryByText("Taken")).not.toBeInTheDocument();
  });
});

describe("History", () => {
  it("shows corrections with original time context and change history", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("button", { name: /Taken/ }));
    await user.click(screen.getByRole("tab", { name: "History" }));
    expect(await screen.findByText(/Originally entered as/)).toHaveTextContent("UTC-06:00");

    await user.click(screen.getByRole("button", { name: /Correct/ }));
    await user.selectOptions(screen.getByLabelText("What happened"), "skip");
    await user.click(screen.getByRole("button", { name: "Save correction" }));

    expect(await screen.findByText(/Metformin 500 mg — Skipped/)).toBeInTheDocument();
    expect(screen.getByText("Change history (1)")).toBeInTheDocument();
  });

  it("keeps undone entries visible and labelled", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("button", { name: /Taken/ }));
    await user.click(screen.getByRole("button", { name: /Undo/ }));
    await user.click(screen.getByRole("tab", { name: "History" }));
    expect(await screen.findByText(/— Undone/)).toBeInTheDocument();
  });
});

describe("Undo vs permanent deletion", () => {
  async function takeAndOpenHistory() {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("button", { name: /Taken/ }));
    await user.click(screen.getByRole("tab", { name: "History" }));
    await screen.findByText(/Metformin 500 mg — Taken/);
    return user;
  }

  it("undone entries can be restored", async () => {
    const user = await takeAndOpenHistory();
    await user.click(screen.getByRole("button", { name: /^Undo/ }));
    expect(await screen.findByText(/— Undone/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^Restore/ }));
    expect(await screen.findByText(/Metformin 500 mg — Taken/)).toBeInTheDocument();
  });

  it("deletion is a separate action behind a confirmation with different wording", async () => {
    const user = await takeAndOpenHistory();
    expect(screen.getByText("Note: felt fine")).toBeInTheDocument();
    // Undo has no confirmation; permanent deletion asks first and can be cancelled.
    await user.click(screen.getByRole("button", { name: /Delete permanently/ }));
    const dialog = await screen.findByRole("alertdialog");
    expect(dialog).toHaveTextContent("This is not Undo");
    expect(dialog).toHaveTextContent("can't be reversed");
    await user.click(screen.getByRole("button", { name: "Keep it" }));
    expect(screen.getByText(/Metformin 500 mg — Taken/)).toBeInTheDocument();
    expect(screen.getByText("Note: felt fine")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Delete permanently/ }));
    await user.click(await screen.findByRole("button", { name: "Delete details permanently" }));
    expect(await screen.findByText(/Metformin 500 mg — Deleted/)).toBeInTheDocument();
    expect(screen.queryByText(/felt fine/)).not.toBeInTheDocument();
    expect(screen.getByText(/Only this marker remains/)).toBeInTheDocument();
    // A tombstone offers neither Restore, Undo, Correct nor a second delete.
    expect(screen.queryByRole("button", { name: /Restore|^Undo|Correct|Delete/ })).not.toBeInTheDocument();
  });
});

describe("Recoverable startup failure", () => {
  it.each([
    ["newer_database", "This data needs a newer MedTracker"],
    ["migration_failed", "MedTracker couldn't upgrade your data"],
    ["corrupt_database", "MedTracker couldn't read your data file"],
  ] as const)("shows a recovery screen for %s instead of the app", async (kind, title) => {
    render(<App api={fakeApi({ startupError: { kind, message: "Your data has not been changed." } })} />);
    expect(await screen.findByRole("heading", { name: title })).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Your data has not been changed.");
    expect(screen.getByRole("alert")).toHaveTextContent("Nothing was deleted or changed");
    expect(screen.queryByRole("tab", { name: "Today" })).not.toBeInTheDocument();
  });
});
