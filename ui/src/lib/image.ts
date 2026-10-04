import { commands } from "../generated/bindings";

/** M6 Image：打开/检查/预览/执行（Inspector/Resize/Compress/Convert/Strip）。
 *
 * Preview 与 Execute 共用同一变换引擎（§64）；默认写新文件（§73），
 * 覆盖源须显式（§74）且经 TextTransform 管线（TOCTOU/备份/原子/历史）。 */

export interface ImageFactsView {
  format: string;
  width: number;
  height: number;
  hasAlpha: boolean;
  colorModel: string;
  bitDepth: number;
  frameCount: number;
  extensionMismatch: boolean;
  exifPresent: boolean;
  gpsPresent: boolean;
  cameraMake: string | null;
  cameraModel: string | null;
  datetime: string | null;
  orientation: number | null;
  iccPresent: boolean;
}

export interface ImageOpenView {
  path: string;
  previewDataUri: string;
  facts: ImageFactsView;
}

export interface ResizeSpec {
  mode: "fit" | "fill" | "exact" | "scale";
  width: number;
  height: number;
  scalePercent: number;
  preventUpscale: boolean;
  filter: "nearest" | "triangle" | "catmullrom" | "lanczos3";
}

/** 与生成 DTO 兼容的 wire 形态（serde tagged）。 */
export interface ImageOperationWire {
  operation: string;
  targetFormat: string | null;
  quality: number | null;
  alphaBackground: string | null;
  resize: {
    mode: string;
    width: number;
    height: number;
    scalePercent: number;
    preventUpscale: boolean;
    filter: string;
  } | null;
  stripMetadata: boolean;
}

export interface ImageOperationSpec {
  operation: "resize" | "compress" | "convert" | "strip_metadata";
  targetFormat: string | null;
  quality: number | null;
  alphaBackground: "white" | "black" | null;
  resize: ResizeSpec | null;
  stripMetadata: boolean;
}

export interface ImagePreviewView {
  previewDataUri: string;
  outputFormat: string;
  outWidth: number;
  outHeight: number;
  estimatedOutputBytes: number;
  lossy: boolean;
  reEncoded: boolean;
  animationDropped: boolean;
  warnings: string[];
}

type CmdResult<T> = { status: "ok"; data: T } | { status: "error"; error: { code: string; message: string } };

function fail<T>(r: CmdResult<T>): T {
  if (r.status === "ok") return r.data;
  throw new Error(`${r.error.code} · ${r.error.message}`);
}

export function imageOpen(path: string): Promise<ImageOpenView> {
  return commands.imageOpen(path).then((r) => fail(r) as ImageOpenView);
}

export function imagePreview(
  path: string,
  operation: ImageOperationSpec,
): Promise<ImagePreviewView> {
  return commands.imagePreview(path, operation as ImageOperationWire as never).then((r) => fail(r) as ImagePreviewView);
}

/** Execute：返回 PlanDto.operationId（历史/撤销经既有 undo_operation）。 */
export function imageExecute(
  path: string,
  operation: ImageOperationSpec,
  overwriteSource: boolean,
  destination: string | null,
): Promise<string> {
  return commands
    .imageExecute(path, operation, {
      overwriteSource,
      destination,
    })
    .then((r) => {
      if (r.status === "ok") return r.data.operationId;
      throw new Error(`${r.error.code} · ${r.error.message}`);
    });
}

export function imageCancel(token: string): Promise<boolean> {
  return commands.imageCancel(token).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}
