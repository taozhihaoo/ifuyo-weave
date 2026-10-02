/**
 * 把 tokens.ts 展平为 CSS 自定义属性并挂到 :root。
 * token 名 camelCase → kebab-case：`color.textMuted` → `--color-text-muted`。
 */

import { tokens } from "./tokens";

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

export function applyTokens(): void {
  const cssVars: Record<string, string> = {};
  flattenTokens(tokens, "", cssVars);
  const root = document.documentElement;
  for (const [name, value] of Object.entries(cssVars)) {
    root.style.setProperty(name, value);
  }
}
