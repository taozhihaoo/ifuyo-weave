import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { AgentNotificationDto, IpcError } from "../generated/bindings";
import { InspectorPanel, type HashJobView } from "../features/files/InspectorPanel";
import { DuplicatesPanel } from "../features/ops/DuplicatesPanel";
import { TextPanel } from "../features/text/TextPanel";
import { DataPanel } from "../features/data/DataPanel";
import { ImagePanel } from "../features/image/ImagePanel";
import { DocumentsPanel } from "../features/documents/DocumentsPanel";
import { UtilitiesPanel } from "../features/utilities/UtilitiesPanel";
import { ComparePanel } from "../features/compare/ComparePanel";
import { SpacePanel } from "../features/space/SpacePanel";
import { ClipboardPanel } from "../features/clipboard/ClipboardPanel";
import { PrivacyPanel } from "../features/privacy/PrivacyPanel";
import { ArchivePanel } from "../features/archive/ArchivePanel";
import { AgentPanel } from "../features/agent/AgentPanel";
import { RecipePanel } from "../features/recipe/RecipePanel";
import { GlintPanel } from "../features/glint/GlintPanel";
import { WispPanel } from "../features/wisp/WispPanel";
import { WebPanel } from "../features/web/WebPanel";
import { MonitorPanel } from "../features/monitor/MonitorPanel";
import { SmartPanel } from "../features/smart/SmartPanel";
import { JobsPanel } from "../features/shell/JobsPanel";
import { SettingsPanel } from "../features/shell/SettingsPanel";
import { WorkflowPanel } from "../features/workflow/WorkflowPanel";
import { CommandPalette } from "../features/palette/CommandPalette";
import { upsertCommand } from "../commands/registry";
import { registerShortcut, dispatchShortcut } from "../lib/shortcuts";
import { suggestionsFor } from "../lib/quickDrop";
import { persistTheme } from "../lib/bootstrap";
import { BatchPanel } from "../features/batch/BatchPanel";
import { HistoryPanel } from "../features/ops/HistoryPanel";
import { OrganizerPanel } from "../features/ops/OrganizerPanel";
import { RenamePanel } from "../features/ops/RenamePanel";
import { ScanPanel } from "../features/files/ScanPanel";
import { commands } from "../generated/bindings";
import { useT } from "../i18n";
import { isTerminalState, pollJob } from "../lib/jobs";
import { useAppStore, type Locale, type Theme } from "../stores/appStore";

const SEP = String.fromCharCode(92); // path separator on Windows

const buttonStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
};

const pillStyle: CSSProperties = {
  ...buttonStyle,
  borderRadius: "var(--radius-full)",
};

/** 线性图标（单色描边，随 currentColor 走；无第三方图标依赖、无裸色值）。 */
const GLYPHS: Record<string, string> = {
  home: "M3 10.5 12 3l9 7.5 M5 9.5V21h5v-6h4v6h5V9.5",
  files: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  analyze: "M3 3v18h18 M7 15l4-4 3 3 5-6",
  content: "M4 4h10l6 6v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M14 4v6h6 M7 15h10 M7 18h7",
  protect: "M12 3l7 3v6c0 4-3 7-7 9-4-2-7-5-7-9V6z M9 12l2 2 4-4",
  web: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M3 12h18 M12 3c2.5 2.5 2.5 15 0 18 M12 3c-2.5 2.5-2.5 15 0 18",
  capture: "M4 8h3l2-3h6l2 3h3v11H4z M12 9a4 4 0 1 0 0 8 4 4 0 0 0 0-8z",
  automate: "M13 2 4 14h7l-1 8 9-12h-7z",
  smart: "M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z M19 17l.8 2.2L22 20l-2.2.8L19 23l-.8-2.2L16 20l2.2-.8z",
  tools: "M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.7 2.7-2.6-.7-.7-2.6z",
  system: "M4 7h16 M4 12h16 M4 17h16 M9 5v4 M15 10v4 M7 15v4",
  bell: "M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9z M10 21h4",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M3 12h18",
  moon: "M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z",
  sun: "M12 5V3 M12 21v-2 M5 12H3 M21 12h-2 M6.3 6.3 4.9 4.9 M19.1 19.1l-1.4-1.4 M17.7 6.3l1.4-1.4 M4.9 19.1l1.4-1.4 M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z",
  upload: "M12 16V4 M7 9l5-5 5 5 M4 16v4h16v-4",
  star: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M12 7v5l3 2",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z M16 16l5 5",
  weave: "M3 6c6 0 6 12 12 12h6 M3 12c6 0 6 6 12 6h6 M3 18c6 0 6 0 12 0h6 M6 3c0 6 12 6 12 12v6 M12 3c0 6 6 6 6 12v6 M18 3c0 6 0 6 0 12v6",
};

