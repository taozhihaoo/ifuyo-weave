import { commands } from "../generated/bindings";
import type {
  HashAlgorithmInfoDto,
  RegexCapabilityDto,
  RegexMatchDto,
  TimestampConversionDto,
  UrlPartDto,
  UuidInfoDto,
} from "../generated/bindings";

/** M9（上）Utilities wrapper：UI 只做 Input/Presentation/Copy（§18），
 * 全部语义经 IPC 落在 weave-utilities。 */

export type {
  HashAlgorithmInfoDto,
  RegexCapabilityDto,
  RegexMatchDto,
  TimestampConversionDto,
  UrlPartDto,
  UuidInfoDto,
};

export interface TextHashResult {
  algorithm: string;
  bytesProcessed: number;
  digestHex: string;
  warnings: { code: string; message: string }[];
}

export interface RegexReplaceResult {
  output: string;
  count: number;
}

export interface ColorResult {
  hex: string;
  r: number;
  g: number;
  b: number;
  a: number;
  h: number;
  sHsl: number;
  l: number;
  sHsv: number;
  v: number;
  contrastOnWhite: number;
  contrastOnBlack: number;
}

export type RegexFlags = {
  caseInsensitive: boolean;
  multiLine: boolean;
  dotMatchesNewline: boolean;
  ignoreWhitespace: boolean;
};

function unwrap<T>(r: { status: string; data?: T; error?: { code: string; message: string } }): T {
  if (r.status !== "ok") {
    throw new Error(`${r.error?.code} · ${r.error?.message}`);
  }
  return r.data as T;
}

export function hashText(
  text: string,
  algorithmId: string,
  upper: boolean,
): Promise<TextHashResult> {
  return commands.utilitiesHashText(text, algorithmId, upper).then((r) => {
    const d = unwrap(r);
    return {
      algorithm: d.algorithm,
      bytesProcessed: d.bytesProcessed ?? 0,
      digestHex: d.digestHex,
      warnings: (d.warnings ?? []).map((w) => ({ code: w.code, message: w.message })),
    };
  });
}

export function hashAlgorithms(): Promise<HashAlgorithmInfoDto[]> {
  // 无 Result 包装的命令（无错误路径）直接返回数据
  return commands.utilitiesHashAlgorithms();
}

export function checksumText(text: string, algorithmId: string): Promise<string> {
  return commands.utilitiesChecksum(text, algorithmId).then(unwrap);
}

export function base64Encode(
  text: string,
  encodingId: string,
  alphabetId: string,
  omitPadding: boolean,
): Promise<string> {
  return commands
    .utilitiesBase64Encode(text, encodingId, alphabetId, omitPadding)
    .then(unwrap);
}

export interface Base64DecodeResult {
  bytesHex: string;
  textPreview: string;
  byteCount: number;
  warnings: string[];
}

export function base64Decode(
  input: string,
  alphabetId: string,
  lenient: boolean,
): Promise<Base64DecodeResult> {
  return commands.utilitiesBase64Decode(input, alphabetId, lenient).then((r) => {
    const d = unwrap(r);
    return {
      bytesHex: d.bytesHex ?? "",
      textPreview: d.textPreview ?? "",
      byteCount: d.byteCount ?? 0,
      warnings: d.warnings ?? [],
    };
  });
}

export function uuidGenerate(
  version: string,
  count: number,
  format: string,
): Promise<UuidInfoDto[]> {
  return commands.utilitiesUuidGenerate(version, count, format).then(unwrap);
}

export function uuidValidate(input: string): Promise<UuidInfoDto> {
  return commands.utilitiesUuidValidate(input).then(unwrap);
}

export function timestampConvert(
  input: string,
  unit: string | null,
  offsetMinutes: number | null,
): Promise<TimestampConversionDto> {
  return commands.utilitiesTimestampConvert(input, unit, offsetMinutes).then(unwrap);
}

export function timestampNow(): Promise<TimestampConversionDto> {
  return commands.utilitiesTimestampNow();
}

export function urlEncode(mode: string, input: string): Promise<string> {
  return commands.utilitiesUrlEncode(mode, input).then(unwrap);
}

export function urlDecode(mode: string, input: string, lenient: boolean): Promise<string> {
  return commands.utilitiesUrlDecode(mode, input, lenient).then(unwrap);
}

export function urlParse(input: string): Promise<UrlPartDto> {
  return commands.utilitiesUrlParse(input).then(unwrap);
}

export function regexFind(
  pattern: string,
  input: string,
  flags: RegexFlags,
): Promise<RegexMatchDto[]> {
  return commands.utilitiesRegexFind(pattern, input, flags).then(unwrap);
}

export function regexReplace(
  pattern: string,
  input: string,
  replacement: string,
  flags: RegexFlags,
): Promise<RegexReplaceResult> {
  return commands.utilitiesRegexReplace(pattern, input, replacement, flags).then((r) => {
    const d = unwrap(r);
    return { output: d.output, count: d.count ?? 0 };
  });
}

export function regexCapabilities(): Promise<RegexCapabilityDto[]> {
  return commands.utilitiesRegexCapabilities();
}

export interface ColorRequest {
  kind: "hex" | "rgb" | "hsl" | "hsv" | "hwb";
  value?: string;
  r?: number;
  g?: number;
  b?: number;
  h?: number;
  s?: number;
  l?: number;
  v?: number;
  w?: number;
  a?: number;
}

export function colorConvert(input: ColorRequest): Promise<ColorResult> {
  return commands.utilitiesColorConvert(input as never).then((r) => {
    const d = unwrap(r);
    return {
      hex: d.hex,
      r: d.r ?? 0,
      g: d.g ?? 0,
      b: d.b ?? 0,
      a: d.a ?? 1,
      h: d.h ?? 0,
      sHsl: d.sHsl ?? 0,
      l: d.l ?? 0,
      sHsv: d.sHsv ?? 0,
      v: d.v ?? 0,
      contrastOnWhite: d.contrastOnWhite ?? 1,
      contrastOnBlack: d.contrastOnBlack ?? 1,
    };
  });
}

export function colorContrast(hexA: string, hexB: string): Promise<number> {
  return commands.utilitiesColorContrast(hexA, hexB).then((r) => unwrap(r) ?? 1);
}

// ── M9（下）：文件 Checksum + 导出 ──

export interface FileChecksumEntry {
  path: string;
  algorithm: string;
  digestHex: string;
  bytesProcessed: number;
  status: string;
}

export interface FileExportResult {
  output: string;
  entryCount: number;
}

export function checksumFile(path: string, algorithmId: string): Promise<FileChecksumEntry> {
  return commands.utilitiesChecksumFile(path, algorithmId).then((r) => {
    const d = unwrap(r);
    return {
      path: d.path,
      algorithm: d.algorithm,
      digestHex: d.digestHex,
      bytesProcessed: d.bytesProcessed ?? 0,
      status: d.status,
    };
  });
}

export function exportReport(
  entries: FileChecksumEntry[],
  format: string,
  destinationDir: string,
  fileName: string,
): Promise<FileExportResult> {
  return commands
    .utilitiesExportReport(entries, format, destinationDir, fileName)
    .then((r) => {
      const d = unwrap(r);
      return { output: d.output, entryCount: d.entryCount ?? 0 };
    });
}
