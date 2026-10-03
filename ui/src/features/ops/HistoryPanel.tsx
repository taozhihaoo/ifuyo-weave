import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type { HistoryEntryDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { formatTimestamp } from "../../lib/jobs";

interface HistoryPanelProps {
  locale: string;
  onUndoDone: (operationId: string, restored: number) => void;
  refreshSignal: number;
}

/** Recent Operations（M2 §56 latest first）+ 每条 Undo 入口。 */
export function HistoryPanel({ locale, onUndoDone, refreshSignal }: HistoryPanelProps) {
  const t = useT();
  const [entries, setEntries] = useState<HistoryEntryDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [undoBusy, setUndoBusy] = useState<string | null>(null);

  const load = useCallback(() => {
    commands
      .getHistory(50)
      .then((r) => {
        if (r.status === "ok") {
          setEntries(r.data);
        } else {
          setError(r.error.code);
        }
      })
      .catch(() => setError("history load failed"));
  }, []);

  useEffect(() => {
    load();
  }, [load, refreshSignal]);

  const doUndo = (operationId: string): void => {
    setUndoBusy(operationId);
    commands
      .undoOperation(operationId)
      .then((r) => {
        if (r.status !== "ok") {
          setUndoBusy(null);
          setError(r.error.code);
          return;
        }
        // 轮询 undo job
        const stop = setInterval(() => {
          void commands.getJob(r.data.jobId).then((status) => {
            if (status.status !== "ok") {
              clearInterval(stop);
              setUndoBusy(null);
              return;
            }
            if (status.data.state !== "running") {
              clearInterval(stop);
              setUndoBusy(null);
              const undo = status.data.undo;
              onUndoDone(operationId, undo?.restored ?? 0);
              load();
            }
          });
        }, 400);
      })
      .catch(() => {
        setUndoBusy(null);
        setError("history.loadFailed");
      });
  };

  return (
    <section
      aria-label={t("history.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("history.title")}</h2>
      {error ? (
        <div
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {error}
        </div>
      ) : null}
      {entries !== null && entries.length === 0 ? (
        <div style={{ color: "var(--color-text-muted)" }}>{t("history.empty")}</div>
      ) : null}
      {entries?.map((entry) => (
        <div
          key={entry.operationId}
          role="row"
          style={{
            display: "flex",
            gap: "var(--spacing-md)",
            alignItems: "center",
            padding: "var(--spacing-sm) var(--spacing-md)",
            border: "1px solid var(--color-border)",
            borderRadius: "var(--radius-sm)",
            background: "var(--color-surface)",
            fontSize: "var(--typography-size-sm)",
          }}
        >
          <span style={{ fontWeight: 600 }}>{entry.summary}</span>
          <span style={{ color: "var(--color-text-muted)" }}>
            {formatTimestamp(entry.timestampMs, locale)}
          </span>
          <span
            style={{
              color: entry.status === "completed" ? "var(--color-success)" : "var(--color-danger)",
            }}
          >
            {t(`history.status.${entry.status}`)}
          </span>
          {entry.undoable ? (
            <button
              type="button"
              disabled={undoBusy === entry.operationId}
              onClick={() => doUndo(entry.operationId)}
              style={undoButtonStyle}
            >
              {undoBusy === entry.operationId ? t("history.undoing") : t("history.undo")}
            </button>
          ) : null}
        </div>
      ))}
    </section>
  );
}

const undoButtonStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  marginLeft: "auto",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
};
