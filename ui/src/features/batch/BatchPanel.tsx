import { useEffect, useRef, useState, type CSSProperties } from "react";
import { useT } from "../../i18n";
import { commands } from "../../generated/bindings";
import { isTerminalState, pollJob } from "../../lib/jobs";
import {
  batchExecute,
  batchJobsList,
  batchPause,
  batchPreview,
  batchResume,
  batchRetryFailed,
  buildStages,
  type BatchJobListItemView,
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
/** 下 §145：结果表分页上限——DOM 有界，不渲染全部行。 */
const PAGE_SIZE = 100;

type StatusFilter = "all" | "success" | "failed" | "skipped" | "cancelled";

/** M7（下）Batch：Linear Pipeline 的 Preview / Execute / Pause / Cancel /
 * Retry Failed / Resume + 结果过滤/搜索/分页（§137-§147）。
 * UI 只做配置投影与结果呈现，引擎逻辑在 weave-batch（§86）。 */
export function BatchPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [inputsText, setInputsText] = useState("");
  const [destDir, setDestDir] = useState("");
  const [preset, setPreset] = useState<BatchPreset>("textTrim");
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const [continueOnError, setContinueOnError] = useState(true);
  const [workers, setWorkers] = useState(1);
  const [preview, setPreview] = useState<BatchJobResultView | null>(null);
  const [result, setResult] = useState<BatchJobResultView | null>(null);
  const [jobId, setJobId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  // 下 §146/§147：结果过滤与搜索；§145 分页
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  // 下 §143：Job 清单（恢复面板）
  const [jobList, setJobList] = useState<BatchJobListItemView[]>([]);
  const pollStop = useRef<(() => void) | null>(null);

  const refreshJobs = (): void => {
    batchJobsList()
      .then((list) => setJobList(list))
      .catch(() => setJobList([]));
  };

  useEffect(() => {
    refreshJobs();
    return () => {
      pollStop.current?.();
    };
  }, []);

  const inputs = inputsText
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  const stages = buildStages(preset, destDir, overwriteExisting);
  const options = { continueOnError, overwriteExisting, workers };

  const watchJob = (id: string): void => {
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
            const b = status.batch;
            setResult({
              items: (b.items ?? []).map((m) => ({
                itemId: m.itemId,
                source: m.source,
                output: m.output ?? null,
                status: m.status,
                error: m.error ?? null,
                retryable: m.retryable ?? true,
                inputBytes: m.inputBytes ?? 0,
                outputBytes: m.outputBytes ?? 0,
              })),
              total: b.total ?? 0,
              succeeded: b.succeeded ?? 0,
              failed: b.failed ?? 0,
              skipped: b.skipped ?? 0,
              cancelled: b.cancelled ?? 0,
              pending: b.pending ?? 0,
              inputBytes: b.inputBytes ?? 0,
              outputBytes: b.outputBytes ?? 0,
              preview: b.preview,
              operationId: b.operationId ?? null,
              resumeConflicts: b.resumeConflicts ?? [],
            });
            refreshJobs();
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
  };

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
        watchJob(id);
      })
      .catch((e: unknown) => {
        setRunning(false);
        setProgress(null);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  const doPause = (): void => {
    if (jobId) {
      batchPause(jobId).catch((e: unknown) =>
        setError(e instanceof Error ? e.message : String(e)),
      );
    }
  };

  const doCancel = (): void => {
    if (jobId) {
      void commands.cancelJob(jobId);
    }
  };

  const doRetryFailed = (): void => {
    if (!jobId) {
      return;
    }
    setError(null);
    setResult(null);
    setRunning(true);
    setProgress(0);
    batchRetryFailed(jobId)
      .then((id) => {
        setJobId(id);
        watchJob(id);
      })
      .catch((e: unknown) => {
        setRunning(false);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  const doResumeJob = (id: string): void => {
    setError(null);
    setRunning(true);
    setProgress(0);
    batchResume(id)
      .then((newId) => {
        setJobId(newId);
        watchJob(newId);
      })
      .catch((e: unknown) => {
        setRunning(false);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  // 下 §147/§146：过滤 + 搜索 + 分页（有界 DOM，§145）
  const shownResult = result ?? preview;
  const filteredItems = (shownResult?.items ?? []).filter((item) => {
    if (filter !== "all" && item.status !== filter) {
      return false;
    }
    if (search && !item.source.toLowerCase().includes(search.toLowerCase())) {
      return false;
    }
    return true;
  });
  const pageCount = Math.max(1, Math.ceil(filteredItems.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const pagedItems = filteredItems.slice(
    safePage * PAGE_SIZE,
    (safePage + 1) * PAGE_SIZE,
  );
  const hasRetryableFailed = (result?.items ?? []).some(
    (i) => i.status === "failed" && i.retryable,
  );

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
          <option value="pdfRotate90">{t("batch.preset.pdfRotate90")}</option>
        </select>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          {t("batch.workers")}
          <select
            aria-label={t("batch.workers")}
            value={workers}
            onChange={(e) => setWorkers(Number(e.target.value))}
            style={{ ...boxStyle, marginLeft: "var(--spacing-xs)" }}
          >
            <option value={1}>1</option>
            <option value={2}>2</option>
            <option value={4}>4</option>
          </select>
        </label>
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
          <button type="button" onClick={doPause} style={boxStyle}>
            {t("batch.pause")}
          </button>
        ) : null}
        {running ? (
          <button type="button" onClick={doCancel} style={boxStyle}>
            {t("batch.cancel")}
          </button>
        ) : null}
        {result && hasRetryableFailed && !running ? (
          <button type="button" onClick={doRetryFailed} style={boxStyle}>
            {t("batch.retryFailed")}
          </button>
        ) : null}
      </div>

      {running ? (
        <div
          role="status"
          aria-live="polite"
          style={{ fontSize: "var(--typography-size-sm)" }}
        >
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

      {preview ? <BatchSummaryBar result={preview} titleKey="batch.previewResult" /> : null}
      {result ? (
        <>
          <BatchSummaryBar result={result} titleKey="batch.executeResult" />
          {result.resumeConflicts.length > 0 ? (
            <div role="alert" style={{ fontSize: "var(--typography-size-sm)" }}>
              <strong>{t("batch.conflicts")}</strong>
              {result.resumeConflicts.map((c, i) => (
                <div key={i} style={{ fontFamily: "var(--typography-mono-family)" }}>
                  {c}
                </div>
              ))}
            </div>
          ) : null}
        </>
      ) : null}

      {shownResult ? (
        <>
          {/* 下 §147：状态过滤 + §146 搜索 */}
          <div style={rowStyle}>
            {(["all", "success", "failed", "skipped", "cancelled"] as const).map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => {
                  setFilter(f);
                  setPage(0);
                }}
                aria-pressed={filter === f}
                style={{
                  ...boxStyle,
                  background: filter === f ? "var(--color-accent-soft)" : "var(--color-surface)",
                  borderColor: filter === f ? "var(--color-accent)" : "var(--color-border)",
                }}
              >
                {t(`batch.filter.${f}`)}
              </button>
            ))}
            <input
              aria-label={t("batch.search")}
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(0);
              }}
              placeholder={t("batch.search")}
              style={{ ...boxStyle, flex: 1, minWidth: "12em" }}
            />
          </div>
          <BatchResultTable
            items={pagedItems}
            offset={safePage * PAGE_SIZE}
          />
          {pageCount > 1 ? (
            <div style={rowStyle}>
              <button
                type="button"
                disabled={safePage === 0}
                onClick={() => setPage(safePage - 1)}
                style={boxStyle}
              >
                {"<"}
              </button>
              <span style={{ fontSize: "var(--typography-size-sm)" }}>
                {t("batch.page", { page: safePage + 1, count: pageCount })}
              </span>
              <button
                type="button"
                disabled={safePage >= pageCount - 1}
                onClick={() => setPage(safePage + 1)}
                style={boxStyle}
              >
                {">"}
              </button>
              <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("batch.matchCount", { count: filteredItems.length })}
              </span>
            </div>
          ) : null}
        </>
      ) : null}

      {/* 下 §143：Job 清单 / 恢复面板 */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
        <div style={rowStyle}>
          <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("batch.jobs")}</strong>
          <button type="button" onClick={refreshJobs} style={boxStyle}>
            {t("batch.jobs.refresh")}
          </button>
        </div>
        {jobList.map((j) => (
          <div key={j.jobId} style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
            <div style={rowStyle}>
              <span
                style={{
                  fontFamily: "var(--typography-mono-family)",
                  fontSize: "var(--typography-size-sm)",
                }}
              >
                {j.jobId.slice(0, 12)}
              </span>
              <span>{t(`batch.jobState.${j.state}`, { defaultValue: j.state })}</span>
              <span style={{ color: "var(--color-text-muted)" }}>
                {t("batch.jobCounts", {
                  settled: j.settledItems,
                  total: j.totalItems,
                  pending: j.pendingItems,
                })}
              </span>
              {j.state === "paused" || j.state === "interrupted" ? (
                <button
                  type="button"
                  disabled={running}
                  onClick={() => doResumeJob(j.jobId)}
                  style={boxStyle}
                >
                  {t("batch.resume")}
                </button>
              ) : null}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function BatchSummaryBar({
  result,
  titleKey,
}: {
  result: BatchJobResultView;
  titleKey: string;
}) {
  const t = useT();
  return (
    <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
      <strong>{t(titleKey)}</strong> ·{" "}
      {t("batch.summary", {
        total: result.total,
        succeeded: result.succeeded,
        failed: result.failed,
        skipped: result.skipped,
        cancelled: result.cancelled,
      })}
      {result.pending > 0 ? ` · ${t("batch.pendingCount", { count: result.pending })}` : ""}
    </div>
  );
}

function BatchResultTable({
  items,
  offset,
}: {
  items: BatchJobResultView["items"];
  offset: number;
}) {
  const t = useT();
  return (
    <div
      style={{
        maxHeight: "18em",
        overflow: "auto",
        border: "1px solid var(--color-border)",
        borderRadius: "var(--radius-sm)",
      }}
    >
      <table
        style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}
      >
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
          {items.map((item, i) => (
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
                {/* 下 §142：不可重试显式说明 */}
                {item.status === "failed" && !item.retryable
                  ? ` · ${t("batch.retryUnavailable")}`
                  : ""}
              </td>
              <td
                style={{
                  padding: "var(--spacing-xs)",
                  fontFamily: "var(--typography-mono-family)",
                }}
              >
                {item.output ?? "—"}
                {item.output ? ` (${offset + i + 1})` : ""}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
