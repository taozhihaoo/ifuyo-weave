import { useCallback, useEffect, useState } from "react";
import type { JobSummaryDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";

/** M2.13：统一 Job Center（§11——当前任务唯一入口；§12 无伪进度）。 */

export function JobsPanel() {
    const t = useT();
    const [jobs, setJobs] = useState<JobSummaryDto[]>([]);
    const [error, setError] = useState<string | null>(null);

    const load = useCallback(() => {
        commands
            .jobsList()
            .then(setJobs)
            .catch(() => {});
    }, []);

    useEffect(() => {
        load();
        const timer = setInterval(load, 2000);
        return () => clearInterval(timer);
    }, [load]);

    const cancel = useCallback(
        (jobId: string) => {
            commands
                .jobsCancel(jobId)
                .then((ok) => {
                    if (!ok) setError(t("jobs.cancelNotCancellable"));
                    load();
                })
                .catch(() => {});
        },
        [load, t],
    );

    const running = jobs.filter((j) => j.state === "running");
    const terminal = jobs.filter((j) => j.state !== "running");

    const row = (j: JobSummaryDto) => (
        <tr key={j.jobId}>
            <td style={{ fontFamily: "monospace", fontSize: 12 }}>{j.jobId.slice(0, 13)}…</td>
            <td>{t(`jobs.state.${j.state}`)}</td>
            <td style={{ fontSize: 12 }}>
                {j.state === "running"
                    ? j.progressCurrent
                        ? t("jobs.progressKnown", { n: j.progressCurrent })
                        : t("jobs.progressUnknown")
                    : "—"}
            </td>
            <td style={{ color: j.errorCode ? "var(--color-danger)" : undefined, fontSize: 12 }}>
                {j.errorCode ?? "—"}
            </td>
            <td>
                {j.state === "running" ? (
                    <button onClick={() => cancel(j.jobId)}>{t("jobs.cancel")}</button>
                ) : null}
            </td>
        </tr>
    );

    return (
        <section aria-label={t("jobs.title")}>
            <h2>{t("jobs.title")}</h2>
            {error ? (
                <p role="alert" style={{ color: "var(--color-danger)" }}>
                    {error}
                </p>
            ) : null}
            {jobs.length === 0 ? (
                <p>{t("jobs.empty")}</p>
            ) : (
                <>
                    {running.length > 0 ? (
                        <>
                            <h3>
                                {t("jobs.running", { n: running.length })}
                            </h3>
                            <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
                                <tbody>{running.map(row)}</tbody>
                            </table>
                        </>
                    ) : null}
                    {terminal.length > 0 ? (
                        <>
                            <h3>{t("jobs.recentTerminal")}</h3>
                            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                                <tbody>{terminal.slice(0, 30).map(row)}</tbody>
                            </table>
                        </>
                    ) : null}
                    <p style={{ fontSize: 11, marginTop: 8 }}>{t("jobs.retentionNote")}</p>
                </>
            )}
        </section>
    );
}
