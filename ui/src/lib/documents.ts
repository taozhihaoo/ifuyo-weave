import { commands } from "../generated/bindings";
import type {
  DocumentFacts,
  PdfMergePlan,
  PdfOperationResultDto,
} from "../generated/bindings";

/** M8（上）Documents：inspect（只读事实 §12/§64）+ PDF 单文档操作
 * （§18-§28）。UI 只做事实呈现与命令调用，域逻辑在 weave-documents。 */

export type { DocumentFacts, PdfMergePlan, PdfOperationResultDto };

/** Field<T> 三态的可读化（§11：解析失败 ≠ 0）。 */
export function fieldText(
  field: { state: "known"; value: unknown } | { state: string } | null | undefined,
): string {
  if (!field) {
    return "—";
  }
  const wide = field as { state: string; value?: unknown };
  if (wide.state === "known") {
    return String(wide.value);
  }
  if (wide.state === "estimated") {
    return `≈${String(wide.value)}`;
  }
  if (field.state === "unavailable") {
    return "不支持";
  }
  return "未知";
}

type CmdResult<T> = { status: "ok"; data: T } | { status: "error"; error: { code: string; message: string } };

function unwrap<T>(r: CmdResult<T>): T {
  if (r.status !== "ok") {
    throw new Error(`${r.error.code} · ${r.error.message}`);
  }
  return r.data;
}

export async function documentInspect(path: string): Promise<DocumentFacts> {
  return unwrap(await commands.documentInspect(path));
}

export async function pdfMergePreview(inputs: string[]): Promise<PdfMergePlan> {
  return unwrap(await commands.pdfMergePreview(inputs));
}

export interface PdfOperationResult {
  outputs: string[];
  pageCounts: number[];
  warnings: string[];
}

function mapResult(d: PdfOperationResultDto): PdfOperationResult {
  return {
    outputs: d.outputs ?? [],
    pageCounts: (d.pageCounts ?? []).map((c) => c ?? 0),
    warnings: d.warnings ?? [],
  };
}

export async function pdfMergeExecute(
  inputs: string[],
  destinationDir: string,
): Promise<PdfOperationResult> {
  return mapResult(unwrap(await commands.pdfMergeExecute(inputs, destinationDir)));
}

export async function pdfExtractExecute(
  input: string,
  ranges: string,
  destinationDir: string,
): Promise<PdfOperationResult> {
  return mapResult(unwrap(await commands.pdfExtractExecute(input, ranges, destinationDir)));
}

export async function pdfRotateExecute(
  input: string,
  degrees: number,
  ranges: string,
  destinationDir: string,
): Promise<PdfOperationResult> {
  return mapResult(unwrap(await commands.pdfRotateExecute(input, degrees, ranges, destinationDir)));
}

export async function pdfSplitEveryNExecute(
  input: string,
  n: number,
  destinationDir: string,
): Promise<PdfOperationResult> {
  return mapResult(unwrap(await commands.pdfSplitEveryNExecute(input, n, destinationDir)));
}
