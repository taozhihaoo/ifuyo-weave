/**
 * 前端 Command Registry 最小版（M0 §35）。
 * 未来：Ctrl+K → Command Registry → Action → Tool。
 * M0 只验证架构：注册、罗列（deterministic 排序）、执行。
 */

export type CommandCategory =
  | "navigation"
  | "tools"
  | "workflow"
  | "history"
  | "settings"
  | "app";

export interface CommandDefinition {
  /** 稳定机器可读 ID，如 "app.ping"。 */
  id: string;
  /** i18n 资源 key，禁止硬编码文案（charter #34）。 */
  labelKey: string;
  run: () => void | Promise<void>;
  /** M11 §17：palette 分类与搜索关键词。 */
  category?: CommandCategory;
  keywords?: string[];
  /** 展示用快捷键（如 "Mod+K"）；实际分发在 ShortcutRegistry。 */
  shortcut?: string;
}

const registry = new Map<string, CommandDefinition>();

export function registerCommand(definition: CommandDefinition): void {
  if (registry.has(definition.id)) {
    throw new Error(`duplicate command id: ${definition.id}`);
  }
  registry.set(definition.id, definition);
}

export function listCommands(): CommandDefinition[] {
  return [...registry.values()].sort((a, b) => a.id.localeCompare(b.id));
}

export function runCommand(id: string): void {
  const command = registry.get(id);
  if (!command) {
    throw new Error(`unknown command: ${id}`);
  }
  void Promise.resolve(command.run());
}

export function clearCommands(): void {
  registry.clear();
}
