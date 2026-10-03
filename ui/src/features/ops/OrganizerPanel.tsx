import { useState } from "react";
import type { CSSProperties } from "react";
import type { PlanDto } from "../../generated/bindings";
import { useT } from "../../i18n";
import {
  buildOrganizerPlan,
  executePlan,
  pollUntilDone,
  type OrganizerCondition,
} from "../../lib/operations";

interface OrganizerPanelProps {
  onOperationDone: (operationId: string, undoable: boolean) => void;
}

interface RuleRow {
  conditionType: "any" | "extensionIn" | "nameContains" | "sizeLargerThan";
  conditionValue: string;
  targetFolder: string;
}

export function OrganizerPanel({ onOperationDone }: OrganizerPanelProps) {
  const t = useT();
  const [root, setRoot] = useState("");
  const [rules, setRules] = useState<RuleRow[]>([
    { conditionType: "extensionIn", conditionValue: "jpg png webp gif", targetFolder: "image" },
  ]);
  const [plan, setPlan] = useState<PlanDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  const doPreview = (): void => {
    setBusy(true);
    setError(null);
    const dtoRules = rules.map((r) => {
      const condition: OrganizerCondition =
        r.conditionType === "any"
          ? { type: "any" }
          : r.conditionType === "extensionIn"
            ? {
                type: "extensionIn",
                extensions: r.conditionValue.split(/[ ,]+/).filter(Boolean),
              }
            : r.conditionType === "nameContains"
              ? { type: "nameContains", text: r.conditionValue }
              : { type: "sizeLargerThan", bytes: Number(r.conditionValue) || 0 };
      return { condition, targetFolder: r.targetFolder };
    });
    buildOrganizerPlan(root, dtoRules)
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

  return (
    <section
      aria-label={t("organizer.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("organizer.title")}</h2>

      <input
        aria-label={t("organizer.root")}
        value={root}
        onChange={(e) => setRoot(e.target.value)}
        placeholder={t("organizer.rootHint")}
        style={{
          fontFamily: "var(--typography-mono-family)",
          fontSize: "var(--typography-size-sm)",
          padding: "var(--spacing-sm)",
          borderRadius: "var(--radius-sm)",
          border: "1px solid var(--color-border)",
        }}
      />

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
        {rules.map((rule, i) => (
          <div key={i} style={{ display: "flex", gap: "var(--spacing-xs)", alignItems: "center" }}>
            <select
              aria-label={t("organizer.conditionType")}
              value={rule.conditionType}
              onChange={(e) => {
                const v = e.target.value as RuleRow["conditionType"];
                setRules((rs) => rs.map((r, j) => (j === i ? { ...r, conditionType: v } : r)));
              }}
              style={controlStyle}
            >
              <option value="extensionIn">{t("organizer.cond.extensionIn")}</option>
              <option value="nameContains">{t("organizer.cond.nameContains")}</option>
              <option value="sizeLargerThan">{t("organizer.cond.sizeLargerThan")}</option>
              <option value="any">{t("organizer.cond.any")}</option>
            </select>
            <input
              aria-label={t("organizer.conditionValue")}
              value={rule.conditionValue}
              onChange={(e) =>
                setRules((rs) =>
                  rs.map((r, j) => (j === i ? { ...r, conditionValue: e.target.value } : r)),
                )
              }
              style={{ ...controlStyle, width: 160 }}
            />
            <span aria-hidden>→</span>
            <input
              aria-label={t("organizer.targetFolder")}
              value={rule.targetFolder}
              onChange={(e) =>
                setRules((rs) =>
                  rs.map((r, j) => (j === i ? { ...r, targetFolder: e.target.value } : r)),
                )
              }
              style={{ ...controlStyle, width: 140, fontFamily: "var(--typography-mono-family)" }}
            />
            <button
              type="button"
              aria-label={t("organizer.removeRule")}
              onClick={() => setRules((rs) => rs.filter((_, j) => j !== i))}
              style={controlStyle}
            >
              ×
            </button>
          </div>
        ))}
        <button
          type="button"
          onClick={() =>
            setRules((rs) => [
              ...rs,
              { conditionType: "extensionIn", conditionValue: "", targetFolder: "" },
            ])
          }
          style={{ ...controlStyle, alignSelf: "flex-start" }}
        >
          {t("organizer.addRule")}
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

      <div style={{ display: "flex", gap: "var(--spacing-sm)" }}>
        <button type="button" onClick={doPreview} disabled={busy || !root} style={controlStyle}>
          {t("organizer.preview")}
        </button>
        {plan && (plan.readyCount ?? 0) > 0 ? (
          confirming ? (
            <>
              <span role="status">
                {t("organizer.confirmHint", { count: plan.readyCount ?? 0 })}
              </span>
              <button
                type="button"
                onClick={() => {
                  setConfirming(false);
                  doExecute(plan.operationId);
                }}
                disabled={busy}
                style={{
                  ...controlStyle,
                  background: "var(--color-accent)",
                  color: "var(--color-surface)",
                }}
              >
                {t("organizer.execute", { count: plan.readyCount ?? 0 })}
              </button>
              <button type="button" onClick={() => setConfirming(false)} style={controlStyle}>
                {t("rename.confirmNo")}
              </button>
            </>
          ) : (
            <button type="button" onClick={() => setConfirming(true)} style={controlStyle}>
              {t("organizer.execute", { count: plan.readyCount ?? 0 })}
            </button>
          )
        ) : null}
      </div>

      {plan ? (
        <div
          role="table"
          aria-label={t("organizer.previewTable")}
          style={{
            maxHeight: 260,
            overflowY: "auto",
            fontSize: "var(--typography-size-sm)",
            fontFamily: "var(--typography-mono-family)",
          }}
        >
          {plan.items.map((item) => (
            <div
              key={item.itemId}
              role="row"
              style={{ display: "flex", gap: "var(--spacing-sm)", padding: "2px 0" }}
            >
              <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis" }}>
                {item.sourcePath}
              </span>
              <span aria-hidden>→</span>
              <span style={{ flex: 1 }}>{item.targetPath || "—"}</span>
              <span>{t(`rename.status.${item.status}`)}</span>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

const controlStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
};
