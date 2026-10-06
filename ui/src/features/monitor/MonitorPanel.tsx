import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { DiffReportDto, MonitorChangeEvent, MonitorSnapshot, MonitorTarget } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.11：Website Monitor 网站监控中心（§31 轻量列表 + 详情）。 */

interface MonitorPanelProps {
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

export function MonitorPanel({ onOperationDone }: MonitorPanelProps) {
    const t = useT();
    const [targets, setTargets] = useState<MonitorTarget[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);
    // 创建表单（§33 精简为单卡——URL/名称/模式/selector/间隔可见即可创建）
    const [name, setName] = useState("");
    const [url, setUrl] = useState("");
    const [mode, setMode] = useState<"wholePage" | "mainContent" | "element">("mainContent");
    const [selector, setSelector] = useState("");
    const [intervalSecs, setIntervalSecs] = useState(1800);
    // 详情（§32 Overview/History/Changes）
    const [detail, setDetail] = useState<MonitorTarget | null>(null);
    const [history, setHistory] = useState<MonitorSnapshot[]>([]);
    const [changes, setChanges] = useState<MonitorChangeEvent[]>([]);
    const [diff, setDiff] = useState<{ seqA: number; seqB: number; report: DiffReportDto } | null>(null);
    const pollStop = useRef<(() => void) | null>(null);

    const fail = useCallback((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setBusy(false);
    }, []);

    const load = useCallback(() => {
        commands
            .monitorList()
            .then(setTargets)
            .catch(() => {});
        if (detail) {
            commands
                .monitorHistory(detail.id)
                .then((r) => setHistory(r))
                .catch(() => {});
        }
    }, [detail]);

    useEffect(() => {
        load();
        const timer = setInterval(load, 5000);
        return () => {
            clearInterval(timer);
            pollStop.current?.();
        };
    }, [load]);

    const openDetail = useCallback((target: MonitorTarget) => {
        setDetail(target);
        setDiff(null);
        commands.monitorHistory(target.id).then(setHistory).catch(() => {});
        commands.monitorChanges(target.id).then(setChanges).catch(() => {});
    }, []);

    const create = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .monitorCreate(name, url, mode, mode === "element" ? selector : null, intervalSecs)
            .then((r) => {
                setBusy(false);
                if (r.status === "ok") {
                    setName("");
                    setUrl("");
                    setSelector("");
                    load();
                    onOperationDone();
                } else {
                    setError(`${r.error.code}: ${r.error.message}`);
                }
            })
            .catch(fail);
    }, [fail, intervalSecs, load, mode, name, onOperationDone, selector, url]);

    const checkNow = useCallback(
        (targetId: string) => {
            setError(null);
            commands
                .monitorCheckNow(targetId)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(`${r.error.code}: ${r.error.message}`);
                        return;
                    }
                    pollStop.current?.();
                    pollStop.current = pollJob(r.data.jobId, (tick) => {
                        if (tick.kind === "status" && isTerminalState(tick.status)) {
                            pollStop.current?.();
                            load();
                            onOperationDone();
                        } else if (tick.kind === "failed") {
                            pollStop.current?.();
                            load();
                        }
                    });
                })
                .catch(fail);
        },
        [fail, load, onOperationDone],
    );

    const togglePaused = useCallback(
        (target: MonitorTarget) => {
            commands
                .monitorSetPaused(target.id, !target.paused)
                .then(() => load())
                .catch(fail);
        },
        [fail, load],
    );

    const remove = useCallback(
        (target: MonitorTarget) => {
            // §63：默认仅删 Target 保留历史（UI 明示；连历史删除暂不暴露）
            commands
                .monitorDelete(target.id, false)
                .then((r) => {
                    if (r.status === "error") setError(r.error.code);
                    if (detail?.id === target.id) setDetail(null);
                    load();
                })
                .catch(fail);
        },
        [detail, fail, load],
    );

    const viewDiff = useCallback((targetId: string) => {
        // 最近两次有正文的快照对比（§56 Current vs Previous）
        const payloadSeqs = history
            .filter((s) => s.payloadFile)
            .map((s) => s.seq ?? 0);
        if (payloadSeqs.length < 2) {
            setError(t("web.monitor.diffNeedTwo"));
            return;
        }
        const seqB = payloadSeqs[0] ?? 0;
        const seqA = payloadSeqs[1] ?? 0;
        commands
            .monitorDiff(targetId, seqA, seqB)
            .then((r) => {
                if (r.status === "ok") {
                    setDiff({ seqA, seqB, report: r.data });
                } else {
                    setError(`${r.error.code}: ${r.error.message}`);
                }
            })
            .catch(fail);
    }, [fail, history, t]);

    const statusLabel = (s: string) => t(`web.monitor.status.${s}`);

    return (
        <section aria-label={t("web.monitor.title")}>
            <h2>{t("web.monitor.title")}</h2>

            {/* 创建（§33：创建前明确比较对象——mode 说明行） */}
            <div style={boxStyle}>
                <div style={rowStyle}>
                    <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("web.monitor.name")} style={{ width: 160 }} />
                    <input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://example.com/news" style={{ flex: 1, minWidth: 220 }} />
                    <select value={mode} onChange={(e) => setMode(e.target.value as typeof mode)}>
                        <option value="wholePage">{t("web.monitor.mode.wholePage")}</option>
                        <option value="mainContent">{t("web.monitor.mode.mainContent")}</option>
                        <option value="element">{t("web.monitor.mode.element")}</option>
                    </select>
                    {mode === "element" ? (
                        <input value={selector} onChange={(e) => setSelector(e.target.value)} placeholder=".article-content" style={{ width: 160 }} />
                    ) : null}
                    <label style={{ fontSize: 12 }}>
                        {t("web.monitor.interval")}
                        <input type="number" min={60} value={intervalSecs} onChange={(e) => setIntervalSecs(Math.max(60, Number(e.target.value) || 1800))} style={{ width: 80 }} />
                        s
                    </label>
                    <button onClick={create} disabled={busy || !name.trim() || !url.trim()}>
                        {t("web.monitor.create")}
                    </button>
                </div>
                <p style={{ fontSize: 11, margin: "4px 0 0" }}>
                    {mode === "wholePage"
                        ? t("web.monitor.scopeNote.wholePage")
                        : mode === "mainContent"
                            ? t("web.monitor.scopeNote.mainContent")
                            : t("web.monitor.scopeNote.element")}
                </p>
                {error ? (
                    <p role="alert" style={{ color: "var(--color-danger)" }}>
                        {error}
                    </p>
                ) : null}
            </div>

            {/* 监控列表（§31） */}
            <h3>{t("web.monitor.list")}</h3>
            {targets.length === 0 ? (
                <p>{t("web.monitor.empty")}</p>
            ) : (
                <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
                    <tbody>
                        {targets.map((tg) => (
                            <tr key={tg.id}>
                                <td>
                                    <button onClick={() => openDetail(tg)} style={{ fontWeight: detail?.id === tg.id ? 700 : 400 }}>
                                        {tg.name}
                                    </button>
                                </td>
                                <td style={{ fontSize: 12 }}>{t("web.monitor.every", { s: tg.intervalSecs })}</td>
                                <td>{statusLabel(tg.status)}</td>
                                <td style={{ fontSize: 12 }}>
                                    {tg.lastCheckedAtMs ? new Date(tg.lastCheckedAtMs).toLocaleTimeString() : "—"}
                                </td>
                                <td>
                                    <button onClick={() => checkNow(tg.id)} disabled={busy || !tg.enabled || tg.paused}>
                                        {t("web.monitor.checkNow")}
                                    </button>
                                    <button onClick={() => togglePaused(tg)} style={{ marginLeft: 4 }}>
                                        {tg.paused ? t("web.monitor.resume") : t("web.monitor.pause")}
                                    </button>
                                    <button onClick={() => remove(tg)} style={{ marginLeft: 4 }}>
                                        {t("web.monitor.delete")}
                                    </button>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            )}

            {/* 详情（§32：Overview + History + Changes + Diff） */}
            {detail ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{detail.name}</strong>
                        <span style={{ fontSize: 12, wordBreak: "break-all" }}>{detail.url}</span>
                        <span>{statusLabel(detail.status)}</span>
                    </div>
                    <p style={{ fontSize: 12, margin: "4px 0" }}>
                        {t("web.monitor.modeLabel")}: {t(`web.monitor.mode.${detail.mode}`)}
                        {detail.selector ? ` (${detail.selector})` : ""} ·{" "}
                        {t("web.monitor.normalizeNote")}
                    </p>
                    <h4>{t("web.monitor.history")}</h4>
                    <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                        <tbody>
                            {history.slice(0, 20).map((s) => (
                                <tr key={s.seq}>
                                    <td>{new Date(s.fetchedAtMs ?? 0).toLocaleTimeString()}</td>
                                    <td>{s.statusCode ?? 0}</td>
                                    <td>{t(`web.monitor.result.${s.result}`)}</td>
                                    <td>{s.textSize ?? 0} B</td>
                                    <td>{Math.round(s.durationMs ?? 0)} ms</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                    <div style={rowStyle}>
                        <button onClick={() => viewDiff(detail.id)}>{t("web.monitor.viewDiff")}</button>
                    </div>
                    {diff ? (
                        <div style={{ marginTop: 4 }}>
                            <p style={{ fontSize: 12, margin: "4px 0" }}>
                                {t("web.monitor.diffTitle", { a: diff.seqA, b: diff.seqB })} ·{" "}
                                {t("web.monitor.diffStats", {
                                    added: diff.report.stats.added ?? 0,
                                    removed: diff.report.stats.removed ?? 0,
                                })}
                            </p>
                            <textarea readOnly value={diff.report.unified} rows={10} style={{ width: "100%", fontFamily: "monospace", fontSize: 12 }} />
                        </div>
                    ) : null}
                    <h4>{t("web.monitor.changes")}</h4>
                    {changes.length === 0 ? (
                        <p style={{ fontSize: 12 }}>{t("web.monitor.noChanges")}</p>
                    ) : (
                        <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                            <tbody>
                                {changes.map((c) => (
                                    <tr key={c.id}>
                                        <td>{new Date(c.detectedAtMs ?? 0).toLocaleString()}</td>
                                        <td>{t(`web.monitor.result.${c.result}`)}</td>
                                        <td>{c.summary}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    )}
                </div>
            ) : null}
        </section>
    );
}
