/**
 * 把 tokens.ts 展平为 CSS 自定义属性并挂到 :root。
 * token 名 camelCase → kebab-case：`color.textMuted` → `--color-text-muted`。
 */

import { darkColorOverrides, tokens } from "./tokens";

function toKebab(key: string): string {
  return key.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
}

export function flattenTokens(node: unknown, prefix: string, out: Record<string, string>): void {
  if (node === null || typeof node !== "object") {
    out[prefix] = String(node);
    return;
  }
  for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
    flattenTokens(value, prefix ? `${prefix}-${toKebab(key)}` : `--${toKebab(key)}`, out);
  }
}

export type AppliedTheme = "light" | "dark";

/// M11 §222：按主题应用 tokens。dark = light 基础 + color/surface 覆盖。
// 数值型 token 家族 → 单位（M11 视觉修复：裸数字在 CSS 中非法，
// `padding: var(--spacing-xl)` 曾整体失效导致全应用间距/圆角为 0）。
const PX_TOKEN_PREFIXES = [
  "--spacing-",
  "--radius-",
  "--typography-size-",
  "--control-height-",
  "--control-padding-",
];

function withUnits(name: string, value: string): string {
  if (/^\d+(\.\d+)?$/.test(value) && PX_TOKEN_PREFIXES.some((p) => name.startsWith(p))) {
    return `${value}px`;
  }
  return value;
}

export function applyTokens(theme: AppliedTheme = "light"): void {
  const cssVars: Record<string, string> = {};
  flattenTokens(tokens, "", cssVars);
  if (theme === "dark") {
    flattenTokens(darkColorOverrides, "", cssVars);
  }
  const root = document.documentElement;
  for (const [name, raw] of Object.entries(cssVars)) {
    root.style.setProperty(name, withUnits(name, raw));
  }
}
