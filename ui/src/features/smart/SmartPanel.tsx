import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { PlanDto, SmartAnalysisResult, SmartPlanBundleDto, SmartSuggestion } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.12：Smart Processing 智能处理中心（§20 Workbench——Advisor 不是 Authority）。 */

interface SmartPanelProps {
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

type Mode = "rename" | "organize" | "classify" | "cleanup";

export function SmartPanel({ onOperationDone }: SmartPanelProps) {
    const t = useT();
    const [inputs, setInputs] = useState("");
    const [mode, setMode] = useState<Mode>("rename");
    const [result, setResult] = useState<SmartAnalysisResult | null>(null);
    const [accepted, setAccepted] = useState<Set<string>>(new Set());
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);
    const [plans, setPlans] = useState<SmartPlanBundleDto | null>(null);
    const [applied, setApplied] = useState<string[]>([]);
    const pollStop = useRef<(() => void) | null>(null);

    const fail = useCallback((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setBusy(false);
    }, []);

    useEffect(() => () => pollStop.current?.(), []);

    const analyze = useCallback(() => {
        const inputList = inputs
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
        if (inputList.length === 0) {
            setError(t("smart.needInput"));
            return;
        }
        setBusy(true);
        setError(null);
        setResult(null);
        setPlans(null);
        setApplied([]);
        commands
            .smartAnalyze(inputList, [mode])
            .then((r) => {
                if (r.status !== "ok") {
                    setError(`${r.error.code}: ${r.error.message}`);
                    setBusy(false);
                    return;
                }
                pollStop.current?.();
                pollStop.current = pollJob(r.data.jobId, (tick) => {
                    if (tick.kind === "status" && isTerminalState(tick.status)) {
                        pollStop.current?.();
                        commands
                            .smartGetResult(r.data.jobId)
                            .then((gr) => {
                                setBusy(false);
                                if (gr.status === "ok") {
                                    setResult(gr.data);
                                    // §21：只默认勾选 High Confidence（绝不默认全选低置信）
                                    setAccepted(
                                        new Set(
                                            gr.data.suggestions
                                                .filter((s) => s.confidence === "high")
                                                .map((s) => s.id),
                                        ),
                                    );
                                } else {
                                    setError(`${gr.error.code}: ${gr.error.message}`);
                                }
                            })
                            .catch(fail);
                    } else if (tick.kind === "failed") {
                        pollStop.current?.();
                        setBusy(false);
                        setError(`${tick.error.code}: ${tick.error.message}`);
                    }
                });
            })
            .catch(fail);
    }, [fail, inputs, mode, t]);

