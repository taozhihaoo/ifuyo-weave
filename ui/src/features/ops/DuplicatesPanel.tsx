import { useEffect, useState, type CSSProperties } from "react";
import type {
  DuplicateGroupDto,
  DuplicateScanReportDto,
  PlanDto,
  RecycleSelectionDto,
} from "../../generated/bindings";
import { useT } from "../../i18n";
import { formatBytes } from "../../lib/format";
import {
  buildRecyclePlan,
  executeRecyclePlan,
  pollUntilDone,
  scanDuplicates,
  undoOperation,
} from "../../lib/operations";

interface DuplicatesPanelProps {
  onOperationDone: (operationId: string, undoable: boolean) => void;
  /** 会话内最近成功扫描的根目录（§106 Recent Root；会话态，不入全局配置）。 */
  recentRoots: string[];
  /** 全局 Drop 落到本页的目录（§106 复用统一 Drag & Drop，不造第二套）。 */
  seedFolder: string | null;
  onSeedConsumed: () => void;
  /** 扫描成功后回传 roots（App 层写会话 recentRoots）。 */
  onScanCompleted: (roots: string[]) => void;
}

interface RecycleResult {
  operationId: string;
  recycled: number;
  failed: number;
  skipped: number;
  undoable: boolean;
}

const boxStyle: CSSProperties = {
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
};

const groupHeaderStyle: CSSProperties = {
  display: "flex",
  gap: "var(--spacing-sm)",
  alignItems: "baseline",
  flexWrap: "wrap",
  padding: "var(--spacing-xs) 0",
  borderBottom: "1px solid var(--color-border)",
};

function fileRowStyle(selected: boolean): CSSProperties {
  return {
    display: "flex",
    gap: "var(--spacing-sm)",
    alignItems: "center",
    padding: "2px 0",
    fontFamily: "var(--typography-mono-family)",
    fontSize: "var(--typography-size-sm)",
    color: selected ? "var(--color-text)" : "var(--color-text-muted)",
  };
}

/** M3 Duplicate Finder：扫描 → 组视图选择 → 回收计划 → 回收站执行 → 撤销。
 *
 * 安全边界（M3）：唯一动作 = Move to Recycle Bin（可撤销）；
 * 每组至少保留一份；执行前服务端 Revalidate（扫描后改动 ⇒ 拒绝该条目）。 */
