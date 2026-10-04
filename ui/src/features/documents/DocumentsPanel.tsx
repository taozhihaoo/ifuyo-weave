import { useState, type CSSProperties } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useT } from "../../i18n";
import {
  documentInspect,
  fieldText,
  pdfExtractExecute,
  pdfMergeExecute,
  pdfMergePreview,
  pdfRotateExecute,
  pdfSplitEveryNExecute,
  type DocumentFacts,
  type PdfMergePlan,
  type PdfOperationResult,
} from "../../lib/documents";

const FORCE_LINE_SEP = String.fromCharCode(10);

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
const tableStyle: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: "var(--typography-size-sm)",
};

/** M8（上）Documents：Drop/打开 → 事实（§14 Overview 优先）→ 操作 →
 * Preview → Execute。UI 不解析文档（§3：域逻辑全在 Rust 层）。 */
export function DocumentsPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [facts, setFacts] = useState<DocumentFacts | null>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [mergeInputs, setMergeInputs] = useState("");
  const [mergeDraft, setMergeDraft] = useState("");
  const [mergePreview, setMergePreview] = useState<PdfMergePlan | null>(null);
  const [destDir, setDestDir] = useState("");
  const [ranges, setRanges] = useState("");
  const [rotateDegrees, setRotateDegrees] = useState(90);
  const [n, setN] = useState(3);
  const [result, setResult] = useState<PdfOperationResult | null>(null);

  const inspect = async (target: string): Promise<void> => {
    setError(null);
    setResult(null);
    setMergePreview(null);
    try {
      const f = await documentInspect(target);
      setFacts(f);
      setPath(target);
    } catch (e) {
      setFacts(null);
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const openFile = async (): Promise<void> => {
    try {
      const selected = await openDialog({ multiple: false, directory: false });
      if (typeof selected === "string" && selected) {
        await inspect(selected);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const runPdf = async (
    op: () => Promise<PdfOperationResult>,
  ): Promise<void> => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await op();
      setResult(r);
      onOperationDone();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doMergePreview = async (): Promise<void> => {
    setError(null);
    setMergePreview(null);
    const inputs = mergeInputs.split("\n").map((l) => l.trim()).filter(Boolean);
    try {
      setMergePreview(await pdfMergePreview(inputs));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const isPdf = facts?.format === "pdf";
  const mergeList = mergeInputs.split("\n").map((l) => l.trim()).filter(Boolean);

  return (
    <section
      aria-label={t("documents.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h3 style={{ margin: 0, fontSize: "var(--typography-size-lg)" }}>{t("documents.title")}</h3>
      <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
        {t("documents.hint")}
      </div>

      <div style={rowStyle}>
        <button type="button" onClick={() => void openFile()} style={boxStyle}>
          {t("documents.open")}
        </button>
        <input
          aria-label={t("documents.path")}
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="C:\path\to\document.pdf"
          style={{ ...boxStyle, flex: 1, minWidth: "18em", fontFamily: "var(--typography-mono-family)" }}
        />
        <button
          type="button"
          disabled={path.length === 0}
          onClick={() => void inspect(path)}
          style={boxStyle}
        >
          {t("documents.inspect")}
        </button>
      </div>

      {error ? (
        <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
          {error}
        </div>
      ) : null}

      {facts ? <FactsView facts={facts} /> : (
        <div style={{ ...boxStyle, fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          {t("documents.emptyNoDoc")}
        </div>
      )}

      {isPdf ? (
        <>
          <h4 style={{ margin: 0 }}>{t("documents.pdfOps")}</h4>
          <div style={rowStyle}>
            <label style={{ fontSize: "var(--typography-size-sm)" }}>{t("documents.ranges")}</label>
            <input
              aria-label={t("documents.ranges")}
              value={ranges}
              onChange={(e) => setRanges(e.target.value)}
              placeholder="1-3,5"
              style={{ ...boxStyle, width: "10em", fontFamily: "var(--typography-mono-family)" }}
            />
            <button
              type="button"
              disabled={busy || !path}
              onClick={() => void runPdf(() => pdfExtractExecute(path, ranges, destDir))}
              style={boxStyle}
            >
              {t("documents.extract")}
            </button>
            <select
              aria-label={t("documents.degrees")}
              value={rotateDegrees}
              onChange={(e) => setRotateDegrees(Number(e.target.value))}
              style={boxStyle}
            >
              <option value={90}>90°</option>
              <option value={180}>180°</option>
              <option value={270}>270°</option>
            </select>
            <button
              type="button"
              disabled={busy || !path}
              onClick={() => void runPdf(() => pdfRotateExecute(path, rotateDegrees, ranges, destDir))}
              style={boxStyle}
            >
              {t("documents.rotate")}
            </button>
            <span style={{ fontSize: "var(--typography-size-sm)" }}>N=</span>
            <input
              aria-label={t("documents.splitN")}
              value={n}
              onChange={(e) => setN(Number(e.target.value))}
              style={{ ...boxStyle, width: "4em" }}
            />
            <button
              type="button"
              disabled={busy || !path}
              onClick={() => void runPdf(() => pdfSplitEveryNExecute(path, n, destDir))}
              style={boxStyle}
            >
              {t("documents.split")}
            </button>
          </div>

          <h4 style={{ margin: 0 }}>{t("documents.merge")}</h4>
          {mergeList.length === 0 ? (
            <div style={{ ...boxStyle, fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
              {t("documents.emptyMerge")}
            </div>
          ) : null}
          <ol style={{ margin: 0, paddingLeft: "1.2em", display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
            {mergeList.map((input, i) => {
              const duplicate = mergeList.indexOf(input) !== i;
              return (
                <li key={`${input}-${i}`} style={{ fontSize: "var(--typography-size-sm)" }}>
                  <span style={{ fontFamily: "var(--typography-mono-family)" }}>{input}</span>
                  {duplicate ? (
                    <span role="alert" style={{ color: "var(--color-danger)" }}> · {t("documents.duplicate")}</span>
                  ) : null}
                  {" "}
                  <button
                    type="button"
                    aria-label={t("documents.moveUp")}
                    disabled={i === 0}
                    onClick={() => {
                      const next = [...mergeList];
                      const removed = next.splice(i - 1, 1)[0];
                      if (removed !== undefined) {
                        next.splice(i, 0, removed);
                      }
                      setMergeInputs(next.join(FORCE_LINE_SEP));
                    }}
                    style={boxStyle}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    aria-label={t("documents.moveDown")}
                    disabled={i === mergeList.length - 1}
                    onClick={() => {
                      const next = [...mergeList];
                      const removed = next.splice(i + 1, 1)[0];
                      if (removed !== undefined) {
                        next.splice(i, 0, removed);
                      }
                      setMergeInputs(next.join(FORCE_LINE_SEP));
                    }}
                    style={boxStyle}
                  >
                    ↓
                  </button>
                </li>
              );
            })}
          </ol>
          <div style={rowStyle}>
            <input
              aria-label={t("documents.mergeInputs")}
              value={mergeDraft}
              onChange={(e) => setMergeDraft(e.target.value)}
              placeholder={t("documents.inputsHint")}
              style={{ ...boxStyle, flex: 1, minWidth: "16em", fontFamily: "var(--typography-mono-family)" }}
            />
            <button
              type="button"
              disabled={mergeDraft.length === 0}
              onClick={() => {
                setMergeInputs([...mergeList, mergeDraft.trim()].join(FORCE_LINE_SEP));
                setMergeDraft("");
              }}
              style={boxStyle}
            >
              {t("documents.addInput")}
            </button>
          </div>
          <div style={rowStyle}>
            <button type="button" disabled={mergeList.length < 2} onClick={() => void doMergePreview()} style={boxStyle}>
              {t("documents.mergePreview")}
            </button>
            <button
              type="button"
              disabled={busy || mergeList.length < 2 || new Set(mergeList).size !== mergeList.length}
              onClick={() => void runPdf(() => pdfMergeExecute(mergeList, destDir))}
              style={boxStyle}
            >
              {t("documents.mergeExecute")}
            </button>
            {new Set(mergeList).size !== mergeList.length ? (
              <span role="alert" style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-danger)" }}>
                {t("documents.dupBlocked")}
              </span>
            ) : null}
          </div>
          {mergePreview ? (
            <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
              {mergePreview.inputs.map((input, i) => (
                <div key={`${input}-${i}`} style={{ fontFamily: "var(--typography-mono-family)" }}>
                  {input} — {mergePreview.inputPageCounts[i]} {t("documents.pages")}
                </div>
              ))}
              <div>
                <strong>{t("documents.totalPages", { count: mergePreview.outputPageCount ?? 0 })}</strong>
              </div>
            </div>
          ) : null}
        </>
      ) : null}

      <div style={rowStyle}>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>{t("documents.destDir")}</label>
        <input
          aria-label={t("documents.destDir")}
          value={destDir}
          onChange={(e) => setDestDir(e.target.value)}
          placeholder="C:\path\to\output"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
      </div>

      {result ? (
        <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
          <strong>{t("documents.result")}</strong>
          {result.outputs.map((o, i) => (
            <div key={o} style={{ fontFamily: "var(--typography-mono-family)" }}>
              {o} ({result.pageCounts[i]} {t("documents.pages")})
            </div>
          ))}
          {result.warnings.map((w, i) => (
            <div key={i} style={{ color: "var(--color-text-muted)" }}>
              {w}
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function FactsView({ facts }: { facts: DocumentFacts }) {
  const t = useT();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
        <strong>{t(`documents.format.${facts.format}`, { defaultValue: facts.format })}</strong>
        {" · "}
        {t("documents.size", { size: facts.size ?? 0 })}
        {facts.extensionMismatch ? ` · ${t("documents.mismatch")}` : ""}
      </div>
      <table style={tableStyle}>
        <tbody>
          <FactRow label={t("documents.f.pages")} value={fieldText(facts.pages)} />
          <FactRow label={t("documents.f.encrypted")} value={fieldText(facts.encrypted)} />
          <FactRow label={t("documents.f.pdfVersion")} value={fieldText(facts.pdfVersion)} />
          <FactRow
            label={t("documents.f.paragraphs")}
            value={fieldText(facts.statistics.paragraphs)}
          />
          <FactRow label={t("documents.f.headings")} value={fieldText(facts.statistics.headings)} />
          <FactRow label={t("documents.f.tables")} value={fieldText(facts.statistics.tables)} />
          <FactRow label={t("documents.f.images")} value={fieldText(facts.statistics.images)} />
          <FactRow label={t("documents.f.words")} value={fieldText(facts.statistics.words)} />
        </tbody>
      </table>
      {facts.sheets?.state === "known" && facts.sheets.value.length > 0 ? (
        <div style={{ ...boxStyle, maxHeight: "12em", overflow: "auto" }}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>{t("documents.f.sheet")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.visibility")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.dimension")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.cells")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.formulas")}</th>
              </tr>
            </thead>
            <tbody>
              {facts.sheets.value.map((s) => (
                <tr key={s.name}>
                  <td>{s.name}</td>
                  <td>{t(`documents.visibility.${s.visibility}`, { defaultValue: s.visibility })}</td>
                  <td>{s.dimension ?? "—"}</td>
                  <td>{s.populatedCells ?? "—"}</td>
                  <td>{s.formulaCells ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {facts.slides?.state === "known" && facts.slides.value.length > 0 ? (
        <div style={{ ...boxStyle, maxHeight: "12em", overflow: "auto" }}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>#</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.hidden")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.text")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.shapes")}</th>
                <th style={{ textAlign: "left" }}>{t("documents.f.images")}</th>
              </tr>
            </thead>
            <tbody>
              {facts.slides.value.map((s) => (
                <tr key={s.index}>
                  <td>{s.index}</td>
                  <td>{s.hidden ? t("common.yes") : t("common.no")}</td>
                  <td>{s.textChars ?? 0}</td>
                  <td>{s.shapeCount ?? "—"}</td>
                  <td>{s.imageCount ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {facts.metadata.length > 0 ? (
        <div style={boxStyle}>
          {facts.metadata.map(([k, v]) => (
            <div key={k} style={{ fontSize: "var(--typography-size-sm)" }}>
              <strong>{k}</strong>: {v}
            </div>
          ))}
        </div>
      ) : null}
      {facts.diagnostics.map((d, i) => (
        <div
          key={i}
          role={d.severity === "error" ? "alert" : "status"}
          style={{
            fontSize: "var(--typography-size-sm)",
            color: d.severity === "error" ? "var(--color-danger)" : "var(--color-text-muted)",
          }}
        >
          {d.message}
        </div>
      ))}
    </div>
  );
}

function FactRow({ label, value }: { label: string; value: string }) {
  return (
    <tr>
      <td style={{ padding: "var(--spacing-xs)", color: "var(--color-text-muted)" }}>{label}</td>
      <td style={{ padding: "var(--spacing-xs)" }}>{value}</td>
    </tr>
  );
}
