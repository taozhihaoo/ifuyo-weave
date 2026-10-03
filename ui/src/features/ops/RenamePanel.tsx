import { useMemo, useState, type CSSProperties } from "react";
import type { PlanDto } from "../../generated/bindings";
import { useT } from "../../i18n";
import { buildRenamePlan, executePlan, pollUntilDone, type RenameRule } from "../../lib/operations";

interface RenamePanelProps {
  onOperationDone: (operationId: string, undoable: boolean) => void;
}

/** 规则块编辑：M2 §60 Rule Builder（结构化数据 + 固定管线顺序）。 */
function ruleSummary(rule: RenameRule): string {
  switch (rule.type) {
    case "prefix":
      return `+ "${rule.text}"`;
    case "suffix":
      return `+ end "${rule.text}"`;
    case "replace":
      return `"${rule.find}" → "${rule.replaceWith}"`;
    case "regexReplace":
      return `re "${rule.pattern}" → "${rule.replacement}"`;
    case "counter":
      return `#${rule.start}+${rule.step} w${rule.width}`;
    case "date":
      return `${rule.field} ${rule.format}`;
    case "case":
      return rule.form;
    case "extension":
      return `.${rule.newExtension}`;
    case "template":
      return rule.template;
  }
}

export function RenamePanel({ onOperationDone }: RenamePanelProps) {
  const t = useT();
  const [pathsText, setPathsText] = useState("");
  const [rules, setRules] = useState<RenameRule[]>([]);
  const [template, setTemplate] = useState("");
  const [plan, setPlan] = useState<PlanDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  const paths = useMemo(
    () =>
      pathsText
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l.length > 0),
    [pathsText],
  );

  const doPreview = (): void => {
    setBusy(true);
    setError(null);
    setConfirmed(false);
    buildRenamePlan(paths, rules, template || null)
      .then((result) => {
        setBusy(false);
        if (result.status === "ok") {
          setPlan(result.data);
        } else {
          setError(`${result.error.code} · ${result.error.message}`);
        }
      })
      .catch(() => setBusy(false));
  };

  const doExecute = (operationId: string): void => {
    setBusy(true);
    executePlan(operationId)
      .then((jobId) => {
        pollUntilDone(
          jobId,
          () => {},
          (status) => {
            setBusy(false);
            const report = status.plan;
            if (report) {
              onOperationDone(report.operationId, report.undoable);
            }
          },
        );
      })
      .catch(() => setBusy(false));
  };

  const visible = (plan?.items ?? []).filter(() => true);

  return (
    <section
      aria-label={t("rename.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("rename.title")}</h2>

      <textarea
        aria-label={t("rename.inputPaths")}
        value={pathsText}
        onChange={(e) => setPathsText(e.target.value)}
        rows={4}
        placeholder={t("rename.inputPathsHint")}
        style={{
          fontFamily: "var(--typography-mono-family)",
          fontSize: "var(--typography-size-sm)",
          padding: "var(--spacing-sm)",
          borderRadius: "var(--radius-sm)",
          border: "1px solid var(--color-border)",
        }}
      />

      <div
        style={{
          display: "flex",
          gap: "var(--spacing-sm)",
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <select
          aria-label={t("rename.addRule")}
          value=""
          onChange={(e) => {
            const type = e.target.value;
            if (!type) return;
            const defaults: Record<string, RenameRule> = {
              prefix: { type: "prefix", text: "" },
              suffix: { type: "suffix", text: "" },
              replace: { type: "replace", find: "", replaceWith: "" },
              regexReplace: { type: "regexReplace", pattern: "", replacement: "" },
              counter: { type: "counter", start: 1, step: 1, width: 3 },
              date: { type: "date", field: "modified", format: "YYYY-MM-DD" },
              case: { type: "case", form: "lower" },
              extension: { type: "extension", newExtension: "" },
              template: { type: "template", template: "" },
            };
            const rule = defaults[type];
            if (rule) {
              setRules((r) => [...r, rule]);
            }
            e.target.value = "";
          }}
          style={controlStyle}
        >
          <option value="">{t("rename.addRule")}</option>
          <option value="prefix">Prefix</option>
          <option value="suffix">Suffix</option>
          <option value="replace">Replace</option>
          <option value="regexReplace">Regex</option>
          <option value="counter">Counter</option>
          <option value="date">Date</option>
          <option value="case">Case</option>
          <option value="extension">Extension</option>
          <option value="template">Template</option>
        </select>
        <input
          aria-label={t("rename.template")}
          value={template}
          onChange={(e) => setTemplate(e.target.value)}
          placeholder={t("rename.templateHint")}
          style={{
            ...controlStyle,
            flex: 1,
            minWidth: 200,
            fontFamily: "var(--typography-mono-family)",
          }}
        />
        <button
          type="button"
          onClick={doPreview}
          disabled={busy || paths.length === 0}
          style={buttonStyle}
        >
          {t("rename.preview")}
        </button>
      </div>

      {rules.length > 0 ? (
        <ul
          aria-label={t("rename.rules")}
          style={{
            margin: 0,
            paddingLeft: "var(--spacing-lg)",
            fontSize: "var(--typography-size-sm)",
          }}
        >
          {rules.map((rule, i) => (
            <li key={i} style={{ display: "flex", gap: "var(--spacing-sm)", alignItems: "center" }}>
              <span style={{ fontFamily: "var(--typography-mono-family)" }}>
                {ruleSummary(rule)}
              </span>
              <button
                type="button"
                aria-label={t("rename.removeRule")}
                onClick={() => setRules((r) => r.filter((_, j) => j !== i))}
                style={{ ...buttonStyle, padding: "0 var(--spacing-xs)" }}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      {error ? (
        <div
          role="alert"
          style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
        >
          {error}
        </div>
      ) : null}

      {plan ? (
        <>
          <div
            role="status"
            style={{
              display: "flex",
              gap: "var(--spacing-md)",
              fontSize: "var(--typography-size-sm)",
              flexWrap: "wrap",
            }}
          >
            <span>
              {t("rename.counts.ready")}: {plan.readyCount ?? 0}
            </span>
            <span style={{ color: "var(--color-danger)" }}>
              {t("rename.counts.conflict")}: {plan.conflictCount ?? 0}
            </span>
            <span style={{ color: "var(--color-text-muted)" }}>
              {t("rename.counts.noop")}: {plan.noopCount ?? 0}
            </span>
            <span style={{ color: "var(--color-danger)" }}>
              {t("rename.counts.invalid")}: {plan.invalidCount ?? 0}
            </span>
          </div>

          <div
            role="table"
            aria-label={t("rename.previewTable")}
            style={{
              maxHeight: 260,
              overflowY: "auto",
              border: "1px solid var(--color-border)",
              borderRadius: "var(--radius-sm)",
            }}
          >
            {visible.map((item) => (
              <div
                key={item.itemId}
                role="row"
                style={{
                  display: "flex",
                  gap: "var(--spacing-sm)",
                  padding: "var(--spacing-xs) var(--spacing-sm)",
                  borderBottom: "1px solid var(--color-border)",
                  fontSize: "var(--typography-size-sm)",
                  fontFamily: "var(--typography-mono-family)",
                }}
              >
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis" }}>
                  {item.sourcePath}
                </span>
                <span aria-hidden>→</span>
                <span
                  style={{
                    flex: 1,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    color:
                      item.status === "ready" ? "var(--color-success)" : "var(--color-text-muted)",
                  }}
                >
                  {item.targetPath || "—"}
                </span>
                <span style={{ minWidth: 70 }}>{t(`rename.status.${item.status}`)}</span>
              </div>
            ))}
          </div>

          <div style={{ display: "flex", gap: "var(--spacing-sm)", alignItems: "center" }}>
            {!confirmed ? (
              <button
                type="button"
                disabled={plan.readyCount === 0}
                onClick={() => setConfirmed(true)}
                style={{
                  ...buttonStyle,
                  background: "var(--color-accent)",
                  color: "var(--color-accentText, var(--color-surface))",
                }}
              >
                {t("rename.execute", { count: plan.readyCount ?? 0 })}
              </button>
            ) : (
              <>
                <span role="status">
                  {t("rename.confirmHint", { count: plan.readyCount ?? 0 })}
                </span>
                <button
                  type="button"
                  onClick={() => {
                    setConfirmed(false);
                    doExecute(plan.operationId);
                  }}
                  disabled={busy}
                  style={{
                    ...buttonStyle,
                    background: "var(--color-accent)",
                    color: "var(--color-surface)",
                  }}
                >
                  {t("rename.confirmYes", { count: plan.readyCount ?? 0 })}
                </button>
                <button type="button" onClick={() => setConfirmed(false)} style={buttonStyle}>
                  {t("rename.confirmNo")}
                </button>
              </>
            )}
          </div>
        </>
      ) : null}
    </section>
  );
}

const buttonStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
};

const controlStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
};
