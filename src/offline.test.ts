import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..");
const read = (p: string) => readFileSync(join(root, p), "utf8");

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? sourceFiles(p) : /\.(tsx?|rs)$/.test(f) ? [p] : [];
  });
}

describe("offline / no-network separation", () => {
  it("CSP allows only self and Tauri IPC", () => {
    const csp: string = JSON.parse(read("src-tauri/tauri.conf.json")).app.security.csp;
    expect(csp).toContain("default-src 'self'");
    expect(csp).toContain("connect-src ipc: http://ipc.localhost");
    expect(csp).not.toMatch(/https?:\/\/(?!ipc\.localhost)/);
    expect(csp).not.toContain("*");
  });

  it("capabilities grant only core permissions (empty network allowlist)", () => {
    const caps = JSON.parse(read("src-tauri/capabilities/default.json"));
    expect(caps.permissions).toEqual(["core:default"]);
  });

  it("has no HTTP client, telemetry, or sync dependencies", () => {
    const cargo = read("src-tauri/Cargo.toml");
    expect(cargo).not.toMatch(/reqwest|hyper|ureq|isahc|tauri-plugin-(http|shell|fs|updater|opener)|sentry/);
    const pkg = JSON.parse(read("package.json"));
    const deps = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies }).join(" ");
    expect(deps).not.toMatch(/axios|sentry|analytics|plugin-http/);
  });

  it("app source never references remote URLs or fetch", () => {
    for (const f of [...sourceFiles(join(root, "src")), ...sourceFiles(join(root, "src-tauri/src"))]) {
      if (f.endsWith("offline.test.ts")) continue;
      const text = readFileSync(f, "utf8");
      expect(text, f).not.toMatch(/https?:\/\/|\bfetch\(|XMLHttpRequest|WebSocket/);
    }
  });
});
