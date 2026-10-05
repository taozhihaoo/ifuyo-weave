import { useCallback, useState, type CSSProperties } from "react";
import type {
    CompareReportDto,
    DifferenceItemDto,
    JobStatusDto,
} from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.1：Compare 对比中心（Read-only 分析 + Actionable 差异经 Plan 复用 M2.0 执行）。 */

interface ComparePanelProps {
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
const listRowStyle: CSSProperties = {
    display: "flex",
    gap: "var(--spacing-md)",
    alignItems: "center",
    padding: "var(--spacing-xs) var(--spacing-sm)",
    borderBottom: "1px solid var(--color-border)",
    fontSize: "var(--typography-size-sm)",
};
const chip = (color: string): CSSProperties => ({
    color,
    fontWeight: 600,
    whiteSpace: "nowrap",
});

function kindColor(kind: string): string {
    if (kind === "same") return "var(--color-text-muted)";
    if (kind === "added") return "var(--color-success)";
    if (kind === "removed" || kind === "error" || kind === "unreadable")
        return "var(--color-danger)";
    return "var(--color-accent)";
}

export function ComparePanel({ locale, onOperationDone }: ComparePanelProps) {
    const t = useT();
    const [leftPath, setLeftPath] = useState("");
    const [rightPath, setRightPath] = useState("");
    const [policy, setPolicy] = useState<"metadata" | "hash">("hash");
    const [report, setReport] = useState<CompareReportDto | null>(null);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [filter, setFilter] = useState<string>("all");
    const [selected, setSelected] = useState<Set<string>>(new Set());
    const [detail, setDetail] = useState<string | null>(null);

    const options = useCallback(
        () => ({
            contentCheck: policy,
            ignoreUser: [] as string[],
            maxTextDetailBytes: null,
        }),
        [policy],
    );

    const doCompare = useCallback(() => {
        if (!leftPath.trim() || !rightPath.trim()) {
            setError(t("compare.error.twoPaths"));
            return;
        }
        setBusy(true);
        setError(null);
        setReport(null);
        setSelected(new Set());
        setDetail(null);
        const leftIsDir = /[\\/]$/.test(leftPath) === false ? undefined : undefined;
        void leftIsDir;
        // 根种类由服务端判定：先按目录尝试（两侧目录 ⇒ job 化），失败再走文件
        commands
            .compareDirectories(leftPath.trim(), rightPath.trim(), options())
            .then((r) => {
                if (r.status === "ok") {
                    const stop = pollJob(r.data.jobId, (tick) => {
                        if (tick.kind === "failed") {
                            stop();
                            // 目录比较拒绝（非目录根）⇒ 文件同步比较
                            void commands
                                .compareFiles(leftPath.trim(), rightPath.trim(), options())
                                .then((fr) => {
                                    setBusy(false);
                                    if (fr.status === "ok") {
                                        setReport(fr.data);
                                    } else {
                                        setError(fr.error.code);
                                    }
                                })
                                .catch((e: unknown) => {
                                    setBusy(false);
                                    setError(e instanceof Error ? e.message : String(e));
                                });
                            return;
                        }
                        const status: JobStatusDto = tick.status;
                        if (isTerminalState(status)) {
                            stop();
                            void commands
                                .compareGetResult(r.data.jobId)
                                .then((gr) => {
                                    setBusy(false);
                                    if (gr.status === "ok") {
                                        setReport(gr.data);
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
                } else if (r.error.code === "compare.rootMissing") {
                    // 非目录 ⇒ 文件同步比较
                    return commands
                        .compareFiles(leftPath.trim(), rightPath.trim(), options())
                        .then((fr) => {
                            setBusy(false);
                            if (fr.status === "ok") {
                                setReport(fr.data);
                            } else {
                                setError(fr.error.code);
                            }
                        });
                } else {
                    setBusy(false);
                    setError(r.error.code);
                }
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [leftPath, rightPath, options, t]);

    const showDetail = useCallback(
        (item: DifferenceItemDto) => {
            if (!report || item.node !== "file") return;
            const left = item.left?.path ?? report.leftRoot + "/" + item.relativePath;
            const right = item.right?.path ?? report.rightRoot + "/" + item.relativePath;
            commands
                .compareFileDetail(left, right, options())
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    if (r.data.tooLarge || !r.data.report) {
                        setDetail(t("compare.detail.tooLarge"));
                        return;
                    }
                    const lines: string[] = [];
                    for (const hunk of r.data.report.hunks) {
                        for (const l of hunk.lines) {
                            const prefix =
                                l.change === "added"
                                    ? "+"
                                    : l.change === "removed"
                                      ? "-"
                                      : l.change === "changed"
                                        ? "~"
                                        : " ";
                            const text =
                                l.change === "added"
                                    ? l.bText
                                    : l.change === "removed"
                                      ? l.aText
                                      : `${l.aText} | ${l.bText}`;
                            lines.push(`${prefix} ${text}`);
                        }
                    }
                    const meta: string[] = [];
                    if (r.data.lineEndingDiffers) meta.push(t("compare.detail.lineEnding"));
                    if (r.data.report.degraded) meta.push(t("compare.detail.degraded"));
                    setDetail(
                        [
                            ...meta,
                            `${t("compare.detail.stats")}: +${r.data.report.stats.added} -${r.data.report.stats.removed} ~${r.data.report.stats.changed}`,
                            "",
                            ...lines,
                        ].join("\n") || t("compare.detail.identical"),
                    );
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [report, options, t],
    );

    const applyCopy = useCallback(
        (direction: "leftToRight" | "rightToLeft") => {
            if (!report || selected.size === 0) return;
            setBusy(true);
            setError(null);
            const actions = report.items
                .filter(
                    (i) =>
                        selected.has(i.relativePath) &&
                        (i.kind === "modified" ||
                            i.kind === "removed" ||
                            i.kind === "added" ||
                            i.kind === "typeChanged"),
                )
                .map((i) => {
                    const src = direction === "leftToRight" ? i.left : i.right;
                    return {
                        relativePath: i.relativePath,
                        direction,
                        size: src?.size ?? null,
                        modifiedMs: src?.modifiedMs ?? null,
                        digestHex: src?.digestHex ?? null,
                    };
                });
            if (actions.length === 0) {
                setBusy(false);
                setError(t("compare.error.nothingApplicable"));
                return;
            }
            commands
                .compareBuildPlan(report.leftRoot, report.rightRoot, actions)
                .then((r) => {
                    if (r.status !== "ok") {
                        setBusy(false);
                        setError(r.error.code);
                        return;
                    }
                    const planId = r.data.operationId;
                    commands
                        .executePlan(planId)
                        .then((er) => {
                            if (er.status !== "ok") {
                                setBusy(false);
                                setError(er.error.code);
                                return;
                            }
                            const stop = pollJob(er.data.jobId, (tick) => {
                                if (tick.kind === "failed") {
                                    stop();
                                    setBusy(false);
                                    setError(tick.error.code);
                                    return;
                                }
                                if (isTerminalState(tick.status)) {
                                    stop();
                                    setBusy(false);
                                    onOperationDone();
                                    doCompare();
                                }
                            });
                        })
                        .catch((e: unknown) => {
                            setBusy(false);
                            setError(e instanceof Error ? e.message : String(e));
                        });
                })
                .catch((e: unknown) => {
                    setBusy(false);
                    setError(e instanceof Error ? e.message : String(e));
                });
        },
        [report, selected, onOperationDone, doCompare, t],
    );

    const applyDelete = useCallback(() => {
        if (!report || selected.size === 0) return;
        void 0;
        setBusy(true);
        setError(null);
        // 删除作用于选中条目所在的一侧（有 left 事实 ⇒ 左侧；否则右侧）
        const leftPaths = report.items
            .filter((i) => selected.has(i.relativePath) && i.left && i.kind !== "added")
            .map((i) => i.relativePath);
        const rightPaths = report.items
            .filter((i) => selected.has(i.relativePath) && !i.left && i.right)
            .map((i) => i.relativePath);
        const run = (root: string, paths: string[]): Promise<void> =>
            paths.length === 0
                ? Promise.resolve()
                : commands.compareRecycle(root, paths).then((r) => {
                      if (r.status !== "ok") throw new Error(r.error.code);
                  });
        Promise.all([run(report.leftRoot, leftPaths), run(report.rightRoot, rightPaths)])
            .then(() => {
                setBusy(false);
                onOperationDone();
                doCompare();
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [report, selected, onOperationDone, doCompare]);

    const s = report?.summary;
    const filtered = report
        ? report.items.filter((i) => filter === "all" || i.kind === filter)
        : [];

    return (
        <section
            aria-label={t("compare.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("compare.title")}</h2>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("compare.hint")}
            </div>

            <div style={rowStyle}>
                <input
                    aria-label={t("compare.left")}
                    value={leftPath}
                    onChange={(e) => setLeftPath(e.target.value)}
                    placeholder="C:\path\left"
                    style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                />
                <span aria-hidden>↔</span>
                <input
                    aria-label={t("compare.right")}
                    value={rightPath}
                    onChange={(e) => setRightPath(e.target.value)}
                    placeholder="C:\path\right"
                    style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                />
                <select
                    aria-label={t("compare.policy")}
                    value={policy}
                    onChange={(e) => setPolicy(e.target.value as typeof policy)}
                    style={boxStyle}
                >
                    <option value="hash">{t("compare.policy.hash")}</option>
                    <option value="metadata">{t("compare.policy.metadata")}</option>
                </select>
                <button type="button" disabled={busy} onClick={doCompare} style={boxStyle}>
                    {busy ? t("compare.comparing") : t("compare.run")}
                </button>
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {report && s ? (
                <>
                    <div role="status" style={{ ...rowStyle, flexWrap: "wrap", fontSize: "var(--typography-size-sm)" }}>
                        <span style={chip("var(--color-text-muted)")}>{t("compare.summary.same")}: {s.same}</span>
                        <span style={chip("var(--color-accent)")}>{t("compare.summary.modified")}: {s.modified}</span>
                        <span style={chip("var(--color-success)")}>{t("compare.summary.added")}: {s.added}</span>
                        <span style={chip("var(--color-danger)")}>{t("compare.summary.removed")}: {s.removed}</span>
                        {(s.typeChanged ?? 0) > 0 ? <span>{t("compare.summary.typeChanged")}: {s.typeChanged}</span> : null}
                        {(s.moved ?? 0) > 0 ? <span>{t("compare.summary.moved")}: {s.moved}</span> : null}
                        {(s.renamed ?? 0) > 0 ? <span>{t("compare.summary.renamed")}: {s.renamed}</span> : null}
                        {(s.unreadable ?? 0) + (s.error ?? 0) > 0 ? (
                            <span style={chip("var(--color-danger)")}>{t("compare.summary.errors")}: {(s.unreadable ?? 0) + (s.error ?? 0)}</span>
                        ) : null}
                        {(s.ignored ?? 0) > 0 ? (
                            <span style={{ color: "var(--color-text-muted)" }}>{t("compare.summary.ignored")}: {s.ignored}</span>
                        ) : null}
                        {report.cancelled ? <span>{t("compare.cancelled")}</span> : null}
                    </div>

                    <div style={rowStyle}>
                        <select
                            aria-label={t("compare.filter")}
                            value={filter}
                            onChange={(e) => setFilter(e.target.value)}
                            style={boxStyle}
                        >
                            <option value="all">{t("compare.filter.all")}</option>
                            <option value="modified">{t("compare.summary.modified")}</option>
                            <option value="added">{t("compare.summary.added")}</option>
                            <option value="removed">{t("compare.summary.removed")}</option>
                            <option value="same">{t("compare.summary.same")}</option>
                            <option value="error">{t("compare.summary.errors")}</option>
                        </select>
                        <button
                            type="button"
                            disabled={busy || selected.size === 0}
                            onClick={() => applyCopy("leftToRight")}
                            style={boxStyle}
                        >
                            {t("compare.action.copyLeftToRight")}
                        </button>
                        <button
                            type="button"
                            disabled={busy || selected.size === 0}
                            onClick={() => applyCopy("rightToLeft")}
                            style={boxStyle}
                        >
                            {t("compare.action.copyRightToLeft")}
                        </button>
                        <button
                            type="button"
                            disabled={busy || selected.size === 0}
                            onClick={applyDelete}
                            style={boxStyle}
                        >
                            {t("compare.action.recycle")}
                        </button>
                        <span style={{ fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                            {t("compare.action.hint")}
                        </span>
                    </div>

                    <div style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", maxHeight: "22em", overflowY: "auto" }}>
                        {filtered.map((item) => (
                            <div key={item.relativePath} style={listRowStyle}>
                                <input
                                    type="checkbox"
                                    aria-label={`${t("compare.select")} ${item.relativePath}`}
                                    checked={selected.has(item.relativePath)}
                                    onChange={(e) => {
                                        const next = new Set(selected);
                                        if (e.target.checked) {
                                            next.add(item.relativePath);
                                        } else {
                                            next.delete(item.relativePath);
                                        }
                                        setSelected(next);
                                    }}
                                />
                                <button
                                    type="button"
                                    onClick={() => showDetail(item)}
                                    style={{
                                        ...boxStyle,
                                        padding: "0 var(--spacing-xs)",
                                        cursor: item.node === "file" ? "pointer" : "default",
                                    }}
                                    title={t("compare.detail.open")}
                                >
                                    {item.relativePath}
                                </button>
                                <span style={chip(kindColor(item.kind))}>{t(`compare.kind.${item.kind}`)}</span>
                                {item.left?.size != null ? (
                                    <span style={{ color: "var(--color-text-muted)" }}>{item.left.size} B</span>
                                ) : null}
                                {item.right?.size != null ? (
                                    <span style={{ color: "var(--color-text-muted)" }}>{item.right.size} B</span>
                                ) : null}
                                <span style={{ marginLeft: "auto", color: "var(--color-text-muted)", fontSize: "var(--typography-size-xs)" }}>
                                    {locale === "zh-CN" ? t(`compare.node.${item.node}`) : t(`compare.node.${item.node}`)}
                                </span>
                            </div>
                        ))}
                        {filtered.length === 0 ? (
                            <div style={{ padding: "var(--spacing-sm)", color: "var(--color-text-muted)" }}>
                                {t("compare.empty")}
                            </div>
                        ) : null}
                    </div>

                    {detail ? (
                        <pre
                            aria-label={t("compare.detail.title")}
                            style={{
                                margin: 0,
                                padding: "var(--spacing-sm)",
                                border: "1px solid var(--color-border)",
                                borderRadius: "var(--radius-sm)",
                                fontFamily: "var(--typography-mono-family)",
                                fontSize: "var(--typography-size-sm)",
                                maxHeight: "18em",
                                overflowY: "auto",
                                whiteSpace: "pre-wrap",
                            }}
                        >
                            {detail}
                        </pre>
                    ) : null}
                </>
            ) : null}
        </section>
    );
}
