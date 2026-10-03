import { useEffect, useRef, useState, type CSSProperties } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listCommands, runCommand } from "../commands/registry";
import type { IpcError } from "../generated/bindings";
import { InspectorPanel, type HashJobView } from "../features/files/InspectorPanel";
import { DuplicatesPanel } from "../features/ops/DuplicatesPanel";
import { HistoryPanel } from "../features/ops/HistoryPanel";
import { OrganizerPanel } from "../features/ops/OrganizerPanel";
import { RenamePanel } from "../features/ops/RenamePanel";
import { ScanPanel } from "../features/files/ScanPanel";
import { commands } from "../generated/bindings";
import { useT } from "../i18n";
import { isTerminalState, pollJob } from "../lib/jobs";
import { useAppStore, type Locale } from "../stores/appStore";

const SEP = String.fromCharCode(92); // path separator on Windows

const buttonStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
};

function App() {
  const t = useT();
  const {
    appInfo,
    ipcStatus,
    lastError,
    locale,
    inspection,
    inspectError,
    scanJob,
    scanStatus,
    hashJob,
    hashStatus,
  } = useAppStore();
  const [dragOver, setDragOver] = useState(false);
  const [view, setView] = useState<"tools" | "rename" | "organizer" | "duplicates" | "history">(
    "tools",
  );
  const [historyRefresh, setHistoryRefresh] = useState(0);
  // M3 §106：统一 Drop 入口落在 Duplicates 页的目录（面板去重后追加到 roots）。
  const [dupSeedFolder, setDupSeedFolder] = useState<string | null>(null);
  const recentRoots = useAppStore((s) => s.recentRoots);
  const hashPollStop = useRef<(() => void) | null>(null);
  const scanPollStop = useRef<(() => void) | null>(null);

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
        } else {
          useAppStore.getState().setInspection(result.data);
        }
      })
      .catch(() => useAppStore.getState().setIpcStatus("failed"));
  };

  useEffect(() => {
    if (import.meta.env.DEV) {
      // Dev-only 自动化冒烟入口（DECISIONS D26）：触发与 Drop 完全相同的
      // dispatchPath 链路（校验 → IPC → Rust → store → 渲染）。
      // 生产构建中 import.meta.env.DEV 为常量 false，此分支被死码消除。
      (window as { __weaveDev?: unknown }).__weaveDev = { dispatch: dispatchPath };
    }
  });

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
    const result = commands.setAppConfig({
      version: 1,
      language: next,
      theme: useAppStore.getState().theme,
    });
    void result.then((saved) => {
      if (saved.status === "error") {
        useAppStore.getState().setError(saved.error);
      }
    });
  };

  const scanTerminal = scanStatus !== null && isTerminalState(scanStatus);

  return (
    <main
      style={{
        minHeight: "100%",
        padding: "var(--spacing-xl)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--spacing-lg)",
      }}
    >
      <header style={{ display: "flex", alignItems: "baseline", gap: "var(--spacing-md)" }}>
        <h1 style={{ margin: 0, fontSize: "var(--typography-size-title)" }}>
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
        <button type="button" onClick={toggleLocale} style={buttonStyle}>
          {locale === "zh-CN" ? "English" : "中文"}
        </button>
      </header>

      <nav
        aria-label={t("app.nav")}
        style={{ display: "flex", gap: "var(--spacing-xs)", flexWrap: "wrap" }}
      >
        {(["tools", "rename", "organizer", "duplicates", "history"] as const).map((v) => (
          <button
            key={v}
            type="button"
            onClick={() => setView(v)}
            aria-current={view === v ? "page" : undefined}
            style={{
              ...buttonStyle,
              background: view === v ? "var(--color-accent-soft)" : "var(--color-surface)",
              borderColor: view === v ? "var(--color-accent)" : "var(--color-border)",
            }}
          >
            {t(`nav.${v}`)}
          </button>
        ))}
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

      <section style={{ display: "flex", gap: "var(--spacing-sm)" }}>
        <button type="button" onClick={() => void openViaDialog(false)} style={buttonStyle}>
          {t("open.file")}
        </button>
        <button type="button" onClick={() => void openViaDialog(true)} style={buttonStyle}>
          {t("open.folder")}
        </button>
        {listCommands().map((command) => (
          <button
            key={command.id}
            type="button"
            onClick={() => runCommand(command.id)}
            style={buttonStyle}
          >
            {t(command.labelKey)}
          </button>
        ))}
      </section>

      {view === "rename" ? (
        <RenamePanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
      ) : null}

      {view === "organizer" ? (
        <OrganizerPanel onOperationDone={() => setHistoryRefresh((n) => n + 1)} />
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

      {view === "history" ? (
        <HistoryPanel
          locale={locale}
          refreshSignal={historyRefresh}
          onUndoDone={() => setHistoryRefresh((n) => n + 1)}
        />
      ) : null}

      {view === "tools" ? (
        <section
          aria-label={t("drop.hint")}
          style={{
            border: "2px dashed " + (dragOver ? "var(--color-accent)" : "var(--color-border)"),
            borderRadius: "var(--radius-lg)",
            padding: "var(--spacing-xl)",
            textAlign: "center",
            background: dragOver ? "var(--color-accent-soft)" : "var(--color-surface)",
            transition: "background var(--motion-normal) var(--motion-ease)",
          }}
        >
          {t("drop.hint")}
        </section>
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