export function DuplicatesPanel({
  onOperationDone,
  recentRoots,
  seedFolder,
  onSeedConsumed,
  onScanCompleted,
}: DuplicatesPanelProps) {
  const t = useT();
  const [rootsText, setRootsText] = useState("");
  const [minSizeText, setMinSizeText] = useState("0");
  const [scanning, setScanning] = useState(false);
  const [report, setReport] = useState<DuplicateScanReportDto | null>(null);
  const [selection, setSelection] = useState<Record<string, string[]>>({});
  const [plan, setPlan] = useState<PlanDto | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<RecycleResult | null>(null);
  const [undoing, setUndoing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const roots = rootsText
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);

  // §106：统一 Drop 入口落进本页的目录 → 追加到 roots（去重，不覆盖已输入）。
  // setState 经定时器回调执行——effect 体内同步 setState 会触发级联渲染告警。
  useEffect(() => {
    if (!seedFolder) {
      return;
    }
    const id = setTimeout(() => {
      setRootsText((prev) => {
        const lines = prev
          .split("\n")
          .map((l) => l.trim())
          .filter(Boolean);
        if (lines.includes(seedFolder)) {
          return prev;
        }
        return lines.length > 0 ? `${prev.trimEnd()}\n${seedFolder}` : seedFolder;
      });
      onSeedConsumed();
    }, 0);
    return () => clearTimeout(id);
  }, [seedFolder, onSeedConsumed]);

  const doScan = (): void => {
    setBusy(true);
    setScanning(true);
    setError(null);
    setReport(null);
    setPlan(null);
    setConfirming(false);
    setResult(null);
    scanDuplicates(roots, Number(minSizeText) > 0 ? Number(minSizeText) : 0)
      .then((jobId) => {
        pollUntilDone(
          jobId,
          () => {},
          (status) => {
            setBusy(false);
            setScanning(false);
            const dup = status.duplicateScan;
            if (dup) {
              onScanCompleted(
                rootsText
                  .split("\n")
                  .map((l) => l.trim())
                  .filter((l) => l.length > 0),
              );
              setReport(dup);
              // 预选：每组保留首个，其余勾选（wasted 语义的默认选择）。
              const auto: Record<string, string[]> = {};
              for (const g of dup.groups) {
                auto[g.groupId] = g.files.slice(1).map((f) => f.path);
              }
              setSelection(auto);
            }
          },
        );
      })
      .catch((e: unknown) => {
        setBusy(false);
        setScanning(false);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  const toggleFile = (groupId: string, path: string): void => {
    setSelection((prev) => {
      const current = prev[groupId] ?? [];
      const next = current.includes(path) ? current.filter((p) => p !== path) : [...current, path];
      return { ...prev, [groupId]: next };
    });
  };

  const selectedCount = Object.values(selection).reduce((n, ps) => n + ps.length, 0);
  const selectedBytes = report
    ? report.groups.reduce(
        (sum, g) => sum + (selection[g.groupId]?.length ?? 0) * (g.fileSize ?? 0),
        0,
      )
    : 0;

  const doPreview = (): void => {
    if (!report) {
      return;
    }
    setBusy(true);
    setError(null);
    setPlan(null);
    setConfirming(false);
    const sels: RecycleSelectionDto[] = Object.entries(selection)
      .filter(([, paths]) => paths.length > 0)
      .map(([groupId, recyclePaths]) => ({ groupId, recyclePaths }));
    buildRecyclePlan(report.scanId, sels)
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          setPlan(r.data);
        } else {
          setError(`${r.error.code} · ${r.error.message}`);
        }
      })
      .catch(() => setBusy(false));
  };

  const doExecute = (operationId: string): void => {
    setBusy(true);
    executeRecyclePlan(operationId)
      .then((jobId) => {
        pollUntilDone(
          jobId,
          () => {},
          (status) => {
            setBusy(false);
            const exec = status.plan;
            if (exec) {
              setResult({
                operationId: exec.operationId,
                recycled: exec.executed ?? 0,
                failed: exec.failed ?? 0,
                skipped: exec.skipped ?? 0,
                undoable: exec.undoable,
              });
              onOperationDone(exec.operationId, exec.undoable);
              setPlan(null);
              setConfirming(false);
            }
          },
        );
      })
      .catch(() => setBusy(false));
  };

  const doUndo = (operationId: string): void => {
    setBusy(true);
    setUndoing(true);
    undoOperation(operationId)
      .then((jobId) => {
        pollUntilDone(
          jobId,
          () => {},
          (status) => {
            setBusy(false);
            setUndoing(false);
            const undo = status.plan;
            if (undo) {
              onOperationDone(operationId, false);
              setResult(null);
              setReport(null);
              setSelection({});
            }
          },
        );
      })
      .catch(() => {
        setBusy(false);
        setUndoing(false);
      });
  };

  const renderGroup = (group: DuplicateGroupDto) => {
    const chosen = selection[group.groupId] ?? [];
    return (
      <div key={group.groupId} style={{ marginBottom: "var(--spacing-md)" }}>
        <div style={groupHeaderStyle}>
          <strong style={{ fontFamily: "var(--typography-mono-family)" }}>
            {t("duplicates.group", { id: group.groupId.slice(4, 12) })}
          </strong>
          <span>
            {t("duplicates.groupFiles", { count: group.fileCount ?? 0 })} ·{" "}
            {t("duplicates.fileSizeEach", { size: formatBytes(group.fileSize) })}
          </span>
          <span style={{ color: "var(--color-accent)" }}>
            {t("duplicates.wasted", { size: formatBytes(group.wastedSize) })}
          </span>
        </div>
        {group.files.map((file) => (
          <label key={file.fileId} style={fileRowStyle(chosen.includes(file.path))}>
            <input
              type="checkbox"
              checked={chosen.includes(file.path)}
              onChange={() => toggleFile(group.groupId, file.path)}
            />
            <span style={{ flex: 1, overflowWrap: "anywhere" }}>{file.path}</span>
            <span>{formatBytes(file.size)}</span>
          </label>
        ))}
      </div>
    );
  };

  return (
    <section
      aria-label={t("duplicates.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("duplicates.title")}</h2>

      <textarea
        aria-label={t("duplicates.roots")}
        value={rootsText}
        onChange={(e) => setRootsText(e.target.value)}
        rows={3}
        placeholder={t("duplicates.rootsHint")}
        style={{
          fontFamily: "var(--typography-mono-family)",
          fontSize: "var(--typography-size-sm)",
          padding: "var(--spacing-sm)",
          borderRadius: "var(--radius-sm)",
          border: "1px solid var(--color-border)",
        }}
      />

      {recentRoots.length > 0 ? (
        <div
          style={{
            display: "flex",
            gap: "var(--spacing-xs)",
            alignItems: "center",
            flexWrap: "wrap",
          }}
        >
          <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {t("duplicates.recent")}
          </span>
          {recentRoots.map((root) => (
            <button
              key={root}
              type="button"
              title={root}
              onClick={() =>
                setRootsText((prev) => {
                  const lines = prev
                    .split("\n")
                    .map((l) => l.trim())
                    .filter(Boolean);
                  if (lines.includes(root)) {
                    return prev;
                  }
                  return lines.length > 0 ? `${prev.trimEnd()}\n${root}` : root;
                })
              }
              style={{
                ...boxStyle,
                maxWidth: "24em",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                fontSize: "var(--typography-size-sm)",
              }}
            >
              {root}
            </button>
          ))}
        </div>
      ) : null}

      <div
        style={{
          display: "flex",
          gap: "var(--spacing-sm)",
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <label>
          {t("duplicates.minSize")}{" "}
          <input
            type="number"
            min={0}
            value={minSizeText}
            onChange={(e) => setMinSizeText(e.target.value)}
            style={{ ...boxStyle, width: "8em" }}
          />
        </label>
        <button
          type="button"
          disabled={busy || scanning || roots.length === 0}
          onClick={doScan}
          style={boxStyle}
        >
          {scanning ? t("duplicates.scanning") : t("duplicates.scan")}
        </button>
      </div>

      {error ? (
        <div
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {error}
        </div>
      ) : null}

      {report ? (
        <>
          <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {t("duplicates.stats", {
              files: report.filesScanned ?? 0,
              candidates: report.candidateFiles ?? 0,
              groups: report.duplicateGroups ?? 0,
              wasted: formatBytes(report.potentialReclaimableSize),
            })}
            {report.partialResult ? ` · ${t("duplicates.partial")}` : ""}
          </div>

          {report.groups.length === 0 ? (
            <div>{t("duplicates.empty")}</div>
          ) : (
            <>
              {report.groups.map(renderGroup)}
              <div
                style={{
                  display: "flex",
                  gap: "var(--spacing-sm)",
                  alignItems: "center",
                  flexWrap: "wrap",
                }}
              >
                <span>
                  {t("duplicates.selected", {
                    count: selectedCount,
                    size: formatBytes(selectedBytes),
                  })}
                </span>
                <button
                  type="button"
                  disabled={busy || selectedCount === 0}
                  onClick={doPreview}
                  style={boxStyle}
                >
                  {t("duplicates.preview")}
                </button>
              </div>
              <div
                style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}
              >
                {t("duplicates.selectHint")}
              </div>
            </>
          )}
        </>
      ) : null}

      {plan && !confirming ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <div>
            {t("duplicates.planReady", { count: plan.readyCount ?? 0 })} ·{" "}
            {formatBytes(selectedBytes)}
          </div>
          <button
            type="button"
            disabled={busy}
            onClick={() => setConfirming(true)}
            style={boxStyle}
          >
            {t("duplicates.execute")}
          </button>
        </div>
      ) : null}

      {plan && confirming ? (
        <div
          role="alertdialog"
          aria-label={t("duplicates.confirmTitle")}
          style={{
            border: "1px solid var(--color-accent)",
            borderRadius: "var(--radius-md)",
            padding: "var(--spacing-md)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--spacing-sm)",
            background: "var(--color-surface)",
          }}
        >
          <strong>{t("duplicates.confirmTitle")}</strong>
          <div style={{ fontSize: "var(--typography-size-sm)" }}>{t("duplicates.confirmHint")}</div>
          <div style={{ display: "flex", gap: "var(--spacing-sm)" }}>
            <button
              type="button"
              disabled={busy}
              onClick={() => doExecute(plan.operationId)}
              style={boxStyle}
            >
              {t("duplicates.confirmYes")}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => setConfirming(false)}
              style={boxStyle}
            >
              {t("duplicates.confirmNo")}
            </button>
          </div>
        </div>
      ) : null}

      {result ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <div>
            {t("duplicates.result", {
              recycled: result.recycled,
              failed: result.failed,
              skipped: result.skipped,
            })}
          </div>
          {result.undoable ? (
            <button
              type="button"
              disabled={busy}
              onClick={() => doUndo(result.operationId)}
              style={boxStyle}
            >
              {undoing ? t("duplicates.undoing") : t("duplicates.undo")}
            </button>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
