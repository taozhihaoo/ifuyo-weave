import { commands } from "../generated/bindings";

/** M6（下）批量：同操作同选项 × 多文件（§79，非通用 pipeline）。 */

export interface BatchFileResultView {
  input: string;
  output: string | null;
  status: string;
  error: string | null;
  inputBytes: number;
  outputBytes: number;
}

export interface BatchResultView {
  results: BatchFileResultView[];
  total: number;
  succeeded: number;
  failed: number;
  cancelled: number;
  inputBytes: number;
  outputBytes: number;
}

export interface BatchOptions {
  destinationDir: string;
  targetFormat: "png" | "jpeg" | "webp" | "bmp" | "tiff";
  quality: number | null;
  overwriteExisting: boolean;
}

export function imageBatchExecute(
  inputs: string[],
  options: BatchOptions,
): Promise<BatchResultView> {
  return commands
    .imageBatchExecute(inputs, {
      destinationDir: options.destinationDir,
      targetFormat: options.targetFormat,
      quality: options.quality,
      overwriteExisting: options.overwriteExisting,
    })
    .then((r) => {
      if (r.status !== "ok") {
        throw new Error(`${r.error.code} · ${r.error.message}`);
      }
      const d = r.data;
      return {
        results: (d.results ?? []).map((m) => ({
          input: m.input,
          output: m.output ?? null,
          status: m.status,
          error: m.error ?? null,
          inputBytes: m.inputBytes ?? 0,
          outputBytes: m.outputBytes ?? 0,
        })),
        total: d.total ?? 0,
        succeeded: d.succeeded ?? 0,
        failed: d.failed ?? 0,
        cancelled: d.cancelled ?? 0,
        inputBytes: d.inputBytes ?? 0,
        outputBytes: d.outputBytes ?? 0,
      };
    });
}
