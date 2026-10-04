import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { useT } from "../../i18n";
import {
  listCommands,
  type CommandDefinition,
} from "../../commands/registry";
import { rankCommands } from "../../lib/commandSearch";

const boxStyle: CSSProperties = {
  padding: "var(--spacing-xs) var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
  fontSize: "var(--typography-size-sm)",
};

const overlayStyle: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "var(--color-border)",
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "center",
  paddingTop: "10vh",
  zIndex: 1000,
};

const panelStyle: CSSProperties = {
  width: "min(560px, 92vw)",
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  maxHeight: "60vh",
};

const inputStyle: CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  padding: "var(--spacing-sm) var(--spacing-md)",
  border: "none",
  borderBottom: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  color: "inherit",
  fontSize: "var(--typography-size-md)",
  outline: "none",
};

const listStyle: CSSProperties = {
  overflowY: "auto",
  margin: 0,
  padding: 0,
  listStyle: "none",
};

const itemBase: CSSProperties = {
  padding: "var(--spacing-xs) var(--spacing-md)",
  cursor: "pointer",
  display: "flex",
  justifyContent: "space-between",
  gap: "var(--spacing-sm)",
  alignItems: "center",
};

/** M11（上）§15-§24 Command Palette：键盘 → 命令 → 动作。
 * 数据源 = 前端 CommandRegistry 单一事实来源（§17）；
 * 排序确定性（§20）；键盘 ↑↓/Enter/Escape（§21）；
 * 空结果（§22）/执行错误（§23）/焦点恢复（§21）。 */
export function CommandPalette({
  open,
  onClose,
  onExecuteError,
}: {
  open: boolean;
  onClose: () => void;
  onExecuteError: (message: string) => void;
}) {
  const t = useT();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const restoreRef = useRef<HTMLElement | null>(null);

  const commands = useMemo(() => listCommands(), [open]); // eslint-disable-line react-hooks/exhaustive-deps
  const ranked = useMemo(
    () => rankCommands(commands, query, (key) => t(key)),
    [commands, query, t],
  );

  // §21：打开时聚焦输入（state 重置经父级 key 重挂载）；关闭时恢复焦点
  useEffect(() => {
    if (open) {
      restoreRef.current = document.activeElement as HTMLElement | null;
      requestAnimationFrame(() => inputRef.current?.focus());
    } else {
      restoreRef.current?.focus?.();
    }
  }, [open]);

  if (!open) {
    return null;
  }

  const execute = (command: CommandDefinition): void => {
    onClose();
    try {
      void Promise.resolve(command.run()).catch((e: unknown) => {
        // §23：执行失败必须可见，不能什么都没发生
        onExecuteError(e instanceof Error ? e.message : String(e));
      });
    } catch (e) {
      onExecuteError(e instanceof Error ? e.message : String(e));
    }
  };

  const onKeyDown = (e: React.KeyboardEvent): void => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, ranked.length - 1));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const item = ranked[activeIndex];
      if (item) {
        execute(item.command);
      }
    }
  };

  return (
    <div style={overlayStyle}>
      <div
        style={panelStyle}
        role="dialog"
        aria-modal="true"
        aria-label={t("palette.title")}
      >
        <div style={{ position: "relative" }}>
        <input
          ref={inputRef}
          style={{ ...inputStyle, paddingRight: "3em" }}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActiveIndex(0);
          }}
          onKeyDown={onKeyDown}
          placeholder={t("palette.placeholder")}
          aria-label={t("palette.placeholder")}
        />
        <button
          type="button"
          aria-label={t("palette.close")}
          onClick={onClose}
          style={{
            position: "absolute",
            right: "var(--spacing-sm)",
            top: "var(--spacing-xs)",
            border: "none",
            background: "transparent",
            cursor: "pointer",
            fontSize: "var(--typography-size-lg)",
            color: "var(--color-text-muted)",
          }}
        >
          ×
        </button>
        </div>
        {ranked.length === 0 ? (
          // §22 空结果：明确提示 + 恢复方式
          <div style={{ padding: "var(--spacing-md)", fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
            {t("palette.noResults")}
            <button
              type="button"
              style={{ ...boxStyle, marginLeft: "var(--spacing-sm)" }}
              onClick={() => setQuery("")}
            >
              {t("palette.clearSearch")}
            </button>
          </div>
        ) : (
          <ul style={listStyle}>
            {ranked.map((item, index) => (
              // §89 语义 button；§21 键盘 Enter 经 onKeyDown 已支持
              <li key={item.command.id} style={{ listStyle: "none" }}>
                <button
                  type="button"
                  style={{
                    ...itemBase,
                    width: "100%",
                    border: "none",
                    background:
                      index === activeIndex ? "var(--color-accent-soft)" : "transparent",
                    textAlign: "left",
                  }}
                  onMouseEnter={() => setActiveIndex(index)}
                  onClick={() => execute(item.command)}
                >
                  <span>{t(item.command.labelKey)}</span>
                  <span style={{ fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                    {item.command.category ?? ""}
                    {item.command.shortcut ? ` · ${item.command.shortcut}` : ""}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
