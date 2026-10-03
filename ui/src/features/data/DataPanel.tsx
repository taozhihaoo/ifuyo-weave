import { useState, type CSSProperties } from "react";
import {
  type ColumnProfileDto,
  type DataPageDto,
  type TransformPreviewDto,
} from "../../generated/bindings";
import { useT } from "../../i18n";
import {
  dataApplyTransform,
  dataExport,
  dataInspectProfiles,
  dataOpen,
  dataPreviewTransform,
  dataSetView,
  pollExport,
  type FilterSpec,
} from "../../lib/data";

type Tab = "table" | "inspector" | "cleaner" | "convert";

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

const OPERATORS: FilterSpec["operator"][] = [
  "contains",
  "equals",
  "notEquals",
  "startsWith",
  "endsWith",
  "empty",
  "notEmpty",
];

/** M5 Data：CSV/TSV/JSON/JSONL 四工具面板（§98-§101 同源布局纪律）。 */
export function DataPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("table");
  const [page, setPage] = useState<DataPageDto | null>(null);
  const [profiles, setProfiles] = useState<ColumnProfileDto[] | null>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [offset, setOffset] = useState(0);
  // filters
  const [filterCol, setFilterCol] = useState("");
  const [filterOp, setFilterOp] = useState<FilterSpec["operator"]>("contains");
  const [filterVal, setFilterVal] = useState("");
  const [filterCase, setFilterCase] = useState(false);
  const [sortCol, setSortCol] = useState("");
  const [sortDesc, setSortDesc] = useState(false);
  // cleaner
  const [planText, setPlanText] = useState("{}");
  const [preview, setPreview] = useState<TransformPreviewDto | null>(null);
  // convert
  const [destPath, setDestPath] = useState("");
  const [exportFormat, setExportFormat] = useState<"csv" | "tsv" | "json" | "jsonl">("csv");
  // §176/§179：导出范围显式化——all = 底表全行；view = 当前过滤/排序视图
  const [exportScope, setExportScope] = useState<"all" | "view">("all");

  const pageTotal = (): number => page?.totalRows ?? 0;

  const fail = (e: unknown): void => {
    setError(e instanceof Error ? e.message : String(e));
    setBusy(false);
  };

  const doOpen = (): void => {
    setBusy(true);
    setError(null);
    setStatus(null);
    setProfiles(null);
    dataOpen(path, null)
      .then((p) => {
        setBusy(false);
        setPage(p);
        setOffset(0);
        setStatus(
          t("data.loaded", {
            rows: p.totalRows ?? 0,
            cols: p.columns.length,
          }),
        );
      })
      .catch(fail);
  };

  const refresh = (newOffset: number): void => {
    if (!page) return;
    setBusy(true);
    dataSetView(page.sessionId, [], null, newOffset)
      .then((p) => {
        setBusy(false);
        setPage(p);
        setOffset(newOffset);
      })
      .catch(fail);
  };

  const applyFilters = (): void => {
    if (!page) return;
    setBusy(true);
    setError(null);
    const rules: FilterSpec[] =
      filterVal.length > 0 || filterOp === "empty" || filterOp === "notEmpty"
        ? [{ columnId: filterCol, operator: filterOp, value: filterVal, caseSensitive: filterCase }]
        : [];
    const sort = sortCol ? { columnId: sortCol, descending: sortDesc, ignoreCase: true } : null;
    dataSetView(page.sessionId, rules, sort, 0)
      .then((p) => {
        setBusy(false);
        setPage(p);
        setOffset(0);
      })
      .catch(fail);
  };

  const loadProfiles = (): void => {
    if (!page) return;
    setBusy(true);
    dataInspectProfiles(page.sessionId)
      .then((p) => {
        setBusy(false);
        setProfiles(p);
      })
      .catch(fail);
  };

  const doPreviewPlan = (): void => {
    if (!page) return;
    setBusy(true);
    setError(null);
    let plan: unknown;
    try {
      plan = JSON.parse(planText);
    } catch (e) {
      setBusy(false);
      setError(`${e instanceof Error ? e.message : String(e)}`);
      return;
    }
    dataPreviewTransform(page.sessionId, plan as Record<string, unknown>)
      .then((p) => {
        setBusy(false);
        setPreview(p);
      })
      .catch(fail);
  };

  const doApplyPlan = (): void => {
    if (!page) return;
    setBusy(true);
    let plan: unknown;
    try {
      plan = JSON.parse(planText);
    } catch (e) {
      setBusy(false);
      setError(`${e instanceof Error ? e.message : String(e)}`);
      return;
    }
    dataApplyTransform(page.sessionId, plan as Record<string, unknown>)
      .then((p) => {
        setBusy(false);
        setPage(p);
        setStatus(t("data.applied"));
      })
      .catch(fail);
  };

  const doExport = (): void => {
    if (!page) return;
    setBusy(true);
    setError(null);
    dataExport(page.sessionId, destPath, exportFormat, true, false, "lf", exportScope)
      .then((operationId) => {
        setStatus(t("data.exporting"));
        return pollExport(operationId).then(() => {
          setBusy(false);
          setStatus(t("data.exported", { path: destPath }));
          onOperationDone();
        });
      })
      .catch(fail);
  };

  return (
    <section
      aria-label={t("data.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("data.title")}</h2>

      <div style={rowStyle}>
        <input
          aria-label={t("data.filePath")}
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="C:\path\to\data.csv"
          style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
        />
        <button
          type="button"
          disabled={busy || path.length === 0}
          onClick={doOpen}
          style={boxStyle}
        >
          {t("data.open")}
        </button>
        {page ? (
          <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {page.format} · {page.encoding}
          </span>
        ) : null}
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

      {page ? (
        <>
          <nav aria-label={t("data.tabs")} style={rowStyle}>
            {(["table", "inspector", "cleaner", "convert"] as const).map((v) => (
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
                {t(`data.tab.${v}`)}
              </button>
            ))}
          </nav>

          {tab === "table" ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
              <div style={rowStyle}>
                <select
                  aria-label={t("data.filterCol")}
                  value={filterCol}
                  onChange={(e) => setFilterCol(e.target.value)}
                  style={boxStyle}
                >
                  <option value="">{t("data.allColumns")}</option>
                  {page.columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
                <select
                  aria-label={t("data.filterOp")}
                  value={filterOp}
                  onChange={(e) => setFilterOp(e.target.value as FilterSpec["operator"])}
                  style={boxStyle}
                >
                  {OPERATORS.map((o) => (
                    <option key={o} value={o}>
                      {o}
                    </option>
                  ))}
                </select>
                <input
                  aria-label={t("data.filterVal")}
                  value={filterVal}
                  onChange={(e) => setFilterVal(e.target.value)}
                  style={boxStyle}
                />
                <label style={{ fontSize: "var(--typography-size-sm)" }}>
                  <input
                    type="checkbox"
                    checked={filterCase}
                    onChange={(e) => setFilterCase(e.target.checked)}
                  />
                  {t("text.ignoreCase")}
                </label>
                <select
                  aria-label={t("data.sortCol")}
                  value={sortCol}
                  onChange={(e) => setSortCol(e.target.value)}
                  style={boxStyle}
                >
                  <option value="">{t("data.noSort")}</option>
                  {page.columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
                <label style={{ fontSize: "var(--typography-size-sm)" }}>
                  <input
                    type="checkbox"
                    checked={sortDesc}
                    onChange={(e) => setSortDesc(e.target.checked)}
                  />
                  {t("data.desc")}
                </label>
                <button type="button" disabled={busy} onClick={applyFilters} style={boxStyle}>
                  {t("data.applyView")}
                </button>
              </div>
              <div
                role="status"
                style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}
              >
                {t("data.rowSummary", {
                  shown: page.rows.length,
                  total: page.totalRows ?? 0,
                  offset: page.offset ?? 0,
                })}
              </div>
              <div style={{ overflowX: "auto" }}>
                <table
                  style={{ borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}
                >
                  <thead>
                    <tr>
                      {page.columns.map((c) => (
                        <th key={c.id} style={{ textAlign: "start", padding: "var(--spacing-xs)" }}>
                          {c.name}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {page.rows.map((row, ri) => (
                      <tr key={page.rowIds[ri] ?? ri}>
                        {page.columns.map((c, ci) => (
                          <td
                            key={c.id}
                            style={{
                              borderTop: "1px solid var(--color-border)",
                              padding: "var(--spacing-xs)",
                              fontFamily: "var(--typography-mono-family)",
                            }}
                          >
                            {row[ci] ?? ""}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div style={rowStyle}>
                <button
                  type="button"
                  disabled={busy || offset <= 0}
                  onClick={() => refresh(Math.max(0, offset - 100))}
                  style={boxStyle}
                >
                  {t("data.prev")}
                </button>
                <button
                  type="button"
                  disabled={busy || offset + page.rows.length >= (page.totalRows ?? 0)}
                  onClick={() => refresh(offset + 100)}
                  style={boxStyle}
                >
                  {t("data.next")}
                </button>
              </div>
            </div>
          ) : null}

          {tab === "inspector" ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
              <button
                type="button"
                disabled={busy}
                onClick={loadProfiles}
                style={{ ...boxStyle, alignSelf: "flex-start" }}
              >
                {t("data.inspect")}
              </button>
              {profiles ? (
                <table
                  style={{ borderCollapse: "collapse", fontSize: "var(--typography-size-sm)" }}
                >
                  <thead>
                    <tr>
                      <th style={{ textAlign: "start", padding: "var(--spacing-xs)" }}>
                        {t("data.col.name")}
                      </th>
                      <th style={{ textAlign: "start", padding: "var(--spacing-xs)" }}>
                        {t("data.col.types")}
                      </th>
                      <th style={{ textAlign: "end", padding: "var(--spacing-xs)" }}>
                        {t("data.col.nullRate")}
                      </th>
                      <th style={{ textAlign: "start", padding: "var(--spacing-xs)" }}>
                        {t("data.col.unique")}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {profiles.map((p) => (
                      <tr key={p.columnId}>
                        <td style={{ padding: "var(--spacing-xs)" }}>{p.name}</td>
                        <td style={{ padding: "var(--spacing-xs)" }}>
                          {p.potentialTypes.join(" | ")}
                        </td>
                        <td style={{ padding: "var(--spacing-xs)", textAlign: "end" }}>
                          {p.nullRate === null ? "—" : `${p.nullRate.toFixed(1)}%`}
                        </td>
                        <td style={{ padding: "var(--spacing-xs)" }}>{p.unique}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : null}
            </div>
          ) : null}

          {tab === "cleaner" ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
              <textarea
                aria-label={t("data.planText")}
                value={planText}
                onChange={(e) => setPlanText(e.target.value)}
                rows={6}
                style={areaStyle}
              />
              <div style={rowStyle}>
                <button type="button" disabled={busy} onClick={doPreviewPlan} style={boxStyle}>
                  {t("text.preview")}
                </button>
                <button
                  type="button"
                  disabled={busy || preview?.ok !== true}
                  onClick={doApplyPlan}
                  style={boxStyle}
                >
                  {t("data.apply")}
                </button>
              </div>
              {preview ? (
                <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                  {preview.ok
                    ? t("data.rowsChanged", { count: preview.rowsChanged ?? 0 })
                    : preview.diagnostics
                        .map((d) => `[${d.severity}] ${d.code} · ${d.message}`)
                        .join("\n")}
                </div>
              ) : null}
              {preview && preview.ok && preview.sampleRows.length > 0 ? (
                <textarea
                  readOnly
                  aria-label={t("text.result")}
                  rows={8}
                  value={preview.sampleRows.map((r) => r.join(" | ")).join("\n")}
                  style={areaStyle}
                />
              ) : null}
            </div>
          ) : null}

          {tab === "convert" ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
              <div style={rowStyle}>
                <span style={{ fontSize: "var(--typography-size-sm)" }}>
                  {t("data.exportScopeLabel", {
                    selected:
                      exportScope === "view" ? (page.totalRows ?? 0) : pageTotal(),
                    total: pageTotal(),
                  })}
                </span>
              </div>
              <div style={rowStyle}>
                <span style={{ fontSize: "var(--typography-size-sm)" }}>
                  {t("data.exportScope")}
                </span>
                {(["all", "view"] as const).map((sc) => (
                  <label key={sc} style={{ fontSize: "var(--typography-size-sm)" }}>
                    <input
                      type="radio"
                      name="exportScope"
                      checked={exportScope === sc}
                      onChange={() => setExportScope(sc)}
                    />
                    {t(sc === "all" ? "data.scope.all" : "data.scope.view")}
                  </label>
                ))}
              </div>
              <div style={rowStyle}>
                <select
                  aria-label={t("data.exportFormat")}
                  value={exportFormat}
                  onChange={(e) => setExportFormat(e.target.value as typeof exportFormat)}
                  style={boxStyle}
                >
                  <option value="csv">csv</option>
                  <option value="tsv">tsv</option>
                  <option value="json">json</option>
                  <option value="jsonl">jsonl</option>
                </select>
                <input
                  aria-label={t("data.exportPath")}
                  value={destPath}
                  onChange={(e) => setDestPath(e.target.value)}
                  placeholder="C:\path\to\out.csv"
                  style={{ ...boxStyle, flex: 1, minWidth: "16em" }}
                />
                <button
                  type="button"
                  disabled={busy || destPath.length === 0}
                  onClick={doExport}
                  style={boxStyle}
                >
                  {t("data.export")}
                </button>
              </div>
            </div>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
