import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type { PasteDecision, SensitivityResult, WispEventDto, WispStatusDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";

/** M2.9：Wisp / Paste Guard（§44 轻量工具页——不做安全软件 dashboard）。 */

interface WispPanelProps {
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

function riskLabel(t: (k: string) => string, risk: string | undefined): string {
    return risk ? t(`wisp.risk.${risk}`) : "";
}

export function WispPanel({ onOperationDone }: WispPanelProps) {
    const t = useT();
    const [status, setStatus] = useState<WispStatusDto | null>(null);
    const [decision, setDecision] = useState<PasteDecision | null>(null);
    const [analysis, setAnalysis] = useState<SensitivityResult | null>(null);
    const [testText, setTestText] = useState("");
    const [testResult, setTestResult] = useState<SensitivityResult | null>(null);
    const [events, setEvents] = useState<WispEventDto[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);
    const [nowTs, setNowTs] = useState(0);

    const fail = useCallback((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setBusy(false);
    }, []);

    const load = useCallback(() => {
        commands
            .wispStatus()
            .then(setStatus)
            .catch(() => {});
        commands
            .wispEvents(50)
            .then(setEvents)
            .catch(() => {});
    }, []);

    useEffect(() => {
        load();
        const timer = setInterval(load, 4000);
        return () => clearInterval(timer);
    }, [load]);
    // TTL 倒计时显示（渲染期禁 Date.now()——react-hooks/purity）
    useEffect(() => {
        const timer = setInterval(() => setNowTs(Date.now()), 1000);
        return () => clearInterval(timer);
    }, []);

    const evaluateCurrent = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .wispEvaluateCurrent(null)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setDecision(r.data.decision);
                setAnalysis(r.data.result);
                load();
            })
            .catch(fail);
    }, [fail, load]);

    const maskCurrent = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .wispMaskCurrent(null)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                load();
                onOperationDone();
            })
            .catch(fail);
    }, [fail, load, onOperationDone]);

    const restoreSession = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .wispRestoreSession()
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                }
                load();
                onOperationDone();
            })
            .catch(fail);
    }, [fail, load, onOperationDone]);

    const cancelSession = useCallback(() => {
        commands
            .wispCancelSession()
            .then((r) => {
                if (r.status === "error") setError(r.error.code);
                load();
            })
            .catch(fail);
    }, [fail, load]);

    const togglePaused = useCallback(() => {
        const next = !(status?.paused ?? false);
        commands.wispSetPaused(next).then(() => load()).catch(() => {});
    }, [load, status?.paused]);

    const runTest = useCallback(() => {
        if (!testText.trim()) return;
        commands
            .wispAnalyzeText(testText)
            .then((r) => {
                setTestResult(r);
                load();
            })
            .catch(() => {});
    }, [load, testText]);

    const session = status?.session ?? null;
    const sessionActive = session?.state === "masked";
    const sessionExpired = session?.state === "expired";
    const sessionTtlLeft =
        session && sessionActive && status
            ? Math.max(0, Math.round(((session.expiresAtMs ?? 0) - nowTs) / 1000))
            : null;

    return (
        <section aria-label={t("wisp.title")}>
            <h2>{t("wisp.title")}</h2>
            {/* 状态（§31：Enabled/Paused/Unsupported 如实） */}
            <div style={boxStyle}>
                <div style={rowStyle}>
                    <strong>{status?.enabled ? t("wisp.state.enabled") : t("wisp.state.disabled")}</strong>
                    {status?.paused ? <span>{t("wisp.state.paused")}</span> : null}
                    <button onClick={togglePaused}>
                        {status?.paused ? t("wisp.action.resume") : t("wisp.action.pause")}
                    </button>
                </div>
                <p style={{ fontSize: 12, margin: "4px 0 0" }}>
                    {t("wisp.capability.limited")}
                </p>
            </div>
            {error ? (
                <p role="alert" style={{ color: "var(--color-danger)" }}>
                    {error}
                </p>
            ) : null}

            {/* 当前剪贴板评估（§17 快速确认） */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <div style={rowStyle}>
                    <strong>{t("wisp.evaluate.title")}</strong>
                    <button onClick={evaluateCurrent} disabled={busy}>
                        {t("wisp.evaluate.run")}
                    </button>
                </div>
                {decision ? (
                    <div style={{ marginTop: "var(--spacing-sm)" }}>
                        <div style={rowStyle}>
                            <span>
                                {t("wisp.risk.label")}: <strong>{riskLabel(t, decision.risk)}</strong>
                            </span>
                            <span>
                                {t("wisp.evaluate.target")}: {decision.target.process ?? t("wisp.target.unknown")}
                                {decision.target.identityConfidence === "likely" ? ` (${t("wisp.target.likely")})` : ""}
                            </span>
                            <span>
                                {t("wisp.decision.label")}: {t(`wisp.decision.${decision.action}`)}
                            </span>
                        </div>
                        <ul style={{ margin: "8px 0", paddingLeft: 20, fontSize: 13 }}>
                            {decision.reasons.map((r, i) => (
                                <li key={i}>{t(`wisp.reason.${r}`) !== `wisp.reason.${r}` ? t(`wisp.reason.${r}`) : r}</li>
                            ))}
                        </ul>
                        {analysis?.findings.length ? (
                            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                                <thead>
                                    <tr>
                                        <th style={{ textAlign: "left" }}>{t("wisp.findings.rule")}</th>
                                        <th style={{ textAlign: "left" }}>{t("wisp.findings.category")}</th>
                                        <th style={{ textAlign: "left" }}>{t("wisp.findings.confidence")}</th>
                                        <th style={{ textAlign: "left" }}>{t("wisp.findings.preview")}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {analysis.findings.map((f, i) => (
                                        <tr key={i}>
                                            <td>{f.ruleId}</td>
                                            <td>{t(`wisp.category.${f.category}`)}</td>
                                            <td>{t(`wisp.confidence.${f.confidence}`)}</td>
                                            <td style={{ fontFamily: "monospace" }}>{f.maskedPreview}</td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        ) : null}
                        <div style={rowStyle}>
                            <button onClick={maskCurrent} disabled={busy || analysis?.level === "normal"}>
                                {t("wisp.action.mask")}
                            </button>
                        </div>
                    </div>
                ) : null}
            </div>

            {/* 保护会话（§12/§19） */}
            {session ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{t("wisp.session.title")}</strong>
                        <span>{t(`wisp.session.state.${session.state}`)}</span>
                        {sessionTtlLeft !== null ? (
                            <span>{t("wisp.session.ttl", { s: sessionTtlLeft })}</span>
                        ) : null}
                    </div>
                    {sessionExpired ? <p style={{ fontSize: 12 }}>{t("wisp.session.expiredNote")}</p> : null}
                    {session.lastRestoreSkipReason ? (
                        <p style={{ fontSize: 12 }}>{t("wisp.session.skippedNote")}</p>
                    ) : null}
                    <div style={rowStyle}>
                        <button onClick={restoreSession} disabled={busy || !sessionActive}>
                            {t("wisp.session.restore")}
                        </button>
                        <button onClick={cancelSession} disabled={!sessionActive}>
                            {t("wisp.session.cancel")}
                        </button>
                    </div>
                </div>
            ) : null}

            {/* 检测测试（§38 误报/漏报自查） */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <div style={rowStyle}>
                    <strong>{t("wisp.test.title")}</strong>
                    <button onClick={runTest}>{t("wisp.test.run")}</button>
                </div>
                <textarea
                    value={testText}
                    onChange={(e) => setTestText(e.target.value)}
                    rows={3}
                    style={{ width: "100%", marginTop: 4 }}
                    placeholder={t("wisp.test.placeholder")}
                />
                {testResult ? (
                    <p style={{ fontSize: 13, margin: "4px 0 0" }}>
                        {t("wisp.test.level")}: <strong>{t(`wisp.level.${testResult.level}`)}</strong>
                        {" · "}
                        {t("wisp.test.findings", { count: testResult.findings.length })}
                    </p>
                ) : null}
            </div>

            {/* 最近安全事件（§45：仅元数据） */}
            <h3>{t("wisp.events.title")}</h3>
            <div style={rowStyle}>
                <button onClick={() => { commands.wispClearEvents().then(load).catch(() => {}); }}>
                    {t("wisp.events.clear")}
                </button>
            </div>
            {events.length === 0 ? (
                <p>{t("wisp.events.empty")}</p>
            ) : (
                <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                    <thead>
                        <tr>
                            <th style={{ textAlign: "left" }}>{t("wisp.events.time")}</th>
                            <th style={{ textAlign: "left" }}>{t("wisp.events.kind")}</th>
                            <th style={{ textAlign: "left" }}>{t("wisp.events.level")}</th>
                            <th style={{ textAlign: "left" }}>{t("wisp.events.target")}</th>
                            <th style={{ textAlign: "left" }}>{t("wisp.findings.rule")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {events.map((e, i) => (
                            <tr key={i}>
                                <td>{new Date(e.atMs ?? 0).toLocaleTimeString()}</td>
                                <td>{t(`wisp.eventKind.${e.kind}`) !== `wisp.eventKind.${e.kind}` ? t(`wisp.eventKind.${e.kind}`) : e.kind}</td>
                                <td>{t(`wisp.level.${e.level}`)}</td>
                                <td>{e.target ?? "—"}</td>
                                <td>{e.ruleIds.join(", ") || "—"}</td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            )}
        </section>
    );
}
