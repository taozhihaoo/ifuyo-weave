import { commands } from "../generated/bindings";

/** M7（上）Batch Engine：JobPlan 管线（§19）+ 同引擎 Preview（§20-§23）+
 * 任务化 Execute（§24-§26）。管线 DTO 直接使用生成类型（镜像
 * weave_batch::StageSpec，tag = "type"）。 */

import type {
  BatchJobResultDto,
  BatchStageDto,
  BatchTextOpDto,
} from "../generated/bindings";

export type { BatchJobResultDto, BatchStageDto, BatchTextOpDto };

export interface BatchItemResultView {
  itemId: string;
  source: string;
  output: string | null;
  status: string;
  error: string | null;
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
  inputBytes: number;
  outputBytes: number;
  preview: boolean;
}

export interface BatchOptions {
  continueOnError: boolean;
  overwriteExisting: boolean;
}

function mapResult(d: BatchJobResultDto): BatchJobResultView {
  return {
    items: (d.items ?? []).map((m) => ({
      itemId: m.itemId,
      source: m.source,
      output: m.output ?? null,
      status: m.status,
      error: m.error ?? null,
      inputBytes: m.inputBytes ?? 0,
      outputBytes: m.outputBytes ?? 0,
    })),
    total: d.total ?? 0,
    succeeded: d.succeeded ?? 0,
    failed: d.failed ?? 0,
    skipped: d.skipped ?? 0,
    cancelled: d.cancelled ?? 0,
    inputBytes: d.inputBytes ?? 0,
    outputBytes: d.outputBytes ?? 0,
    preview: d.preview,
  };
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
    })
    .then((r) => {
      if (r.status !== "ok") {
        throw new Error(`${r.error.code} · ${r.error.message}`);
      }
      return mapResult(r.data);
    });
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
    })
    .then((r) => {
      if (r.status !== "ok") {
        throw new Error(`${r.error.code} · ${r.error.message}`);
      }
      return r.data.jobId;
    });
}

/** 内置管线（§11 Linear Only；UI 只做配置投影，不承载引擎逻辑 §86）。 */
export type BatchPreset = "textTrim" | "textReplace" | "imageResizePng";

export function buildStages(preset: BatchPreset, destDir: string, overwrite: boolean): BatchStageDto[] {
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
          operations: [
            { type: "trimLines" },
            { type: "lowercase" },
          ],
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
