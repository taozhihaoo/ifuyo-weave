import { commands } from "../generated/bindings";

/** M7 Batch Engine：JobPlan 管线（§19）+ 同引擎 Preview（§20-§23）+
 * 任务化 Execute（§24-§26）+ Pause/Resume/Retry/JobList（下 §90/§126/§142）。
 * 管线 DTO 直接使用生成类型（镜像 weave_batch::StageSpec，tag = "type"）。 */

import type {
  BatchJobResultDto,
  BatchJobListItemDto,
  BatchStageDto,
  BatchTextOpDto,
} from "../generated/bindings";

export type { BatchJobResultDto, BatchStageDto, BatchTextOpDto, BatchJobListItemDto };

export interface BatchItemResultView {
  itemId: string;
  source: string;
  output: string | null;
  status: string;
  error: string | null;
  retryable: boolean;
  inputBytes: number;
  outputBytes: number;
}

export interface BatchJobResultView {
  items: BatchItemResultView[];
  total: number;
  succeeded: number;
  failed: number;
  skipped: number;
  cancelled: number;
  pending: number;
  inputBytes: number;
  outputBytes: number;
  preview: boolean;
  operationId: string | null;
  resumeConflicts: string[];
}

export interface BatchJobListItemView {
  jobId: string;
  state: string;
  totalItems: number;
  settledItems: number;
  pendingItems: number;
  conflicts: string[];
  createdMs: number | null;
  destinationDir: string | null;
}

export interface BatchOptions {
  continueOnError: boolean;
  overwriteExisting: boolean;
  workers: number;
}

function mapResult(d: BatchJobResultDto): BatchJobResultView {
  return {
    items: (d.items ?? []).map((m) => ({
      itemId: m.itemId,
      source: m.source,
      output: m.output ?? null,
      status: m.status,
      error: m.error ?? null,
      retryable: m.retryable ?? true,
      inputBytes: m.inputBytes ?? 0,
      outputBytes: m.outputBytes ?? 0,
    })),
    total: d.total ?? 0,
    succeeded: d.succeeded ?? 0,
    failed: d.failed ?? 0,
    skipped: d.skipped ?? 0,
    cancelled: d.cancelled ?? 0,
    pending: d.pending ?? 0,
    inputBytes: d.inputBytes ?? 0,
    outputBytes: d.outputBytes ?? 0,
    preview: d.preview,
    operationId: d.operationId ?? null,
    resumeConflicts: d.resumeConflicts ?? [],
  };
}

function invoke<T>(r: { status: "ok"; data: T } | { status: "error"; error: { code: string; message: string } }): T {
  if (r.status !== "ok") {
    throw new Error(`${r.error.code} · ${r.error.message}`);
  }
  return r.data;
}

/** Preview（§21：无副作用模拟，不落盘）。 */
export function batchPreview(
  inputs: string[],
  stages: BatchStageDto[],
  destinationDir: string,
  options: BatchOptions,
): Promise<BatchJobResultView> {
  return commands
    .batchPreview(inputs, stages, destinationDir, {
      continueOnError: options.continueOnError,
      overwriteExisting: options.overwriteExisting,
      workers: options.workers,
    })
    .then((r) => mapResult(invoke(r)));
}

/** Execute（任务化；返回 job_id，经 get_job 轮询、cancel_job 取消）。 */
export function batchExecute(
  inputs: string[],
  stages: BatchStageDto[],
  destinationDir: string,
  options: BatchOptions,
): Promise<string> {
  return commands
    .batchExecute(inputs, stages, destinationDir, {
      continueOnError: options.continueOnError,
      overwriteExisting: options.overwriteExisting,
      workers: options.workers,
    })
    .then((r) => invoke(r).jobId);
}

/** 下 §90 Pause：协作暂停（安全点后未开始条目 = pending）。 */
export function batchPause(jobId: string): Promise<string> {
  return commands.batchPause(jobId).then((r) => invoke(r));
}

/** 下 §126 Resume：journal 驱动恢复；返回新 run 的 job_id。 */
export function batchResume(jobId: string): Promise<string> {
  return commands.batchResume(jobId).then((r) => invoke(r).jobId);
}

/** 下 §142 Retry Failed：失败子集 + 全新快照；返回新 job_id。 */
export function batchRetryFailed(jobId: string): Promise<string> {
  return commands.batchRetryFailed(jobId).then((r) => invoke(r).jobId);
}

/** 下 §183 Job 清单（跨重启状态）。 */
export function batchJobsList(): Promise<BatchJobListItemView[]> {
  return commands.batchJobsList().then((r) =>
    invoke(r).map((j) => ({
      jobId: j.jobId,
      state: j.state,
      totalItems: j.totalItems ?? 0,
      settledItems: j.settledItems ?? 0,
      pendingItems: j.pendingItems ?? 0,
      conflicts: j.conflicts ?? [],
      createdMs: j.createdMs ?? null,
      destinationDir: j.destinationDir ?? null,
    })),
  );
}

/** 内置管线（§11 Linear Only；UI 只做配置投影，不承载引擎逻辑 §86）。 */
export type BatchPreset = "textTrim" | "textReplace" | "imageResizePng";

export function buildStages(
  preset: BatchPreset,
  destDir: string,
  overwrite: boolean,
): BatchStageDto[] {
  const exportStage: BatchStageDto = {
    type: "export",
    destination_dir: destDir,
    overwrite,
  };
  switch (preset) {
    case "textTrim":
      return [
        { type: "source" },
        {
          type: "filter",
          extensions_in: ["txt", "md", "csv", "json"],
          max_bytes: null,
        },
        {
          type: "textTransform",
          operations: [{ type: "trimLines" }],
        },
        { type: "encode", format: "txt", quality: null },
        exportStage,
      ];
    case "textReplace":
      return [
        { type: "source" },
        {
          type: "textTransform",
          operations: [{ type: "trimLines" }, { type: "lowercase" }],
        },
        { type: "encode", format: "txt", quality: null },
        exportStage,
      ];
    case "imageResizePng":
      return [
        { type: "source" },
        {
          type: "filter",
          extensions_in: ["png", "jpeg", "jpg", "webp", "bmp", "tiff"],
          max_bytes: null,
        },
        {
          type: "imageResize",
          width: 1920,
          height: 1080,
          mode: "fit",
          prevent_upscale: true,
        },
        { type: "encode", format: "png", quality: null },
        exportStage,
      ];
  }
}
