import type { CSSProperties, ReactNode } from "react";
import type { FileInspectionDto, HashResultDto, IpcError } from "../../generated/bindings";
import { useT } from "../../i18n";
import { formatBytes } from "../../lib/format";
import { formatTimestamp } from "../../lib/jobs";

export type HashJobView =
  | { phase: "idle" }
  | { phase: "running"; processed: number | null }
  | { phase: "done"; result: HashResultDto }
  | { phase: "failed"; error: IpcError };

interface InspectorPanelProps {
  inspection: FileInspectionDto;
  locale: string;
  hash: HashJobView;
  onStartHash: () => void;
  onCancelHash: () => void;
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: "flex", gap: "var(--spacing-md)", lineHeight: 1.9 }}>
      <span
        style={{
          minWidth: 120,
          color: "var(--color-text-muted)",
          fontSize: "var(--typography-size-sm)",
        }}
      >
        {label}
      </span>
      <span style={{ fontFamily: "var(--typography-mono-family)" }}>{children}</span>
    </div>
  );
}

export function InspectorPanel({
  inspection,
  locale,
  hash,
  onStartHash,
  onCancelHash,
}: InspectorPanelProps) {
  const t = useT();
  const textLike = inspection.encoding !== null;

  return (
    <section
      aria-label={t("inspector.title")}
      style={{
        border: "1px solid var(--color-border)",
        borderRadius: "var(--radius-md)",
        padding: "var(--spacing-lg)",
        background: "var(--color-surface)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--spacing-sm)",
      }}
    >
      <header style={{ display: "flex", alignItems: "baseline", gap: "var(--spacing-md)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{inspection.name}</h2>
        <span style={{ color: "var(--color-text-muted)" }}>
          {t(`inspector.kind.${inspection.kind}`)}
        </span>
        {inspection.status === "partial" ? (
          <span
            role="status"
            style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-danger)" }}
          >
            {t("inspector.partial")}
          </span>
        ) : null}
      </header>

      <Row label="Path">{inspection.normalizedPath}</Row>
      <Row label={t("inspector.size")}>{formatBytes(inspection.size)}</Row>
      <Row label={t("inspector.type")}>
        {t(`inspector.category.${inspection.classification.category}`)}
        {inspection.classification.mime ? ` · ${inspection.classification.mime}` : ""}
      </Row>
      <Row label={t("inspector.created")}>{formatTimestamp(inspection.createdMs, locale)}</Row>
      <Row label={t("inspector.modified")}>{formatTimestamp(inspection.modifiedMs, locale)}</Row>
      <Row label={t("inspector.accessed")}>{formatTimestamp(inspection.accessedMs, locale)}</Row>
      <Row label={t("inspector.attributes")}>
        {inspection.readonly ? t("inspector.readonly") : t("inspector.writable")}
        {inspection.hidden === true ? ` · ${t("inspector.hidden")}` : ""}
      </Row>
      <Row label={t("inspector.encoding")}>
        {textLike
          ? t(`inspector.encodingValue.${inspection.encoding ?? "unknown"}`)
          : t("inspector.encodingNotApplicable")}
      </Row>

      <div
        style={{
          borderTop: "1px solid var(--color-border)",
          marginTop: "var(--spacing-sm)",
          paddingTop: "var(--spacing-sm)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--spacing-sm)",
        }}
      >
        <div style={{ display: "flex", gap: "var(--spacing-md)", alignItems: "center" }}>
          <strong>SHA-256</strong>
          {hash.phase === "idle" ? (
            <button type="button" onClick={onStartHash} style={buttonStyle}>
              {t("inspector.hash.calculate")}
            </button>
          ) : null}
          {hash.phase === "running" ? (
            <>
              <span role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                {t("inspector.hash.calculating")}
                {hash.processed !== null ? ` · ${formatBytes(hash.processed)}` : ""}
              </span>
              <button type="button" onClick={onCancelHash} style={buttonStyle}>
                {t("inspector.hash.cancel")}
              </button>
            </>
          ) : null}
        </div>
        {hash.phase === "done" ? (
          hash.result.status === "cancelled" ? (
            <div role="status" style={{ color: "var(--color-text-muted)" }}>
              {t("inspector.hash.cancelled")}
            </div>
          ) : (
            <Row label={t("inspector.hash.digest")}>
              <span style={{ wordBreak: "break-all" }}>{hash.result.digestHex}</span>
            </Row>
          )
        ) : null}
        {hash.phase === "done" && hash.result.status === "unstable" ? (
          <div
            role="alert"
            style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
          >
            {t("inspector.hash.unstable")}
          </div>
        ) : null}
        {hash.phase === "failed" ? (
          <div
            role="alert"
            style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}
          >
            {hash.error.code} · {hash.error.message}
          </div>
        ) : null}
      </div>

      {inspection.warnings.length > 0 ? (
        <ul
          style={{
            margin: 0,
            paddingLeft: "var(--spacing-lg)",
            color: "var(--color-text-muted)",
            fontSize: "var(--typography-size-sm)",
          }}
        >
          {inspection.warnings.map((warning) => (
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
