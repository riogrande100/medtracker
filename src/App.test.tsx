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
