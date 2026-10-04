// M11（下）§25-§33/§177-§178：Quick Drop 建议——纯映射函数。
//
// detect/inspect 由 M1 完成（dispatchPath 链路），这里只做
// kind/extension → 建议 view 的路由映射（§178 Application 层职责；
// §195 不执行操作，只路由到既有工具页）。无 AI 无 magic（§26）。

/// 文件扩展名 → 建议工具页（按真实已实现能力映射；§28 只显示存在的）。
const EXTENSION_SUGGESTIONS: Record<string, string[]> = {
  png: ["image", "documents"],
  jpg: ["image"],
  jpeg: ["image"],
  webp: ["image"],
  bmp: ["image"],
  tiff: ["image"],
  gif: ["image"],
  pdf: ["documents"],
  docx: ["documents"],
  xlsx: ["data", "documents"],
  pptx: ["documents"],
  csv: ["data"],
  tsv: ["data"],
  json: ["data"],
  jsonl: ["data"],
  txt: ["text"],
  md: ["text"],
};

/// 按检测结果给出建议 view（去重、保序；未知类型 → Inspector 已在首页）。
export function suggestionsFor(kind: string, extension: string): string[] {
  if (kind === "directory") {
    return ["duplicates", "workflow"];
  }
  if (kind !== "file") {
    return [];
  }
  const ext = extension.replace(/^\./, "").toLowerCase();
  return EXTENSION_SUGGESTIONS[ext] ?? [];
}
