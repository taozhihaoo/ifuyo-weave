import { commands } from "../generated/bindings";


/** M10（上）Workflow wrapper：Workflow 定义经 JSON string 过 IPC
 * （§111 typed/bounded；§102 JSON 只作 Data）。UI 只管理编辑态。 */

export interface WorkflowStep {
  id: string;
  type: "input" | "filter" | "tool" | "export";
  name: string;
  kind: string;
  config: Record<string, unknown>;
  enabled: boolean;
}

export interface WorkflowModel {
  schemaVersion: number;
  id: string;
  name: string;
  description: string;
  steps: WorkflowStep[];
  parameters: unknown[];
}

export interface WorkflowListItem {
  id: string;
  name: string;
  description: string;
  schemaVersion: number;
  stepCount: number;
}

export interface ValidationIssue {
  severity: string;
  code: string;
  message: string;
  stepId: string | null;
}

export interface PreviewSummary {
  inputCount: number;
  succeeded: number;
  failed: number;
  skipped: number;
  cancelled: number;
  outputBytes: number;
  issues: string[];
}

export interface WorkflowRunHandle {
  runId: string;
  jobId: string;
}

function unwrap<T>(r: { status: string; data?: T; error?: { code: string; message: string } }): T {
  if (r.status !== "ok") {
    throw new Error(`${r.error?.code} · ${r.error?.message}`);
  }
  return r.data as T;
}

function parseWorkflow(json: string): WorkflowModel {
  return JSON.parse(json) as WorkflowModel;
}

export function workflowValidate(
  workflowJson: string,
): Promise<ValidationIssue[]> {
  return commands.workflowValidate(workflowJson).then((r) =>
    unwrap(r).map((i) => ({
      severity: i.severity,
      code: i.code,
      message: i.message,
      stepId: i.stepId ?? null,
    })),
  );
}

export function workflowSave(workflow: WorkflowModel): Promise<string> {
  return commands.workflowSave(JSON.stringify(workflow)).then(unwrap);
}

export function workflowList(): Promise<WorkflowListItem[]> {
  return commands.workflowList().then((r) =>
    unwrap(r).map((w) => ({
      id: w.id,
      name: w.name,
      description: w.description,
      schemaVersion: w.schemaVersion ?? 1,
      stepCount: w.stepCount ?? 0,
    })),
  );
}

export function workflowGet(id: string): Promise<WorkflowModel> {
  return commands.workflowGet(id).then((r) => parseWorkflow(unwrap(r)));
}

export function workflowDelete(id: string): Promise<boolean> {
  return commands.workflowDelete(id).then(unwrap);
}

export function workflowDuplicate(
  id: string,
  newId: string,
  newName: string,
): Promise<string> {
  return commands.workflowDuplicate(id, newId, newName).then(unwrap);
}

export function workflowImportJson(json: string): Promise<string> {
  return commands.workflowImportJson(json).then(unwrap);
}

export function workflowExportJson(id: string): Promise<string> {
  return commands.workflowExportJson(id).then(unwrap);
}

export function workflowPreview(
  workflowJson: string,
  inputs: string[],
): Promise<PreviewSummary> {
  return commands.workflowPreview(workflowJson, inputs).then((r) => {
    const d = unwrap(r);
    return {
      inputCount: d.inputCount ?? 0,
      succeeded: d.succeeded ?? 0,
      failed: d.failed ?? 0,
      skipped: d.skipped ?? 0,
      cancelled: d.cancelled ?? 0,
      outputBytes: d.outputBytes ?? 0,
      issues: d.issues ?? [],
    };
  });
}

export function workflowRun(
  workflowJson: string,
  inputs: string[],
): Promise<WorkflowRunHandle> {
  return commands.workflowRun(workflowJson, inputs).then(unwrap);
}
