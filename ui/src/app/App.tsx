import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listCommands, runCommand } from "../commands/registry";
import { useT } from "../i18n";
import { formatBytes } from "../lib/format";
import { commands } from "../generated/bindings";
import { useAppStore, type Locale } from "../stores/appStore";

function App() {
  const t = useT();
  const { appInfo, ipcStatus, lastError, probe, probeError, locale } = useAppStore();
  const [dragOver, setDragOver] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    // Drop → IPC → Rust 校验 → UI 展示（M0 §37/§58 的架构冒烟）。
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDragOver(true);
        } else if (event.payload.type === "drop") {
          setDragOver(false);
          const path = event.payload.paths[0];
          if (path) {
            void commands.inspectPath(path).then((result) => {
              if (result.status === "ok") {
                useAppStore.getState().setProbe(result.data);
              } else {
                useAppStore.getState().setProbeError(result.error);
              }
            });
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
        // 非 Tauri 环境（如纯浏览器/vitest）没有拖放事件；不视为失败。
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

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
        <button
          type="button"
          onClick={toggleLocale}
          style={{
            marginLeft: "auto",
            height: "var(--control-height-sm)",
            padding: "0 var(--control-padding-x)",
            borderRadius: "var(--radius-sm)",
            border: "1px solid var(--color-border)",
            background: "var(--color-surface)",
            cursor: "pointer",
          }}
        >
          {locale === "zh-CN" ? "English" : "中文"}
        </button>
      </header>

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
        {listCommands().map((command) => (
          <button
            key={command.id}
            type="button"
            onClick={() => runCommand(command.id)}
            style={{
              height: "var(--control-height-md)",
              padding: "0 var(--control-padding-x)",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: "var(--color-surface)",
              cursor: "pointer",
            }}
          >
            {t(command.labelKey)}
          </button>
        ))}
      </section>

      <section
        aria-label={t("drop.hint")}
        style={{
          border: `2px dashed ${dragOver ? "var(--color-accent)" : "var(--color-border)"}`,
          borderRadius: "var(--radius-lg)",
          padding: "var(--spacing-xxl)",
          textAlign: "center",
          background: dragOver ? "var(--color-accent-soft)" : "var(--color-surface)",
          transition: `background var(--motion-normal) var(--motion-ease)`,
        }}
      >
        {t("drop.hint")}
      </section>

      {probe ? (
        <section
          style={{
            border: "1px solid var(--color-border)",
            borderRadius: "var(--radius-md)",
            padding: "var(--spacing-md)",
            background: "var(--color-surface)",
            fontFamily: "var(--typography-mono-family)",
            fontSize: "var(--typography-size-sm)",
            lineHeight: 1.8,
          }}
        >
          <strong>{t("drop.result.title")}</strong>
          <div>{probe.normalized}</div>
          <div>
            {t("drop.result.kind")}: {probe.kind}
            {" · "}
            {t("drop.result.size")}: {formatBytes(probe.sizeBytes)}
            {probe.extension ? ` · .${probe.extension}` : ""}
          </div>
        </section>
      ) : null}

      {probeError ? (
        <section
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {t("error.title")}: {probeError.code} · {probeError.message}
          {probeError.suggestion ? ` — ${t("error.suggestion")}: ${probeError.suggestion}` : ""}
        </section>
      ) : null}
    </main>
  );
}

export default App;
