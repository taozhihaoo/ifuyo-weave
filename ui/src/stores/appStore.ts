import { create } from "zustand";
import type { AppInfo, IpcError, PathProbe } from "../generated/bindings";

export type IpcStatus = "connecting" | "connected" | "failed";
export type Locale = "zh-CN" | "en";
export type Theme = "system" | "light" | "dark";

/**
 * M0 允许的状态（M0 §67）：app info、locale、theme、IPC smoke、探针结果。
 * 禁止提前创建 useFileStore / useBatchStore 等未来 store。
 */
interface AppState {
  locale: Locale;
  theme: Theme;
  appInfo: AppInfo | null;
  ipcStatus: IpcStatus;
  lastError: IpcError | null;
  probe: PathProbe | null;
  probeError: IpcError | null;
  setLocale: (locale: Locale) => void;
  setTheme: (theme: Theme) => void;
  setAppInfo: (info: AppInfo) => void;
  setIpcStatus: (status: IpcStatus) => void;
  setError: (error: IpcError) => void;
  clearError: () => void;
  setProbe: (probe: PathProbe) => void;
  setProbeError: (error: IpcError) => void;
}

export const useAppStore = create<AppState>((set) => ({
  locale: "zh-CN",
  theme: "system",
  appInfo: null,
  ipcStatus: "connecting",
  lastError: null,
  probe: null,
  probeError: null,
  setLocale: (locale) => set({ locale }),
  setTheme: (theme) => set({ theme }),
  setAppInfo: (appInfo) => set({ appInfo }),
  setIpcStatus: (ipcStatus) => set({ ipcStatus }),
  setError: (lastError) => set({ lastError }),
  clearError: () => set({ lastError: null }),
  setProbe: (probe) => set({ probe, probeError: null }),
  setProbeError: (probeError) => set({ probeError, probe: null }),
}));
