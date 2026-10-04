/**
 * Design tokens 单一事实源（M0 §11）。
 *
 * 视觉方向（charter §31 + M11 视觉刷新）：柔和、时尚、低饱和、轻投影、
 * 大圆角；accent = 柔和紫罗兰，中性色偏冷灰蓝。业务代码禁止出现裸色值；
 * `design/` 是唯一例外（M0 §11.3，有自动检查守护）。
 * 生成的占位应用图标（scripts/generate-icon.mjs）与这里的 accent 保持一致。
 */

export const tokens = {
  color: {
    background: "#f6f7fb",
    surface: "#ffffff",
    surfaceHover: "#f3f4fa",
    inset: "#eef0f7",
    text: "#2b2e38",
    textMuted: "#7c8194",
    border: "#e5e8f2",
    accent: "#8b7cf6",
    accentSoft: "#efecfe",
    danger: "#e07a8b",
    success: "#54b48c",
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
    sm: 8,
    md: 12,
    lg: 16,
    full: 999,
  },
  typography: {
    fontFamily: "'Segoe UI Variable Text', 'Segoe UI', 'Microsoft YaHei UI', system-ui, sans-serif",
    monoFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, monospace",
    sizeSm: 12,
    sizeMd: 14,
    sizeLg: 16,
    sizeXl: 22,
    sizeTitle: 30,
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
    focusRing: "0 0 0 3px rgba(139, 124, 246, 0.28)",
    disabledOpacity: 0.45,
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    overlay: "rgba(43, 46, 56, 0.4)",
  },
  shadow: {
    sm: "0 1px 2px rgba(43, 46, 56, 0.06)",
    md: "0 2px 10px rgba(43, 46, 56, 0.08)",
    lg: "0 12px 32px rgba(43, 46, 56, 0.14)",
  },
} as const;

export type Tokens = typeof tokens;

/// M11 §222：dark 主题覆盖（柔和深色：低对比灰蓝底 + 柔紫 accent；
/// 仅 color/surface/shadow 类，其余 token 与 light 共享）。
export const darkColorOverrides = {
  color: {
    background: "#131419",
    surface: "#1b1d24",
    surfaceHover: "#22252e",
    inset: "#23262f",
    text: "#e7e9f2",
    textMuted: "#9298ab",
    border: "#2c303b",
    accent: "#a296ff",
    accentSoft: "#2a2647",
    danger: "#ec8a97",
    success: "#6cc9a2",
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    overlay: "rgba(0, 0, 0, 0.55)",
  },
  shadow: {
    sm: "0 1px 2px rgba(0, 0, 0, 0.4)",
    md: "0 2px 10px rgba(0, 0, 0, 0.45)",
    lg: "0 12px 32px rgba(0, 0, 0, 0.6)",
  },
} as const;

export type ThemeName = "light" | "dark";
