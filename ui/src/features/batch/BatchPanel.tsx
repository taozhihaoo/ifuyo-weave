import { useEffect, useRef, useState, type CSSProperties } from "react";
import { useT } from "../../i18n";
import { commands } from "../../generated/bindings";
import { isTerminalState, pollJob } from "../../lib/jobs";
import {
  batchExecute,
  batchPreview,
  buildStages,
  type BatchJobResultView,
  type BatchPreset,
} from "../../lib/batch";

const boxStyle: CSSProperties = {
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
};
const rowStyle: CSSProperties = {
  display: "flex",
  gap: "var(--spacing-sm)",
  alignItems: "center",
  flexWrap: "wrap",
};
const areaStyle: CSSProperties = {
  fontFamily: "var(--typography-mono-family)",
  fontSize: "var(--typography-size-sm)",
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  width: "100%",
  boxSizing: "border-box",
};

/** M7（上）Batch：Linear Pipeline 的 Preview / Execute / Cancel。
 * UI 只做配置投影与结果呈现，引擎逻辑在 weave-batch（§86）。 */
export function BatchPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [inputsText, setInputsText] = useState("");
  const [destDir, setDestDir] = useState("");
  const [preset, setPreset] = useState<BatchPreset>("textTrim");
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const [continueOnError, setContinueOnError] = useState(true);
  const [preview, setPreview] = useState<BatchJobResultView | null>(null);
  const [result, setResult] = useState<BatchJobResultView | null>(null);
  const [jobId, setJobId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollStop = useRef<(() => void) | null>(null);

  useEffect(
    () => () => {
      pollStop.current?.();
    },
    [],
  );

  const inputs = inputsText
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  const stages = buildStages(preset, destDir, overwriteExisting);
  const options = { continueOnError, overwriteExisting };

  const doPreview = (): void => {
    setError(null);
    setPreview(null);
    setResult(null);
    batchPreview(inputs, stages, destDir, options)
      .then((r) => setPreview(r))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  const doExecute = (): void => {
    setError(null);
    setPreview(null);
    setResult(null);
    setRunning(true);
    setProgress(0);
    batchExecute(inputs, stages, destDir, options)
      .then((id) => {
        setJobId(id);
        pollStop.current?.();
        pollStop.current = pollJob(id, (tick) => {
          if (tick.kind === "status") {
            const status = tick.status;
            if (status.state === "running") {
              setProgress(status.progressCurrent ?? 0);
            } else if (isTerminalState(status)) {
              pollStop.current?.();
              pollStop.current = null;
              setRunning(false);
              if (status.state === "failed") {
                setError(
                  `${status.error?.code ?? "job.failed"} · ${status.error?.message ?? "batch job failed"}`,
                );
              } else if (status.batch) {
                setResult({
                  items: (status.batch.items ?? []).map((m) => ({
                    itemId: m.itemId,
                    source: m.source,
                    output: m.output ?? null,
                    status: m.status,
                    error: m.error ?? null,
                    inputBytes: m.inputBytes ?? 0,
                    outputBytes: m.outputBytes ?? 0,
                  })),
                  total: status.batch.total ?? 0,
                  succeeded: status.batch.succeeded ?? 0,
                  failed: status.batch.failed ?? 0,
                  skipped: status.batch.skipped ?? 0,
                  cancelled: status.batch.cancelled ?? 0,
                  inputBytes: status.batch.inputBytes ?? 0,
                  outputBytes: status.batch.outputBytes ?? 0,
                  preview: status.batch.preview,
                });
                onOperationDone();
              }
            }
          } else {
            pollStop.current?.();
            pollStop.current = null;
            setRunning(false);
            setError(`${tick.error.code} · ${tick.error.message}`);
          }
        });
      })
      .catch((e: unknown) => {
        setRunning(false);
        setProgress(null);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  const doCancel = (): void => {
    if (jobId) {
      void commands.cancelJob(jobId);
    }
  };

  return (
    <section
      aria-label={t("batch.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h3 style={{ margin: 0, fontSize: "var(--typography-size-lg)" }}>{t("batch.title")}</h3>
      <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
        {t("batch.hint")}
      </div>

      <div style={rowStyle}>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>{t("batch.preset")}</label>
        <select
          aria-label={t("batch.preset")}
          value={preset}
          onChange={(e) => setPreset(e.target.value as BatchPreset)}
          style={boxStyle}
        >
          <option value="textTrim">{t("batch.preset.textTrim")}</option>
          <option value="textReplace">{t("batch.preset.textReplace")}</option>
          <option value="imageResizePng">{t("batch.preset.imageResizePng")}</option>
        </select>
      </div>

      <textarea
        aria-label={t("batch.inputs")}
        value={inputsText}
        onChange={(e) => setInputsText(e.target.value)}
        rows={6}
        placeholder={t("batch.inputsHint")}
        style={areaStyle}
      />

      <div style={rowStyle}>
        <input
          aria-label={t("batch.destDir")}
          value={destDir}
          onChange={(e) => setDestDir(e.target.value)}
          placeholder="C:\path\to\output"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input
            type="checkbox"
            checked={overwriteExisting}
            onChange={(e) => setOverwriteExisting(e.target.checked)}
          />
          {t("batch.overwrite")}
        </label>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input
            type="checkbox"
            checked={continueOnError}
            onChange={(e) => setContinueOnError(e.target.checked)}
          />
          {t("batch.continueOnError")}
        </label>
        <button
          type="button"
          disabled={running || inputs.length === 0 || destDir.length === 0}
          onClick={doPreview}
          style={boxStyle}
        >
          {t("batch.preview")}
        </button>
        <button
          type="button"
          disabled={running || inputs.length === 0 || destDir.length === 0}
          onClick={doExecute}
          style={boxStyle}
        >
          {t("batch.execute")}
        </button>
        {running ? (
          <button type="button" onClick={doCancel} style={boxStyle}>
            {t("batch.cancel")}
          </button>
        ) : null}
      </div>

      {running ? (
        <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
          {t("batch.running", { count: progress ?? 0 })}
        </div>
      ) : null}

      {error ? (
        <div
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {error}
        </div>
      ) : null}

      {preview ? <BatchResultTable result={preview} titleKey="batch.previewResult" /> : null}
      {result ? <BatchResultTable result={result} titleKey="batch.executeResult" /> : null}
    </section>
  );
}

function BatchResultTable({
  result,
  titleKey,
}: {
  result: BatchJobResultView;
  titleKey: string;
}) {
  const t = useT();
  return (
    <>
      <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
        <strong>{t(titleKey)}</strong> ·{" "}
        {t("batch.summary", {
          total: result.total,
          succeeded: result.succeeded,
          failed: result.failed,
          skipped: result.skipped,
          cancelled: result.cancelled,
        })}
      </div>
      <div
        style={{
          maxHeight: "18em",
          overflow: "auto",
          border: "1px solid var(--color-border)",
          borderRadius: "var(--radius-sm)",
        }}
      >
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}>
          <thead>
            <tr>
              <th style={{ textAlign: "left", padding: "var(--spacing-xs)" }}>
                {t("batch.col.item")}
              </th>
              <th style={{ textAlign: "left", padding: "var(--spacing-xs)" }}>
                {t("batch.col.status")}
              </th>
              <th style={{ textAlign: "left", padding: "var(--spacing-xs)" }}>
                {t("batch.col.output")}
              </th>
            </tr>
          </thead>
          <tbody>
            {result.items.map((item) => (
              <tr key={item.itemId}>
                <td
                  style={{
                    padding: "var(--spacing-xs)",
                    fontFamily: "var(--typography-mono-family)",
                  }}
                >
                  {item.source}
                </td>
                <td
                  style={{
                    padding: "var(--spacing-xs)",
                    color:
                      item.status === "failed"
                        ? "var(--color-danger)"
                        : item.status === "success"
                          ? "var(--color-success)"
                          : "var(--color-text-muted)",
                  }}
                >
                  {t(`batch.status.${item.status}`)}
                  {item.error ? ` · ${item.error}` : ""}
                </td>
                <td
                  style={{
                    padding: "var(--spacing-xs)",
                    fontFamily: "var(--typography-mono-family)",
                  }}
                >
                  {item.output ?? "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
