import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type { CleanupResultDto, SpaceReportDto, VolumeInfoDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.2：Space 空间分析中心（分析优先，清理 = 候选→计划→重验→回收站）。 */

interface SpacePanelProps {
    locale: string;
    onOperationDone: () => void;
}

const rowStyle: CSSProperties = {
    display: "flex",
    gap: "var(--spacing-sm)",
    alignItems: "center",
    flexWrap: "wrap",
};
const boxStyle: CSSProperties = {
    padding: "var(--spacing-sm)",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--color-border)",
};
const volumeBar: CSSProperties = {
    height: "0.6em",
    width: "12em",
    background: "var(--color-border)",
    borderRadius: "999px",
    overflow: "hidden",
    display: "inline-block",
    verticalAlign: "middle",
};
const volumeFill = (pct: number): CSSProperties => ({
    height: "100%",
    width: `${Math.min(100, Math.max(0, pct))}%`,
    background: pct > 85 ? "var(--color-danger)" : "var(--color-accent)",
});

function fmtBytes(bytes: number | null | undefined): string {
    if (bytes == null || !Number.isFinite(bytes)) return "—";
    const units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let v = Math.abs(bytes);
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
        v /= 1024;
        i += 1;
    }
    const signed = bytes < 0 ? "-" : "";
    return `${signed}${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function SpacePanel({ locale, onOperationDone }: SpacePanelProps) {
    const t = useT();
    const [root, setRoot] = useState("");
    const [volumes, setVolumes] = useState<VolumeInfoDto[]>([]);
    const [report, setReport] = useState<SpaceReportDto | null>(null);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [currentRel, setCurrentRel] = useState("");
    const [selected, setSelected] = useState<Set<string>>(new Set());
    const [cleanupResult, setCleanupResult] = useState<CleanupResultDto | null>(null);
    const [scanJobId, setScanJobId] = useState<string | null>(null);
    const [progress, setProgress] = useState<number | null>(null);

    useEffect(() => {
        commands
            .spaceListVolumes()
            .then((r) => {
                if (r.status === "ok") setVolumes(r.data);
            })
            .catch(() => {
                // 卷列表失败不打断页面（诚实降级：区域隐藏）
            });
    }, []);

    const doScan = useCallback(() => {
        if (!root.trim()) {
            setError(t("space.error.root"));
            return;
        }
        setBusy(true);
        setError(null);
        setReport(null);
        setCurrentRel("");
        setSelected(new Set());
        setCleanupResult(null);
        setProgress(null);
        commands
            .spaceScan({
                root: root.trim(),
                topN: null,
                largeFileBytes: 1024 * 1024 * 1024,
                oldFileDays: 180,
                candidates: true,
            })
            .then((r) => {
                if (r.status !== "ok") {
                    setBusy(false);
                    setError(r.error.code);
                    return;
                }
                setScanJobId(r.data.jobId);
                const stop = pollJob(r.data.jobId, (tick) => {
                    if (tick.kind === "failed") {
                        stop();
                        setBusy(false);
                        setError(tick.error.code);
                        return;
                    }
                    if (isTerminalState(tick.status)) {
                        stop();
                        setScanJobId(null);
                        commands
                            .spaceGetResult(r.data.jobId)
                            .then((gr) => {
                                setBusy(false);
                                if (gr.status === "ok") {
                                    setReport(gr.data);
                                    setProgress(null);
                                } else {
                                    setError(gr.error.code);
                                }
                            })
                            .catch((e: unknown) => {
                                setBusy(false);
                                setError(e instanceof Error ? e.message : String(e));
                            });
                    }
                });
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [root]);

    const doCancel = useCallback(() => {
        if (scanJobId) {
            void commands.cancelJob(scanJobId);
        }
    }, [scanJobId]);

    const currentNode = report?.directories.find((d) => d.rel === currentRel);
    const children = report
        ? report.directories
              .filter((d) => d.parent === currentRel && d.rel !== "")
              .sort((a, b) => (b.logicalSize ?? 0) - (a.logicalSize ?? 0))
        : [];
    const parentSize = currentNode?.logicalSize ?? report?.logicalBytes ?? 0;
    const crumbs: { rel: string; label: string }[] = [{ rel: "", label: report?.root ?? "" }];
    if (currentRel) {
        const parts = currentRel.split("/");
        for (let i = 0; i < parts.length; i += 1) {
            crumbs.push({ rel: parts.slice(0, i + 1).join("/"), label: parts[i] ?? "" });
        }
    }

    const doCleanup = useCallback(() => {
        if (!report || selected.size === 0) return;
        setBusy(true);
        setError(null);
        const items = report.candidates
            .filter((c) => selected.has(c.path))
            .map((c) => ({ path: c.path, size: c.size ?? 0, modifiedMs: c.modifiedMs }));
        commands
            .spaceCleanup(items)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setCleanupResult(r.data);
                setSelected(new Set());
                onOperationDone();
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [report, selected, onOperationDone, t]);

    const typeMax = Math.max(1, ...(report?.typeStats.map((x) => x.bytes ?? 0) ?? [1]));

    return (
        <section
            aria-label={t("space.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("space.title")}</h2>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("space.hint")}
            </div>

            <div style={rowStyle}>
                <input
                    aria-label={t("space.root")}
                    value={root}
                    onChange={(e) => setRoot(e.target.value)}
                    placeholder="C:\Users\me\Documents"
                    style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
                />
                <button type="button" disabled={busy} onClick={doScan} style={boxStyle}>
                    {busy ? t("space.scanning") : t("space.scan")}
                </button>
                {scanJobId ? (
                    <button type="button" onClick={doCancel} style={boxStyle}>
                        {t("space.cancel")}
                    </button>
                ) : null}
            </div>

            {busy && progress != null ? (
                <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                    {t("space.progress", { count: progress })}
                </div>
            ) : null}

            {volumes.length > 0 ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                    <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("space.volumes")}</strong>
                    {volumes.map((v) => (
                        <div key={v.drive} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                            <strong>{v.drive}</strong>
                            <span style={volumeBar}>
                                <span style={volumeFill(v.usedPercent ?? 0)} />
                            </span>
                            <span style={{ color: "var(--color-text-muted)" }}>
                                {fmtBytes((v.totalBytes ?? 0) - (v.freeBytes ?? 0))} / {fmtBytes(v.totalBytes)}
                                {v.usedPercent != null ? ` · ${v.usedPercent.toFixed(0)}%` : ""}
                            </span>
                            <button
                                type="button"
                                style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                                onClick={() => {
                                    setRoot(v.drive.endsWith("\\") ? `${v.drive}\\` : `${v.drive}\\`);
                                }}
                            >
                                {t("space.pick")}
                            </button>
                        </div>
                    ))}
                </div>
            ) : null}

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {report ? (
                <>
                    <div role="status" style={{ ...rowStyle, fontSize: "var(--typography-size-sm)", flexWrap: "wrap" }}>
                        <span style={{ fontWeight: 600 }}>{fmtBytes(report.logicalBytes)}</span>
                        <span style={{ color: "var(--color-text-muted)" }}>
                            {t("space.stats.files", { count: report.fileCount ?? 0 })} ·{" "}
                            {t("space.stats.dirs", { count: report.dirCount ?? 0 })} ·{" "}
                            {report.durationMs} ms · {report.consistency}
                        </span>
                        {report.cancelled ? <span>{t("space.cancelled")}</span> : null}
                        {(report.unreadableDirs ?? 0) + (report.unreadableFiles ?? 0) + (report.disappeared ?? 0) > 0 ? (
                            <span style={{ color: "var(--color-danger)" }}>
                                {t("space.stats.unreadable", {
                                    dirs: report.unreadableDirs ?? 0,
                                    files: (report.unreadableFiles ?? 0) + (report.disappeared ?? 0),
                                })}
                            </span>
                        ) : null}
                        {report.growthDeltaBytes != null ? (
                            <span style={{ color: report.growthDeltaBytes > 0 ? "var(--color-danger)" : "var(--color-success)" }}>
                                {t("space.growth")}: {fmtBytes(report.growthDeltaBytes)}
                                {report.growthPercent != null
                                    ? ` (${report.growthPercent > 0 ? "+" : ""}${report.growthPercent.toFixed(1)}%)`
                                    : ""}
                            </span>
                        ) : null}
                    </div>

                    {/* 目录钻取（§14）：breadcrumb + 子目录排行 + 占比条（无需重新扫描） */}
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                        <div style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                            {crumbs.map((c, i) => (
                                <span key={c.rel} style={{ display: "flex", gap: "var(--spacing-xs)", alignItems: "center" }}>
                                    {i > 0 ? <span aria-hidden>/</span> : null}
                                    <button
                                        type="button"
                                        onClick={() => setCurrentRel(c.rel)}
                                        style={{ ...boxStyle, padding: "0 var(--spacing-xs)", cursor: "pointer" }}
                                    >
                                        {c.label}
                                    </button>
                                </span>
                            ))}
                            <span style={{ color: "var(--color-text-muted)", marginLeft: "auto" }}>
                                {fmtBytes(parentSize)}
                            </span>
                        </div>
                        {children.map((d) => {
                            const pct = parentSize > 0 ? ((d.logicalSize ?? 0) * 100) / parentSize : 0;
                            return (
                                <div key={d.rel} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                                    <button
                                        type="button"
                                        onClick={() => setCurrentRel(d.rel)}
                                        style={{ ...boxStyle, padding: "0 var(--spacing-xs)", cursor: "pointer" }}
                                    >
                                        {d.rel.split("/").pop()}
                                    </button>
                                    <span style={volumeBar}>
                                        <span style={volumeFill(pct)} />
                                    </span>
                                    <span style={{ color: "var(--color-text-muted)" }}>
                                        {fmtBytes(d.logicalSize)} · {pct.toFixed(0)}%
                                    </span>
                                </div>
                            );
                        })}
                        {currentRel && children.length === 0 ? (
                            <div style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-sm)" }}>
                                {t("space.noSubdirs")}
                            </div>
                        ) : null}
                    </div>

                    {/* 最大文件（§9：分析结果，不代表应删） */}
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                        <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("space.largestFiles")}</strong>
                        {report.largestFiles.slice(0, 20).map((f) => (
                            <div key={f.path} style={{ ...listRow(), fontSize: "var(--typography-size-sm)" }}>
                                <span title={f.path}>{f.name}</span>
                                <span style={{ color: "var(--color-text-muted)" }}>{fmtBytes(f.size)}</span>
                                <span style={{ color: "var(--color-text-muted)", marginLeft: "auto" }}>
                                    {f.category}
                                    {f.extension ? ` · .${f.extension}` : ""}
                                </span>
                            </div>
                        ))}
                    </div>

                    {/* 文件类型统计（§10） */}
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                        <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("space.types")}</strong>
                        {report.typeStats.slice(0, 10).map((x) => (
                            <div key={`${x.extension}-${x.category}`} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                                <span style={{ minWidth: "6em" }}>{x.extension === "" ? t("space.types.none") : `.${x.extension}`}</span>
                                <span style={volumeBar}>
                                    <span style={volumeFill(((x.bytes ?? 0) * 100) / typeMax)} />
                                </span>
                                <span style={{ color: "var(--color-text-muted)" }}>
                                    {fmtBytes(x.bytes)} · {x.count}
                                </span>
                            </div>
                        ))}
                    </div>

                    {/* 清理候选（§29-§32：候选≠垃圾，带 Evidence） */}
                    {report.candidates.length > 0 ? (
                        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                            <strong style={{ fontSize: "var(--typography-size-sm)" }}>
                                {t("space.candidates")}（{report.candidates.length}）
                            </strong>
                            {report.candidates.slice(0, 30).map((c) => (
                                <div key={c.path} style={{ ...listRow(), alignItems: "flex-start" }}>
                                    <input
                                        type="checkbox"
                                        aria-label={`${t("space.select")} ${c.path}`}
                                        checked={selected.has(c.path)}
                                        onChange={(e) => {
                                            const next = new Set(selected);
                                            if (e.target.checked) next.add(c.path);
                                            else next.delete(c.path);
                                            setSelected(next);
                                        }}
                                    />
                                    <div style={{ display: "flex", flexDirection: "column", gap: 2, flex: 1 }}>
                                        <span title={c.path} style={{ fontSize: "var(--typography-size-sm)" }}>
                                            {c.path.split(/[\\/]/).pop()}{" "}
                                            <span style={{ color: "var(--color-text-muted)" }}>
                                                {fmtBytes(c.size)} · {t(`space.candidate.${c.kind}`)} · {t(`space.risk.${c.risk}`)}
                                            </span>
                                        </span>
                                        <span style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-xs)" }}>
                                            {c.reasons.join("；")}
                                        </span>
                                    </div>
                                </div>
                            ))}
                            <div style={rowStyle}>
                                <button
                                    type="button"
                                    disabled={busy || selected.size === 0}
                                    onClick={doCleanup}
                                    style={boxStyle}
                                >
                                    {t("space.clean")}
                                </button>
                                <span style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-xs)" }}>
                                    {t("space.cleanHint")}
                                </span>
                            </div>
                        </div>
                    ) : null}

                    {cleanupResult ? (
                        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                            <div>
                                {t("space.cleanup.done", {
                                    recycled: cleanupResult.recycled ?? 0,
                                    stale: cleanupResult.staleSkipped ?? 0,
                                })}
                            </div>
                            <div style={{ color: "var(--color-text-muted)" }}>
                                {t("space.cleanup.expected")}: {fmtBytes(cleanupResult.expectedLogicalFreed)} ·{" "}
                                {t("space.cleanup.observed")}:{" "}
                                {cleanupResult.observedFreeDeltaBytes != null
                                    ? fmtBytes(cleanupResult.observedFreeDeltaBytes)
                                    : locale === "zh-CN"
                                      ? "不可测"
                                      : "n/a"}
                            </div>
                        </div>
                    ) : null}
                </>
            ) : null}
        </section>
    );
}

function listRow(): CSSProperties {
    return {
        display: "flex",
        gap: "var(--spacing-md)",
        alignItems: "center",
        padding: "var(--spacing-xs) var(--spacing-sm)",
        borderBottom: "1px solid var(--color-border)",
    };
}
