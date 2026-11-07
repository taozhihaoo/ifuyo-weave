import { commands } from "../generated/bindings";
import { isLocale } from "../i18n";
import { applyTokens, type AppliedTheme } from "../design/applyTokens";
import { useAppStore, type Locale, type Theme } from "../stores/appStore";

const THEMES: readonly Theme[] = ["system", "light", "dark"] as const;

function asTheme(value: string): Theme {
  return (THEMES as readonly string[]).includes(value) ? (value as Theme) : "system";
}

/// M11 §222/§223：解析 system（跟随 prefers-color-scheme）并应用 tokens。
export function applyTheme(theme: Theme): void {
  // 非 Tauri 环境（Vite 浏览器预览 / jsdom）可能没有 matchMedia；
  // 缺失时按浅色降级，绝不因主题解析抛错拖垮启动。
  const systemPrefersDark =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved: AppliedTheme =
    theme === "dark"
      ? "dark"
      : theme === "light"
        ? "light"
        : systemPrefersDark
          ? "dark"
          : "light";
  applyTokens(resolved);
}

/**
 * 应用启动序列：IPC ping 验证连通 → app info → 持久化配置。
 * 全部走真实 IPC；任何结构化错误都进入 store 供 UI 呈现，不静默吞掉。
 */
export async function bootstrapApp(): Promise<void> {
  const store = useAppStore.getState();
  store.setIpcStatus("connecting");
  try {
    const pong = await commands.ping();
    useAppStore.getState().setIpcStatus(pong.message === "pong" ? "connected" : "failed");
  } catch {
    useAppStore.getState().setIpcStatus("failed");
    return;
  }

  const info = await commands.getAppInfo();
  if (info.status === "ok") {
    useAppStore.getState().setAppInfo(info.data);
  } else {
    useAppStore.getState().setError(info.error);
  }

  const config = await commands.getAppConfig();
  if (config.status === "ok") {
    const locale: Locale = isLocale(config.data.language ?? "")
      ? (config.data.language as Locale)
      : "zh-CN";
    useAppStore.getState().setLocale(locale);
    const theme = asTheme(config.data.theme ?? "system");
    useAppStore.getState().setTheme(theme);
    applyTheme(theme);
    // M11 §34/§38：收藏加载；启动时清理失效引用（§38 stale → remove safely）
    const favorites = (config.data.favorites ?? []).filter(
      (f) => f.kind === "view" || f.kind === "workflow",
    );
    useAppStore.getState().setFavorites(
      favorites.map((f) => ({ kind: f.kind as "view" | "workflow", id: f.id })),
    );
  } else {
    useAppStore.getState().setError(config.error);
  }
}

/** M2.9 §32 设置持久化纪律：set_app_config 是整体替换——任何保存都必须
 * 先取全量再改字段（否则缺失的扩展段（clipboard/wisp）会被 serde default
 * 静默重置——M2.3/M2.9 设置曾随主题切换丢失）。 */
export async function persistAppConfig(
  mutate: (config: import("../generated/bindings").AppConfig) => import("../generated/bindings").AppConfig,
): Promise<void> {
  const current = await commands.getAppConfig();
  if (current.status !== "ok") {
    useAppStore.getState().setError(current.error);
    return;
  }
  const merged = mutate(current.data);
  const result = await commands.setAppConfig(merged);
  if (result.status === "error") {
    useAppStore.getState().setError(result.error);
  }
}

/** M11 §223 theme 持久化 + 应用（§224 fallback = system）。 */
export async function persistTheme(theme: Theme): Promise<void> {
  // 视觉反馈不依赖后端可达性：先即时应用，再持久化（结构化失败仍由
  // persistAppConfig 写入 store 呈现，绝不静默伪装成功）。
  applyTheme(theme);
  await persistAppConfig((config) => ({ ...config, theme }));
}

/** M11 §40 收藏持久化（AppConfig.favorites；失败经 store 错误呈现）。 */
export async function persistFavorites(): Promise<void> {
  const current = useAppStore.getState();
  await persistAppConfig((config) => ({
    ...config,
    favorites: current.favorites.map((f) => ({ kind: f.kind, id: f.id })),
  }));
}

/** 语言切换持久化：保存失败时把结构化错误交给 UI，不静默。 */
export async function persistLocale(locale: Locale): Promise<void> {
  await persistAppConfig((config) => ({ ...config, language: locale }));
}
