import { trackVisualViewport } from "./visualViewport";

function fakeWindow(withVisualViewport: boolean) {
  const vv = Object.assign(new EventTarget(), { height: 800, offsetTop: 0 });
  const doc = document.implementation.createHTMLDocument("t");
  const win = { document: doc, visualViewport: withVisualViewport ? vv : undefined } as unknown as Window;
  return { win, vv, root: doc.documentElement };
}

describe("trackVisualViewport", () => {
  it("mirrors the visible height and offset into CSS variables and follows resize and scroll", () => {
    const { win, vv, root } = fakeWindow(true);
    const stop = trackVisualViewport(win);
    expect(root.style.getPropertyValue("--vv-height")).toBe("800px");

    vv.height = 420;
    vv.dispatchEvent(new Event("resize"));
    expect(root.style.getPropertyValue("--vv-height")).toBe("420px");

    vv.offsetTop = 60;
    vv.dispatchEvent(new Event("scroll"));
    expect(root.style.getPropertyValue("--vv-top")).toBe("60px");

    stop();
    expect(root.style.getPropertyValue("--vv-height")).toBe("");
    expect(root.style.getPropertyValue("--vv-top")).toBe("");
    vv.height = 300;
    vv.dispatchEvent(new Event("resize"));
    expect(root.style.getPropertyValue("--vv-height")).toBe("");
  });

  it("does nothing where VisualViewport is unavailable", () => {
    const { win, root } = fakeWindow(false);
    const stop = trackVisualViewport(win);
    expect(root.style.getPropertyValue("--vv-height")).toBe("");
    expect(() => stop()).not.toThrow();
  });
});
