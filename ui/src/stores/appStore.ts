import { create } from "zustand";
import type {
  FileInspectionDto,
  HashResultDto,
  IpcError,
  JobStatusDto,
} from "../generated/bindings";

export type IpcStatus = "connecting" | "connected" | "failed";
export type Locale = "zh-CN" | "en";
export type Theme = "system" | "light" | "dark";

export interface ActiveJob {
  jobId: string;
  path: string;
}

/**
 * M0 允许的状态（app info、locale、theme、IPC smoke）+ M1 文件核心状态
 * （inspector 结果、扫描/哈希任务）。禁止 useFileStore/useBatchStore 等
 * 未落地场景的 store（M0 §67）。
 */
interface AppState {
  locale: Locale;
  theme: Theme;
  appInfo: AppInfoDto | null;
  ipcStatus: IpcStatus;
  lastError: IpcError | null;

  inspection: FileInspectionDto | null;
  inspectError: IpcError | null;
  scanJob: ActiveJob | null;
  scanStatus: JobStatusDto | null;
  hashJob: ActiveJob | null;
  hashStatus: JobStatusDto | null;
  hashResult: HashResultDto | null;
  // M3：会话内最近使用的重复扫描根目录（charter #37：工具参数不入全局配置，
  // 这里只是 UI 会话态，重启即清）。
  recentRoots: string[];

  setLocale: (locale: Locale) => void;
  setRecentRoots: (roots: string[]) => void;
  setTheme: (theme: Theme) => void;
  setAppInfo: (info: AppInfoDto) => void;
  setIpcStatus: (status: IpcStatus) => void;
  setError: (error: IpcError) => void;
  clearError: () => void;
  setInspection: (inspection: FileInspectionDto) => void;
  setInspectError: (error: IpcError) => void;
  clearFileViews: () => void;
  setScanJob: (job: ActiveJob) => void;
  setScanStatus: (status: JobStatusDto) => void;
  setHashJob: (job: ActiveJob) => void;
  setHashStatus: (status: JobStatusDto) => void;
  setHashResult: (result: HashResultDto) => void;
  clearHash: () => void;
}

// bindings 生成的 AppInfo 与本地别名保持同步。
type AppInfoDto = import("../generated/bindings").AppInfo;

export const useAppStore = create<AppState>((set) => ({
  locale: "zh-CN",
  theme: "system",
  appInfo: null,
  ipcStatus: "connecting",
  lastError: null,
  inspection: null,
  inspectError: null,
  scanJob: null,
  scanStatus: null,
  hashJob: null,
  hashStatus: null,
  hashResult: null,
  recentRoots: [],
  setLocale: (locale) => set({ locale }),
  setRecentRoots: (recentRoots) => set({ recentRoots }),
  setTheme: (theme) => set({ theme }),
  setAppInfo: (appInfo) => set({ appInfo }),
  setIpcStatus: (ipcStatus) => set({ ipcStatus }),
  setError: (lastError) => set({ lastError }),
  clearError: () => set({ lastError: null }),
  setInspection: (inspection) =>
    set({ inspection, inspectError: null, scanJob: null, scanStatus: null }),
  setInspectError: (inspectError) => set({ inspectError, inspection: null }),
  clearFileViews: () =>
    set({ inspection: null, inspectError: null, scanJob: null, scanStatus: null }),
  setScanJob: (scanJob) => set({ scanJob, scanStatus: null, inspection: null, inspectError: null }),
  setScanStatus: (scanStatus) => set({ scanStatus }),
  setHashJob: (hashJob) => set({ hashJob, hashStatus: null, hashResult: null }),
  setHashStatus: (hashStatus) => set({ hashStatus }),
  setHashResult: (hashResult) => set({ hashResult }),
  clearHash: () => set({ hashJob: null, hashStatus: null, hashResult: null }),
}));
