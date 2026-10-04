/**
 * Design tokens 单一事实源（M0 §11）。
 *
 * 视觉方向（charter §31）：干净、克制、现代、中性、有一点 ifuyo 情绪。
 * 业务代码禁止出现裸色值；`design/` 是唯一例外（M0 §11.3，有自动检查守护）。
 * 生成的占位应用图标（scripts/generate-icon.mjs）与这里的 accent 保持一致。
 */

export const tokens = {
  color: {
    background: "#f6f7f9",
    surface: "#ffffff",
    surfaceHover: "#f0f2f5",
    inset: "#eceef1",
    text: "#1d2129",
    textMuted: "#697079",
    border: "#dfe3e8",
    accent: "#3d6ffe",
    accentSoft: "#e3ecff",
    danger: "#d0454c",
    success: "#2f9e63",
  },
  spacing: {
    xs: 4,
    sm: 8,
    md: 12,
    lg: 16,
    xl: 24,
    xxl: 36,
  },
  radius: {
    sm: 6,
    md: 10,
    lg: 14,
    full: 999,
  },
  typography: {
    fontFamily: "'Segoe UI Variable Text', 'Segoe UI', 'Microsoft YaHei UI', system-ui, sans-serif",
    monoFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, monospace",
    sizeSm: 12,
    sizeMd: 14,
    sizeLg: 16,
    sizeXl: 22,
    sizeTitle: 32,
  },
  zIndex: {
    base: 0,
    dropdown: 100,
    modal: 200,
    toast: 300,
  },
  motion: {
    fast: "120ms",
    normal: "200ms",
    ease: "cubic-bezier(0.2, 0, 0, 1)",
  },
  control: {
    heightSm: 28,
    heightMd: 34,
    paddingX: 14,
    focusRing: "0 0 0 2px rgba(61, 111, 254, 0.35)",
    disabledOpacity: 0.45,
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    overlay: "rgba(29, 33, 41, 0.4)",
  },
} as const;

export type Tokens = typeof tokens;

/// M11 §222：dark 主题覆盖（仅 color/surface 类；其余 token 与 light 共享）。
export const darkColorOverrides = {
  color: {
    background: "#16181d",
    surface: "#1f2329",
    surfaceHover: "#262b33",
    inset: "#2a3038",
    text: "#e8eaed",
    textMuted: "#9aa0a8",
    border: "#343a44",
    accent: "#5b8cff",
    accentSoft: "#1c2a4a",
    danger: "#e0676d",
    success: "#4cbf80",
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    overlay: "rgba(0, 0, 0, 0.55)",
  },
} as const;

export type ThemeName = "light" | "dark";
