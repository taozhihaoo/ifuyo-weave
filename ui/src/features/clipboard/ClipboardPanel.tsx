import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type {
    ClipboardDetailDto,
    ClipboardEntryDto,
    ClipboardStatusDto,
} from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { formatTimestamp } from "../../lib/jobs";

/** M2.3：Clipboard 历史中心（§23/§70：查看/搜索/筛选/详情/重复制/收藏/删除/清空/暂停）。 */

interface ClipboardPanelProps {
    locale: string;
    onOperationDone: () => void;
}

const PAGE_SIZE = 50;

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
const kindColor: Record<string, string> = {
    text: "var(--color-accent)",
    image: "var(--color-success)",
    file: "var(--color-warning, var(--color-accent))",
    html: "var(--color-text-muted)",
    unknown: "var(--color-text-muted)",
};

function fmtSize(bytes: number | null | undefined): string {
    const v = bytes ?? 0;
    if (v < 1024) return `${v} B`;
    if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} KiB`;
    if (v < 1024 * 1024 * 1024) return `${(v / 1024 / 1024).toFixed(1)} MiB`;
    return `${(v / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}

const FILTERS = ["all", "text", "image", "file", "favorite", "sensitive"] as const;

export function ClipboardPanel({ locale, onOperationDone }: ClipboardPanelProps) {
    const t = useT();
    const [status, setStatus] = useState<ClipboardStatusDto | null>(null);
    const [items, setItems] = useState<ClipboardEntryDto[]>([]);
    const [total, setTotal] = useState(0);
    const [filter, setFilter] = useState<(typeof FILTERS)[number]>("all");
    const [query, setQuery] = useState("");
    const [offset, setOffset] = useState(0);
    const [detail, setDetail] = useState<ClipboardDetailDto | null>(null);
    const [thumbUrl, setThumbUrl] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);

    const loadStatus = useCallback(() => {
        commands
            .clipboardStatus()
            .then((r) => {
                if (r.status === "ok") setStatus(r.data);
            })
            .catch(() => {});
    }, []);

    const load = useCallback(
        (nextOffset: number, append: boolean) => {
            commands
                .clipboardList({
                    filter,
                    query,
                    offset: nextOffset,
                    limit: PAGE_SIZE,
                })
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    setTotal(r.data.total ?? 0);
                    setItems((prev) =>
                        append ? [...prev, ...r.data.items] : r.data.items,
                    );
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [filter, query],
    );

    useEffect(() => {
        loadStatus();
    }, [loadStatus]);

    useEffect(() => {
        load(0, false);
    }, [load]);

    // 轻量状态轮询（§17 用户可见的 monitoring/paused 状态）
    useEffect(() => {
        const timer = setInterval(loadStatus, 4000);
        return () => clearInterval(timer);
    }, [loadStatus]);

    const openDetail = useCallback(
        (id: string) => {
            commands
                .clipboardDetail(id)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    setDetail(r.data);
                    setThumbUrl(null);
                    if (r.data.entry.kind === "image") {
                        commands
                            .clipboardThumbnail(id)
                            .then((tr) => {
                                if (tr.status === "ok") {
                                    const bin = String.fromCharCode(...tr.data);
                                    setThumbUrl(`data:image/png;base64,${btoa(bin)}`);
                                }
                            })
                            .catch(() => {});
                    }
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [],
    );

    const recopy = useCallback(
        (id: string) => {
            commands
                .clipboardRecopy(id)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    setError(null);
                    onOperationDone();
                    load(0, false);
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [load, onOperationDone],
    );

    const remove = useCallback(
        (id: string) => {
            commands
                .clipboardDelete(id)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    if (detail?.entry.id === id) setDetail(null);
                    loadStatus();
                    load(0, false);
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [load, loadStatus, detail],
    );

    const toggleFavorite = useCallback(
        (id: string, favorite: boolean) => {
            commands
                .clipboardSetFavorite(id, favorite)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    loadStatus();
                    load(offset, offset > 0);
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [load, loadStatus, offset],
    );

    const setPaused = useCallback(
        (paused: boolean) => {
            commands
                .clipboardSetPaused(paused)
                .then((r) => {
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    loadStatus();
                })
                .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
        },
        [loadStatus],
    );

    const clearHistory = useCallback(() => {
        commands
            .clipboardClearHistory()
            .then((r) => {
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setDetail(null);
                setItems([]);
                setTotal(0);
                loadStatus();
                onOperationDone();
            })
            .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    }, [loadStatus, onOperationDone]);

    const paused = status?.paused ?? false;
    const enabled = status?.enabled ?? true;

    return (
        <section
            aria-label={t("clipboard.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <div style={rowStyle}>
                <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("clipboard.title")}</h2>
                <span
                    role="status"
                    style={{
                        fontSize: "var(--typography-size-sm)",
                        color: paused || !enabled ? "var(--color-danger)" : "var(--color-success)",
                        fontWeight: 600,
                    }}
                >
                    {enabled ? (paused ? `○ ${t("clipboard.status.paused")}` : `● ${t("clipboard.status.live")}`) : t("clipboard.status.disabled")}
                </span>
                <button type="button" style={{ ...boxStyle, marginLeft: "auto" }} onClick={() => setPaused(!paused)}>
                    {paused ? t("clipboard.resume") : t("clipboard.pause")}
                </button>
                <button type="button" style={boxStyle} onClick={clearHistory}>
                    {t("clipboard.clear")}
                </button>
            </div>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("clipboard.hint")}
            </div>

            <div style={rowStyle}>
                <input
                    aria-label={t("clipboard.search")}
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder={t("clipboard.searchPlaceholder")}
                    style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                />
                {FILTERS.map((f) => (
                    <button
                        key={f}
                        type="button"
                        onClick={() => setFilter(f)}
                        style={{
                            ...boxStyle,
                            padding: "0 var(--spacing-xs)",
                            fontWeight: filter === f ? 700 : 400,
                            borderColor: filter === f ? "var(--color-accent)" : "var(--color-border)",
                        }}
                    >
                        {t(`clipboard.filter.${f}`)}
                    </button>
                ))}
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {status ? (
                <div style={{ ...rowStyle, fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                    {t("clipboard.stats", {
                        total: status.total ?? 0,
                        storage: fmtSize(status.storageBytes),
                    })}
                    {(status.sensitiveCount ?? 0) > 0 ? ` · ${t("clipboard.sensitiveCount")}: ${status.sensitiveCount}` : ""}
                </div>
            ) : null}

            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                {items.map((entry) => (
                    <div key={entry.id} style={{ ...listRow() }}>
                        <button
                            type="button"
                            onClick={() => openDetail(entry.id)}
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)", cursor: "pointer", flex: 1, textAlign: "left", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                            title={entry.preview}
                        >
                            {entry.preview || t("clipboard.emptyPreview")}
                        </button>
                        <span style={{ color: kindColor[entry.kind] ?? "var(--color-text-muted)", fontSize: "var(--typography-size-xs)", whiteSpace: "nowrap" }}>
                            {t(`clipboard.kind.${entry.kind}`)}
                        </span>
                        {entry.sensitivity === "potentiallySensitive" ? (
                            <span style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-xs)" }}>
                                {t("clipboard.sensitiveBadge")}
                            </span>
                        ) : null}
                        <span style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-xs)", whiteSpace: "nowrap" }}>
                            {formatTimestamp(entry.updatedAtMs, locale)}
                        </span>
                        <span style={{ color: "var(--color-text-muted)", fontSize: "var(--typography-size-xs)" }}>
                            {fmtSize(entry.size)}
                        </span>
                        <button
                            type="button"
                            aria-label={t("clipboard.recopy")}
                            onClick={() => recopy(entry.id)}
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                        >
                            ⧉
                        </button>
                        <button
                            type="button"
                            aria-label={t("clipboard.favorite")}
                            onClick={() => toggleFavorite(entry.id, !entry.isFavorite)}
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                        >
                            {entry.isFavorite ? "★" : "☆"}
                        </button>
                        <button
                            type="button"
                            aria-label={t("clipboard.delete")}
                            onClick={() => remove(entry.id)}
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                        >
                            ✕
                        </button>
                    </div>
                ))}
                {items.length === 0 ? (
                    <div style={{ color: "var(--color-text-muted)", padding: "var(--spacing-sm) 0" }}>
                        {paused ? t("clipboard.empty.paused") : t("clipboard.empty")}
                    </div>
                ) : null}
                {items.length < total ? (
                    <button
                        type="button"
                        style={{ ...boxStyle, alignSelf: "flex-start" }}
                        onClick={() => {
                            const next = offset + PAGE_SIZE;
                            setOffset(next);
                            load(next, true);
                        }}
                    >
                        {t("clipboard.loadMore")}
                    </button>
                ) : null}
            </div>

            {detail ? (
                <div
                    aria-label={t("clipboard.detail.title")}
                    style={{ ...boxStyle, display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}
                >
                    <div style={rowStyle}>
                        <strong>{t("clipboard.detail.title")}</strong>
                        <button
                            type="button"
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)", marginLeft: "auto" }}
                            onClick={() => setDetail(null)}
                        >
                            ✕
                        </button>
                    </div>
                    <div style={{ fontSize: "var(--typography-size-sm)", display: "flex", flexDirection: "column", gap: 4 }}>
                        <span>{t("clipboard.detail.kind")}: {t(`clipboard.kind.${detail.entry.kind}`)}</span>
                        <span>{t("clipboard.detail.time")}: {formatTimestamp(detail.entry.createdAtMs, locale)}</span>
                        <span>{t("clipboard.detail.size")}: {fmtSize(detail.entry.size)}</span>
                        <span>{t("clipboard.detail.status")}: {detail.entry.status}</span>
                        <span style={{ fontFamily: "var(--typography-mono-family)", fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                            {detail.contentHash.slice(0, 16)}…
                        </span>
                        {(detail.entry.fileCount ?? 0) > 0 ? (
                            <div>
                                <div>{t("clipboard.detail.files", { count: detail.entry.fileCount ?? 0 })}</div>
                                {detail.fileStates.map(([path, state]) => (
                                    <div key={path} style={{ fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                                        {state === "available" ? "✓" : "✕"} {path}
                                        {state !== "available" ? ` · ${t(`clipboard.fileState.${state}`)}` : ""}
                                    </div>
                                ))}
                            </div>
                        ) : null}
                    </div>
                    {thumbUrl ? (
                        <img src={thumbUrl} alt={t("clipboard.thumbnailAlt")} style={{ maxWidth: "320px", borderRadius: "var(--radius-sm)" }} />
                    ) : null}
                    <div style={rowStyle}>
                        <button
                            type="button"
                            disabled={detail.entry.status === "missing" || detail.entry.status === "corrupted"}
                            onClick={() => recopy(detail.entry.id)}
                            style={boxStyle}
                        >
                            {t("clipboard.recopy")}
                        </button>
                        <button
                            type="button"
                            onClick={() => toggleFavorite(detail.entry.id, !detail.entry.isFavorite)}
                            style={boxStyle}
                        >
                            {detail.entry.isFavorite ? t("clipboard.unfavorite") : t("clipboard.favorite")}
                        </button>
                        {detail.entry.status === "missing" || detail.entry.status === "corrupted" ? (
                            <span style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                                {t("clipboard.cannotRestore")}
                            </span>
                        ) : null}
                    </div>
                </div>
            ) : null}
        </section>
    );
}

function listRow(): CSSProperties {
    return {
        display: "flex",
        gap: "var(--spacing-xs)",
        alignItems: "center",
        padding: "var(--spacing-xs) var(--spacing-sm)",
        borderBottom: "1px solid var(--color-border)",
    };
}
