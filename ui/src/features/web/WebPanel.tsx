import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { UrlCleanResult, WebBatchReport, WebConfig, WebDocument, WebLink } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.10：Web 内容处理中心（§59/§60 统一 Workspace——不是浏览器）。 */

interface WebPanelProps {
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

type Tab = "text" | "markdown" | "links" | "metadata";

export function WebPanel({ onOperationDone }: WebPanelProps) {
    const t = useT();
    const [input, setInput] = useState("");
    const [clean, setClean] = useState<UrlCleanResult | null>(null);
    const [doc, setDoc] = useState<WebDocument | null>(null);
    const [docJobId, setDocJobId] = useState<string | null>(null);
    const [tab, setTab] = useState<Tab>("text");
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);
    const [allowPrivate, setAllowPrivate] = useState(false);
    const [destDir, setDestDir] = useState("");
    const [exportMsg, setExportMsg] = useState<string | null>(null);
    // 批量（§41-§45）
    const [batchText, setBatchText] = useState("");
    const [batchFormat, setBatchFormat] = useState("md");
    const [batchReport, setBatchReport] = useState<WebBatchReport | null>(null);
    const pollStop = useRef<(() => void) | null>(null);
    const [cfg, setCfg] = useState<WebConfig | null>(null);

    const fail = useCallback((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setBusy(false);
    }, []);

    useEffect(() => {
        commands.webGetConfig().then(setCfg).catch(() => {});
        return () => pollStop.current?.();
    }, []);

    const runClean = useCallback(() => {
        setError(null);
        commands
            .webCleanUrl(input, true)
            .then((r) => {
                if (r.status === "ok") {
                    setClean(r.data);
                } else {
                    setError(`${r.error.code}: ${r.error.message}`);
                }
            })
            .catch(fail);
    }, [fail, input]);

    const runFetch = useCallback(() => {
        setError(null);
        setBusy(true);
        setExportMsg(null);
        commands
            .webFetchUrl(input, allowPrivate)
            .then((r) => {
                if (r.status !== "ok") {
                    setError(`${r.error.code}: ${r.error.message}`);
                    setBusy(false);
                    return;
                }
                pollStop.current?.();
                pollStop.current = pollJob(r.data.jobId, (tick) => {
                    if (tick.kind === "status") {
                        if (isTerminalState(tick.status)) {
                            pollStop.current?.();
                            setBusy(false);
                            commands
                                .webGetResult(r.data.jobId)
                                .then((gr) => {
                                    if (gr.status === "ok") {
                                        setDoc(gr.data);
                                        setDocJobId(r.data.jobId);
                                    } else {
                                        setError(`${gr.error.code}: ${gr.error.message}`);
                                    }
                                })
                                .catch(fail);
                        }
                    } else {
                        pollStop.current?.();
                        setBusy(false);
                        setError(`${tick.error.code}: ${tick.error.message}`);
                    }
                });
            })
            .catch(fail);
    }, [allowPrivate, fail, input]);

