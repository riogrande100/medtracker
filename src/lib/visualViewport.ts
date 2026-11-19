/**
 * Mirrors the visual viewport (what is actually visible above the on-screen keyboard) into CSS custom
 * properties on <html>: --vv-top and --vv-height. Dialog CSS (`.dialog-content` in index.css) reads them,
 * so the dialog shrinks to the visible area and its sticky footer stays above the keyboard.
 * Guarded: where VisualViewport is missing, nothing is set and the dvh/vh fallbacks apply.
 * Returns a cleanup function.
 */
export function trackVisualViewport(target: Window = window): () => void {
  const vv = target.visualViewport;
  if (!vv) return () => {};
  const root = target.document.documentElement;
  const update = () => {
    root.style.setProperty("--vv-height", `${vv.height}px`);
    root.style.setProperty("--vv-top", `${vv.offsetTop}px`);
  };
  update();
  vv.addEventListener("resize", update);
  vv.addEventListener("scroll", update);
  return () => {
    vv.removeEventListener("resize", update);
    vv.removeEventListener("scroll", update);
    root.style.removeProperty("--vv-height");
    root.style.removeProperty("--vv-top");
  };
}
