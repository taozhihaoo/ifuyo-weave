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

  return (
    <main
      style={{
        minHeight: "100%",
        padding: "var(--spacing-xl)",
        maxWidth: "1080px",
        margin: "0 auto",
        display: "flex",
        flexDirection: "column",
        gap: "var(--spacing-lg)",
      }}
    >
      <header style={{ display: "flex", alignItems: "baseline", gap: "var(--spacing-md)" }}>
        <h1 style={{ margin: 0, fontSize: "var(--typography-size-title)", fontWeight: 650, letterSpacing: "-0.02em" }}>
          {appInfo?.name ?? "Weave"}
        </h1>
        <span style={{ color: "var(--color-text-muted)" }}>{appInfo?.vendor ?? "ifuyo"}</span>
        {appInfo ? (
          <span
            style={{
              padding: "2px 10px",
              borderRadius: "var(--radius-full)",
              border: "1px solid var(--color-border)",
              fontSize: "var(--typography-size-sm)",
            }}
          >
            v{appInfo.version} · {appInfo.environment}
          </span>
        ) : null}
        <button type="button" onClick={toggleLocale} style={pillStyle}>
          {locale === "zh-CN" ? "English" : "中文"}
        </button>
        <button
          type="button"
          onClick={() => {
            const next: Theme =
              theme === "system" ? "light" : theme === "light" ? "dark" : "system";
            useAppStore.getState().setTheme(next);
            void persistTheme(next);
          }}
          style={pillStyle}
          aria-label={t("app.theme")}
        >
          {t(`app.themeName.${theme}`)}
        </button>
        <button
          type="button"
          onClick={() => setNotifOpen((o) => !o)}
          style={{ ...pillStyle, position: "relative" }}
          aria-label={t("notif.title")}
        >
          {t("notif.title")}
          {notifications.length > 0 ? (
            <span
              style={{
                position: "absolute",
                top: -6,
                right: -6,
                background: "var(--color-danger)",
                color: "var(--color-surface)",
                borderRadius: "var(--radius-full)",
                fontSize: 11,
                padding: "0 5px",
              }}
            >
              {notifications.length}
            </span>
          ) : null}
        </button>
        {notifOpen ? (
          <div
            role="dialog"
            aria-label={t("notif.title")}
            style={{
              position: "absolute",
              top: 60,
              right: 24,
              zIndex: 30,
              width: 360,
              maxHeight: 420,
              overflow: "auto",
              background: "var(--color-surface)",
              border: "1px solid var(--color-border)",
              borderRadius: "var(--radius-md)",
              padding: "var(--spacing-sm)",
              boxShadow: "var(--shadow-lg)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong>{t("notif.title")}</strong>
              <button onClick={() => { commands.agentClearNotifications().then(loadNotifications).catch(() => {}); }}>
                {t("notif.clear")}
              </button>
            </div>
            {notifications.length === 0 ? (
              <p style={{ fontSize: 12 }}>{t("notif.empty")}</p>
            ) : (
              notifications.map((n) => (
                <div
                  key={n.code}
                  style={{
                    borderTop: "1px solid var(--color-border)",
                    padding: "var(--spacing-xs) 0",
                    fontSize: 12,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                    <strong>
                      {(n.count ?? 0) > 1 ? `×${n.count ?? 0} ` : ""}
                      {n.code}
                    </strong>
                    <button onClick={() => { commands.agentDismissNotification(n.code).then(loadNotifications).catch(() => {}); }}>
                      {t("notif.dismiss")}
                    </button>
                  </div>
                  <div>{n.message}</div>
                  {(() => {
                    const targetView = notifTarget(n.code);
                    return targetView ? (
                      <button
                        onClick={() => {
                          setNotifOpen(false);
                          setView(targetView);
                        }}
                      >
                        {t("notif.open")}
                      </button>
                    ) : null;
                  })()}
                </div>
              ))
            )}
          </div>
        ) : null}
      </header>

      <nav
        aria-label={t("app.nav")}
        style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}
      >
        {(
          [
            {
              labelKey: "nav.group.files",
              views: ["rename", "organizer", "duplicates", "batch"] as const,
            },
            {
              labelKey: "nav.group.analyze",
              views: ["compare", "space"] as const,
            },
            {
              labelKey: "nav.group.content",
              views: ["text", "data", "image", "documents"] as const,
            },
            {
              labelKey: "nav.group.protect",
              views: ["archive", "privacy"] as const,
            },
            {
              labelKey: "nav.group.web",
              views: ["web"] as const,
            },
            {
              labelKey: "nav.group.capture",
              views: ["clipboard", "glint", "wisp"] as const,
            },
            {
              labelKey: "nav.group.automate",
              views: ["agent", "recipes", "monitor"] as const,
            },
            {
              labelKey: "nav.group.smart",
              views: ["smart"] as const,
            },
            {
              labelKey: "nav.group.tools",
              views: ["utilities", "workflow"] as const,
            },
          ] as const
        ).map((group) => (
          <div key={group.labelKey} style={{ display: "flex", gap: "var(--spacing-xs)", alignItems: "baseline", flexWrap: "wrap" }}>
            <span
              style={{
                fontSize: "var(--typography-size-sm)",
                color: "var(--color-text-muted)",
                minWidth: 72,
              }}
            >
              {t(group.labelKey)}
            </span>
            {group.views.map((v) => (
              <button
                key={v}
                type="button"
                onClick={() => setView(v)}
                aria-current={view === v ? "page" : undefined}
                style={{
                  ...pillStyle,
                  background: view === v ? "var(--color-accent-soft)" : "var(--color-surface)",
                  borderColor: view === v ? "var(--color-accent)" : "var(--color-border)",
                  color: view === v ? "var(--color-accent)" : "inherit",
                  fontWeight: view === v ? 600 : 400,
                }}
              >
                {t(`nav.${v}`)}
              </button>
            ))}
          </div>
        ))}
        <div style={{ display: "flex", gap: "var(--spacing-xs)", alignItems: "baseline", flexWrap: "wrap" }}>
          <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)", minWidth: 72 }}>
            {t("nav.group.system")}
          </span>
          {(["history", "jobs", "settings"] as const).map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => setView(v)}
              aria-current={view === v ? "page" : undefined}
              style={{
                ...pillStyle,
                background: view === v ? "var(--color-accent-soft)" : "var(--color-surface)",
                borderColor: view === v ? "var(--color-accent)" : "var(--color-border)",
                color: view === v ? "var(--color-accent)" : "inherit",
                fontWeight: view === v ? 600 : 400,
              }}
            >
              {t(`nav.${v}`)}
            </button>
          ))}
        </div>
      </nav>

      <section
        style={{
          display: "flex",
          gap: "var(--spacing-md)",
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <span style={{ fontSize: "var(--typography-size-lg)" }}>{t("app.milestone")}</span>
        <span
          role="status"
          style={{
            color: ipcStatus === "connected" ? "var(--color-success)" : "var(--color-danger)",
            fontSize: "var(--typography-size-sm)",
          }}
        >
          {t(`ipc.status.${ipcStatus}`)}
        </span>
      </section>

      {lastError ? (
        <section
          role="alert"
          style={{
            border: "1px solid var(--color-danger)",
            borderRadius: "var(--radius-md)",
            padding: "var(--spacing-md)",
            background: "var(--color-surface)",
          }}
        >
          <strong>{t("error.title")}</strong>
          <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {lastError.code} · {lastError.message}
          </div>
          {lastError.suggestion ? (
            <div style={{ fontSize: "var(--typography-size-sm)" }}>
              {t("error.suggestion")}: {lastError.suggestion}
            </div>
          ) : null}
        </section>
      ) : null}

      {/* M2.14 §42/§71：开发期命令按钮墙已移除——命令统一走 Command
          Palette（Ctrl+K / Ctrl+P），模块入口走分组导航 */}
      <section style={{ display: "flex", gap: "var(--spacing-sm)", alignItems: "center" }}>
        <button type="button" onClick={() => void openViaDialog(false)} style={buttonStyle}>
          {t("open.file")}
        </button>
        <button type="button" onClick={() => void openViaDialog(true)} style={buttonStyle}>
          {t("open.folder")}
        </button>
        <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          {t("app.paletteHint")}
        </span>
      </section>

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

      {view === "agent" ? (
        <AgentPanel locale={locale} />
      ) : null}

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

      {view === "tools" ? (
        <section
          aria-label={t("drop.hint")}
          style={{
            border: "2px dashed " + (dragOver ? "var(--color-accent)" : "var(--color-border)"),
            borderRadius: "var(--radius-lg)",
            padding: "var(--spacing-xxl)",
            textAlign: "center",
            fontSize: "var(--typography-size-lg)",
            color: dragOver ? "var(--color-accent)" : "var(--color-text-muted)",
            background: dragOver ? "var(--color-accent-soft)" : "var(--color-surface)",
            boxShadow: "var(--shadow-sm)",
            transition:
              "background var(--motion-normal) var(--motion-ease), border-color var(--motion-normal) var(--motion-ease)",
          }}
        >
          {t("drop.hint")}
        </section>
      ) : null}

      {view === "tools" ? (
        <>
          {/* M11 §34/§42：首页收藏（引用而非第二套 Registry） */}
          <section
            aria-label={t("favorites.title")}
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "var(--spacing-sm)",
              background: "var(--color-surface)",
              border: "1px solid var(--color-border)",
              borderRadius: "var(--radius-lg)",
              boxShadow: "var(--shadow-sm)",
              padding: "var(--spacing-lg)",
            }}
          >
            <strong style={{ fontSize: "var(--typography-size-sm)", letterSpacing: "0.02em" }}>{t("favorites.title")}</strong>
            {favorites.length === 0 ? (
              <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("favorites.empty")}
              </div>
            ) : (
              favorites.map((f) => (
                <div key={`${f.kind}:${f.id}`} style={{ display: 'flex', gap: 'var(--spacing-xs)', alignItems: 'center' }}>
                  <button
                    type="button"
                    onClick={() => {
                      if (f.kind === "view") {
                        setView(f.id as typeof view);
                      } else {
                        setView("workflow");
                      }
                    }}
                    style={buttonStyle}
                  >
                    {f.kind === "workflow"
                      ? t("favorites.workflowPrefix")
                      : t(`nav.${f.id}`, { defaultValue: f.id })}
                  </button>
                  <button
                    type="button"
                    aria-label={t("favorites.remove")}
                    onClick={() => {
                      toggleFavorite(f);
                      void import("../lib/bootstrap").then((m) => m.persistFavorites());
                    }}
                    style={buttonStyle}
                  >
                    {t("favorites.remove")}
                  </button>
                </div>
              ))
            )}
          </section>

          {/* M11 §43/§45：Recent Operations（真实 M2 History） */}
          <section
            aria-label={t("recent.title")}
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "var(--spacing-sm)",
              background: "var(--color-surface)",
              border: "1px solid var(--color-border)",
              borderRadius: "var(--radius-lg)",
              boxShadow: "var(--shadow-sm)",
              padding: "var(--spacing-lg)",
            }}
          >
            <strong style={{ fontSize: "var(--typography-size-sm)", letterSpacing: "0.02em" }}>{t("recent.title")}</strong>
            {recentHistory.length === 0 ? (
              <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("recent.empty")}
              </div>
            ) : (
              <ul style={{ margin: 0, paddingLeft: "1.2em", fontSize: "var(--typography-size-sm)" }}>
                {recentHistory.map((r) => (
                  <li key={r.operationId}>
                    {r.summary} · <span style={{ color: "var(--color-text-muted)" }}>{r.status}</span>
                  </li>
                ))}
              </ul>
            )}
            <button
              type="button"
              onClick={() => setView("history")}
              style={{ ...buttonStyle, alignSelf: "flex-start", fontSize: "var(--typography-size-sm)" }}
            >
              {t("recent.openHistory")}
            </button>
          </section>
        </>
      ) : null}

      {view === "tools" && inspection ? (
        <div style={{ display: 'flex', gap: 'var(--spacing-sm)', alignItems: 'center', flexWrap: 'wrap' }} role='status'>
          <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {t("quickDrop.suggestions")}
          </span>
          {suggestionsFor(inspection.kind, inspection.extension ?? '').map((viewId) => (
            <button
              key={viewId}
              type="button"
              onClick={() => setView(viewId as typeof view)}
              style={buttonStyle}
            >
              {t(`nav.${viewId}`, { defaultValue: viewId })}
            </button>
          ))}
        </div>
      ) : null}

      {view === "tools" && inspectError ? (
        <section
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {t("error.title")}: {inspectError.code} · {inspectError.message}
        </section>
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
        <div
          role="status"
          style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-sm)" }}
        >
          {t("scan.cancelledNotice", { count: scanStatus.scan.entriesProcessed ?? 0 })}
        </div>
      ) : null}
    </main>
  );
}

export default App;