    // 批量：从文本提取 URL（§10）→ 有界批量任务
    const runBatch = useCallback(() => {
        setError(null);
        setBusy(true);
        setBatchReport(null);
        commands
            .webExtractUrlsText(batchText)
            .then((links: WebLink[]) => {
                if (links.length === 0) {
                    setError(t("web.batch.noUrls"));
                    setBusy(false);
                    return;
                }
                if (!destDir.trim()) {
                    setError(t("web.batch.needDest"));
                    setBusy(false);
                    return;
                }
                return commands
                    .webBatchFetch(
                        links.map((l) => l.absoluteUrl),
                        destDir,
                        batchFormat,
                        allowPrivate,
                    )
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
                                setBusy(false);
                                commands
                                    .webBatchResult(r.data.jobId)
                                    .then((br) => {
                                        if (br.status === "ok") setBatchReport(br.data);
                                    })
                                    .catch(() => {});
                                onOperationDone();
                            } else if (tick.kind === "failed") {
                                pollStop.current?.();
                                setBusy(false);
                                setError(`${tick.error.code}: ${tick.error.message}`);
                            }
                        });
                    });
            })
            .catch(fail);
    }, [allowPrivate, batchFormat, batchText, destDir, fail, onOperationDone, t]);

    const copyText = useCallback(
        (text: string | undefined | null, label: string) => {
            if (!text) return;
            commands
                .webCopyText(text)
                .then(() => setExportMsg(label))
                .catch(fail);
        },
        [fail],
    );

    const runExport = useCallback(
        (format: "txt" | "md" | "html") => {
            if (!docJobId && !doc) return;
            setError(null);
            setExportMsg(null);
            if (!destDir.trim()) {
                setError(t("web.export.needDest"));
                return;
            }
            commands
                .webExportDocument({
                    jobId: docJobId,
                    content: null,
                    format,
                    destDir,
                    fileName: null,
                })
                .then((r) => {
                    if (r.status === "ok") {
                        setExportMsg(`${t("web.export.done")}: ${r.data.output}`);
                        onOperationDone();
                    } else {
                        setError(`${r.error.code}: ${r.error.message}`);
                    }
                })
                .catch(fail);
        },
        [destDir, doc, docJobId, fail, onOperationDone, t],
    );

    const saveCfg = useCallback((next: WebConfig) => {
        setCfg(next);
        commands.webApplyConfig(next).catch(() => {});
        void import("../../lib/bootstrap").then((m) => m.persistAppConfig((c) => ({ ...c, web: next })));
    }, []);

    const info = doc?.info;

    return (
        <section aria-label={t("web.title")}>
            <h2>{t("web.title")}</h2>

            {/* 输入区（§60 顶部） */}
            <div style={boxStyle}>
                <div style={rowStyle}>
                    <input
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        placeholder={t("web.input.placeholder")}
                        style={{ flex: 1, minWidth: 280 }}
                        aria-label={t("web.input.label")}
                    />
                    <button onClick={runClean} disabled={!input.trim()}>
                        {t("web.action.clean")}
                    </button>
                    <button onClick={runFetch} disabled={busy || !input.trim()}>
                        {busy ? t("web.action.fetching") : t("web.action.fetch")}
                    </button>
                    <label style={{ display: "flex", gap: 4, alignItems: "center", fontSize: 12 }}>
                        <input type="checkbox" checked={allowPrivate} onChange={(e) => setAllowPrivate(e.target.checked)} />
                        {t("web.action.allowPrivate")}
                    </label>
                </div>
                {error ? (
                    <p role="alert" style={{ color: "var(--color-danger)" }}>
                        {error}
                    </p>
                ) : null}
                {exportMsg ? <p style={{ fontSize: 12, margin: "4px 0 0" }}>{exportMsg}</p> : null}
            </div>

            {/* URL 清理（§9：删除了什么/为什么） */}
            {clean ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{t("web.clean.title")}</strong>
                        <button onClick={() => copyText(clean.cleanedUrl, t("web.copy.cleaned"))}>
                            {t("web.action.copyClean")}
                        </button>
                    </div>
                    <p style={{ fontSize: 12, margin: "4px 0", wordBreak: "break-all" }}>
                        {t("web.clean.original")}: {clean.originalUrl}
                    </p>
                    <p style={{ fontSize: 12, margin: "4px 0", wordBreak: "break-all" }}>
                        {t("web.clean.cleaned")}: <strong>{clean.cleanedUrl}</strong>
                    </p>
                    {clean.removedParameters.length > 0 ? (
                        <p style={{ fontSize: 12, margin: "4px 0" }}>
                            {t("web.clean.removed")}: {clean.removedParameters.map((p) => p.name).join(", ")}
                        </p>
                    ) : null}
                    {clean.unknownParameters.length > 0 ? (
                        <p style={{ fontSize: 12, margin: "4px 0" }}>
                            {t("web.clean.unknownKept")}: {clean.unknownParameters.map((p) => p.name).join(", ")}
                        </p>
                    ) : null}
                    {clean.warnings.map((w, i) => (
                        <p key={i} style={{ fontSize: 12, margin: "4px 0", color: "var(--color-danger)" }}>
                            {w}
                        </p>
                    ))}
                </div>
            ) : null}

            {/* 摘要（§39 Preview 元数据 + §27 推断声明） */}
            {doc && info ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{doc.metadata.title?.value ?? t("web.summary.untitled")}</strong>
                        <span style={{ fontSize: 12 }}>
                            {info.status} · {info.contentTypeRaw || "—"} · {info.charset || "—"} ·{" "}
                            {Math.round(info.sizeBytes ?? 0)} B · {Math.round(info.durationMs ?? 0)} ms ·{" "}
                            {t("web.summary.redirects", { count: info.redirectCount ?? 0 })}
                        </span>
                    </div>
                    <p style={{ fontSize: 12, margin: "4px 0", wordBreak: "break-all" }}>
                        {t("web.summary.finalUrl")}: {info.finalUrl}
                    </p>
                    <p style={{ fontSize: 12, margin: "4px 0" }}>
                        {t("web.summary.confidence")}: {t(`web.confidence.${doc.contentConfidence}`)}
                        {" · "}
                        {t("web.summary.heuristicNote")}
                    </p>
                    <div style={rowStyle}>
                        {(["text", "markdown", "links", "metadata"] as Tab[]).map((tabId) => (
                            <button
                                key={tabId}
                                onClick={() => setTab(tabId)}
                                style={tab === tabId ? { fontWeight: 700, textDecoration: "underline" } : undefined}
                            >
                                {t(`web.tab.${tabId}`)}
                            </button>
                        ))}
                    </div>
                    {tab === "text" ? (
                        <textarea readOnly value={doc.plainText} rows={12} style={{ width: "100%", marginTop: 4 }} />
                    ) : null}
                    {tab === "markdown" ? (
                        <textarea readOnly value={doc.markdown} rows={12} style={{ width: "100%", marginTop: 4 }} />
                    ) : null}
                    {tab === "links" ? (
                        <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                            <tbody>
                                {doc.links.slice(0, 100).map((l, i) => (
                                    <tr key={i}>
                                        <td style={{ fontFamily: "monospace", wordBreak: "break-all" }}>{l.absoluteUrl}</td>
                                        <td>{t(`web.linkType.${l.linkType}`)}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    ) : null}
                    {tab === "metadata" ? (
                        <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                            <tbody>
                                {Object.entries({
                                    [t("web.meta.title")]: doc.metadata.title,
                                    [t("web.meta.description")]: doc.metadata.description,
                                    [t("web.meta.author")]: doc.metadata.author,
                                    [t("web.meta.published")]: doc.metadata.publishedTime,
                                    [t("web.meta.modified")]: doc.metadata.modifiedTime,
                                    [t("web.meta.language")]: doc.metadata.language,
                                    [t("web.meta.canonical")]: doc.metadata.canonicalUrl,
                                    [t("web.meta.ogTitle")]: doc.metadata.ogTitle,
                                    [t("web.meta.ogDescription")]: doc.metadata.ogDescription,
                                    [t("web.meta.ogImage")]: doc.metadata.ogImage,
                                })
                                    .filter(([, v]) => v != null)
                                    .map(([k, v]) => (
                                        <tr key={k}>
                                            <td>{k}</td>
                                            <td>{(v as { value: string }).value}</td>
                                            <td style={{ color: "var(--color-border)" }}>({(v as { source: string }).source})</td>
                                        </tr>
                                    ))}
                            </tbody>
                        </table>
                    ) : null}
                    <div style={rowStyle}>
                        <button onClick={() => copyText(doc.plainText, t("web.copy.text"))}>{t("web.action.copyText")}</button>
                        <button onClick={() => copyText(doc.markdown, t("web.copy.markdown"))}>{t("web.action.copyMarkdown")}</button>
                        <button onClick={() => runExport("txt")}>{t("web.action.exportTxt")}</button>
                        <button onClick={() => runExport("md")}>{t("web.action.exportMd")}</button>
                        <button onClick={() => runExport("html")}>{t("web.action.exportHtml")}</button>
                    </div>
                    <p style={{ fontSize: 11, margin: "4px 0 0" }}>{t("web.export.htmlNote")}</p>
                </div>
            ) : null}

            {/* 导出/批量目标目录 */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <div style={rowStyle}>
                    <label style={{ fontSize: 12 }}>{t("web.export.destDir")}</label>
                    <input
                        value={destDir}
                        onChange={(e) => setDestDir(e.target.value)}
                        placeholder="C:\\export\\web"
                        style={{ flex: 1, minWidth: 240 }}
                    />
                </div>
                <div style={rowStyle}>
                    <strong style={{ fontSize: 13 }}>{t("web.batch.title")}</strong>
                    <select value={batchFormat} onChange={(e) => setBatchFormat(e.target.value)}>
                        <option value="txt">txt</option>
                        <option value="md">md</option>
                        <option value="html">html</option>
                    </select>
                    <button onClick={runBatch} disabled={busy || !batchText.trim()}>
                        {t("web.batch.run")}
                    </button>
                </div>
                <textarea
                    value={batchText}
                    onChange={(e) => setBatchText(e.target.value)}
                    rows={4}
                    style={{ width: "100%" }}
                    placeholder={t("web.batch.placeholder")}
                />
                {batchReport ? (
                    <div style={{ marginTop: 4 }}>
                        <p style={{ fontSize: 12, margin: "4px 0" }}>
                            {t("web.batch.summary", {
                                total: batchReport.total ?? 0,
                                ok: batchReport.succeeded ?? 0,
                                failed: batchReport.failed ?? 0,
                                cancelled: batchReport.cancelled ?? 0,
                                skipped: batchReport.skipped ?? 0,
                            })}
                        </p>
                        <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                            <tbody>
                                {batchReport.items.slice(0, 50).map((item, i) => (
                                    <tr key={i}>
                                        <td style={{ wordBreak: "break-all" }}>{item.maskedUrl}</td>
                                        <td>{t(`web.batch.status.${item.status}`)}</td>
                                        <td>{item.errorCode ?? item.fileName ?? ""}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                ) : null}
            </div>

            {/* 设置（§84） */}
            {cfg ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <strong style={{ fontSize: 13 }}>{t("web.settings.title")}</strong>
                    <div style={rowStyle}>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input type="checkbox" checked={cfg.allowNetwork} onChange={(e) => saveCfg({ ...cfg, allowNetwork: e.target.checked })} />
                            {t("web.settings.allowNetwork")}
                        </label>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input type="checkbox" checked={cfg.allowPrivateTargets} onChange={(e) => saveCfg({ ...cfg, allowPrivateTargets: e.target.checked })} />
                            {t("web.settings.allowPrivate")}
                        </label>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input type="checkbox" checked={cfg.sensitiveQueryMasking} onChange={(e) => saveCfg({ ...cfg, sensitiveQueryMasking: e.target.checked })} />
                            {t("web.settings.masking")}
                        </label>
                    </div>
                    <div style={rowStyle}>
                        <label>
                            {t("web.settings.timeout")}
                            <input type="number" min={1} max={120} value={cfg.timeoutSecs ?? 30} onChange={(e) => saveCfg({ ...cfg, timeoutSecs: Math.max(1, Number(e.target.value) || 30) })} style={{ width: 70 }} />
                        </label>
                        <label>
                            {t("web.settings.maxMb")}
                            <input type="number" min={1} max={50} value={cfg.maxResponseMb ?? 5} onChange={(e) => saveCfg({ ...cfg, maxResponseMb: Math.max(1, Number(e.target.value) || 5) })} style={{ width: 70 }} />
                        </label>
                        <label>
                            {t("web.settings.redirects")}
                            <input type="number" min={0} max={20} value={cfg.maxRedirects ?? 10} onChange={(e) => saveCfg({ ...cfg, maxRedirects: Math.max(0, Number(e.target.value) || 10) })} style={{ width: 70 }} />
                        </label>
                        <label>
                            {t("web.settings.concurrent")}
                            <input type="number" min={1} max={8} value={cfg.maxConcurrent ?? 4} onChange={(e) => saveCfg({ ...cfg, maxConcurrent: Math.max(1, Number(e.target.value) || 4) })} style={{ width: 70 }} />
                        </label>
                    </div>
                </div>
            ) : null}
        </section>
    );
}
