//! M11（上）§51-§55：ShortcutRegistry——快捷键单一事实来源。
//! scope 区分（§53）避免文本输入框被全局快捷键抢占（§53 Ctrl+Z 例子）。

export type ShortcutScope = "global";

export interface ShortcutDefinition {
  id: string;
  /** "Mod+K" 形式；Mod = Windows Ctrl（§55，未来跨平台可映射 Cmd）。 */
  keys: string;
  scope: "global";
  descriptionKey: string;
  action: () => void;
}

const shortcuts = new Map<string, ShortcutDefinition>();

function normalize(keys: string): string {
  return keys
    .split("+")
    .map((k) => k.trim().toLowerCase())
    .join("+");
}

export function registerShortcut(definition: ShortcutDefinition): void {
  // §54 冲突检查：同组合键若已属于其他 id ⇒ 拒绝；同 id 重注册（组件
  // 重挂载/热更新）= 幂等 upsert
  const key = `${definition.scope}:${normalize(definition.keys)}`;
  const existing = shortcuts.get(key);
  if (existing && existing.id !== definition.id) {
    throw new Error(
      `shortcut conflict: ${key} already bound to '${existing.id}'`,
    );
  }
  shortcuts.set(key, definition);
}

export function listShortcuts(): ShortcutDefinition[] {
  return [...shortcuts.values()];
}

/// §54 冲突检查：注册时已按 scope+keys 唯一化；重复注册抛错。
/// 键盘事件分发：全局 scope 的快捷键在非输入焦点时触发。
export function dispatchShortcut(event: KeyboardEvent): boolean {
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) {
    parts.push("mod");
  }
  if (event.shiftKey) {
    parts.push("shift");
  }
  if (event.altKey) {
    parts.push("alt");
  }
  const key = event.key.toLowerCase();
  if (key === "control" || key === "meta" || key === "shift" || key === "alt") {
    return false;
  }
  parts.push(key);
  const combo = `global:${parts.join("+")}`;
  const shortcut = shortcuts.get(combo);
  if (!shortcut) {
    return false;
  }
  // §53：文本输入焦点下只放行含修饰键的组合
  const target = event.target as HTMLElement | null;
  const inText = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA";
  if (inText && !event.ctrlKey && !event.metaKey) {
    return false;
  }
  event.preventDefault();
  shortcut.action();
  return true;
}

export function clearShortcuts(): void {
  shortcuts.clear();
}
