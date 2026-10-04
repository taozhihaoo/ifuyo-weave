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
  const resolved: AppliedTheme =
    theme === "dark"
      ? "dark"
      : theme === "light"
        ? "light"
        : window.matchMedia("(prefers-color-scheme: dark)").matches
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

/** M11 §223 theme 持久化 + 应用（§224 fallback = system）。 */
export async function persistTheme(theme: Theme): Promise<void> {
  const current = useAppStore.getState();
  const result = await commands.setAppConfig({
    version: 1,
    language: current.locale,
    theme,
    favorites: current.favorites.map((f) => ({ kind: f.kind, id: f.id })),
  });
  if (result.status === "error") {
    useAppStore.getState().setError(result.error);
    return;
  }
  applyTheme(theme);
}

/** M11 §40 收藏持久化（AppConfig.favorites；失败经 store 错误呈现）。 */
export async function persistFavorites(): Promise<void> {
  const current = useAppStore.getState();
  const result = await commands.setAppConfig({
    version: 1,
    language: current.locale,
    theme: current.theme,
    favorites: current.favorites.map((f) => ({ kind: f.kind, id: f.id })),
  });
  if (result.status === "error") {
    useAppStore.getState().setError(result.error);
  }
}

/** 语言切换持久化：保存失败时把结构化错误交给 UI，不静默。 */
export async function persistLocale(locale: Locale): Promise<void> {
  const current = useAppStore.getState();
  const result = await commands.setAppConfig({
    version: 1,
    language: locale,
    theme: current.theme,
    favorites: current.favorites.map((f) => ({ kind: f.kind, id: f.id })),
  });
  if (result.status === "error") {
    useAppStore.getState().setError(result.error);
  }
}
