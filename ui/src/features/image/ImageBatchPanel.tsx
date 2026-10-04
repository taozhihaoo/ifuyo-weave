import { useState, type CSSProperties } from "react";
import { useT } from "../../i18n";
import { imageBatchExecute, type BatchResultView } from "../../lib/imageBatch";

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
const areaStyle: CSSProperties = {
  fontFamily: "var(--typography-mono-family)",
  fontSize: "var(--typography-size-sm)",
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  width: "100%",
  boxSizing: "border-box",
};

/** M6（下）批量：同操作 × 多文件（§79；§170 partial results；§171 retry
 * boundary = same operation + failed inputs only）。 */
export function ImageBatchPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [inputsText, setInputsText] = useState("");
  const [destDir, setDestDir] = useState("");
  const [targetFormat, setTargetFormat] = useState<"png" | "jpeg" | "webp" | "bmp" | "tiff">(
    "webp",
  );
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const [result, setResult] = useState<BatchResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const doRun = (): void => {
    const inputs = inputsText
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    if (inputs.length === 0) {
      setError(t("image.batch.noInputs"));
      return;
    }
    setBusy(true);
    setError(null);
    setResult(null);
    imageBatchExecute(inputs, {
      destinationDir: destDir,
      targetFormat,
      quality: null,
      overwriteExisting,
    })
      .then((r) => {
        setBusy(false);
        setResult(r);
        onOperationDone();
      })
      .catch((e: unknown) => {
        setBusy(false);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  return (
    <section
      aria-label={t("image.batch.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h3 style={{ margin: 0, fontSize: "var(--typography-size-lg)" }}>{t("image.batch.title")}</h3>
      <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
        {t("image.batch.hint")}
      </div>

      <textarea
        aria-label={t("image.batch.inputs")}
        value={inputsText}
        onChange={(e) => setInputsText(e.target.value)}
        rows={6}
        placeholder={t("image.batch.inputsHint")}
        style={areaStyle}
      />

      <div style={rowStyle}>
        <input
          aria-label={t("image.batch.destDir")}
          value={destDir}
          onChange={(e) => setDestDir(e.target.value)}
          placeholder="C:\path\to\output"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
        <select
          aria-label={t("data.exportFormat")}
          value={targetFormat}
          onChange={(e) => setTargetFormat(e.target.value as typeof targetFormat)}
          style={boxStyle}
        >
          <option value="webp">webp</option>
          <option value="png">png</option>
          <option value="jpeg">jpeg</option>
          <option value="bmp">bmp</option>
          <option value="tiff">tiff</option>
        </select>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input
            type="checkbox"
            checked={overwriteExisting}
            onChange={(e) => setOverwriteExisting(e.target.checked)}
          />
          {t("image.batch.overwrite")}
        </label>
        <button
          type="button"
          disabled={busy || inputsText.length === 0 || destDir.length === 0}
          onClick={doRun}
          style={boxStyle}
        >
          {t("image.batch.run")}
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

      {result ? (
        <>
          <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
            {t("image.batch.summary", {
              total: result.total,
              succeeded: result.succeeded,
              failed: result.failed,
              cancelled: result.cancelled,
            })}
          </div>
          {result.results.some((r) => r.status === "failed") ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
              {result.results
                .filter((r) => r.status === "failed")
                .map((r, i) => (
                  <div
                    key={i}
                    role="alert"
                    style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
                  >
                    {r.input}: {r.error ?? "failed"}
                  </div>
                ))}
            </div>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
