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

// M11 §37 收藏引用（type+id；label 从当前 Registry 重新解析）。
export interface FavoriteEntry {
  kind: "view" | "workflow";
  id: string;
}

// M11 §45 最近操作（真实来源：M2 History / 本会话 M7 任务完成）。
export interface RecentOperation {
  kind: string;
  summary: string;
  // success | failed | cancelled | partial
  status: string;
  timestampMs: number;
  operationId: string | null;
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
  // M11 §34 用户收藏（持久化在 AppConfig.favorites）。
  favorites: FavoriteEntry[];
  // M11 §43 最近操作（真实来源；会话内环形，上限 20）。
  recentOps: RecentOperation[];
  // M11 §15 Command Palette 开关。
  paletteOpen: boolean;

  setLocale: (locale: Locale) => void;
  setRecentRoots: (roots: string[]) => void;
  setFavorites: (favorites: FavoriteEntry[]) => void;
  toggleFavorite: (entry: FavoriteEntry) => void;
  recordRecent: (op: RecentOperation) => void;
  setPaletteOpen: (open: boolean) => void;
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
  favorites: [],
  recentOps: [],
  paletteOpen: false,
  setLocale: (locale) => set({ locale }),
  setRecentRoots: (recentRoots) => set({ recentRoots }),
  setFavorites: (favorites) => set({ favorites }),
  toggleFavorite: (entry) =>
    set((state) => {
      const exists = state.favorites.some(
        (f) => f.kind === entry.kind && f.id === entry.id,
      );
      return {
        favorites: exists
          ? state.favorites.filter((f) => !(f.kind === entry.kind && f.id === entry.id))
          : [...state.favorites, entry],
      };
    }),
  recordRecent: (op) =>
    set((state) => ({
      recentOps: [op, ...state.recentOps.filter((r) => r.operationId !== op.operationId)].slice(
        0,
        20,
      ),
    })),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
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
