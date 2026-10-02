import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { flattenTokens } from "./applyTokens";
import { tokens } from "./tokens";

const here = dirname(fileURLToPath(import.meta.url));
const srcRoot = join(here, "..");

// design tokens 本身允许色值；generated/ 不受控。其余业务源码禁止裸色值（M0 §11.3）。
const EXCLUDED = new Set(["design", "generated"]);
const BARE_COLOR = /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/;

function collect(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (EXCLUDED.has(entry.name)) continue;
      collect(full, out);
    } else if (/\.(ts|tsx|css)$/.test(entry.name) && !entry.name.endsWith(".test.ts")) {
      out.push(full);
    }
  }
  return out;
}

describe("design token discipline", () => {
  it("business sources contain no bare color literals", () => {
    const offenders: string[] = [];
    for (const file of collect(srcRoot)) {
      const content = readFileSync(file, "utf-8");
      if (BARE_COLOR.test(content)) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });

  it("tokens cover all eight required categories (M0 §11.1)", () => {
    for (const category of [
      "color",
      "spacing",
      "radius",
      "typography",
      "zIndex",
      "motion",
      "control",
      "surface",
    ]) {
      expect(Object.keys(tokens)).toContain(category);
    }
  });

  it("flattens to kebab-case CSS custom properties", () => {
    const out: Record<string, string> = {};
    flattenTokens(tokens.color, "", out);
    expect(Object.keys(out)).toContain("--text-muted");
    expect(out["--accent"]).toBe("#3d6ffe");
  });
});
