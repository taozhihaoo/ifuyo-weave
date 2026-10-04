import { useState, type CSSProperties } from "react";
import { ImageBatchPanel } from "./ImageBatchPanel";
import { useT } from "../../i18n";
import {
  imageExecute,
  imageOpen,
  imagePreview,
  type ImageOperationSpec,
  type ImageOpenView,
  type ImagePreviewView,
} from "../../lib/image";

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
const imgStyle: CSSProperties = {
  maxWidth: "100%",
  maxHeight: 320,
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
};

type Operation = "resize" | "compress" | "convert" | "strip_metadata";

/** M6 Image：Inspector + Resize/Compress/Convert/Strip 统一面板。
 *
 * Preview 与 Execute 同引擎（§64）；预览有界（§65）；默认写新文件（§73）。 */
export function ImagePanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [path, setPath] = useState("");
  const [open, setOpen] = useState<ImageOpenView | null>(null);
  const [preview, setPreview] = useState<ImagePreviewView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [operation, setOperation] = useState<Operation>("resize");
  const [targetFormat, setTargetFormat] = useState("png");
  const [quality, setQuality] = useState(85);
  const [resizeMode, setResizeMode] = useState<"fit" | "fill" | "exact" | "scale">("fit");
  const [resizeW, setResizeW] = useState(1920);
  const [resizeH, setResizeH] = useState(1080);
  const [scalePercent, setScalePercent] = useState(50);
  const [preventUpscale, setPreventUpscale] = useState(true);
  const [filter, setFilter] = useState<"nearest" | "triangle" | "catmullrom" | "lanczos3">(
    "lanczos3",
  );
  const [stripMetadata, setStripMetadata] = useState(false);
  const [overwriteSource, setOverwriteSource] = useState(false);
  const [destPath, setDestPath] = useState("");

  const fail = (e: unknown): void => {
    setError(e instanceof Error ? e.message : String(e));
    setBusy(false);
  };

  const doOpen = (): void => {
    setBusy(true);
    setError(null);
    setStatus(null);
    setPreview(null);
    imageOpen(path)
      .then((o) => {
        setBusy(false);
        setOpen(o);
        setDestPath(
          path.replace(/\.[^.]+$/, `.m6.${o.facts.format === "jpeg" ? "jpg" : o.facts.format}`),
        );
      })
      .catch(fail);
  };

  const buildOperation = (): ImageOperationSpec => ({
    operation,
    targetFormat: operation === "resize" ? null : targetFormat,
    quality: operation === "compress" || targetFormat === "jpeg" ? quality : null,
    alphaBackground: "white",
    resize:
      operation === "resize"
        ? {
            mode: resizeMode,
            width: resizeW,
            height: resizeH,
            scalePercent,
            preventUpscale,
            filter,
          }
        : null,
    stripMetadata: stripMetadata || operation === "strip_metadata",
  });

  const doPreview = (): void => {
    setBusy(true);
    setError(null);
    setPreview(null);
    imagePreview(path, buildOperation())
      .then((p) => {
        setBusy(false);
        setPreview(p);
      })
      .catch(fail);
  };

  const doExecute = (overwrite: boolean, dest: string | null): void => {
    setBusy(true);
    setError(null);
    imageExecute(path, buildOperation(), overwrite, dest)
      .then((operationId) => {
        setBusy(false);
        setStatus(`${t("image.executed")} · ${operationId}`);
        onOperationDone();
      })
      .catch(fail);
  };

  return (
    <section
      aria-label={t("image.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("image.title")}</h2>

      <div style={rowStyle}>
        <input
          aria-label={t("image.filePath")}
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="C:\path\to\photo.jpg"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
        <button
          type="button"
          disabled={busy || path.length === 0}
          onClick={doOpen}
          style={boxStyle}
        >
          {t("image.open")}
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
      {status ? (
        <div
          role="status"
          style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}
        >
          {status}
        </div>
      ) : null}

      {open ? (
        <>
          <div style={rowStyle}>
            <img src={open.previewDataUri} alt={t("image.previewAlt")} style={imgStyle} />
            <table style={{ borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}>
              <tbody>
                <tr>
                  <td>{t("image.facts.format")}</td>
                  <td>{open.facts.format}</td>
                </tr>
                <tr>
                  <td>{t("image.facts.dimensions")}</td>
                  <td>
                    {open.facts.width} × {open.facts.height}
                  </td>
                </tr>
                <tr>
                  <td>{t("image.facts.alpha")}</td>
                  <td>{open.facts.hasAlpha ? "yes" : "no"}</td>
                </tr>
                <tr>
                  <td>{t("image.facts.exif")}</td>
                  <td>{open.facts.exifPresent ? "present" : "—"}</td>
                </tr>
                <tr>
                  <td>{t("image.facts.gps")}</td>
                  <td
                    style={{
                      color: open.facts.gpsPresent ? "var(--color-warning, orange)" : undefined,
                    }}
                  >
                    {open.facts.gpsPresent ? "PRESENT (privacy)" : "—"}
                  </td>
                </tr>
                {open.facts.extensionMismatch ? (
                  <tr>
                    <td>{t("image.facts.mismatch")}</td>
                    <td style={{ color: "var(--color-danger)" }}>extension ≠ content</td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>

          <div style={rowStyle}>
            <select
              aria-label={t("image.operation")}
              value={operation}
              onChange={(e) => setOperation(e.target.value as Operation)}
              style={boxStyle}
            >
              <option value="resize">{t("image.op.resize")}</option>
              <option value="compress">{t("image.op.compress")}</option>
              <option value="convert">{t("image.op.convert")}</option>
              <option value="strip_metadata">{t("image.op.strip")}</option>
            </select>
            {operation !== "strip_metadata" ? (
              <select
                aria-label={t("image.targetFormat")}
                value={targetFormat}
                onChange={(e) => setTargetFormat(e.target.value)}
                style={boxStyle}
              >
                {["png", "jpeg", "webp", "bmp", "tiff"].map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            ) : null}
            {targetFormat === "jpeg" || operation === "compress" ? (
              <label style={{ fontSize: "var(--typography-size-sm)" }}>
                {t("image.quality")} {quality}
                <input
                  type="range"
                  min={1}
                  max={100}
                  value={quality}
                  onChange={(e) => setQuality(Number(e.target.value))}
                />
              </label>
            ) : null}
            <label style={{ fontSize: "var(--typography-size-sm)" }}>
              <input
                type="checkbox"
                checked={stripMetadata}
                onChange={(e) => setStripMetadata(e.target.checked)}
              />
              {t("image.stripAll")}
            </label>
            <button type="button" disabled={busy} onClick={doPreview} style={boxStyle}>
              {t("text.preview")}
            </button>
          </div>

          {operation === "resize" ? (
            <div style={rowStyle}>
              <select
                aria-label={t("image.resizeMode")}
                value={resizeMode}
                onChange={(e) => setResizeMode(e.target.value as typeof resizeMode)}
                style={boxStyle}
              >
                <option value="fit">fit</option>
                <option value="fill">fill</option>
                <option value="exact">exact</option>
                <option value="scale">scale %</option>
              </select>
              {resizeMode === "scale" ? (
                <input
                  type="number"
                  min={1}
                  max={1000}
                  value={scalePercent}
                  onChange={(e) => setScalePercent(Number(e.target.value))}
                  style={{ ...boxStyle, width: "6em" }}
                />
              ) : (
                <>
                  <input
                    type="number"
                    aria-label={t("image.width")}
                    value={resizeW}
                    onChange={(e) => setResizeW(Number(e.target.value))}
                    style={{ ...boxStyle, width: "7em" }}
                  />
                  <input
                    type="number"
                    aria-label={t("image.height")}
                    value={resizeH}
                    onChange={(e) => setResizeH(Number(e.target.value))}
                    style={{ ...boxStyle, width: "7em" }}
                  />
                </>
              )}
              <label style={{ fontSize: "var(--typography-size-sm)" }}>
                <input
                  type="checkbox"
                  checked={preventUpscale}
                  onChange={(e) => setPreventUpscale(e.target.checked)}
                />
                {t("image.preventUpscale")}
              </label>
              <select
                aria-label={t("image.filter")}
                value={filter}
                onChange={(e) => setFilter(e.target.value as typeof filter)}
                style={boxStyle}
              >
                <option value="lanczos3">lanczos3</option>
                <option value="catmullrom">catmullrom</option>
                <option value="triangle">triangle</option>
                <option value="nearest">nearest</option>
              </select>
            </div>
          ) : null}

          {preview ? (
            <>
              {preview.warnings.map((w) => (
                <div
                  key={w}
                  role="note"
                  style={{
                    fontSize: "var(--typography-size-sm)",
                    color: "var(--color-text-muted)",
                  }}
                >
                  ⚠ {w}
                </div>
              ))}
              <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                {t("image.previewSummary", {
                  w: preview.outWidth,
                  h: preview.outHeight,
                  bytes: preview.estimatedOutputBytes,
                })}
                {preview.lossy ? ` · ${t("image.lossy")}` : ""}
              </div>
              <img src={preview.previewDataUri} alt={t("image.previewAlt")} style={imgStyle} />
              <div style={rowStyle}>
                <label style={{ fontSize: "var(--typography-size-sm)" }}>
                  <input
                    type="checkbox"
                    checked={overwriteSource}
                    onChange={(e) => setOverwriteSource(e.target.checked)}
                  />
                  {t("image.overwriteSource")}
                </label>
                {!overwriteSource ? (
                  <input
                    aria-label={t("image.destPath")}
                    value={destPath}
                    onChange={(e) => setDestPath(e.target.value)}
                    placeholder="C:\path\to\out.png"
                    style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
                  />
                ) : null}
                <button
                  type="button"
                  disabled={busy || (!overwriteSource && destPath.length === 0)}
                  onClick={() => doExecute(overwriteSource, destPath.length > 0 ? destPath : null)}
                  style={boxStyle}
                >
                  {t("image.execute")}
                </button>
              </div>
            </>
          ) : null}
        </>
      ) : null}
          <ImageBatchPanel onOperationDone={() => {}} />
    </section>
  );
}
