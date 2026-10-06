import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { Annotation, CaptureRecord, MonitorInfo, OcrResultDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { useAppStore } from "../../stores/appStore";

/** M2.8：Glint 屏幕捕获中心（§23 首页快进快出；§25 临时/已存边界）。 */

interface GlintPanelProps {
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

type Tool = "rect" | "ellipse" | "arrow" | "freehand" | "text" | "blur" | "pixelate";
const TOOLS: Tool[] = ["rect", "ellipse", "arrow", "freehand", "text", "blur", "pixelate"];
const MIN_REGION = 8; // 图像像素；小于此视为误触

function fmtBytes(n: number): string {
    if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    if (n >= 1024) return `${(n / 1024).toFixed(0)} KB`;
    return `${n} B`;
}

export function GlintPanel({ onOperationDone }: GlintPanelProps) {
    const t = useT();
    const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
    const [monitorIndex, setMonitorIndex] = useState(0);
    const [recent, setRecent] = useState<CaptureRecord[]>([]);
    const [current, setCurrent] = useState<CaptureRecord | null>(null);
    const [currentSource, setCurrentSource] = useState("");
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);
    const [thumbs, setThumbs] = useState<Record<string, string>>({});
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);
    const [autoCopy, setAutoCopy] = useState(false);
    // 模式：查看 / 区域裁剪 / 标注
    const [mode, setMode] = useState<"view" | "region" | "annotate">("view");
    const [tool, setTool] = useState<Tool>("rect");
    const [annotations, setAnnotations] = useState<Annotation[]>([]);
    const [ocr, setOcr] = useState<OcrResultDto | null>(null);
    // 拖拽（图像像素坐标）
    const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
    const [freehand, setFreehand] = useState<[number, number][]>([]);
    const imgRef = useRef<HTMLImageElement | null>(null);

    const fail = useCallback((e: unknown) => {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        setBusy(false);
    }, []);

    const loadRecent = useCallback(() => {
        commands
            .glintListRecent()
            .then((r) => {
                if (r.status === "ok") setRecent(r.data);
            })
            .catch(() => {});
    }, []);

    useEffect(() => {
        commands
            .glintListMonitors()
            .then((r) => {
                if (r.status === "ok") {
                    setMonitors(r.data);
                    const primary = r.data.findIndex((m) => m.isPrimary);
                    if (primary >= 0) setMonitorIndex(primary);
                }
            })
            .catch(() => {});
        loadRecent();
    }, [loadRecent]);

    // 最近列表缩略图（前 24 张；懒加载 + 上限 §36）
    useEffect(() => {
        const targets = recent.slice(0, 24).filter((c) => !thumbs[c.fileName]);
        let cancelled = false;
        for (const c of targets) {
            commands
                .glintThumbnail(c.fileName)
                .then((r) => {
                    if (r.status !== "ok" || cancelled) return;
                    const bin = String.fromCharCode(...r.data);
                    setThumbs((prev) => ({ ...prev, [c.fileName]: `data:image/png;base64,${btoa(bin)}` }));
                })
                .catch(() => {});
        }
        return () => {
            cancelled = true;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [recent]);

    const openCapture = useCallback((record: CaptureRecord, source: string) => {
        setCurrent(record);
        setCurrentSource(source);
        setAnnotations([]);
        setOcr(null);
        setMode("view");
        setPreviewUrl(null);
        commands
            .glintPreview(record.fileName)
            .then((r) => {
                if (r.status !== "ok") return;
                const bin = String.fromCharCode(...r.data);
                setPreviewUrl(`data:image/png;base64,${btoa(bin)}`);
            })
            .catch(() => {});
    }, []);

    const captureScreen = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .glintCaptureScreen(monitorIndex)
            .then((r) => {
                if (r.status !== "ok") {
                    setError(r.error.code);
                    setBusy(false);
                    return;
                }
                setBusy(false);
                openCapture(r.data.record, r.data.source);
                loadRecent();
                if (autoCopy) {
                    void commands.glintCopyCaptureToClipboard(r.data.record.fileName).then(() => onOperationDone());
                }
            })
            .catch(fail);
    }, [autoCopy, fail, loadRecent, monitorIndex, onOperationDone, openCapture]);

    const captureWindow = useCallback(() => {
        setBusy(true);
        setError(null);
        commands
            .glintCaptureForegroundWindow()
            .then((r) => {
                if (r.status !== "ok") {
                    setError(r.error.code);
                    setBusy(false);
                    return;
                }
                setBusy(false);
                openCapture(r.data.record, r.data.source);
                loadRecent();
            })
            .catch(fail);
    }, [fail, loadRecent, openCapture]);

    // palette 捕获请求消费（tick 防重复触发；ref 避免闭包过期）
    const captureRef = useRef({ screen: captureScreen, window: captureWindow });
    captureRef.current = { screen: captureScreen, window: captureWindow };
    const glintCaptureRequest = useAppStore((s) => s.glintCaptureRequest);
    const lastTickRef = useRef(0);
    useEffect(() => {
        if (!glintCaptureRequest || glintCaptureRequest.tick === lastTickRef.current) return;
        lastTickRef.current = glintCaptureRequest.tick;
        if (glintCaptureRequest.kind === "screen") captureRef.current.screen();
        else captureRef.current.window();
    }, [glintCaptureRequest]);

    // 预览图显示坐标 → 图像物理像素坐标
    const toImageCoords = (e: { clientX: number; clientY: number }): [number, number] | null => {        const img = imgRef.current;
        if (!img) return null;
        const rect = img.getBoundingClientRect();
        const scale = img.naturalWidth / Math.max(rect.width, 1);
        const x = Math.round((e.clientX - rect.left) * scale);
        const y = Math.round((e.clientY - rect.top) * scale);
        return [
            Math.max(0, Math.min(img.naturalWidth - 1, x)),
            Math.max(0, Math.min(img.naturalHeight - 1, y)),
        ];
    };

    const onPointerDown = (e: React.PointerEvent) => {
        if (mode === "view" || busy) return;
        const p = toImageCoords(e);
        if (!p) return;
        setError(null);
        if (mode === "annotate" && tool === "text") {
            const text = window.prompt(t("glint.annotate.textPrompt"));
            if (text) {
                setAnnotations((prev) => [
                    ...prev,
                    {
                        id: `a${Date.now()}`,
                        shape: { kind: "text", x: p[0], y: p[1], text },
                        style: { strokeWidth: 2, color: [255, 64, 64, 255] as [number, number, number, number], fontSize: 24, filled: false },
                    },
                ]);
            }
            return;
        }
        if (mode === "annotate" && tool === "freehand") {
            setFreehand([p]);
        }
        setDrag({ x0: p[0], y0: p[1], x1: p[0], y1: p[1] });
    };

    const onPointerMove = (e: React.PointerEvent) => {
        if (!drag) return;
        const p = toImageCoords(e);
        if (!p) return;
        if (freehand.length > 0 && mode === "annotate" && tool === "freehand") {
            setFreehand((prev) => [...prev, p]);
        }
        setDrag({ ...drag, x1: p[0], y1: p[1] });
    };

    const finishDrag = () => {
        if (!drag) return;
        const d = drag;
        setDrag(null);
        if (mode === "region") {
            // 有效选区保留（显示尺寸提示 + Crop 按钮）；Enter 或按钮确认
            if (Math.abs(d.x1 - d.x0) >= MIN_REGION && Math.abs(d.y1 - d.y0) >= MIN_REGION) {
                setDrag({ ...d });
            }
            return;
        }
        if (mode === "annotate") {
            if (tool === "freehand") {
                if (freehand.length >= 2) {
                    setAnnotations((prev) => [
                        ...prev,
                        {
                            id: `a${Date.now()}`,
                            shape: { kind: "freehand", points: freehand },
                            style: { strokeWidth: 3, color: [255, 64, 64, 255] as [number, number, number, number], fontSize: 24, filled: false },
                        },
                    ]);
                }
                setFreehand([]);
                return;
            }
            const x = Math.min(d.x0, d.x1);
            const y = Math.min(d.y0, d.y1);
            const w = Math.abs(d.x1 - d.x0);
            const h = Math.abs(d.y1 - d.y0);
            if (w < MIN_REGION || h < MIN_REGION) return;
            const style = { strokeWidth: 2, color: [255, 64, 64, 255] as [number, number, number, number], fontSize: 24, filled: false };
            const shape =
                tool === "rect" ? { kind: "rectangle" as const, x, y, w, h }
                : tool === "ellipse" ? { kind: "ellipse" as const, x, y, w, h }
                : tool === "arrow" ? { kind: "arrow" as const, x1: d.x0, y1: d.y0, x2: d.x1, y2: d.y1 }
                : tool === "blur" ? { kind: "blur" as const, x, y, w, h }
                : { kind: "pixelate" as const, x, y, w, h, block: 12 };
            setAnnotations((prev) => [...prev, { id: `a${Date.now()}`, shape, style }]);
        }
    };

    const cropRegion = useCallback(() => {
        if (!current || !drag) return;
        const x = Math.min(drag.x0, drag.x1);
        const y = Math.min(drag.y0, drag.y1);
        const w = Math.abs(drag.x1 - drag.x0);
        const h = Math.abs(drag.y1 - drag.y0);
        if (w < MIN_REGION || h < MIN_REGION) return;
        setBusy(true);
        setDrag(null);
        commands
            .glintCropCapture(current.fileName, x, y, w, h)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                openCapture(r.data.record, r.data.source);
                loadRecent();
            })
            .catch(fail);
    }, [current, drag, fail, loadRecent, openCapture]);

    // 区域模式：Enter 确认 / Esc 取消（§6）
    useEffect(() => {
        if (mode !== "region") return;
        const onKey = (e: KeyboardEvent) => {
            if (e.key === "Enter") cropRegion();
            if (e.key === "Escape") setDrag(null);
        };
        window.addEventListener("keydown", onKey);
        return () => window.removeEventListener("keydown", onKey);
    }, [cropRegion, mode]);

    const saveAnnotated = useCallback(() => {
        if (!current || annotations.length === 0) return;
        setBusy(true);
        commands
            .glintSaveAnnotated(current.fileName, annotations, "png", 90)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setAnnotations([]);
                openCapture(r.data.record, r.data.source);
                loadRecent();
            })
            .catch(fail);
    }, [annotations, current, fail, loadRecent, openCapture]);

    const copyImage = useCallback(() => {
        if (!current) return;
        commands
            .glintCopyCaptureToClipboard(current.fileName)
            .then(() => {
                onOperationDone();
            })
            .catch(fail);
    }, [current, fail, onOperationDone]);

    const runOcr = useCallback(() => {
        if (!current) return;
        setBusy(true);
        commands
            .glintOcrCapture(current.fileName, null)
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(`${r.error.code}: ${r.error.message}`);
                    return;
                }
                setOcr(r.data);
            })
            .catch(fail);
    }, [current, fail]);

    const copyOcrText = useCallback(() => {
        if (!ocr) return;
        void navigator.clipboard.writeText(ocr.text).then(() => onOperationDone());
    }, [ocr, onOperationDone]);

    const removeCapture = useCallback(
        (fileName: string) => {
            commands
                .glintDeleteCapture(fileName)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    if (current?.fileName === fileName) {
                        setCurrent(null);
                        setPreviewUrl(null);
                    }
                    loadRecent();
                })
                .catch(fail);
        },
        [current, fail, loadRecent],
    );

    const regionRect = drag
        ? {
              left: Math.min(drag.x0, drag.x1),
              top: Math.min(drag.y0, drag.y1),
              width: Math.abs(drag.x1 - drag.x0),
              height: Math.abs(drag.y1 - drag.y0),
          }
        : null;
    const imgScale = imgRef.current && previewUrl ? imgRef.current.clientWidth / imgRef.current.naturalWidth : 0;

    return (
        <section aria-label={t("glint.title")}>
            <h2>{t("glint.title")}</h2>
            <div style={rowStyle}>
                <select
                    value={monitorIndex}
                    onChange={(e) => setMonitorIndex(Number(e.target.value))}
                    aria-label={t("glint.monitor")}
                >
                    {monitors.map((m) => (
                        <option key={m.id} value={m.index}>
                            {t("glint.monitorItem", { index: m.index + 1, w: m.width, h: m.height })}
                            {m.isPrimary ? " ★" : ""}
                        </option>
                    ))}
                </select>
                <button onClick={captureScreen} disabled={busy || monitors.length === 0}>
                    {t("glint.capture.full")}
                </button>
                <button onClick={captureWindow} disabled={busy}>
                    {t("glint.capture.window")}
                </button>
                <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                    <input type="checkbox" checked={autoCopy} onChange={(e) => setAutoCopy(e.target.checked)} />
                    {t("glint.autoCopy")}
                </label>
            </div>
            {error ? (
                <p role="alert" style={{ color: "var(--color-danger)" }}>
                    {error}
                </p>
            ) : null}

            {current && previewUrl ? (
                <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <strong>{current.fileName}</strong>
                        <span>
                            {current.width}×{current.height} · {fmtBytes(current.size ?? 0)} ·{" "}
                            {current.modifiedMs ? new Date(current.modifiedMs).toLocaleString() : ""} ·{" "}
                            {currentSource || t("glint.source.unknown")}
                        </span>
                    </div>
                    <div style={rowStyle}>
                        {mode === "view" ? (
                            <button onClick={() => setMode("region")}>{t("glint.region.start")}</button>
                        ) : (
                            <button onClick={() => { setMode("view"); setDrag(null); }}>
                                {t("glint.region.exit")}
                            </button>
                        )}
                        <button onClick={() => setMode(mode === "annotate" ? "view" : "annotate")}>
                            {mode === "annotate" ? t("glint.annotate.exit") : t("glint.annotate.start")}
                        </button>
                        <button onClick={copyImage}>{t("glint.action.copy")}</button>
                        <button onClick={runOcr} disabled={busy}>{t("glint.action.ocr")}</button>
                        <button onClick={() => removeCapture(current.fileName)}>
                            {t("glint.action.delete")}
                        </button>
                    </div>
                    {mode === "annotate" ? (
                        <div style={rowStyle}>
                            {TOOLS.map((toolId) => (
                                <button
                                    key={toolId}
                                    onClick={() => setTool(toolId)}
                                    style={
                                        tool === toolId
                                            ? { fontWeight: 700, textDecoration: "underline" }
                                            : undefined
                                    }
                                >
                                    {t(`glint.tool.${toolId}`)}
                                </button>
                            ))}
                            <button onClick={() => setAnnotations((prev) => prev.slice(0, -1))} disabled={annotations.length === 0}>
                                {t("glint.annotate.undo")}
                            </button>
                            <button onClick={saveAnnotated} disabled={busy || annotations.length === 0}>
                                {t("glint.annotate.save", { count: annotations.length })}
                            </button>
                        </div>
                    ) : null}
                    {mode === "region" ? (
                        <p>
                            {t("glint.region.hint")}
                            {regionRect && imgScale > 0 ? (
                                <>
                                    {" · "}
                                    {t("glint.region.size", {
                                        w: regionRect.width,
                                        h: regionRect.height,
                                    })}
                                    <button onClick={cropRegion} style={{ marginLeft: 8 }}>
                                        {t("glint.region.confirm")}
                                    </button>
                                </>
                            ) : null}
                        </p>
                    ) : null}
                    <div
                        style={{ position: "relative", display: "inline-block", maxWidth: "100%", touchAction: "none" }}
                        onPointerDown={onPointerDown}
                        onPointerMove={onPointerMove}
                        onPointerUp={finishDrag}
                        onPointerLeave={finishDrag}
                    >
                        <img
                            ref={imgRef}
                            src={previewUrl}
                            alt={current.fileName}
                            style={{ maxWidth: "100%", maxHeight: 480, display: "block", cursor: mode === "view" ? "default" : "crosshair" }}
                            draggable={false}
                        />
                        {/* 选区/标注拖拽框（显示坐标 = 图像坐标 × 缩放） */}
                        {regionRect && imgScale > 0 ? (
                            <div
                                style={{
                                    position: "absolute",
                                    border: "2px solid var(--color-danger)",
                                    pointerEvents: "none",
                                    left: regionRect.left * imgScale,
                                    top: regionRect.top * imgScale,
                                    width: regionRect.width * imgScale,
                                    height: regionRect.height * imgScale,
                                }}
                            />
                        ) : null}
                        {/* 已添加标注的简单可视化（拖拽框/自由手线条） */}
                        {annotations.map((a) => {
                            if (a.shape.kind === "rectangle" || a.shape.kind === "ellipse" || a.shape.kind === "blur" || a.shape.kind === "pixelate") {
                                const s = a.shape;
                                return (
                                    <div
                                        key={a.id}
                                        style={{
                                            position: "absolute",
                                            border: "2px solid var(--color-danger)",
                                            pointerEvents: "none",
                                            left: (s.x ?? 0) * imgScale,
                                            top: (s.y ?? 0) * imgScale,
                                            width: (s.w ?? 0) * imgScale,
                                            height: (s.h ?? 0) * imgScale,
                                            borderRadius: a.shape.kind === "ellipse" ? "50%" : 0,
                                        }}
                                    />
                                );
                            }
                            if (a.shape.kind === "freehand" && imgScale > 0) {
                                const pts = a.shape.points.map(([px, py]) => `${(px ?? 0) * imgScale},${(py ?? 0) * imgScale}`).join(" ");
                                return (
                                    <svg key={a.id} style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "visible" }}>
                                        <polyline points={pts} fill="none" stroke="var(--color-danger)" strokeWidth={2} />
                                    </svg>
                                );
                            }
                            return null;
                        })}
                        {/* 进行中的自由手 */}
                        {freehand.length >= 2 && imgScale > 0 ? (
                            <svg style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "visible" }}>
                                <polyline
                                    points={freehand.map(([px, py]) => `${px * imgScale},${py * imgScale}`).join(" ")}
                                    fill="none"
                                    stroke="var(--color-danger)"
                                    strokeWidth={2}
                                />
                            </svg>
                        ) : null}
                    </div>
                    {ocr ? (
                        <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                            <div style={rowStyle}>
                                <strong>{t("glint.ocr.title")}</strong>
                                {ocr.engineLanguage ? <span>{ocr.engineLanguage}</span> : null}
                                <button onClick={copyOcrText}>{t("glint.ocr.copy")}</button>
                            </div>
                            <textarea readOnly value={ocr.text} rows={4} style={{ width: "100%" }} />
                        </div>
                    ) : null}
                </div>
            ) : null}

            <h3>{t("glint.recent")}</h3>
            {recent.length === 0 ? (
                <p>{t("glint.recent.empty")}</p>
            ) : (
                <div style={{ ...rowStyle, alignItems: "flex-start" }}>
                    {recent.map((c) => (
                        <div key={c.fileName} style={{ ...boxStyle, width: 168 }}>
                            {thumbs[c.fileName] ? (
                                <img
                                    src={thumbs[c.fileName]}
                                    alt={c.fileName}
                                    style={{ width: "100%", cursor: "pointer" }}
                                    onClick={() => openCapture(c, "")}
                                />
                            ) : (
                                <div style={{ width: "100%", height: 90, background: "var(--color-border)" }} />
                            )}
                            <div style={{ fontSize: 12, marginTop: 4 }}>
                                {c.width}×{c.height} · {fmtBytes(c.size ?? 0)}
                            </div>
                            <div style={{ fontSize: 12 }}>
                                {c.modifiedMs ? new Date(c.modifiedMs).toLocaleString() : ""}
                            </div>
                            <div style={rowStyle}>
                                <button onClick={() => openCapture(c, "")}>{t("glint.action.open")}</button>
                                <button onClick={() => removeCapture(c.fileName)}>{t("glint.action.delete")}</button>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </section>
    );
}
