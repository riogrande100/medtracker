import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { Button } from "./components/ui";
import { STORAGE_KEY } from "./lib/uiPrefs";
import { fakeApi } from "./test/fakeApi";

const root = () => document.documentElement;

beforeEach(() => {
  try { localStorage.clear(); } catch { /* storage unavailable in this environment */ }
  root().removeAttribute("style");
  delete root().dataset.motion;
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function openSettings() {
  const user = userEvent.setup();
  render(<App api={fakeApi()} />);
  await user.click(await screen.findByRole("tab", { name: "More" }));
  await user.click(screen.getByRole("button", { name: "Settings" }));
  return user;
}

describe("Navigation via More", () => {
  it("shows Today, History, Medications and More as tabs, and Reports, Backup and Settings under More", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    const tabs = await screen.findAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["Today", "History", "Medications", "More"]);
    expect(screen.queryByRole("tab", { name: "Reports" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Backup" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "More" }));
    expect(screen.getByRole("button", { name: "Reports" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Backup" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to More" }));
    expect(screen.getByRole("button", { name: "Reports" })).toBeInTheDocument();
  });

  it("tabs are keyboard accessible with arrow keys", async () => {
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    await user.click(await screen.findByRole("tab", { name: "Today" }));
    await user.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}");
    expect(screen.getByRole("tab", { name: "More" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("button", { name: "Backup" })).toBeInTheDocument();
  });

  it("keeps the tab targets at least 44px tall", async () => {
    render(<App api={fakeApi()} />);
    const tab = await screen.findByRole("tab", { name: "More" });
    expect(tab.className).toContain("min-h-12");
  });
});

describe("Settings", () => {
  it("applies the accent through CSS variables and the brand token", async () => {
    const user = await openSettings();
    expect(root().style.getPropertyValue("--accent")).toBe("#1d4ed8");
    await user.click(screen.getByRole("radio", { name: "Green" }));
    expect(root().style.getPropertyValue("--accent")).toBe("#15803d");
    expect(root().style.getPropertyValue("--accent-hover")).toBe("#166534");
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).accent).toBe("green");
  });

  it("keeps Cancel and Delete styling independent of the accent", () => {
    render(
      <>
        <Button>Cancel</Button>
        <Button variant="danger">Delete</Button>
        <Button variant="primary">Confirm</Button>
      </>,
    );
    expect(screen.getByRole("button", { name: "Cancel" }).className).not.toContain("brand");
    expect(screen.getByRole("button", { name: "Delete" }).className).not.toContain("brand");
    expect(screen.getByRole("button", { name: "Delete" }).className).toContain("text-red-800");
    expect(screen.getByRole("button", { name: "Confirm" }).className).toContain("bg-brand");
  });

  it("changes the text size through the root font size and remembers it", async () => {
    const user = await openSettings();
    await user.click(screen.getByRole("radio", { name: "Extra large" }));
    expect(root().style.fontSize).toBe("140%");

    const { unmount } = render(<span />);
    unmount();
    // A fresh app reads the saved choice back.
    render(<App api={fakeApi()} />);
    expect(root().style.fontSize).toBe("140%");
  });

  it("offers Reduce motion and records it on the root element", async () => {
    const user = await openSettings();
    expect(root().dataset.motion).toBe("system");
    await user.click(screen.getByRole("radio", { name: "Reduce motion" }));
    expect(root().dataset.motion).toBe("reduce");
  });

  it("shows the About and privacy statements without clinical claims", async () => {
    await openSettings();
    const section = screen.getByRole("heading", { name: "About and privacy" }).closest("section")!;
    expect(section).toHaveTextContent("Your data stays on this device.");
    expect(section).toHaveTextContent("no accounts");
    expect(section).toHaveTextContent("no analytics");
    expect(section).toHaveTextContent("The network allowlist is currently empty.");
    expect(section.textContent).not.toMatch(/clinical|effective|improve|benefit|safe to take|medical advice/i);
  });
});

describe("storage that throws", () => {
  it("renders the app and still applies choices in memory", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new DOMException("denied", "SecurityError"); });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("full", "QuotaExceededError"); });
    const user = userEvent.setup();
    render(<App api={fakeApi()} />);
    expect(await screen.findByText("Due now")).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "More" }));
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: "Purple" }));
    expect(root().style.getPropertyValue("--accent")).toBe("#7e22ce");
    expect(await screen.findByText(/wouldn't save your choice/)).toBeInTheDocument();
  });

  it("renders when accessing window.localStorage itself throws", async () => {
    const getter = vi.spyOn(window, "localStorage", "get").mockImplementation(() => {
      throw new DOMException("blocked", "SecurityError");
    });
    render(<App api={fakeApi()} />);
    expect(await screen.findByText("Due now")).toBeInTheDocument();
    getter.mockRestore();
  });
});
