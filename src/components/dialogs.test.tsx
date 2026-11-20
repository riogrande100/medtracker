import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { App } from "../App";
import type { Api } from "../lib/api";
import { fakeApi } from "../test/fakeApi";

async function openAddMedication(api: Api) {
  const user = userEvent.setup();
  render(<App api={api} />);
  await user.click(await screen.findByRole("tab", { name: "Medications" }));
  await user.click(await screen.findByRole("button", { name: "Add medication" }));
  return { user, dialog: await screen.findByRole("dialog", { name: "Add medication" }) };
}

describe("Add medication dialog layout", () => {
  it("gives time and text controls the width and box classes that keep them inside the dialog", async () => {
    const { dialog } = await openAddMedication(fakeApi({ meds: [] }));
    for (const control of [within(dialog).getByLabelText("Name"), within(dialog).getByLabelText("Time of day")]) {
      expect(control).toHaveClass("w-full", "min-w-0", "max-w-full", "box-border", "min-h-12");
    }
    expect(dialog).toHaveClass("dialog-content", "overflow-hidden");
  });

  it("keeps Save and Cancel in a sticky footer outside the scrolling body, inside the same form", async () => {
    const { dialog } = await openAddMedication(fakeApi({ meds: [] }));
    const save = within(dialog).getByRole("button", { name: "Save" });
    expect(save).toHaveAttribute("type", "submit");

    const footer = save.closest(".sticky") as HTMLElement;
    expect(footer).toHaveClass("sticky", "bottom-0", "bg-white", "shrink-0");
    expect(footer.className).toContain("env(safe-area-inset-bottom)");

    const scroller = within(dialog).getByLabelText("Name").closest(".overflow-y-auto") as HTMLElement;
    expect(scroller).not.toBeNull();
    expect(scroller.contains(save)).toBe(false);
    expect(footer.closest("form")).toBe(scroller.closest("form"));
  });

  it("moves focus to the next field on Enter and submits from the last field", async () => {
    const createMedication = vi.fn(async () => "m1");
    const { user, dialog } = await openAddMedication({ ...fakeApi({ meds: [] }), createMedication });

    await user.click(within(dialog).getByRole("checkbox", { name: /Show on Today every day/ }));
    const name = within(dialog).getByLabelText("Name");
    const strength = within(dialog).getByLabelText(/Strength/);
    const form = within(dialog).getByLabelText(/Form/);

    name.focus();
    await user.keyboard("Ibuprofen{Enter}");
    expect(strength).toHaveFocus();
    await user.keyboard("200 mg{Enter}");
    expect(form).toHaveFocus();
    expect(createMedication).not.toHaveBeenCalled();

    await user.keyboard("tablet{Enter}");
    expect(createMedication).toHaveBeenCalledWith({ name: "Ibuprofen", strength: "200 mg", form: "tablet", times: [] });
  });
});