function Glyph({ name, size = 16 }: { name: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={GLYPHS[name] ?? ""} />
    </svg>
  );
}

/** 分组导航唯一事实源（M2.13 §4：24 视图全部可达；此前 System 组为复制粘贴的第二份渲染）。 */
const NAV_GROUPS: readonly {
  labelKey: string;
  icon: string;
  views: readonly string[];
}[] = [
  { labelKey: "nav.group.files", icon: "files", views: ["rename", "organizer", "duplicates", "batch"] },
  { labelKey: "nav.group.analyze", icon: "analyze", views: ["compare", "space"] },
  { labelKey: "nav.group.content", icon: "content", views: ["text", "data", "image", "documents"] },
  { labelKey: "nav.group.protect", icon: "protect", views: ["archive", "privacy"] },
  { labelKey: "nav.group.web", icon: "web", views: ["web"] },
  { labelKey: "nav.group.capture", icon: "capture", views: ["clipboard", "glint", "wisp"] },
  { labelKey: "nav.group.automate", icon: "automate", views: ["agent", "recipes", "monitor"] },
  { labelKey: "nav.group.smart", icon: "smart", views: ["smart"] },
  { labelKey: "nav.group.tools", icon: "tools", views: ["utilities", "workflow"] },
  { labelKey: "nav.group.system", icon: "system", views: ["history", "jobs", "settings"] },
];

// M2.13 §35：通知 code 前缀 → 目标视图（通知点击精准返回上下文；
// 未映射的 code 返回 undefined = 仅展示，不导航）。
function notifTarget(
  code: string,
): "monitor" | "wisp" | "clipboard" | "smart" | "web" | undefined {
  if (code.startsWith("monitor.")) return "monitor";
  if (code.startsWith("wisp.")) return "wisp";
  if (code.startsWith("clipboard")) return "clipboard";
  if (code.startsWith("smart.")) return "smart";
  if (code.startsWith("web.")) return "web";
  return undefined;
}

