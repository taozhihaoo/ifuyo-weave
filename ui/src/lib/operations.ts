import type { IpcError, JobStatusDto, PlanDto, PlanItemDto } from "../generated/bindings";
import { commands } from "../generated/bindings";
import { pollJob } from "../lib/jobs";

/** Rename 规则（与 Rust RenameRuleDto 对齐，tagged by type）。 */
export type RenameRule =
  | { type: "prefix"; text: string }
  | { type: "suffix"; text: string }
  | { type: "replace"; find: string; replaceWith: string }
  | { type: "regexReplace"; pattern: string; replacement: string }
  | { type: "counter"; start: number; step: number; width: number }
  | { type: "date"; field: string; format: string }
  | { type: "case"; form: string }
  | { type: "extension"; newExtension: string }
  | { type: "template"; template: string };

/** DTO wire 形态（camelCase，与 Rust serde tag 对齐）。 */
export function renameRuleToDto(rule: RenameRule): Record<string, unknown> {
  switch (rule.type) {
    case "prefix":
      return { type: "prefix", text: rule.text };
    case "suffix":
      return { type: "suffix", text: rule.text };
    case "replace":
      return { type: "replace", find: rule.find, replaceWith: rule.replaceWith };
    case "regexReplace":
      return { type: "regexReplace", pattern: rule.pattern, replacement: rule.replacement };
    case "counter":
      return { type: "counter", start: rule.start, step: rule.step, width: rule.width };
    case "date":
      return { type: "date", field: rule.field, format: rule.format };
    case "case":
      return { type: "case", form: rule.form };
    case "extension":
      return { type: "extension", newExtension: rule.newExtension };
    case "template":
      return { type: "template", template: rule.template };
  }
}

export type OrganizerCondition =
  | { type: "any" }
  | { type: "extensionIn"; extensions: string[] }
  | { type: "nameContains"; text: string }
  | { type: "namePattern"; pattern: string }
  | { type: "sizeLargerThan"; bytes: number }
  | { type: "sizeSmallerThan"; bytes: number }
  | { type: "modifiedBefore"; epochMs: number }
  | { type: "modifiedAfter"; epochMs: number };

export function organizerConditionToDto(condition: OrganizerCondition): Record<string, unknown> {
  switch (condition.type) {
    case "any":
      return { type: "any" };
    case "extensionIn":
      return { type: "extensionIn", extensions: condition.extensions };
    case "nameContains":
      return { type: "nameContains", text: condition.text };
    case "namePattern":
      return { type: "namePattern", pattern: condition.pattern };
    case "sizeLargerThan":
      return { type: "sizeLargerThan", bytes: condition.bytes };
    case "sizeSmallerThan":
      return { type: "sizeSmallerThan", bytes: condition.bytes };
    case "modifiedBefore":
      return { type: "modifiedBefore", epochMs: condition.epochMs };
    case "modifiedAfter":
      return { type: "modifiedAfter", epochMs: condition.epochMs };
  }
}

/** 构建 Rename 计划（预览，无副作用）。 */
export function buildRenamePlan(
  inputs: string[],
  rules: RenameRule[],
  template: string | null,
): Promise<{ status: "ok"; data: PlanDto } | { status: "error"; error: IpcError }> {
  const dtos = rules.map(renameRuleToDto);
  return commands.buildRenamePlan(inputs, dtos as never, template);
}

/** 构建 Organizer 计划（预览，无副作用）。 */
export function buildOrganizerPlan(
  root: string,
  rules: { condition: OrganizerCondition; targetFolder: string }[],
): Promise<{ status: "ok"; data: PlanDto } | { status: "error"; error: IpcError }> {
  const dtos = rules.map((r) => ({
    condition: organizerConditionToDto(r.condition),
    targetFolder: r.targetFolder,
  }));
  return commands.buildOrganizerPlan(root, dtos as never);
}

/** 执行已确认的计划；返回 job id 供轮询。 */
export function executePlan(operationId: string): Promise<string> {
  return commands.executePlan(operationId).then((r) => {
    if (r.status === "ok") return r.data.jobId;
    throw new IpcCommandError(r.error);
  });
}

export class IpcCommandError extends Error {
  constructor(public readonly ipcError: IpcError) {
    super(`${ipcError.code}: ${ipcError.message}`);
  }
}

export function undoOperation(operationId: string): Promise<string> {
  return commands.undoOperation(operationId).then((r) => {
    if (r.status === "ok") return r.data.jobId;
    throw new IpcCommandError(r.error);
  });
}

export type { JobStatusDto, PlanDto, PlanItemDto, IpcError };

/** 轮询直到终态；返回停止函数。 */
export function pollUntilDone(
  jobId: string,
  onTick: (status: JobStatusDto) => void,
  onDone: (status: JobStatusDto) => void,
): () => void {
  return pollJob(jobId, (tick) => {
    if (tick.kind === "status") {
      onTick(tick.status);
      if (isTerminal(tick.status)) {
        onDone(tick.status);
      }
    }
  });
}

import { isTerminalState } from "./jobs";
function isTerminal(status: JobStatusDto): boolean {
  return isTerminalState(status);
}
