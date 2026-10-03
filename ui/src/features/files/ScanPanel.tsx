import type { CSSProperties } from "react";
import type { JobStatusDto } from "../../generated/bindings";
import { useT } from "../../i18n";
import { formatBytes } from "../../lib/format";

interface ScanPanelProps {
  root: string;
  status: JobStatusDto | null;
  onCancel: () => void;
  onOpenFile: (path: string) => void;
}

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div
      style={{
        border: "1px solid var(--color-border)",
        borderRadius: "var(--radius-sm)",
        padding: "var(--spacing-sm) var(--spacing-md)",
        minWidth: 96,
      }}
    >
      <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
        {label}
      </div>
      <div
        style={{
          fontFamily: "var(--typography-mono-family)",
          fontSize: "var(--typography-size-lg)",
        }}
      >
        {value}
      </div>
    </div>
  );
}

function statusLabelKey(status: string): string {
  return `scan.status.${status}`;
}

export function ScanPanel({ root, status, onCancel, onOpenFile }: ScanPanelProps) {
  const t = useT();

  if (status === null || status.state === "running") {
    return (
      <section
        aria-label={t("scan.title")}
        style={{
          border: "1px solid var(--color-border)",
          borderRadius: "var(--radius-md)",
          padding: "var(--spacing-lg)",
          background: "var(--color-surface)",
          display: "flex",
          gap: "var(--spacing-md)",
          alignItems: "center",
        }}
      >
        <span role="status">
          {t("scan.scanning")}
          {status?.progressCurrent !== null && status?.progressCurrent !== undefined
            ? ` · ${t("scan.progressEntries", { count: status.progressCurrent })}`
            : ""}
        </span>
        <button type="button" onClick={onCancel} style={buttonStyle}>
          {t("scan.cancel")}
        </button>
      </section>
    );
  }

  if (status.state === "failed" || status.error !== null) {
    const error = status.error;
    return (
      <section role="alert" aria-label={t("scan.title")} style={{ color: "var(--color-danger)" }}>
        {t("error.title")}: {error?.code ?? "unknown"} · {error?.message ?? ""}
      </section>
    );
  }

  const report = status.scan;
  if (report === null || report === undefined) {
    return null;
  }
  const maxCount = Math.max(1, ...report.fileTypeDistribution.map((d) => d.count ?? 0));
  const other = report.otherEntries ?? 0;
  const errorCount = report.errorCount ?? 0;

  return (
    <section
      aria-label={t("scan.title")}
      style={{
        border: "1px solid var(--color-border)",
        borderRadius: "var(--radius-md)",
        padding: "var(--spacing-lg)",
        background: "var(--color-surface)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--spacing-md)",
      }}
    >
      <header style={{ display: "flex", gap: "var(--spacing-md)", alignItems: "baseline" }}>
        <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("scan.title")}</h2>
        <span
          role="status"
          style={{
            fontSize: "var(--typography-size-sm)",
            color:
              report.status === "completed" ? "var(--color-success)" : "var(--color-text-muted)",
          }}
        >
          {t(statusLabelKey(report.status))}
        </span>
        <span style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          {(report.durationMs ?? 0).toFixed(0)} ms
        </span>
      </header>

      <div
        style={{
          fontFamily: "var(--typography-mono-family)",
          fontSize: "var(--typography-size-sm)",
        }}
      >
        {root}
      </div>

      <div style={{ display: "flex", gap: "var(--spacing-sm)", flexWrap: "wrap" }}>
        <StatBox label={t("scan.stats.files")} value={String(report.filesScanned ?? 0)} />
        <StatBox
          label={t("scan.stats.directories")}
          value={String(report.directoriesScanned ?? 0)}
        />
        <StatBox label={t("scan.stats.size")} value={formatBytes(report.totalSize ?? 0)} />
        <StatBox label={t("scan.stats.maxDepth")} value={String(report.maxDepth ?? 0)} />
        {other > 0 ? <StatBox label={t("scan.stats.other")} value={String(other)} /> : null}
        {errorCount > 0 ? (
          <StatBox label={t("scan.stats.errors")} value={String(errorCount)} />
        ) : null}
      </div>

      {report.limited ? (
        <div
          role="status"
          style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-danger)" }}
        >
          {t("scan.limited")} — {report.limitedReason ?? ""}
        </div>
      ) : null}

      <div>
        <h3 style={{ margin: "0 0 var(--spacing-sm) 0", fontSize: "var(--typography-size-md)" }}>
          {t("scan.types")}
        </h3>
        {report.fileTypeDistribution.map((entry) => (
          <div
            key={entry.label}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "var(--spacing-sm)",
              marginBottom: 4,
            }}
          >
            <span style={{ minWidth: 90, fontSize: "var(--typography-size-sm)" }}>
              {entry.label}
            </span>
            <span
              aria-hidden
              style={{
                height: 10,
                borderRadius: "var(--radius-full)",
                background: "var(--color-accent)",
                width: `${Math.max(2, ((entry.count ?? 0) / maxCount) * 320)}px`,
              }}
            />
            <span
              style={{
                fontSize: "var(--typography-size-sm)",
                fontFamily: "var(--typography-mono-family)",
              }}
            >
              {entry.count ?? 0}
            </span>
          </div>
        ))}
      </div>

      {(
        [
          ["scan.largest", report.largestFiles],
          ["scan.oldest", report.oldestFiles],
          ["scan.newest", report.newestFiles],
        ] as const
      ).map(([labelKey, items]) =>
        items.length > 0 ? (
          <div key={labelKey}>
            <h3
              style={{ margin: "0 0 var(--spacing-xs) 0", fontSize: "var(--typography-size-md)" }}
            >
              {t(labelKey)}
            </h3>
            <ul
              style={{
                margin: 0,
                paddingLeft: "var(--spacing-lg)",
                fontSize: "var(--typography-size-sm)",
                lineHeight: 1.8,
              }}
            >
              {items.map((file) => (
                <li key={file.relativePath}>
                  <button
                    type="button"
                    onClick={() => onOpenFile(file.relativePath)}
                    style={{
                      ...buttonStyle,
                      border: "none",
                      padding: 0,
                      textDecoration: "underline",
                    }}
                  >
                    {file.relativePath}
                  </button>
                  {" · "}
                  {formatBytes(file.size)}
                </li>
              ))}
            </ul>
          </div>
        ) : null,
      )}

      {report.emptyDirectories.length > 0 ? (
        <div>
          <h3 style={{ margin: "0 0 var(--spacing-xs) 0", fontSize: "var(--typography-size-md)" }}>
            {t("scan.empty")} ({report.emptyDirectories.length}
            {report.emptyDirectoriesTruncated ? "+" : ""})
          </h3>
          <div
            style={{
              fontSize: "var(--typography-size-sm)",
              fontFamily: "var(--typography-mono-family)",
              lineHeight: 1.8,
            }}
          >
            {report.emptyDirectories.slice(0, 50).join(" · ")}
          </div>
        </div>
      ) : null}

      {report.warnings.length > 0 ? (
        <ul
          style={{
            margin: 0,
            paddingLeft: "var(--spacing-lg)",
            color: "var(--color-text-muted)",
            fontSize: "var(--typography-size-sm)",
          }}
        >
          {report.warnings.map((warning) => (
            <li key={warning}>{warning}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

const buttonStyle: CSSProperties = {
  height: "var(--control-height-sm)",
  padding: "0 var(--control-padding-x)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  cursor: "pointer",
};