function App() {
  const t = useT();
  const {
    appInfo,
    ipcStatus,
    lastError,
    locale,
    theme,
    inspection,
    inspectError,
    scanJob,
    scanStatus,
    hashJob,
    hashStatus,
    paletteOpen,
    favorites,
    toggleFavorite,
  } = useAppStore();
  const [dragOver, setDragOver] = useState(false);
  const [recentHistory, setRecentHistory] = useState<
    { operationId: string; kind: string; summary: string; status: string; timestampMs: number | null }[]
  >([]);
  const [view, setView] = useState<
    | "tools"
    | "rename"
    | "organizer"
    | "duplicates"
    | "compare"
    | "space"
    | "clipboard"
    | "privacy"
    | "archive"
    | "agent"
    | "recipes"
    | "glint"
    | "wisp"
    | "web"
    | "monitor"
    | "smart"
    | "jobs"
    | "settings"
    | "text"
    | "data"
    | "image"
    | "batch"
    | "documents"
    | "utilities"
    | "workflow"
    | "history"
  >("tools");
  const [historyRefresh, setHistoryRefresh] = useState(0);
  // M2.13 §23/§35：通知中心 UI（Agent 通知聚合展示 + 代码前缀 → 视图跳转）
  const [notifications, setNotifications] = useState<
    AgentNotificationDto[]
  >([]);
  const [notifOpen, setNotifOpen] = useState(false);
  const loadNotifications = useCallback(() => {
    commands
      .agentStatus()
      .then((r) => {
        if (r.status === "ok") setNotifications(r.data.notifications);
      })
      .catch(() => {});
  }, []);
  useEffect(() => {
    loadNotifications();
    const timer = setInterval(loadNotifications, 4000);
    return () => clearInterval(timer);
  }, [loadNotifications]);
  // M3 §106：统一 Drop 入口落在 Duplicates 页的目录（面板去重后追加到 roots）。
  const [dupSeedFolder, setDupSeedFolder] = useState<string | null>(null);
  // M4 下 §102：统一 Drop 入口落在文本页的文件（面板直接加载）。
  const [textSeedFile, setTextSeedFile] = useState<string | null>(null);
  const recentRoots = useAppStore((s) => s.recentRoots);
  const hashPollStop = useRef<(() => void) | null>(null);
  const scanPollStop = useRef<(() => void) | null>(null);

  // M11 §16/§17：注册导航命令（真实能力 = setView；keywords 含中英文 §19/§65）
  useEffect(() => {
    const views: [string, string, string][] = [
      ["tools", "nav.tools", "home inspector drop"],
      ["rename", "nav.rename", "batch rename files 重命名"],
      ["organizer", "nav.organizer", "organize files 整理"],
      ["duplicates", "nav.duplicates", "duplicate finder 重复文件"],
      ["compare", "nav.compare", "compare diff folder file 对比"],
      ["space", "nav.space", "space disk usage 空间分析 磁盘"],
      ["clipboard", "nav.clipboard", "clipboard history 剪贴板 paste"],
      ["privacy", "nav.privacy", "privacy share preflight 隐私预检 脱敏"],
      ["archive", "nav.archive", "archive zip integrity hash 归档 完整性"],
      ["agent", "nav.agent", "agent background scheduler 后台 调度"],
      ["recipes", "nav.recipes", "recipe automation 配方 自动化"],
      ["glint", "nav.glint", "glint screenshot capture 截图 屏幕捕获"],
      ["wisp", "nav.wisp", "wisp paste guard sensitive 敏感 粘贴防护 剪贴板"],
      ["web", "nav.web", "web url fetch extract markdown 清理 抓取 网页"],
      ["monitor", "nav.monitor", "website monitor change detection 网站监控 变化检测"],
      ["smart", "nav.smart", "smart rename organize classify cleanup 智能处理 建议"],
      ["text", "nav.text", "text editor 文本 format"],
      ["data", "nav.data", "csv json data 数据"],
      ["image", "nav.image", "image resize convert 图片"],
      ["batch", "nav.batch", "batch execution 批量"],
      ["documents", "nav.documents", "pdf docx xlsx 文档"],
      ["utilities", "nav.utilities", "hash base64 uuid 工具集"],
      ["workflow", "nav.workflow", "workflow pipeline 工作流"],
      ["history", "nav.history", "history undo 历史"],
    ];
    for (const [viewId, labelKey, keywords] of views) {
      upsertCommand({
        id: `navigation.${viewId}`,
        labelKey,
        category: "navigation",
        keywords: keywords.split(" "),
        run: () => setView(viewId as typeof view),
      });
    }
    upsertCommand({
      id: "app.toggle-language",
      labelKey: "app.language",
      category: "settings",
      keywords: ["language", "语言", "english", "中文"],
      run: () => {
        const s = useAppStore.getState();
        const next = s.locale === "zh-CN" ? "en" : "zh-CN";
        s.setLocale(next);
        // M2.9 §32：整体替换语义 ⇒ 全量 round-trip（否则 clipboard/wisp 被重置）
        void import("../lib/bootstrap").then((m) => m.persistLocale(next));
      },
    });
    // M2.8 §46：捕获命令（导航到 Glint + 触发捕获请求）
    const glintCapture = (kind: "screen" | "window") => {
      setView("glint");
      useAppStore.getState().setGlintCaptureRequest({ kind, tick: Date.now() });
    };
    upsertCommand({
      id: "glint.capture-full-screen",
      labelKey: "glint.capture.full",
      category: "tools",
      keywords: ["capture", "screen", "fullscreen", "screenshot", "全屏截图", "截图"],
      run: () => glintCapture("screen"),
    });
    upsertCommand({
      id: "glint.capture-window",
      labelKey: "glint.capture.window",
      category: "tools",
      keywords: ["capture", "window", "screenshot", "窗口截图", "截图"],
      run: () => glintCapture("window"),
    });
    // M2.9 §31：Paste Guard 命令（状态面板如实呈现 enabled/paused）
    upsertCommand({
      id: "wisp.open",
      labelKey: "nav.wisp",
      category: "tools",
      keywords: ["wisp", "paste", "guard", "敏感", "粘贴", "防护"],
      run: () => setView("wisp"),
    });
    upsertCommand({
      id: "wisp.toggle-paused",
      labelKey: "wisp.action.togglePaused",
      category: "tools",
      keywords: ["wisp", "pause", "resume", "暂停", "恢复"],
      run: () => {
        setView("wisp");
        void commands.wispStatus().then((st) => {
          void commands.wispSetPaused(!(st.paused ?? false));
        });
      },
    });
    upsertCommand({
      id: "wisp.recent-events",
      labelKey: "wisp.events.title",
      category: "tools",
      keywords: ["wisp", "security", "events", "安全", "事件"],
      run: () => setView("wisp"),
    });
    // M2.10 §61：Web 命令（导航；输入预填由面板状态承担）
    upsertCommand({
      id: "web.open",
      labelKey: "nav.web",
      category: "tools",
      keywords: ["web", "url", "fetch", "抓取", "网页", "清理"],
      run: () => setView("web"),
    });
    upsertCommand({
      id: "web.fetch-url",
      labelKey: "web.action.fetch",
      category: "tools",
      keywords: ["fetch", "url", "抓取"],
      run: () => setView("web"),
    });
    upsertCommand({
      id: "monitor.open",
      labelKey: "nav.monitor",
      category: "tools",
      keywords: ["monitor", "website", "change", "网站监控", "变化"],
      run: () => setView("monitor"),
    });
    upsertCommand({
      id: "smart.open",
      labelKey: "nav.smart",
      category: "tools",
      keywords: ["smart", "rename", "organize", "classify", "智能", "整理", "建议"],
      run: () => setView("smart"),
    });
    upsertCommand({
      id: "web.clean-url",
      labelKey: "web.action.clean",
      category: "tools",
      keywords: ["clean", "url", "tracking", "清理", "追踪"],
      run: () => setView("web"),
    });
  }, []);
  const startHash = (path: string): void => {
    hashPollStop.current?.();
    void commands
      .hashFile(path)
      .then((result) => {
        if (result.status === "ok") {
          useAppStore.getState().setHashJob({ jobId: result.data.jobId, path });
          hashPollStop.current = pollJob(result.data.jobId, (tick) => {
            if (tick.kind === "status") {
              useAppStore.getState().setHashStatus(tick.status);
              if (isTerminalState(tick.status)) {
                hashPollStop.current?.();
                const hash = tick.status.hash;
                if (hash) {
                  useAppStore.getState().setHashResult(hash);
                }
              }
            } else {
              useAppStore.getState().setError(tick.error);
            }
          });
        } else {
          useAppStore.getState().setError(result.error);
        }
      })
      .catch(() => useAppStore.getState().setIpcStatus("failed"));
  };

  const startScan = (path: string): void => {
    scanPollStop.current?.();
    void commands
      .analyzeDirectory(path, null)
      .then((result) => {
        if (result.status === "ok") {
          useAppStore.getState().setScanJob({ jobId: result.data.jobId, path });
          scanPollStop.current = pollJob(result.data.jobId, (tick) => {
            if (tick.kind === "status") {
              useAppStore.getState().setScanStatus(tick.status);
              if (isTerminalState(tick.status)) {
                scanPollStop.current?.();
              }
            } else {
              useAppStore.getState().setError(tick.error);
            }
          });
        } else {
          useAppStore.getState().setError(result.error);
        }
      })
      .catch(() => useAppStore.getState().setIpcStatus("failed"));
  };

  const dispatchPath = (path: string): void => {
    // 先 inspect 判定 File / Directory，再分发（M1 §20/§21）。
    void commands
      .inspectFile(path)
      .then((result) => {
        if (result.status !== "ok") {
          useAppStore.getState().setInspectError(result.error);
          return;
        }
        if (result.data.kind === "directory") {
          if (view === "duplicates") {
            // §106：Duplicates 页复用全局 Drop 作为扫描根入口，不另造第二套。
            setDupSeedFolder(result.data.normalizedPath);
          } else {
            startScan(result.data.normalizedPath);
          }
        } else if (view === "text") {
          // M4 下 §102：文本页 Drop 文本文件 → 直接加载（二进制 ⇒ 结构化错误）
          setTextSeedFile(result.data.normalizedPath);
        } else {
          useAppStore.getState().setInspection(result.data);
        }
      })
      .catch(() => useAppStore.getState().setIpcStatus("failed"));
  };

  // M11 §51/§55：全局快捷键单一分发（Mod+K = palette；注册进 ShortcutRegistry）
  useEffect(() => {
    registerShortcut({
      id: "app.open-command-palette",
      keys: "Mod+K",
      scope: "global",
      descriptionKey: "palette.title",
      action: () => {
        const s = useAppStore.getState();
        s.setPaletteOpen(!s.paletteOpen);
      },
    });
    // M2.14 §26：Ctrl/Cmd+P = Quick Open（同一 palette 入口）
    registerShortcut({
      id: "app.quick-open",
      keys: "Mod+P",
      scope: "global",
      descriptionKey: "palette.title",
      action: () => {
        const s = useAppStore.getState();
        s.setPaletteOpen(true);
      },
    });
    const handler = (e: KeyboardEvent): void => {
      // Esc：先关通知抽屉（M2.14 通知抽屉），再走全局分发（palette 自身处理 Esc）
      if (e.key === "Escape") {
        setNotifOpen(false);
      }
      dispatchShortcut(e);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  useEffect(() => {
    if (import.meta.env.DEV) {
      // Dev-only 自动化冒烟入口（DECISIONS D26）：触发与 Drop 完全相同的
      // dispatchPath 链路（校验 → IPC → Rust → store → 渲染）。
      // 生产构建中 import.meta.env.DEV 为常量 false，此分支被死码消除。
      (window as { __weaveDev?: unknown }).__weaveDev = { dispatch: dispatchPath };
    }
  });

  // M11 §43/§45：Recent = M2 History 最新条目（真实数据，非前端 fake）
  useEffect(() => {
    if (view !== "tools") {
      return;
    }
    let cancelled = false;
    void commands
      .getHistory(8)
      .then((r) => {
        if (r.status === "ok" && !cancelled) {
          setRecentHistory(
            r.data.map((e) => ({
              operationId: e.operationId,
              kind: e.kind,
              summary: e.summary,
              status: e.status,
              timestampMs: e.timestampMs,
            })),
          );
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRecentHistory([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [view, historyRefresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    // Drop → Path Validation → File/Directory 判定 → Inspector / Analyzer（M1 §21）。
    // 非 Tauri 环境（Vite 浏览器预览 / 单测）里 __TAURI_INTERNALS__ 缺失，
    // getCurrentWebview() 会同步抛错；此前只 catch 了 Promise，导致整树卸载白屏。
    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            setDragOver(true);
          } else if (event.payload.type === "drop") {
            setDragOver(false);
            const path = event.payload.paths[0];
            if (path) {
              dispatchPath(path);
            }
          } else {
            setDragOver(false);
          }
        })
        .then((stop) => {
          if (disposed) {
            stop();
          } else {
            unlisten = stop;
          }
        })
        .catch(() => {
          // 非 Tauri 环境（浏览器/测试）没有拖放事件，不视为失败。
        });
    } catch {
      // 拖放能力不可用，界面照常渲染（能力降级，不伪装成失败）。
    }
    return () => {
      disposed = true;
      unlisten?.();
      hashPollStop.current?.();
      scanPollStop.current?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openFromScan = (relativePath: string): void => {
    if (!scanJob) {
      return;
    }
    dispatchPath(scanJob.path + SEP + relativePath);
  };

  const hashView: HashJobView = (() => {
    if (hashStatus?.state === "running" || (hashJob !== null && hashStatus === null)) {
      return { phase: "running", processed: hashStatus?.progressCurrent ?? null };
    }
    if (hashStatus?.state === "failed") {
      const error: IpcError = hashStatus.error ?? {
        kind: "internal",
        code: "job.failed",
        message: "hash job failed",
        location: null,
        recoverability: "fatal",
        suggestion: null,
      };
      return { phase: "failed", error };
    }
    if (hashStatus?.state === "completed" && hashStatus.hash) {
      return { phase: "done", result: hashStatus.hash };
    }
    return { phase: "idle" };
  })();

  const openViaDialog = async (directory: boolean): Promise<void> => {
    // M1 §20：Open File / Open Folder 与 Drop 同级的入口（dialog 为用户显式发起）。
    try {
      const selected = await openDialog({
        multiple: false,
        directory,
        title: directory ? t("open.folder") : t("open.file"),
      });
      if (typeof selected === "string" && selected) {
        dispatchPath(selected);
      }
    } catch (e) {
      // 对话框失败必须可见（本机曾出现插件挂起——见 Known Limitations）。
      useAppStore.getState().setError(
        e instanceof Error
          ? {
              kind: "internal",
              code: "dialog.failed",
              message: e.message,
              location: null,
              recoverability: "retryable",
              suggestion: null,
            }
          : (e as IpcError),
      );
    }
  };

  const toggleLocale = (): void => {
    const next: Locale = locale === "zh-CN" ? "en" : "zh-CN";
    useAppStore.getState().setLocale(next);
    // M2.9 §32：整体替换语义 ⇒ 全量 round-trip
    void import("../lib/bootstrap").then((m) => m.persistLocale(next));
  };

  const scanTerminal = scanStatus !== null && isTerminalState(scanStatus);

  const viewLabel = view === "tools" ? t("nav.home") : t(`nav.${view}`);

  return (
    <div className="w-shell">
      {/* ── 侧栏：品牌 / 命令入口 / 分组导航 / 运行状态 ── */}
      <aside className="w-sidebar" aria-label={t("app.nav")}>
        <div className="w-brand">
          <span className="w-brand-mark">
            <Glyph name="weave" size={22} />
          </span>
          <div>
            <h1>{appInfo?.name ?? "Weave"}</h1>
            <div className="w-brand-sub">{appInfo?.vendor ?? "ifuyo"}</div>
          </div>
        </div>

        <button
          type="button"
          className="w-palette-trigger"
          onClick={() => useAppStore.getState().setPaletteOpen(true)}
        >
          <Glyph name="search" size={15} />
          <span>{t("palette.title")}</span>
          <kbd className="w-kbd">⌘K</kbd>
        </button>

        <nav className="w-nav" aria-label={t("app.nav")}>
          <div className="w-nav-group">
            <button
              type="button"
              className={`w-nav-item ${view === "tools" ? "is-active" : ""}`}
              aria-current={view === "tools" ? "page" : undefined}
              onClick={() => setView("tools")}
            >
              <Glyph name="home" />
              {t("nav.home")}
            </button>
          </div>
          {NAV_GROUPS.map((group) => (
            <div className="w-nav-group" key={group.labelKey}>
              <div className="w-nav-group-title">{t(group.labelKey)}</div>
              {group.views.map((v) => (
                <button
                  key={v}
                  type="button"
                  className={`w-nav-item ${view === v ? "is-active" : ""}`}
                  aria-current={view === v ? "page" : undefined}
                  onClick={() => setView(v as typeof view)}
                >
                  <Glyph name={group.icon} />
                  {t(`nav.${v}`)}
                </button>
              ))}
            </div>
          ))}
        </nav>

        <div className="w-sidebar-foot">
          <span
            role="status"
            className="w-status"
            style={{ color: ipcStatus === "connected" ? "var(--color-success)" : "var(--color-danger)" }}
          >
            <span
              className={`w-dot ${ipcStatus === "connected" ? "" : "is-bad"}`}
            />
            {t(`ipc.status.${ipcStatus}`)}
          </span>
          <span className="w-milestone">{t("app.milestone")}</span>
        </div>
      </aside>

      {/* ── 主区：状态栏 + 内容 ── */}
      <main className="w-main">
        <header className="w-topbar">
          <div className="w-topbar-title">
            <h2>{viewLabel}</h2>
            <span className="w-topbar-sub">{t("app.paletteHint")}</span>
          </div>
          <div className="w-topbar-actions">
            {appInfo ? (
              <span className="w-pill w-version">
                v{appInfo.version} · {appInfo.environment}
              </span>
            ) : null}
            <button
              type="button"
              className="w-icon-btn"
              title={t("app.language")}
              onClick={toggleLocale}
            >
              <Glyph name="globe" size={15} />
              {locale === "zh-CN" ? "English" : "中文"}
            </button>
            <button
              type="button"
              className="w-icon-btn"
              onClick={() => {
                const next: Theme =
                  theme === "system" ? "light" : theme === "light" ? "dark" : "system";
                useAppStore.getState().setTheme(next);
                // 传输层不可达（浏览器预览）时主题已即时生效；结构化错误由
                // persistAppConfig 写入 store，不在此吞掉业务错误。
                void persistTheme(next).catch(() => {});
              }}
              aria-label={t("app.theme")}
              title={t("app.theme")}
            >
              <Glyph name={theme === "dark" ? "moon" : "sun"} size={15} />
              {t(`app.themeName.${theme}`)}
            </button>
            <button
              type="button"
              className="w-icon-btn"
              onClick={() => setNotifOpen((o) => !o)}
              aria-label={t("notif.title")}
              title={t("notif.title")}
            >
              <Glyph name="bell" size={15} />
              {notifications.length > 0 ? (
                <span className="w-badge">{notifications.length}</span>
              ) : null}
            </button>
          </div>

          {notifOpen ? (
            <div
              role="dialog"
              aria-label={t("notif.title")}
              className="w-notif"
            >
              <div className="w-notif-head">
                <strong>{t("notif.title")}</strong>
                <button
                  type="button"
                  onClick={() => {
                    commands
                      .agentClearNotifications()
                      .then(loadNotifications)
                      .catch(() => {});
                  }}
                >
                  {t("notif.clear")}
                </button>
              </div>
              {notifications.length === 0 ? (
                <div className="w-empty">{t("notif.empty")}</div>
              ) : (
                notifications.map((n) => {
                  const targetView = notifTarget(n.code);
                  return (
                    <div className="w-notif-item" key={n.code}>
                      <div className="w-notif-row">
                        <strong>
                          {(n.count ?? 0) > 1 ? `×${n.count ?? 0} ` : ""}
                          {n.code}
                        </strong>
                        <button
                          type="button"
                          onClick={() => {
                            commands
                              .agentDismissNotification(n.code)
                              .then(loadNotifications)
                              .catch(() => {});
                          }}
                        >
                          {t("notif.dismiss")}
                        </button>
                      </div>
                      <div>{n.message}</div>
                      {targetView ? (
                        <button
                          type="button"
                          onClick={() => {
                            setNotifOpen(false);
                            setView(targetView);
                          }}
                        >
                          {t("notif.open")}
                        </button>
                      ) : null}
                    </div>
                  );
                })
              )}
            </div>
          ) : null}
        </header>

        <div className="w-content" key={view}>
          {lastError ? (
            <section role="alert" className="w-alert">
              <strong>{t("error.title")}</strong>
              <div style={{ fontSize: "var(--typography-size-sm)" }}>
                {lastError.code} · {lastError.message}
              </div>
              {lastError.suggestion ? (
                <div style={{ fontSize: "var(--typography-size-sm)" }}>
                  {t("error.suggestion")}: {lastError.suggestion}
                </div>
              ) : null}
            </section>
          ) : null}

          {view === "tools" ? (
            <section className="w-hero w-fade">
              <div className="w-hero-copy">
                <span className="w-hero-badge">{t("app.localOnly")}</span>
                <h2>{t("app.tagline")}</h2>
              </div>
              <div className="w-hero-art">
                <Glyph name="weave" size={132} />
              </div>
            </section>
          ) : null}

          {view === "tools" ? (
            <section
              aria-label={t("drop.hint")}
              className={`w-drop w-fade ${dragOver ? "is-over" : ""}`}
            >
              <div className="w-drop-icon">
                <Glyph name="upload" size={22} />
              </div>
              <div className="w-drop-title">{t("app.dropTitle")}</div>
              <div className="w-drop-hint">{t("drop.hint")}</div>
              <div className="w-actions">
                <span className="w-drop-hint">{t("app.quickActions")}</span>
                <button type="button" onClick={() => void openViaDialog(false)} style={buttonStyle}>
                  {t("open.file")}
                </button>
                <button type="button" onClick={() => void openViaDialog(true)} style={buttonStyle}>
                  {t("open.folder")}
                </button>
              </div>
            </section>
          ) : null}

          {/* 工具面板：统一卡片容器（无面板时 :empty 自动隐藏） */}
          <div className="w-panel w-fade">
            {view === "rename" ? (
              <RenamePanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "organizer" ? (
              <OrganizerPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "text" ? (
              <TextPanel
                onOperationDone={() => setHistoryRefresh((n) => n + 1)}
                seedFile={textSeedFile}
                onSeedConsumed={() => setTextSeedFile(null)}
              />
            ) : null}

            {view === "recipes" ? (
              <RecipePanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "glint" ? (
              <GlintPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "wisp" ? (
              <WispPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "web" ? (
              <WebPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "monitor" ? (
              <MonitorPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "smart" ? (
              <SmartPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "jobs" ? <JobsPanel /> : null}

            {view === "settings" ? <SettingsPanel /> : null}

            {view === "agent" ? <AgentPanel locale={locale} /> : null}

            {view === "archive" ? (
              <ArchivePanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "privacy" ? (
              <PrivacyPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "clipboard" ? (
              <ClipboardPanel locale={locale} onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "space" ? (
              <SpacePanel locale={locale} onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "compare" ? (
              <ComparePanel locale={locale} onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "duplicates" ? (
              <DuplicatesPanel
                onOperationDone={() => setHistoryRefresh((n) => n + 1)}
                recentRoots={recentRoots}
                seedFolder={dupSeedFolder}
                onSeedConsumed={() => setDupSeedFolder(null)}
                onScanCompleted={(roots) =>
                  useAppStore.getState().setRecentRoots([...new Set(roots)].slice(0, 5))
                }
              />
            ) : null}

            {view === "data" ? (
              <DataPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "image" ? (
              <ImagePanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "batch" ? (
              <BatchPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "documents" ? (
              <DocumentsPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "utilities" ? <UtilitiesPanel /> : null}

            {view === "workflow" ? (
              <WorkflowPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
            ) : null}

            {view === "history" ? (
              <HistoryPanel
                locale={locale}
                refreshSignal={historyRefresh}
                onUndoDone={() => setHistoryRefresh((n) => n + 1)}
              />
            ) : null}
          </div>

          {view === "tools" ? (
            <div className="w-grid w-fade">
              {/* M11 §34/§42：首页收藏（引用而非第二套 Registry） */}
              <section className="w-card" aria-label={t("favorites.title")}>
                <div className="w-card-head">
                  <div className="w-card-title">
                    <span className="w-card-icon">
                      <Glyph name="star" size={15} />
                    </span>
                    {t("favorites.title")}
                  </div>
                </div>
                {favorites.length === 0 ? (
                  <div className="w-empty">{t("favorites.empty")}</div>
                ) : (
                  <ul className="w-list">
                    {favorites.map((f) => (
                      <li key={`${f.kind}:${f.id}`}>
                        <button
                          type="button"
                          className="w-link"
                          onClick={() => {
                            if (f.kind === "view") {
                              setView(f.id as typeof view);
                            } else {
                              setView("workflow");
                            }
                          }}
                        >
                          {f.kind === "workflow"
                            ? t("favorites.workflowPrefix")
                            : t(`nav.${f.id}`, { defaultValue: f.id })}
                        </button>
                        <button
                          type="button"
                          className="w-icon-btn"
                          aria-label={t("favorites.remove")}
                          onClick={() => {
                            toggleFavorite(f);
                            void import("../lib/bootstrap").then((m) => m.persistFavorites());
                          }}
                        >
                          ×
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              {/* M11 §43/§45：Recent Operations（真实 M2 History） */}
              <section className="w-card" aria-label={t("recent.title")}>
                <div className="w-card-head">
                  <div className="w-card-title">
                    <span className="w-card-icon">
                      <Glyph name="clock" size={15} />
                    </span>
                    {t("recent.title")}
                  </div>
                </div>
                {recentHistory.length === 0 ? (
                  <div className="w-empty">{t("recent.empty")}</div>
                ) : (
                  <ul className="w-list">
                    {recentHistory.map((r) => (
                      <li key={r.operationId}>
                        <span>{r.summary}</span>
                        <span style={{ color: "var(--color-text-muted)" }}>{r.status}</span>
                      </li>
                    ))}
                  </ul>
                )}
                <button
                  type="button"
                  onClick={() => setView("history")}
                  style={{ ...pillStyle, alignSelf: "flex-start", fontSize: "var(--typography-size-sm)" }}
                >
                  {t("recent.openHistory")}
                </button>
              </section>
            </div>
          ) : null}

          {view === "tools" && inspection ? (
            <div className="w-chips" aria-live="polite">
              <span className="w-drop-hint">{t("quickDrop.suggestions")}</span>
              {suggestionsFor(inspection.kind, inspection.extension ?? "").map((viewId) => (
                <button
                  key={viewId}
                  type="button"
                  className="w-chip"
                  onClick={() => setView(viewId as typeof view)}
                >
                  {t(`nav.${viewId}`, { defaultValue: viewId })}
                </button>
              ))}
            </div>
          ) : null}

          {view === "tools" && inspectError ? (
            <div className="w-alert" aria-live="polite">
              <strong>{t("error.title")}</strong>
              <span>
                {inspectError.code} · {inspectError.message}
              </span>
            </div>
          ) : null}

          {view === "tools" && inspection ? (
            <InspectorPanel
              inspection={inspection}
              locale={locale}
              hash={hashView}
              onStartHash={() => startHash(inspection.normalizedPath)}
              onCancelHash={() => {
                if (hashJob) {
                  void commands.cancelJob(hashJob.jobId);
                }
              }}
            />
          ) : null}

          {view === "tools" && scanJob ? (
            <ScanPanel
              root={scanJob.path}
              status={scanStatus}
              onCancel={() => {
                void commands.cancelJob(scanJob.jobId);
              }}
              onOpenFile={openFromScan}
            />
          ) : null}

          {scanTerminal && scanStatus?.scan && scanStatus.scan.status === "cancelled" ? (
            <div className="w-drop-hint" aria-live="polite">
              {t("scan.cancelledNotice", { count: scanStatus.scan.entriesProcessed ?? 0 })}
            </div>
          ) : null}
        </div>
      </main>

      <CommandPalette
        key={paletteOpen ? "open" : "closed"}
        open={paletteOpen}
        onClose={() => useAppStore.getState().setPaletteOpen(false)}
        onExecuteError={(message) =>
          useAppStore.getState().setError({
            kind: "internal",
            code: "command.executeFailed",
            message,
            location: null,
            recoverability: "retryable",
            suggestion: null,
          })
        }
      />
    </div>
  );
}

export default App;
