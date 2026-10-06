import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type {
    AgentNotificationDto,
    AgentStatusDto,
    ScheduleDto,
} from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";

/** M2.6：Agent 状态面板（§40/§65 状态优先——非装饰性 Dashboard）。 */

interface AgentPanelProps {
    locale: string;
}

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

export function AgentPanel({ locale }: AgentPanelProps) {
    const t = useT();
    const [status, setStatus] = useState<AgentStatusDto | null>(null);
    const [schedules, setSchedules] = useState<ScheduleDto[]>([]);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const load = useCallback(() => {
        commands
            .agentStatus()
            .then((r) => {
                if (r.status === "ok") setStatus(r.data);
            })
            .catch(() => {});
        commands
            .agentListSchedules()
            .then((r) => {
                if (r.status === "ok") setSchedules(r.data);
            })
            .catch(() => {});
    }, []);

    useEffect(() => {
        load();
        const timer = setInterval(load, 4000);
        return () => clearInterval(timer);
    }, [load]);

    const setPaused = useCallback(
        (paused: boolean) => {
            setBusy(true);
            commands
                .agentSetPaused(paused)
                .then((r) => {
                    setBusy(false);
                    if (r.status !== "ok") setError(r.error.code);
                    load();
                })
                .catch((e: unknown) =>
                    setError(e instanceof Error ? e.message : String(e)),
                );
        },
        [load],
    );

    const stateLabel =
        status == null
            ? "…"
            : status.paused
              ? t("agent.state.paused")
              : t(`agent.state.${status.state ?? "stopped"}`);
    const stateColor =
        status == null || status.paused || status.state === "stopped"
            ? "var(--color-text-muted)"
            : "var(--color-success)";

    return (
        <section
            aria-label={t("agent.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <div style={rowStyle}>
                <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("agent.title")}</h2>
                <span
                    role="status"
                    style={{ color: stateColor, fontWeight: 600, fontSize: "var(--typography-size-sm)" }}
                >
                    {status?.paused || status?.state === "paused" ? `○ ${stateLabel}` : `● ${stateLabel}`}
                </span>
                <button
                    type="button"
                    disabled={busy}
                    style={{ ...boxStyle, marginLeft: "auto" }}
                    onClick={() => setPaused(!(status?.paused ?? false))}
                >
                    {status?.paused ? t("agent.resume") : t("agent.pause")}
                </button>
            </div>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("agent.hint")}
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {status ? (
                <>
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                        <strong style={{ fontSize: "var(--typography-size-sm)" }}>
                            {t("agent.components")}
                        </strong>
                        {status.components.map(([name, compState]) => (
                            <div key={name} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                                <span style={{ fontWeight: 600 }}>{t(`agent.component.${name}`)}</span>
                                <span
                                    style={{
                                        color:
                                            compState === "running"
                                                ? "var(--color-success)"
                                                : compState === "faulted"
                                                  ? "var(--color-danger)"
                                                  : "var(--color-text-muted)",
                                    }}
                                >
                                    {t(`agent.componentState.${compState}`)}
                                </span>
                            </div>
                        ))}
                    </div>

                    <div style={{ ...rowStyle, fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                        {t("agent.queue", {
                            len: status.queueLen ?? 0,
                            cap: status.queueCapacity ?? 0,
                            running: status.gateRunning ?? 0,
                            max: status.gateMax ?? 0,
                            dropped: status.queueDropped ?? 0,
                        })}
                    </div>

                    {schedules.length > 0 ? (
                        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                            <strong style={{ fontSize: "var(--typography-size-sm)" }}>
                                {t("agent.schedules")}
                            </strong>
                            {schedules.map((s) => (
                                <div key={s.id} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                                    <span style={{ fontWeight: 600 }}>{s.name}</span>
                                    <span style={{ color: "var(--color-text-muted)" }}>{s.kind}</span>
                                    <span style={{ color: s.enabled ? "var(--color-success)" : "var(--color-text-muted)" }}>
                                        {s.enabled ? t("agent.schedule.enabled") : t("agent.schedule.disabled")}
                                    </span>
                                </div>
                            ))}
                        </div>
                    ) : null}

                    {(status.notifications?.length ?? 0) > 0 ? (
                        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                            <strong style={{ fontSize: "var(--typography-size-sm)" }}>
                                {t("agent.notifications")}
                            </strong>
                            {status.notifications.map((n: AgentNotificationDto) => (
                                <div
                                    key={n.code}
                                    style={{
                                        fontSize: "var(--typography-size-sm)",
                                        color:
                                            n.level === "error"
                                                ? "var(--color-danger)"
                                                : n.level === "warning"
                                                  ? "var(--color-accent)"
                                                  : "var(--color-text-muted)",
                                    }}
                                >
                                    [{n.level}] {n.message}
                                    {(n.count ?? 1) > 1 ? ` ×${n.count}` : ""}
                                </div>
                            ))}
                        </div>
                    ) : null}

                    {status.lastError ? (
                        <div style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                            {t("agent.lastError")}: {status.lastError}
                        </div>
                    ) : null}

                    <div style={{ ...boxStyle, fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                        {locale === "zh-CN" ? t("agent.queue") : t("agent.queue")}
                    </div>
                </>
            ) : null}
        </section>
    );
}
