import { describe, expect, it } from "vitest";
import { rankCommands, type RankedCommand } from "./commandSearch";
import type { CommandDefinition } from "../commands/registry";

const commands: CommandDefinition[] = [
  { id: "tool.open.text", labelKey: "nav.text", run: () => undefined, category: "tools", keywords: ["editor", "文本"] },
  { id: "tool.open.image", labelKey: "nav.image", run: () => undefined, category: "tools", keywords: ["img", "图片", "resize"] },
  { id: "tool.open.rename", labelKey: "nav.rename", run: () => undefined, category: "tools", keywords: ["重命名", "batch rename"] },
  { id: "workflow.run", labelKey: "workflow.run", run: () => undefined, category: "workflow" },
  { id: "history.undo", labelKey: "history.undo", run: () => undefined, category: "history" },
];

const label = (key: string): string =>
  ({
    "nav.text": "文本",
    "nav.image": "图片",
    "nav.rename": "重命名",
    "workflow.run": "运行工作流",
    "history.undo": "撤销",
  })[key] ?? key;

const ids = (rs: RankedCommand[]): string[] => rs.map((r) => r.command.id);

describe("rankCommands (§20 确定性排序)", () => {
  it("empty query lists all deterministic", () => {
    const r1 = rankCommands(commands, "", label);
    const r2 = rankCommands(commands, "", label);
    expect(ids(r1)).toHaveLength(commands.length);
    expect(ids(r1)).toEqual(ids(r2));
  });

  it("exact beats prefix beats keyword", () => {
    const r = rankCommands(commands, "图片", label);
    expect(r.at(0)?.command.id).toBe("tool.open.image");
    expect(r.at(0)?.rank).toBe("exact");
  });

  it("chinese query finds localized label (§19)", () => {
    const r = rankCommands(commands, "重命名", label);
    expect(ids(r)[0]).toBe("tool.open.rename");
  });

  it("keyword match hits keywords list", () => {
    const r = rankCommands(commands, "resize", label);
    expect(ids(r)[0]).toBe("tool.open.image");
    expect(r.at(0)?.rank).toBe("keyword");
  });

  it("token match: all tokens prefix-match words", () => {
    const r = rankCommands(commands, "运行 工作流", label);
    expect(ids(r)[0]).toBe("workflow.run");
    expect(r.at(0)?.rank).toBe("token");
  });

  it("fuzzy subsequence last resort (id)", () => {
    const r = rankCommands(commands, "txt", label);
    expect(ids(r)).toContain("tool.open.text");
    expect(r.at(0)?.rank).toBe("fuzzy");
  });

  it("no result stays empty (§22)", () => {
    expect(rankCommands(commands, "zzzz-none", label)).toEqual([]);
  });

  it("deterministic across calls (§20 tie-break)", () => {
    const a = rankCommands(commands, "文", label).map((r) => r.command.id);
    const b = rankCommands(commands, "文", label).map((r) => r.command.id);
    expect(a).toEqual(b);
  });
});
