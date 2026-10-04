//! M11（上）Command Palette（§15-§24）：确定性排序的命令搜索。
//!
//! Ranking（§20）：exact > prefix > token > keyword > fuzzy，
//! tie-break：category → label → id（全确定性，无 AI 排序）。
//! 纯函数、无 React 依赖——可独立测试（§24）。

import type { CommandDefinition } from "../commands/registry";

export type SearchRank = "exact" | "prefix" | "token" | "keyword" | "fuzzy";

export interface RankedCommand {
  command: CommandDefinition;
  rank: SearchRank;
}

const RANK_ORDER: Record<SearchRank, number> = {
  exact: 0,
  prefix: 1,
  token: 2,
  keyword: 3,
  fuzzy: 4,
};

function normalize(s: string): string {
  return s.trim().toLowerCase();
}

/// 模糊 = 子序列匹配（大小写不敏感）。
function isSubsequence(query: string, target: string): boolean {
  let i = 0;
  for (const ch of target) {
    if (ch === query[i]) {
      i += 1;
      if (i === query.length) {
        return true;
      }
    }
  }
  return false;
}

function rankOne(
  command: CommandDefinition,
  label: string,
  query: string,
): SearchRank | "none" {
  const q = normalize(query);
  const l = normalize(label);
  const id = normalize(command.id);
  if (l === q || id === q) {
    return "exact";
  }
  if (l.startsWith(q) || id.startsWith(q)) {
    return "prefix";
  }
  const tokens = q.split(/\s+/).filter(Boolean);
  if (tokens.length > 1 && tokens.every((t) => l.includes(t))) {
    return "token";
  }
  const keywords = (command.keywords ?? []).map(normalize);
  if (keywords.some((k) => k === q || k.startsWith(q))) {
    return "keyword";
  }
  if (l.includes(q) || id.includes(q)) {
    return "keyword";
  }
  if (isSubsequence(q, l) || isSubsequence(q, id)) {
    return "fuzzy";
  }
  return "none";
}

/// §20/§61/§103：确定性排序。相同 rank 内按 category → label → id。
export function rankCommands(
  commands: CommandDefinition[],
  query: string,
  resolveLabel: (key: string) => string,
): RankedCommand[] {
  const q = normalize(query);
  const ranked: RankedCommand[] = [];
  for (const command of commands) {
    if (!q) {
      // 空查询 = 全部命令按 id 稳定列出（palette 打开默认视图）
      ranked.push({ command, rank: "prefix" });
      continue;
    }
    const label = resolveLabel(command.labelKey);
    const rank = rankOne(command, label, q);
    if (rank !== "none") {
      ranked.push({ command, rank });
    }
  }
  ranked.sort((a, b) => {
    if (RANK_ORDER[a.rank] !== RANK_ORDER[b.rank]) {
      return RANK_ORDER[a.rank] - RANK_ORDER[b.rank];
    }
    const ca = a.command.category ?? "";
    const cb = b.command.category ?? "";
    if (ca !== cb) {
      return ca.localeCompare(cb);
    }
    const la = normalize(resolveLabel(a.command.labelKey));
    const lb = normalize(resolveLabel(b.command.labelKey));
    if (la !== lb) {
      return la.localeCompare(lb);
    }
    return a.command.id.localeCompare(b.command.id);
  });
  return ranked;
}