    const toggle = useCallback((id: string) => {
        setAccepted((prev) => {
            const next = new Set(prev);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
    }, []);

    const selectAllHigh = useCallback(() => {
        if (!result) return;
        setAccepted(
            new Set(result.suggestions.filter((s) => s.confidence === "high").map((s) => s.id)),
        );
    }, [result]);

    const buildPlan = useCallback(() => {
        setError(null);
        commands
            .smartBuildPlan([...accepted])
            .then((r) => {
                if (r.status !== "ok") {
                    setError(`${r.error.code}: ${r.error.message}`);
                    return;
                }
                setPlans(r.data);
            })
            .catch(fail);
    }, [accepted, fail]);

    const execute = useCallback(
        (operationId: string, recycle: boolean) => {
            setError(null);
            const call = recycle
                ? commands.executeRecyclePlan(operationId)
                : commands.executePlan(operationId);
            call.then((r) => {
                if (r.status !== "ok") {
                    setError(`${r.error.code}: ${r.error.message}`);
                    return;
                }
                pollStop.current?.();
                pollStop.current = pollJob(r.data.jobId, (tick) => {
                    if (tick.kind === "status" && isTerminalState(tick.status)) {
                        pollStop.current?.();
                        setApplied((prev) => [...prev, operationId]);
                        onOperationDone();
                    } else if (tick.kind === "failed") {
                        pollStop.current?.();
                        setError(`${tick.error.code}: ${tick.error.message}`);
                    }
                });
            }).catch(fail);
        },
        [fail, onOperationDone],
    );

    const planLine = (plan: PlanDto | undefined, recycle: boolean, key: string) => {
        if (!plan) return null;
        const done = applied.includes(plan.operationId);
        return (
            <div style={rowStyle} key={key}>
                <span style={{ fontSize: 12 }}>
                    {t("smart.plan.kind", { kind: plan.kind })} ·{" "}
                    {t("smart.plan.counts", {
                        ready: plan.readyCount ?? 0,
                        conflict: plan.conflictCount ?? 0,
                        noop: plan.noopCount ?? 0,
                    })}
                </span>
                <button onClick={() => execute(plan.operationId, recycle)} disabled={done || busy}>
                    {done ? t("smart.plan.done") : t("smart.plan.execute")}
                </button>
            </div>
        );
    };

    const stats = result?.stats;

    return (
        <section aria-label={t("smart.title")}>
            <h2>{t("smart.title")}</h2>

            {/* 输入 + 模式（§20） */}
            <div style={boxStyle}>
                <div style={rowStyle}>
                    <select value={mode} onChange={(e) => setMode(e.target.value as Mode)}>
                        <option value="rename">{t("smart.mode.rename")}</option>
                        <option value="organize">{t("smart.mode.organize")}</option>
                        <option value="classify">{t("smart.mode.classify")}</option>
                        <option value="cleanup">{t("smart.mode.cleanup")}</option>
                    </select>
                    <button onClick={analyze} disabled={busy}>
                        {busy ? t("smart.analyzing") : t("smart.analyze")}
                    </button>
                    <span style={{ fontSize: 11 }}>{t("smart.localNote")}</span>
                </div>
                <textarea
                    value={inputs}
                    onChange={(e) => setInputs(e.target.value)}
                    rows={3}
                    style={{ width: "100%" }}
                    placeholder={t("smart.inputPlaceholder")}
                />
                {error ? (
                    <p role="alert" style={{ color: "var(--color-danger)" }}>
                        {error}
                    </p>
                ) : null}
            </div>

            {/* 分析结果（§20 摘要 + §21 Review 列表） */}
            {result && stats ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{t("smart.result.title", { count: stats.filesAnalyzed ?? 0 })}</strong>
                        <span style={{ fontSize: 12 }}>
                            {t("smart.result.high", { n: stats.highConfidence ?? 0 })} ·{" "}
                            {t("smart.result.medium", { n: stats.mediumConfidence ?? 0 })} ·{" "}
                            {t("smart.result.low", { n: stats.lowConfidence ?? 0 })}
                        </span>
                    </div>
                    <div style={rowStyle}>
                        <button onClick={selectAllHigh}>{t("smart.selectHigh")}</button>
                        <button onClick={buildPlan} disabled={accepted.size === 0}>
                            {t("smart.previewPlan", { count: accepted.size })}
                        </button>
                    </div>
                    {result.suggestions.length === 0 ? (
                        <p style={{ fontSize: 12 }}>{t("smart.noSuggestions")}</p>
                    ) : (
                    <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                        <tbody>
                            {result.suggestions.slice(0, 100).map((s: SmartSuggestion) => (
                                <tr key={s.id}>
                                    <td>
                                        <input
                                            type="checkbox"
                                            checked={accepted.has(s.id)}
                                            onChange={() => toggle(s.id)}
                                        />
                                    </td>
                                    <td style={{ wordBreak: "break-all" }}>
                                        {s.originalValue || s.targetPath}
                                    </td>
                                    <td>
                                        → <strong>{s.suggestedValue}</strong>
                                        {s.conflict ? (
                                            <span style={{ color: "var(--color-danger)" }}> ⚠ {s.conflict}</span>
                                        ) : null}
                                    </td>
                                    <td>{t(`smart.confidence.${s.confidence}`)}</td>
                                    <td style={{ fontSize: 11, color: "var(--color-border)" }}>
                                        {s.evidence.map((e) => e.detail).join("; ")}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                    )}

                    {/* Plan 预览 + 执行（§22：经 M2.0 执行核心） */}
                    {plans ? (
                        <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                            <strong style={{ fontSize: 13 }}>{t("smart.plan.title")}</strong>
                            {planLine(plans.renamePlan ?? undefined, false, "rename")}
                            {planLine(plans.movePlan ?? undefined, false, "move")}
                            {planLine(plans.deletePlan ?? undefined, true, "delete")}
                            <p style={{ fontSize: 11, margin: "4px 0 0" }}>
                                {t("smart.plan.note")}
                            </p>
                        </div>
                    ) : null}
                </div>
            ) : null}
        </section>
    );
}
