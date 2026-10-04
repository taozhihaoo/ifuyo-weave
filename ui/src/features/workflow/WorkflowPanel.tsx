import { useState, type CSSProperties } from "react";
import { useT } from "../../i18n";
import { commands } from "../../generated/bindings";
import { isTerminalState, pollJob } from "../../lib/jobs";
import {
  workflowDelete,
  workflowDuplicate,
  workflowGet,
  workflowList,
  workflowPreview,
  workflowRun,
  workflowSave,
  workflowValidate,
  type PreviewSummary,
  type ValidationIssue,
  type WorkflowListItem,
  type WorkflowModel,
  type WorkflowStep,
} from "../../lib/workflow";

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

/** M10（上）Workflow Builder：垂直 step list（§52 线性，无节点画布）。
 * UI 只管理编辑态（§113/§115）；定义/校验/编译在 weave-workflow。 */

let idCounter = 0;
function newId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${Date.now()}-${idCounter}`;
}

function emptyWorkflow(): WorkflowModel {
  return {
    schemaVersion: 1,
    id: newId("wf"),
    name: "Untitled Workflow",
    description: "",
    steps: [
      { id: newId("input"), type: "input", name: "Input", kind: "files", config: {}, enabled: true },
      { id: newId("export"), type: "export", name: "Export", kind: "export.files", config: { destinationDir: "" }, enabled: true },
    ],
    parameters: [],
  };
}

const TOOL_PALETTE: { kind: string; type: WorkflowStep["type"]; labelKey: string; config: Record<string, unknown> }[] = [
  { kind: "filter.extension", type: "filter", labelKey: "workflow.palette.extFilter", config: { extensionsIn: ["pdf"] } },
  { kind: "document.inspect", type: "tool", labelKey: "workflow.palette.docInspect", config: {} },
  { kind: "pdf.rotate", type: "tool", labelKey: "workflow.palette.pdfRotate", config: { degrees: 90, pages: "" } },
  { kind: "image.resize", type: "tool", labelKey: "workflow.palette.imgResize", config: { width: 1920, height: 1080, mode: "fit", preventUpscale: true } },
  { kind: "image.encode", type: "tool", labelKey: "workflow.palette.imgEncode", config: { format: "png", quality: null } },
];

export function WorkflowPanel({ onOperationDone }: { onOperationDone: () => void }) {
  const t = useT();
  const [wf, setWf] = useState<WorkflowModel>(emptyWorkflow);
  const [library, setLibrary] = useState<WorkflowListItem[]>([]);
  const [inputs, setInputs] = useState("");
  const [preview, setPreview] = useState<PreviewSummary | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[] | null>(null);
  const [selectedStep, setSelectedStep] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshLibrary = (): void => {
    workflowList().then(setLibrary).catch(() => setLibrary([]));
  };
  // 首次挂载拉取列表（惰性；不阻塞渲染）
  useState(() => {
    refreshLibrary();
  });

  const wfJson = JSON.stringify(wf);
  const inputsList = inputs
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);

  const doValidate = async (): Promise<void> => {
    setError(null);
    try {
      setIssues(await workflowValidate(wfJson));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const doPreview = async (): Promise<void> => {
    setError(null);
    setIssues(null);
    setPreview(null);
    try {
      setPreview(await workflowPreview(wfJson, inputsList));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const doSave = async (): Promise<void> => {
    setError(null);
    try {
      const issues = await workflowValidate(wfJson);
      setIssues(issues);
      if (issues.some((i) => i.severity === "error")) {
        return; // §29 ERROR 阻止保存
      }
      await workflowSave(wf);
      refreshLibrary();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const doRun = (): void => {
    setError(null);
    setRunning(true);
    workflowRun(wfJson, inputsList)
      .then((h) => {
        const stop = pollJob(h.jobId, (tick) => {
          if (tick.kind === "status" && isTerminalState(tick.status)) {
            stop();
            setRunning(false);
            if (tick.status.state === "failed") {
              setError(
                `${tick.status.error?.code ?? "job.failed"} · ${tick.status.error?.message ?? "run failed"}`,
              );
            }
            onOperationDone();
          } else if (tick.kind === "failed") {
            stop();
            setRunning(false);
            setError(`${tick.error.code} · ${tick.error.message}`);
          }
        });
      })
      .catch((e: unknown) => {
        setRunning(false);
        setError(e instanceof Error ? e.message : String(e));
      });
  };

  const doOpen = async (id: string): Promise<void> => {
    setError(null);
    try {
      setWf(await workflowGet(id));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const doDelete = async (id: string): Promise<void> => {
    setError(null);
    try {
      await workflowDelete(id); // §87：只删定义
      refreshLibrary();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const doDuplicate = async (item: WorkflowListItem): Promise<void> => {
    setError(null);
    try {
      await workflowDuplicate(item.id, newId("wf"), `${item.name} (Copy)`);
      refreshLibrary();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const addStep = (palette: (typeof TOOL_PALETTE)[number]): void => {
    const insertAt = wf.steps.length - 1; // Export 前
    const step: WorkflowStep = {
      id: newId(palette.type),
      type: palette.type,
      name: t(palette.labelKey),
      kind: palette.kind,
      config: structuredClone(palette.config),
      enabled: true,
    };
    const steps = [...wf.steps];
    steps.splice(insertAt, 0, step);
    setWf({ ...wf, steps });
    setSelectedStep(step.id);
  };

  const moveStep = (index: number, delta: number): void => {
    const target = index + delta;
    if (index === 0 || target <= 0 || target >= wf.steps.length - 1) {
      return; // Input 首 / Export 末不可动（§6 线性约束）
    }
    const steps = [...wf.steps];
    const removed = steps.splice(index, 1)[0];
    if (removed !== undefined) {
      steps.splice(target, 0, removed);
    }
    setWf({ ...wf, steps });
  };

  const updateStep = (id: string, patch: Partial<WorkflowStep>): void => {
    setWf({
      ...wf,
      steps: wf.steps.map((s) => (s.id === id ? { ...s, ...patch } : s)),
    });
  };

  return (
    <section
      aria-label={t("workflow.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h3 style={{ margin: 0, fontSize: "var(--typography-size-lg)" }}>{t("workflow.title")}</h3>

      {/* Library（§77） */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
        <div style={rowStyle}>
          <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("workflow.library")}</strong>
          <button type="button" onClick={() => refreshLibrary()} style={boxStyle}>
            {t("workflow.refresh")}
          </button>
          <button type="button" onClick={() => setWf(emptyWorkflow())} style={boxStyle}>
            {t("workflow.new")}
          </button>
        </div>
        {library.map((item) => (
          <div key={item.id} style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
            <div style={rowStyle}>
              <strong>{item.name}</strong>
              <span style={{ color: "var(--color-text-muted)" }}>{item.description}</span>
              <span style={{ color: "var(--color-text-muted)" }}>
                {t("workflow.stepCount", { count: item.stepCount })}
              </span>
              <button type="button" onClick={() => void doOpen(item.id)} style={boxStyle}>
                {t("workflow.open")}
              </button>
              <button type="button" onClick={() => void doDuplicate(item)} style={boxStyle}>
                {t("workflow.duplicate")}
              </button>
              <button type="button" onClick={() => void doDelete(item.id)} style={boxStyle}>
                {t("workflow.delete")}
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* Builder（§51/§52） */}
      <div style={rowStyle}>
        <input
          aria-label={t("workflow.name")}
          value={wf.name}
          onChange={(e) => setWf({ ...wf, name: e.target.value })}
          style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
        />
        <input
          aria-label={t("workflow.inputs")}
          value={inputs}
          onChange={(e) => setInputs(e.target.value)}
          placeholder={t("workflow.inputsHint")}
          style={{ ...boxStyle, flex: 2, minWidth: "18em", fontFamily: "var(--typography-mono-family)" }}
        />
      </div>

      <div style={rowStyle}>
        <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          {t("workflow.paletteLabel")}
        </span>
        {TOOL_PALETTE.map((p) => (
          <button key={p.kind} type="button" onClick={() => addStep(p)} style={boxStyle}>
            {t(p.labelKey)}
          </button>
        ))}
      </div>

      <ol style={{ margin: 0, paddingLeft: "1.2em", display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
        {wf.steps.map((step, i) => {
          const isFirst = i === 0;
          const isLast = i === wf.steps.length - 1;
          return (
            <li key={step.id} style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
              <div style={rowStyle}>
                <strong>
                  {i + 1}. {step.type === "input" ? t("workflow.step.input") : step.type === "export" ? t("workflow.step.export") : step.kind}
                </strong>
                {!step.enabled ? (
                  <span style={{ color: "var(--color-text-muted)" }}>({t("workflow.disabled")})</span>
                ) : null}
                {!isFirst ? (
                  <button type="button" aria-label={t("workflow.moveUp")} onClick={() => moveStep(i, -1)} style={boxStyle}>↑</button>
                ) : null}
                {!isLast ? (
                  <button type="button" aria-label={t("workflow.moveDown")} onClick={() => moveStep(i, 1)} style={boxStyle}>↓</button>
                ) : null}
                {step.type === "tool" || step.type === "filter" ? (
                  <button
                    type="button"
                    onClick={() => updateStep(step.id, { enabled: !step.enabled })}
                    style={boxStyle}
                  >
                    {step.enabled ? t("workflow.disable") : t("workflow.enable")}
                  </button>
                ) : null}
                {step.type === "tool" || step.type === "filter" ? (
                  <button
                    type="button"
                    onClick={() => setWf({ ...wf, steps: wf.steps.filter((s) => s.id !== step.id) })}
                    style={boxStyle}
                  >
                    {t("workflow.stepDelete")}
                  </button>
                ) : null}
                <button
                  type="button"
                  onClick={() => setSelectedStep(selectedStep === step.id ? null : step.id)}
                  style={boxStyle}
                >
                  {t("workflow.configure")}
                </button>
              </div>
              {selectedStep === step.id ? (
                <div style={{ marginTop: "var(--spacing-xs)" }}>
                  {step.type === "filter" && step.kind === "filter.extension" ? (
                    <input
                      aria-label={t("workflow.config.extensions")}
                      value={(step.config.extensionsIn as string[] | undefined)?.join(",") ?? ""}
                      onChange={(e) =>
                        updateStep(step.id, {
                          config: {
                            extensionsIn: e.target.value
                              .split(",")
                              .map((s) => s.trim())
                              .filter(Boolean),
                          },
                        })
                      }
                      style={{ ...boxStyle, width: "20em", fontFamily: "var(--typography-mono-family)" }}
                    />
                  ) : null}
                  {step.kind === "pdf.rotate" ? (
                    <input
                      aria-label={t("workflow.config.degrees")}
                      value={String(typeof step.config.degrees === "number" ? step.config.degrees : 90)}
                      onChange={(e) => updateStep(step.id, { config: { ...step.config, degrees: Number(e.target.value) } })}
                      style={{ ...boxStyle, width: "6em" }}
                    />
                  ) : null}
                  {step.type === "export" ? (
                    <input
                      aria-label={t("documents.destDir")}
                      value={typeof step.config.destinationDir === "string" ? step.config.destinationDir : ""}
                      onChange={(e) => updateStep(step.id, { config: { ...step.config, destinationDir: e.target.value } })}
                      style={{ ...boxStyle, width: "22em", fontFamily: "var(--typography-mono-family)" }}
                    />
                  ) : null}
                </div>
              ) : null}
            </li>
          );
        })}
      </ol>

      <div style={rowStyle}>
        <button type="button" onClick={() => void doValidate()} style={boxStyle}>
          {t("workflow.validate")}
        </button>
        <button type="button" onClick={() => void doPreview()} style={boxStyle}>
          {t("workflow.preview")}
        </button>
        <button type="button" onClick={() => void doSave()} style={boxStyle}>
          {t("workflow.save")}
        </button>
        <button type="button" disabled={running || inputsList.length === 0} onClick={doRun} style={boxStyle}>
          {t("workflow.run")}
        </button>
        {running ? <span role="status">{t("workflow.running")}</span> : null}
      </div>

      <ErrorLine error={error} />

      {issues ? (
        <div role={issues.some((i) => i.severity === "error") ? "alert" : "status"} style={{ fontSize: "var(--typography-size-sm)" }}>
          {issues.length === 0 ? <div>{t("workflow.noIssues")}</div> : null}
          {issues.map((i, idx) => (
            <div
              key={idx}
              style={{ color: i.severity === "error" ? "var(--color-danger)" : "var(--color-text-muted)" }}
            >
              [{i.severity}] {i.code}: {i.message}
            </div>
          ))}
        </div>
      ) : null}

      {preview ? (
        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
          <strong>{t("workflow.previewResult")}</strong>
          <div>
            {t("workflow.previewSummary", {
              input: preview.inputCount,
              success: preview.succeeded,
              failed: preview.failed,
              skipped: preview.skipped,
            })}
          </div>
          {preview.issues.slice(0, 10).map((issue, i) => (
            <div key={i} style={{ color: "var(--color-danger)" }}>
              {issue}
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function ErrorLine({ error }: { error: string | null }) {
  if (!error) {
    return null;
  }
  return (
    <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
      {error}
    </div>
  );
}

// 避免未使用导入告警：commands 在后续扩展（导出/导入 JSON）使用
void commands;
