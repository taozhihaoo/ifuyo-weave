import { useCallback, useState, type CSSProperties } from "react";
import type { PrivacyFindingDto, PrivacyReportDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.4：Share Preflight 隐私预检（Scan→Findings→Review→Safe Copy→Re-scan §77）。 */

interface PrivacyPanelProps {
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
const listRow: CSSProperties = {
    display: "flex",
    gap: "var(--spacing-sm)",
    alignItems: "center",
    padding: "var(--spacing-xs) var(--spacing-sm)",
    borderBottom: "1px solid var(--color-border)",
    fontSize: "var(--typography-size-sm)",
};

function severityColor(severity: string): string {
    if (severity === "high") return "var(--color-danger)";
    if (severity === "medium") return "var(--color-accent)";
    return "var(--color-text-muted)";
}

export function PrivacyPanel({ onOperationDone }: PrivacyPanelProps) {
    const t = useT();
    const [root, setRoot] = useState("");
    const [outputDir, setOutputDir] = useState("");
    const [report, setReport] = useState<PrivacyReportDto | null>(null);
    const [beforeJobId, setBeforeJobId] = useState<string | null>(null);
    const [comparison, setComparison] = useState<{ resolved: number | null; remaining: number | null; new: number | null } | null>(null);
    const [selected, setSelected] = useState<Set<string>>(new Set());
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [disclaimer, setDisclaimer] = useState(false);

    const doScan = useCallback(
        (isRescan: boolean) => {
            if (!root.trim()) {
                setError(t("privacy.error.root"));
                return;
            }
            setBusy(true);
            setError(null);
            commands
                .privacyScan({ root: root.trim() })
                .then((r) => {
                    if (r.status !== "ok") {
                        setBusy(false);
                        setError(r.error.code);
                        return;
                    }
                    if (isRescan) setBeforeJobId(r.data.jobId);
                    const stop = pollJob(r.data.jobId, (tick) => {
                        if (tick.kind === "failed") {
                            stop();
                            setBusy(false);
                            setError(tick.error.code);
                            return;
                        }
                        if (isTerminalState(tick.status)) {
                            stop();
                            commands
                                .privacyGetResult(r.data.jobId)
                                .then((gr) => {
                                    setBusy(false);
                                    if (gr.status !== "ok") {
                                        setError(gr.error.code);
                                        return;
                                    }
                                    setReport(gr.data);
                                    // 默认勾选：低风险 Confirmed 推荐项（§34）
                                    setSelected(
                                        new Set(
                                            gr.data.findings
                                                .filter((f) => f.recommended)
                                                .map((f) => f.fingerprint),
                                        ),
                                    );
                                    if (isRescan && beforeJobId) {
                                        commands
                                            .privacyCompare({
                                                beforeJobId,
                                                afterJobId: r.data.jobId,
                                            })
                                            .then((cr) => {
                                                if (cr.status === "ok") setComparison(cr.data);
                                            })
                                            .catch(() => {});
                                    }
                                })
                                .catch((e: unknown) =>
                                    setError(e instanceof Error ? e.message : String(e)),
                                );
                        }
                    });
                })
                .catch((e: unknown) => {
                    setBusy(false);
                    setError(e instanceof Error ? e.message : String(e));
                });
        },
        [root, beforeJobId, t],
    );

    const doExport = useCallback(() => {
        if (!report || selected.size === 0 || !outputDir.trim()) {
            setError(t("privacy.error.exportNeeds"));
            return;
        }
        setBusy(true);
        setError(null);
        // 按 relative path 分组：同一文件的 findings 合成一个 item
        const byPath = new Map<string, PrivacyFindingDto[]>();
        for (const f of report.findings) {
            if (!selected.has(f.fingerprint)) continue;
            const list = byPath.get(f.relativePath) ?? [];
            list.push(f);
            byPath.set(f.relativePath, list);
        }
        const items = [...byPath.entries()].map(([rel, findings]) => {
            const abs = `${report.root.replace(/[\\/]+$/, "")}\\${rel.replace(/\//g, "\\")}`;
            const safeName = rel.split("/").pop() ?? "file";
            const dot = safeName.lastIndexOf(".");
            const outputSafe =
                dot > 0
                    ? `${safeName.slice(0, dot)}.safe${safeName.slice(dot)}`
                    : `${safeName}.safe`;
            return {
                source: abs,
                output: `${outputDir.trim().replace(/[\\/]+$/, "")}\\${outputSafe}`,
                size: 0,
                modifiedMs: null,
                findings: findings.map((f) => ({
                    relativePath: f.relativePath,
                    subCategory: f.subCategory,
                    cleanup: f.cleanup,
                })),
            };
        });
        commands
            .privacyExportSafeCopy(items)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setDisclaimer(true);
                onOperationDone();
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [report, selected, outputDir, onOperationDone, t]);

    // 导出时需要源文件 stat（stale 重验）——从 report 推不出 size/mtime，
    // 由 export 命令内部现读文件系统（服务端已实现）；此处 UI 只传路径与选择。
    const high = report?.highCount ?? 0;
    const medium = report?.mediumCount ?? 0;
    const low = report?.lowCount ?? 0;

    return (
        <section
            aria-label={t("privacy.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <div style={rowStyle}>
                <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("privacy.title")}</h2>
            </div>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("privacy.hint")}
            </div>

            <div style={rowStyle}>
                <input
                    aria-label={t("privacy.root")}
                    value={root}
                    onChange={(e) => setRoot(e.target.value)}
                    placeholder="C:\path\to\share"
                    style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
                />
                <button type="button" disabled={busy} onClick={() => doScan(false)} style={boxStyle}>
                    {busy ? t("privacy.scanning") : t("privacy.scan")}
                </button>
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {report ? (
                <>
                    <div role="status" style={{ ...rowStyle, fontSize: "var(--typography-size-sm)", flexWrap: "wrap" }}>
                        <span style={{ color: "var(--color-text-muted)" }}>
                            {t("privacy.stats.files", {
                                scanned: report.filesScanned ?? 0,
                                unsupported: report.filesUnsupported ?? 0,
                                failed: report.filesFailed ?? 0,
                            })}
                        </span>
                        <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>
                            High {high}
                        </span>
                        <span style={{ color: "var(--color-accent)", fontWeight: 600 }}>
                            Medium {medium}
                        </span>
                        <span style={{ color: "var(--color-text-muted)", fontWeight: 600 }}>
                            Low {low}
                        </span>
                        <span style={{ color: "var(--color-text-muted)" }}>
                            {t("privacy.stats.confirmed")}: {report.confirmedCount} ·{" "}
                            {t("privacy.stats.suspected")}: {report.suspectedCount}
                        </span>
                    </div>

                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                        {report.findings.map((f) => (
                            <div key={f.fingerprint} style={listRow}>
                                <input
                                    type="checkbox"
                                    aria-label={`${t("privacy.select")} ${f.subCategory}`}
                                    checked={selected.has(f.fingerprint)}
                                    onChange={(e) => {
                                        const next = new Set(selected);
                                        if (e.target.checked) next.add(f.fingerprint);
                                        else next.delete(f.fingerprint);
                                        setSelected(next);
                                    }}
                                />
                                <span style={{ color: severityColor(f.severity), fontWeight: 600 }}>
                                    {f.severity.toUpperCase()}
                                </span>
                                <span title={f.description}>{f.subCategory}</span>
                                <span style={{ color: "var(--color-text-muted)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                    {f.relativePath} · {f.location}
                                </span>
                                <span style={{ fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)", whiteSpace: "nowrap" }}>
                                    {f.evidence === "confirmed" ? t("privacy.evidence.confirmed") : t("privacy.evidence.suspected")}
                                </span>
                            </div>
                        ))}
                        {report.findings.length === 0 ? (
                            <div style={{ color: "var(--color-text-muted)" }}>{t("privacy.noFindings")}</div>
                        ) : null}
                    </div>

                    {report.notFullyChecked.length > 0 ? (
                        <div style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                            <strong>{t("privacy.notChecked.title")}</strong>
                            {report.notFullyChecked.map(([path, reason]) => (
                                <div key={path} style={{ color: "var(--color-text-muted)" }}>
                                    {path}: {reason}
                                </div>
                            ))}
                        </div>
                    ) : null}

                    <div style={rowStyle}>
                        <input
                            aria-label={t("privacy.outputDir")}
                            value={outputDir}
                            onChange={(e) => setOutputDir(e.target.value)}
                            placeholder="C:\path\to\safe-output"
                            style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
                        />
                        <button
                            type="button"
                            disabled={busy || selected.size === 0 || !outputDir.trim()}
                            onClick={doExport}
                            style={boxStyle}
                        >
                            {t("privacy.export")}
                        </button>
                        <span style={{ fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                            {t("privacy.originalUntouched")}
                        </span>
                    </div>

                    {comparison ? (
                        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                            {t("privacy.rescan.resolved")}: {comparison.resolved} ·{" "}
                            {t("privacy.rescan.remaining")}: {comparison.remaining} ·{" "}
                            {t("privacy.rescan.new")}: {comparison.new}
                        </div>
                    ) : null}

                    {report.findings.length > 0 ? (
                        <button
                            type="button"
                            disabled={busy}
                            onClick={() => doScan(true)}
                            style={{ ...boxStyle, alignSelf: "flex-start" }}
                        >
                            {t("privacy.rescan")}
                        </button>
                    ) : null}

                    {disclaimer ? (
                        <div role="status" style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                            {t("privacy.disclaimer")}
                        </div>
                    ) : null}
                </>
            ) : null}
        </section>
    );
}
