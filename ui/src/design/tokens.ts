/**
 * Design tokens 单一事实源（M0 §11）。
 *
 * 视觉方向（charter §31 + M11 / 2.0 视觉刷新）：柔和、时尚、低饱和、轻投影、
 * 大圆角；accent = 柔和紫罗兰，中性色偏冷灰蓝。业务代码禁止出现裸色值；
 * `design/` 是唯一例外（M0 §11.3，有自动检查守护）。
 * 生成的占位应用图标（scripts/generate-icon.mjs）与这里的 accent 保持一致。
 *
 * 补齐历史欠账：--typography-size-xs / --color-warning 此前被 8 / 3 个业务
 * 文件引用却从未定义，CSS 变量回退导致"小字号"与"警告色"实际未生效。
 */

export const tokens = {
  color: {
    background: "#f6f7fb",
    surface: "#ffffff",
    surfaceHover: "#f3f4fa",
    inset: "#eef0f7",
    text: "#2b2e38",
    textMuted: "#7c8194",
    textFaint: "#a2a8ba",
    border: "#e5e8f2",
    borderStrong: "#d6dae9",
    accent: "#8b7cf6",
    accentStrong: "#6d5ce0",
    accentAlt: "#5aa9c8",
    accentSoft: "#efecfe",
    onAccent: "#ffffff",
    glowA: "#eee9ff",
    glowB: "#e3f1f7",
    danger: "#e07a8b",
    dangerSoft: "#fbe9ed",
    success: "#54b48c",
    successSoft: "#e6f5ee",
    warning: "#cf9350",
    warningSoft: "#fbf0dd",
  },
  spacing: {
    xs: 4,
    sm: 8,
    md: 12,
    lg: 16,
    xl: 24,
    xxl: 36,
    xxxl: 48,
  },
  radius: {
    sm: 8,
    md: 12,
    lg: 16,
    xl: 22,
    full: 999,
  },
  typography: {
    fontFamily: "'Segoe UI Variable Text', 'Segoe UI', 'Microsoft YaHei UI', system-ui, sans-serif",
    monoFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, monospace",
    sizeXs: 11,
    sizeSm: 12,
    sizeMd: 14,
    sizeLg: 16,
    sizeXl: 22,
    sizeTitle: 30,
    sizeHero: 26,
    lineHeight: "1.55",
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
    slow: "360ms",
    ease: "cubic-bezier(0.2, 0, 0, 1)",
    spring: "cubic-bezier(0.34, 1.4, 0.64, 1)",
  },
  control: {
    heightSm: 28,
    heightMd: 34,
    heightLg: 40,
    paddingX: 14,
    focusRing: "0 0 0 3px rgba(139, 124, 246, 0.28)",
    disabledOpacity: 0.45,
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    sidebar: "var(--color-surface)",
    topbar: "rgba(255, 255, 255, 0.72)",
    overlay: "rgba(43, 46, 56, 0.4)",
    ambient:
      "radial-gradient(760px 460px at 8% -10%, var(--color-glow-a) 0%, transparent 68%), radial-gradient(640px 420px at 100% 0%, var(--color-glow-b) 0%, transparent 66%)",
    hero: "linear-gradient(125deg, var(--color-accent-strong) 0%, var(--color-accent) 46%, var(--color-accent-alt) 130%)",
    heroPattern:
      "radial-gradient(circle at 84% 16%, rgba(255,255,255,0.28) 0, transparent 42%), radial-gradient(circle at 68% 120%, rgba(255,255,255,0.14) 0, transparent 55%)",
  },
  shadow: {
    sm: "0 1px 2px rgba(43, 46, 56, 0.06)",
    md: "0 2px 10px rgba(43, 46, 56, 0.08)",
    lg: "0 12px 32px rgba(43, 46, 56, 0.14)",
    card: "0 1px 2px rgba(43, 46, 56, 0.05), 0 10px 28px rgba(43, 46, 56, 0.06)",
    glow: "0 10px 30px rgba(109, 92, 224, 0.28)",
    inset: "inset 0 1px 0 rgba(255, 255, 255, 0.5)",
  },
} as const;

export type Tokens = typeof tokens;

/// M11 §222：dark 主题覆盖（柔和深色：低对比灰蓝底 + 柔紫 accent）。
/// ambient / hero / heroPattern 只引用 CSS 变量，dark 下自动随 color 覆盖生效，
/// 无需重复定义（避免两套渐变漂移）。
export const darkColorOverrides = {
  color: {
    background: "#131419",
    surface: "#1b1d24",
    surfaceHover: "#22252e",
    inset: "#23262f",
    text: "#e7e9f2",
    textMuted: "#9298ab",
    textFaint: "#6e7488",
    border: "#2c303b",
    borderStrong: "#3b4150",
    accent: "#a296ff",
    accentStrong: "#b5aaff",
    accentAlt: "#6ec3dd",
    accentSoft: "#2a2647",
    onAccent: "#15161d",
    glowA: "#201d38",
    glowB: "#142a33",
    danger: "#ec8a97",
    dangerSoft: "#331f26",
    success: "#6cc9a2",
    successSoft: "#16302a",
    warning: "#e5b56f",
    warningSoft: "#33291b",
  },
  surface: {
    page: "var(--color-background)",
    card: "var(--color-surface)",
    sidebar: "var(--color-surface)",
    topbar: "rgba(27, 29, 36, 0.72)",
    overlay: "rgba(0, 0, 0, 0.55)",
  },
  shadow: {
    sm: "0 1px 2px rgba(0, 0, 0, 0.4)",
    md: "0 2px 10px rgba(0, 0, 0, 0.45)",
    lg: "0 12px 32px rgba(0, 0, 0, 0.6)",
    card: "0 1px 2px rgba(0, 0, 0, 0.35), 0 10px 28px rgba(0, 0, 0, 0.28)",
    glow: "0 10px 30px rgba(120, 105, 255, 0.24)",
    inset: "inset 0 1px 0 rgba(255, 255, 255, 0.04)",
  },
} as const;

export type ThemeName = "light" | "dark";
