import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type { MonitorConfig, WebConfig } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { persistAppConfig } from "../../lib/bootstrap";
import { useAppStore, type Locale, type Theme } from "../../stores/appStore";

/** M2.13：统一 Settings 页（§24 分区；§25 设置真正生效——全量 round-trip +
 * 各模块 apply IPC）。模块特有高级设置继续留在各模块面板（Modules 原则）。 */

const rowStyle: CSSProperties = {
    display: "flex",
    gap: "var(--spacing-sm)",
    alignItems: "center",
    flexWrap: "wrap",
};
const boxStyle: CSSProperties = {
    padding: "var(--spacing-sm)",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--color-border)",
};

export function SettingsPanel() {
    const t = useT();
    const locale = useAppStore((s) => s.locale);
    const theme = useAppStore((s) => s.theme);
    const [clipboardEnabled, setClipboardEnabled] = useState<boolean | null>(null);
    const [wispEnabled, setWispEnabled] = useState<boolean | null>(null);
    const [web, setWeb] = useState<WebConfig | null>(null);
    const [monitor, setMonitor] = useState<MonitorConfig | null>(null);
    const [saved, setSaved] = useState(false);

    useEffect(() => {
        commands
            .getAppConfig()
            .then((r) => {
                if (r.status === "ok") {
                    setClipboardEnabled(r.data.clipboard?.enabled ?? true);
                    setWispEnabled(r.data.wisp?.enabled ?? true);
                }
            })
            .catch(() => {});
        commands.webGetConfig().then(setWeb).catch(() => {});
        commands.monitorGetConfig().then(setMonitor).catch(() => {});
    }, []);

    const flashSaved = useCallback(() => {
        setSaved(true);
        setTimeout(() => setSaved(false), 1500);
    }, []);

    // §25：一次保存 = persistAppConfig（全量 round-trip 持久化）+ 模块
    // apply IPC（运行时立即生效）——两者都在 mutate 回调外完成后提示。
    const saveGeneral = useCallback(
        (mutate: (c: import("../../generated/bindings").AppConfig) => import("../../generated/bindings").AppConfig) => {
            void persistAppConfig(mutate).then(flashSaved);
        },
        [flashSaved],
    );

    const setLocalePersist = useCallback((next: Locale) => {
        useAppStore.getState().setLocale(next);
        void import("../../lib/bootstrap").then((m) => m.persistLocale(next));
    }, []);

    const setThemePersist = useCallback((next: Theme) => {
        useAppStore.getState().setTheme(next);
        void import("../../lib/bootstrap").then((m) => m.persistTheme(next));
    }, []);

    const setClipboardEnabledPersist = useCallback(
        (checked: boolean) => {
            setClipboardEnabled(checked);
            void persistAppConfig((c) => {
                const next = { ...(c.clipboard ?? {}), enabled: checked };
                commands.clipboardApplyConfig(next).catch(() => {});
                return { ...c, clipboard: next };
            }).then(flashSaved);
        },
        [flashSaved],
    );

    const setWispEnabledPersist = useCallback(
        (checked: boolean) => {
            setWispEnabled(checked);
            void persistAppConfig((c) => {
                const next = { ...(c.wisp ?? {}), enabled: checked };
                commands.wispApplyConfig(next).catch(() => {});
                return { ...c, wisp: next };
            }).then(flashSaved);
        },
        [flashSaved],
    );

    const setWebPersist = useCallback(
        (next: WebConfig) => {
            setWeb(next);
            commands.webApplyConfig(next).catch(() => {});
            saveGeneral((c) => ({ ...c, web: next }));
        },
        [saveGeneral],
    );

    const setMonitorPersist = useCallback(
        (next: MonitorConfig) => {
            setMonitor(next);
            commands.monitorApplyConfig(next).catch(() => {});
            saveGeneral((c) => ({ ...c, monitor: next }));
        },
        [saveGeneral],
    );

    return (
        <section aria-label={t("settings.title")}>
            <h2>{t("settings.title")}</h2>
            {saved ? (
                <p style={{ fontSize: 12, color: "var(--color-success)" }} role="status">
                    {t("settings.saved")}
                </p>
            ) : null}

            {/* General（§24） */}
            <div style={boxStyle}>
                <strong>{t("settings.general")}</strong>
                <div style={rowStyle}>
                    <label>
                        {t("app.language")}{" "}
                        <select value={locale} onChange={(e) => setLocalePersist(e.target.value as Locale)}>
                            <option value="zh-CN">中文</option>
                            <option value="en">English</option>
                        </select>
                    </label>
                    <label>
                        {t("app.theme")}{" "}
                        <select value={theme} onChange={(e) => setThemePersist(e.target.value as Theme)}>
                            <option value="system">{t("app.themeName.system")}</option>
                            <option value="light">{t("app.themeName.light")}</option>
                            <option value="dark">{t("app.themeName.dark")}</option>
                        </select>
                    </label>
                </div>
            </div>

            {/* Modules → Clipboard（§24：模块特有高级项仍在模块面板） */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <strong>{t("settings.clipboard")}</strong>
                {clipboardEnabled !== null ? (
                    <div style={rowStyle}>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input
                                type="checkbox"
                                checked={clipboardEnabled}
                                onChange={(e) => setClipboardEnabledPersist(e.target.checked)}
                            />
                            {t("settings.clipboardEnabled")}
                        </label>
                    </div>
                ) : null}
                <p style={{ fontSize: 11, margin: "4px 0 0" }}>{t("settings.clipboardNote")}</p>
            </div>

            {/* Modules → Wisp */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <strong>{t("settings.wisp")}</strong>
                {wispEnabled !== null ? (
                    <div style={rowStyle}>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input
                                type="checkbox"
                                checked={wispEnabled}
                                onChange={(e) => setWispEnabledPersist(e.target.checked)}
                            />
                            {t("settings.wispEnabled")}
                        </label>
                    </div>
                ) : null}
                <p style={{ fontSize: 11, margin: "4px 0 0" }}>{t("settings.wispNote")}</p>
            </div>

            {/* Modules → Web */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <strong>{t("settings.web")}</strong>
                {web ? (
                    <div style={rowStyle}>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input
                                type="checkbox"
                                checked={web.allowNetwork}
                                onChange={(e) => setWebPersist({ ...web, allowNetwork: e.target.checked })}
                            />
                            {t("web.settings.allowNetwork")}
                        </label>
                        <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
                            <input
                                type="checkbox"
                                checked={web.allowPrivateTargets}
                                onChange={(e) => setWebPersist({ ...web, allowPrivateTargets: e.target.checked })}
                            />
                            {t("web.settings.allowPrivate")}
                        </label>
                    </div>
                ) : null}
            </div>

            {/* Modules → Monitor */}
            <div style={{ ...boxStyle, marginTop: "var(--spacing-sm)" }}>
                <strong>{t("settings.monitor")}</strong>
                {monitor ? (
                    <div style={rowStyle}>
                        <label>
                            {t("settings.monitorInterval")}
                            <input
                                type="number"
                                min={60}
                                value={monitor.defaultIntervalSecs ?? 1800}
                                onChange={(e) =>
                                    setMonitorPersist({
                                        ...monitor,
                                        defaultIntervalSecs: Math.max(60, Number(e.target.value) || 1800),
                                    })
                                }
                                style={{ width: 90 }}
                            />
                            s
                        </label>
                    </div>
                ) : null}
            </div>
        </section>
    );
}
