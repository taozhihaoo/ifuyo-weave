import { commands, type DataPageDto } from "../generated/bindings";
import { pollUntilDone } from "./operations";

/** M5 Data：CSV/TSV/JSON/JSONL 会话 + Inspector + Cleaner + Converter。
 *
 * Preview-first（§76）；session 为 ephemeral（§17），分页防 IPC 爆炸（§85）。 */

export interface FilterSpec {
  columnId: string;
  operator: "contains" | "equals" | "notEquals" | "startsWith" | "endsWith" | "empty" | "notEmpty";
  value: string;
  caseSensitive: boolean;
}

export interface SortUpdate {
  columnId: string;
  descending: boolean;
  ignoreCase: boolean;
}

export {
  type DataPageDto,
  type ColumnProfileDto,
  type TransformPreviewDto,
} from "../generated/bindings";

export function dataOpen(path: string, hasHeader: "yes" | "no" | null): Promise<DataPageDto> {
  return commands.dataOpen(path, hasHeader).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}

export function dataPage(sessionId: string, offset: number): Promise<DataPageDto> {
  return commands.dataPage(sessionId, offset).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}

export function dataSetView(
  sessionId: string,
  filters: FilterSpec[],
  sort: SortUpdate | null,
  offset: number,
): Promise<DataPageDto> {
  return commands
    .dataSetView(
      sessionId,
      filters.map((f) => ({
        columnId: f.columnId,
        operator: f.operator,
        value: f.value,
        caseSensitive: f.caseSensitive,
      })),
      sort
        ? { columnId: sort.columnId, descending: sort.descending, ignoreCase: sort.ignoreCase }
        : null,
      offset,
    )
    .then((r) => {
      if (r.status === "ok") return r.data;
      throw new Error(`${r.error.code} · ${r.error.message}`);
    });
}

export function dataInspectProfiles(sessionId: string) {
  return commands.dataInspectProfiles(sessionId).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}

export function dataPreviewTransform(sessionId: string, plan: Record<string, unknown>) {
  return commands.dataPreviewTransform(sessionId, plan as never).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}

export function dataApplyTransform(
  sessionId: string,
  plan: Record<string, unknown>,
): Promise<DataPageDto> {
  return commands.dataApplyTransform(sessionId, plan as never).then((r) => {
    if (r.status === "ok") return r.data;
    throw new Error(`${r.error.code} · ${r.error.message}`);
  });
}

export function dataExport(
  sessionId: string,
  destination: string,
  format: "csv" | "tsv" | "json" | "jsonl",
  includeHeader: boolean,
  typed: boolean,
  lineEnding: "lf" | "crlf",
): Promise<string> {
  return commands
    .dataExport(sessionId, {
      destination,
      format,
      includeHeader,
      typed,
      lineEnding,
    })
    .then((r) => {
      if (r.status === "ok") return r.data.operationId;
      throw new Error(`${r.error.code} · ${r.error.message}`);
    });
}

/** Export 走 TextTransform 任务；轮询至终态后返回（历史可撤销）。 */
export function pollExport(operationId: string): Promise<void> {
  return new Promise((resolve, reject) => {
    pollUntilDone(
      operationId,
      () => {},
      (status) => {
        if (status.state === "failed") {
          reject(new Error(status.error?.message ?? "export failed"));
        } else {
          resolve();
        }
      },
    );
  });
}
