import { commands } from "../generated/bindings";
import { isLocale } from "../i18n";
import { useAppStore, type Locale, type Theme } from "../stores/appStore";

const THEMES: readonly Theme[] = ["system", "light", "dark"] as const;

function asTheme(value: string): Theme {
  return (THEMES as readonly string[]).includes(value) ? (value as Theme) : "system";
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
    useAppStore.getState().setTheme(asTheme(config.data.theme ?? "system"));
  } else {
    useAppStore.getState().setError(config.error);
  }
}

/** 语言切换持久化：保存失败时把结构化错误交给 UI，不静默。 */
export async function persistLocale(locale: Locale): Promise<void> {
  const current = useAppStore.getState();
  const result = await commands.setAppConfig({
    version: 1,
    language: locale,
    theme: current.theme,
  });
  if (result.status === "error") {
    useAppStore.getState().setError(result.error);
  }
}
