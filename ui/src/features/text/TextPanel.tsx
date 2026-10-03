import { useState, type CSSProperties } from "react";
import { commands, type DiffReportDto, type IpcError } from "../../generated/bindings";
import { useT } from "../../i18n";
import { pollUntilDone } from "../../lib/operations";

interface TextPanelProps {
  onOperationDone: (operationId: string, undoable: boolean) => void;
}

type Tab = "format" | "transform" | "extract" | "compare";

/** Preview 截断上限（§85：截断只影响显示，Apply 始终基于完整内容）。 */
const PREVIEW_LIMIT = 20000;

function truncate(text: string): string {
  if (text.length <= PREVIEW_LIMIT) {
    return text;
  }
  return `${text.slice(0, PREVIEW_LIMIT)}\n…`;
}

const boxStyle: CSSProperties = {
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
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

const rowStyle: CSSProperties = {
  display: "flex",
  gap: "var(--spacing-sm)",
  alignItems: "center",
  flexWrap: "wrap",
};

function errText(e: IpcError): string {
  return `${e.code} · ${e.message}${e.suggestion ? ` (${e.suggestion})` : ""}`;
}

/** M4 Text：Format / Compare / Extract / Transform 四工具统一面板。
 *
 * Preview-first（§84–§86）：预览无副作用；Apply 才写回且需确认（§87/§89）；
 * 写回走服务端 Plan（TOCTOU 快照 §90）+ 备份 + 原子替换 + 历史/Undo。 */
export function TextPanel({ onOperationDone }: TextPanelProps) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("format");
  const [content, setContent] = useState("");
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const [docEncoding, setDocEncoding] = useState("utf8");
  const [docBom, setDocBom] = useState("none");
  const [docSize, setDocSize] = useState(0);
  const [docModified, setDocModified] = useState<number | null>(null);
  const [loadPath, setLoadPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  // format
  const [format, setFormat] = useState("auto");
  const [operation, setOperation] = useState("format");
  const [formatResult, setFormatResult] = useState<string | null>(null);
  const [formatNotes, setFormatNotes] = useState<string[]>([]);
  // transform
  const [transformOp, setTransformOp] = useState("trimLines");
  const [findText, setFindText] = useState("");
  const [replaceText, setReplaceText] = useState("");
  const [prefixText, setPrefixText] = useState("");
  const [transformStat, setTransformStat] = useState<string | null>(null);
  const [transformResult, setTransformResult] = useState<string | null>(null);
  // extract
  const [extractKind, setExtractKind] = useState("url");
  const [regexPattern, setRegexPattern] = useState("");
  const [uniqueValues, setUniqueValues] = useState(false);
  const [extractRows, setExtractRows] = useState<{ value: string; line: number; column: number }[]>(
    [],
  );
  // compare
  const [compareText, setCompareText] = useState("");
  const [compareResult, setCompareResult] = useState<DiffReportDto | null>(null);

  const detectFormat = (text: string): string => {
    const head = text.trimStart();
    if (head.startsWith("{") || head.startsWith("[")) {
      return "json";
    }
    if (head.startsWith("<?xml") || head.startsWith("<")) {
      return "xml";
    }
    return "json";
  };

  const doLoad = (): void => {
    setBusy(true);
    setError(null);
    setStatus(null);
    commands
      .loadTextDocument(loadPath, null)
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          setContent(r.data.content);
          setLoadedPath(r.data.path);
          setDocEncoding(r.data.encoding);
          setDocBom(r.data.bom);
          setDocSize(r.data.byteSize ?? 0);
          setDocModified(r.data.modifiedMs);
          setStatus(
            t("text.loaded", {
              size: r.data.byteSize ?? 0,
              encoding: r.data.encoding,
            }),
          );
        } else {
          setError(errText(r.error));
        }
      })
      .catch(() => setBusy(false));
  };

  const doFormatPreview = (): void => {
    setBusy(true);
    setError(null);
    setFormatResult(null);
    setFormatNotes([]);
    const fmt = format === "auto" ? detectFormat(content) : format;
    commands
      .formatText(fmt, operation, content, 2, true)
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          if (r.data.diagnostics.length > 0) {
            setFormatNotes(
              r.data.diagnostics.map((d) => `[${d.severity}] ${d.code} · ${d.message}`),
            );
          }
          setFormatResult(r.data.content ?? "");
          if (r.data.content === null && r.data.diagnostics.length === 0) {
            setFormatNotes([t("text.validateOk")]);
          }
        } else {
          setError(errText(r.error));
        }
      })
      .catch(() => setBusy(false));
  };

  const doTransformPreview = (): void => {
    setBusy(true);
    setError(null);
    setTransformResult(null);
    setTransformStat(null);
    const op = buildTransformOp(transformOp);
    commands
      .transformText(content, op)
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          setTransformResult(r.data.content);
          if (r.data.matchCount !== null) {
            setTransformStat(t("text.matchCount", { count: r.data.matchCount }));
          } else if (r.data.removedLines !== null) {
            setTransformStat(t("text.removedLines", { count: r.data.removedLines }));
          }
        } else {
          setError(errText(r.error));
        }
      })
      .catch(() => setBusy(false));
  };

  const buildTransformOp = (op: string): Parameters<typeof commands.transformText>[1] => {
    switch (op) {
      case "trimDocument":
        return { type: "trimDocument" };
      case "deduplicate":
        return { type: "deduplicateLines", keep: "first", blank: "included" };
      case "sortAsc":
        return {
          type: "sortLines",
          descending: false,
          caseSensitive: false,
          blank: "last",
        };
      case "sortDesc":
        return {
          type: "sortLines",
          descending: true,
          caseSensitive: false,
          blank: "last",
        };
      case "prefix":
        return { type: "addPrefix", text: prefixText, skipBlank: true };
      case "suffix":
        return { type: "addSuffix", text: prefixText, skipBlank: true };
      case "upper":
        return { type: "caseConvert", form: "upper" };
      case "lower":
        return { type: "caseConvert", form: "lower" };
      case "number":
        return { type: "numberLines", start: 1, step: 1, separator: ". ", pad: "none" };
      case "replace":
        return {
          type: "findReplace",
          find: findText,
          replacement: replaceText,
          regex: false,
          caseInsensitive: false,
          firstOnly: false,
        };
      case "regexReplace":
        return {
          type: "findReplace",
          find: findText,
          replacement: replaceText,
          regex: true,
          caseInsensitive: false,
          firstOnly: false,
        };
      default:
        return { type: "trimLines" };
    }
  };

  const doExtract = (): void => {
    setBusy(true);
    setError(null);
    setExtractRows([]);
    commands
      .extractText(
        content,
        extractKind,
        extractKind === "regex" ? regexPattern : null,
        uniqueValues,
      )
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          setExtractRows(
            r.data.map((m) => ({
              value: m.value,
              line: m.line ?? 0,
              column: m.column ?? 0,
            })),
          );
        } else {
          setError(errText(r.error));
        }
      })
      .catch(() => setBusy(false));
  };

  const doCompare = (): void => {
    setBusy(true);
    setError(null);
    setCompareResult(null);
    commands
      .compareText(content, compareText, "none", false)
      .then((r) => {
        setBusy(false);
        if (r.status === "ok") {
          setCompareResult(r.data);
        } else {
          setError(errText(r.error));
        }
      })
      .catch(() => setBusy(false));
  };

  const copyResult = (text: string): void => {
    void navigator.clipboard?.writeText(text);
    setStatus(t("text.copied"));
  };

  const doApply = (newContent: string): void => {
    if (!loadedPath) {
      setError(t("text.noFileLoaded"));
      return;
    }
    setBusy(true);
    setError(null);
    commands
      .buildTextWritePlan(loadedPath, newContent, docEncoding, docBom, docSize, docModified)
      .then((r) => {
        if (r.status !== "ok") {
          setBusy(false);
          setError(errText(r.error));
          return;
        }
        // §87/§89：写回需显式确认
        if (!window.confirm(t("text.applyConfirm"))) {
          setBusy(false);
          return;
        }
        return commands.executeTextPlan(r.data.operationId).then((x) => {
          if (x.status !== "ok") {
            setBusy(false);
            setError(errText(x.error));
            return;
          }
          pollUntilDone(
            x.data.jobId,
            () => {},
            (st) => {
              setBusy(false);
              if (st.state === "failed") {
                setError(
                  errText(
                    st.error ?? {
                      kind: "internal",
                      code: "text.applyFailed",
                      message: "text apply failed",
                      location: null,
                      recoverability: "retryable",
                      suggestion: null,
                    },
                  ),
                );
                return;
              }
              const report = st.plan;
              if (report) {
                onOperationDone(report.operationId, report.undoable);
                setStatus(t("text.applied", { bytes: docSize }));
                // 重新加载以刷新快照（Preview 基于新状态）
                doLoad();
              }
            },
          );
        });
      })
      .catch(() => setBusy(false));
  };

  return (
    <section
      aria-label={t("text.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("text.title")}</h2>

      <div style={rowStyle}>
        <input
          aria-label={t("text.filePath")}
          value={loadPath}
          onChange={(e) => setLoadPath(e.target.value)}
          placeholder="C:\path\to\file.txt"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
        <button
          type="button"
          disabled={busy || loadPath.length === 0}
          onClick={doLoad}
          style={boxStyle}
        >
          {t("text.load")}
        </button>
        {loadedPath ? (
          <span style={{ fontSize: "var(--typography-size-sm)" }}>{loadedPath}</span>
        ) : null}
      </div>

      <textarea
        aria-label={t("text.input")}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        rows={8}
        placeholder={t("text.inputHint")}
        style={areaStyle}
      />

      <nav aria-label={t("text.tabs")} style={rowStyle}>
        {(["format", "transform", "extract", "compare"] as const).map((v) => (
          <button
            key={v}
            type="button"
            onClick={() => setTab(v)}
            aria-current={tab === v ? "true" : undefined}
            style={{
              ...boxStyle,
              background: tab === v ? "var(--color-accent-soft)" : "var(--color-surface)",
              borderColor: tab === v ? "var(--color-accent)" : "var(--color-border)",
            }}
          >
            {t(`text.tab.${v}`)}
          </button>
        ))}
      </nav>

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

      {tab === "format" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <div style={rowStyle}>
            <select
              aria-label={t("text.format")}
              value={format}
              onChange={(e) => setFormat(e.target.value)}
              style={boxStyle}
            >
              {["auto", "json", "xml", "yaml", "sql", "javascript", "css", "markdown"].map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
            <select
              aria-label={t("text.operation")}
              value={operation}
              onChange={(e) => setOperation(e.target.value)}
              style={boxStyle}
            >
              {["format", "minify", "validate", "sort", "normalize"].map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={busy || content.length === 0}
              onClick={doFormatPreview}
              style={boxStyle}
            >
              {t("text.preview")}
            </button>
          </div>
          {formatNotes.map((note) => (
            <div
              key={note}
              style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}
            >
              {note}
            </div>
          ))}
          {formatResult !== null ? (
            <>
              <textarea
                readOnly
                aria-label={t("text.result")}
                value={truncate(formatResult)}
                rows={10}
                style={areaStyle}
              />
              <div style={rowStyle}>
                <button type="button" onClick={() => copyResult(formatResult)} style={boxStyle}>
                  {t("text.copy")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => doApply(formatResult)}
                  style={boxStyle}
                >
                  {t("text.applyOverwrite")}
                </button>
              </div>
            </>
          ) : null}
        </div>
      ) : null}

      {tab === "transform" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <div style={rowStyle}>
            <select
              aria-label={t("text.transformOp")}
              value={transformOp}
              onChange={(e) => setTransformOp(e.target.value)}
              style={boxStyle}
            >
              {(
                [
                  ["trimLines", "text.op.trimLines"],
                  ["trimDocument", "text.op.trimDocument"],
                  ["deduplicate", "text.op.deduplicate"],
                  ["sortAsc", "text.op.sortAsc"],
                  ["sortDesc", "text.op.sortDesc"],
                  ["prefix", "text.op.prefix"],
                  ["suffix", "text.op.suffix"],
                  ["upper", "text.op.upper"],
                  ["lower", "text.op.lower"],
                  ["number", "text.op.number"],
                  ["replace", "text.op.replace"],
                  ["regexReplace", "text.op.regexReplace"],
                ] as const
              ).map(([v, key]) => (
                <option key={v} value={v}>
                  {t(key)}
                </option>
              ))}
            </select>
            {transformOp === "prefix" || transformOp === "suffix" ? (
              <input
                aria-label={t("text.prefixText")}
                value={prefixText}
                onChange={(e) => setPrefixText(e.target.value)}
                style={boxStyle}
              />
            ) : null}
            {transformOp === "replace" || transformOp === "regexReplace" ? (
              <>
                <input
                  aria-label={t("text.find")}
                  value={findText}
                  onChange={(e) => setFindText(e.target.value)}
                  placeholder={t("text.find")}
                  style={boxStyle}
                />
                <input
                  aria-label={t("text.replaceWith")}
                  value={replaceText}
                  onChange={(e) => setReplaceText(e.target.value)}
                  placeholder={t("text.replaceWith")}
                  style={boxStyle}
                />
              </>
            ) : null}
            <button
              type="button"
              disabled={busy || content.length === 0}
              onClick={doTransformPreview}
              style={boxStyle}
            >
              {t("text.preview")}
            </button>
          </div>
          {transformStat ? (
            <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
              {transformStat}
            </div>
          ) : null}
          {transformResult !== null ? (
            <>
              <textarea
                readOnly
                aria-label={t("text.result")}
                value={truncate(transformResult)}
                rows={10}
                style={areaStyle}
              />
              <div style={rowStyle}>
                <button type="button" onClick={() => copyResult(transformResult)} style={boxStyle}>
                  {t("text.copy")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => doApply(transformResult)}
                  style={boxStyle}
                >
                  {t("text.applyOverwrite")}
                </button>
              </div>
            </>
          ) : null}
        </div>
      ) : null}

      {tab === "extract" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <div style={rowStyle}>
            <select
              aria-label={t("text.extractKind")}
              value={extractKind}
              onChange={(e) => setExtractKind(e.target.value)}
              style={boxStyle}
            >
              {[
                "url",
                "email",
                "filePath",
                "number",
                "ipv4",
                "ipv6",
                "json",
                "markdownLink",
                "regex",
              ].map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </select>
            {extractKind === "regex" ? (
              <input
                aria-label={t("text.regex")}
                value={regexPattern}
                onChange={(e) => setRegexPattern(e.target.value)}
                placeholder="\\d+"
                style={boxStyle}
              />
            ) : null}
            <label style={{ fontSize: "var(--typography-size-sm)" }}>
              <input
                type="checkbox"
                checked={uniqueValues}
                onChange={(e) => setUniqueValues(e.target.checked)}
              />
              {t("text.uniqueValues")}
            </label>
            <button
              type="button"
              disabled={busy || content.length === 0}
              onClick={doExtract}
              style={boxStyle}
            >
              {t("text.extract")}
            </button>
          </div>
          {extractRows.length > 0 ? (
            <>
              <table style={{ borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: "start" }}>{t("text.col.value")}</th>
                    <th>{t("text.col.line")}</th>
                    <th>{t("text.col.column")}</th>
                  </tr>
                </thead>
                <tbody>
                  {extractRows.map((row, i) => (
                    <tr key={`${row.value}-${i}`}>
                      <td
                        style={{
                          fontFamily: "var(--typography-mono-family)",
                          overflowWrap: "anywhere",
                        }}
                      >
                        {row.value}
                      </td>
                      <td style={{ textAlign: "end" }}>{row.line}</td>
                      <td style={{ textAlign: "end" }}>{row.column}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <button
                type="button"
                onClick={() => copyResult(extractRows.map((r) => r.value).join("\n"))}
                style={{ ...boxStyle, alignSelf: "flex-start" }}
              >
                {t("text.copyAll")}
              </button>
            </>
          ) : null}
        </div>
      ) : null}

      {tab === "compare" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
          <textarea
            aria-label={t("text.compareWith")}
            value={compareText}
            onChange={(e) => setCompareText(e.target.value)}
            rows={6}
            placeholder={t("text.compareWith")}
            style={areaStyle}
          />
          <button
            type="button"
            disabled={busy || content.length === 0}
            onClick={doCompare}
            style={{ ...boxStyle, alignSelf: "flex-start" }}
          >
            {t("text.compare")}
          </button>
          {compareResult ? (
            <>
              <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                {t("text.diffStats", {
                  added: compareResult.stats.added ?? 0,
                  removed: compareResult.stats.removed ?? 0,
                  changed: compareResult.stats.changed ?? 0,
                  moved: compareResult.stats.moved ?? 0,
                })}
                {compareResult.degraded ? ` · ${t("text.diffDegraded")}` : ""}
              </div>
              <textarea
                readOnly
                aria-label={t("text.diffUnified")}
                value={truncate(compareResult.unified)}
                rows={12}
                style={areaStyle}
              />
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
