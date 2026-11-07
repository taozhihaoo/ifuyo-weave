import { useCallback, useState, type CSSProperties } from "react";
import type { ArchiveInspectionDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";
import { isTerminalState, pollJob } from "../../lib/jobs";

/** M2.5：Archive / Integrity 归档与完整性中心。 */

interface ArchivePanelProps {
    onOperationDone: () => void;
}

type Tab = "create" | "extract" | "integrity";

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
const areaStyle: CSSProperties = {
    fontFamily: "var(--typography-mono-family)",
    fontSize: "var(--typography-size-xs)",
    padding: "var(--spacing-sm)",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--color-border)",
    width: "100%",
    boxSizing: "border-box",
    whiteSpace: "pre-wrap",
    maxHeight: "16em",
    overflowY: "auto",
};

function fmtBytes(v: number | null | undefined): string {
    const n = v ?? 0;
    if (n < 1024) return `${n} B`;
    if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KiB`;
    if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MiB`;
    return `${(n / 1024 ** 3).toFixed(2)} GiB`;
}

export function ArchivePanel({ onOperationDone }: ArchivePanelProps) {
    const t = useT();
    const [tab, setTab] = useState<Tab>("create");

    // ── Create ──
    const [sources, setSources] = useState("");
    const [output, setOutput] = useState("");
    const [level, setLevel] = useState("normal");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [createResult, setCreateResult] = useState<string | null>(null);

    // ── Inspect / Extract ──
    const [zipPath, setZipPath] = useState("");
    const [inspection, setInspection] = useState<ArchiveInspectionDto | null>(null);
    const [destDir, setDestDir] = useState("");
    const [overwrite, setOverwrite] = useState("skip");
    const [extractResult, setExtractResult] = useState<string | null>(null);

    // ── Integrity ──
    const [hashPath, setHashPath] = useState("");
    const [expectedHash, setExpectedHash] = useState("");
    const [verifyResult, setVerifyResult] = useState<string | null>(null);
    const [manifestRoot, setManifestRoot] = useState("");
    const [manifestText, setManifestText] = useState("");
    const [manifestResult, setManifestResult] = useState<string | null>(null);

    const runJob = useCallback(
        (
            invoke: Promise<
                | { status: "ok"; data: { jobId: string } }
                | { status: "error"; error: { code: string } }
            >,
            onDone: (jobId: string) => void,
        ) => {
            setBusy(true);
            setError(null);
            void invoke.then((r) => {
                if (r.status !== "ok") {
                    setBusy(false);
                    setError(r.error?.code ?? "unknown");
                    return;
                }
                const stop = pollJob(r.data.jobId, (tick) => {
                    if (tick.kind === "failed") {
                        stop();
                        setBusy(false);
                        setError(tick.error.code);
                        return;
                    }
                    if (isTerminalState(tick.status)) {
                        stop();
                        setBusy(false);
                        onDone(r.data.jobId);
                    }
                });
            }).catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
        },
        [],
    );

    const doCreate = useCallback(() => {
        const sourceList = sources
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
        if (sourceList.length === 0 || !output.trim()) {
            setError(t("archive.error.inputs"));
            return;
        }
        runJob(
            commands.archiveCreate({
                sources: sourceList,
                output: output.trim(),
                level,
            }),
            (jobId) => {
                commands
                    .archiveGetResult(jobId)
                    .then((r) => {
                        if (r.status !== "ok") {
                            setError(r.error.code);
                            return;
                        }
                        const d = r.data;
                        setCreateResult(
                            t("archive.create.done", {
                                files: d.fileCount ?? 0,
                                input: fmtBytes(d.inputBytes),
                                outputSize: fmtBytes(d.archiveBytes),
                                ratio:
                                    d.compressionRatio != null
                                        ? ` · ${t("archive.create.ratio")}: ${(d.compressionRatio * 100).toFixed(0)}%`
                                        : "",
                            }) +
                                (d.cancelled ? ` (${t("space.cancelled")})` : "") +
                                (d.failedFiles?.length
                                    ? `\n${t("archive.create.failedFiles")}: ${d.failedFiles.length}`
                                    : ""),
                        );
                        onOperationDone();
                    })
                    .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
            },
        );
    }, [sources, output, level, onOperationDone, t, runJob]);

    const doInspect = useCallback(() => {
        setBusy(true);
        setError(null);
        setInspection(null);
        commands
            .archiveInspect(zipPath.trim())
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setInspection(r.data);
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [zipPath]);

    const doExtract = useCallback(() => {
        if (!zipPath.trim() || !destDir.trim()) {
            setError(t("archive.error.extractNeeds"));
            return;
        }
        setExtractResult(null);
        runJob(
            commands.archiveExtract({
                archive: zipPath.trim(),
                destination: destDir.trim(),
                overwrite,
                maxTotalUncompressed: 0,
            }),
            (jobId) => {
                commands
                    .archiveGetResult(jobId)
                    .then((r) => {
                        if (r.status !== "ok") {
                            setError(r.error.code);
                            return;
                        }
                        const d = r.data;
                        setExtractResult(
                            t("archive.extract.done", {
                                files: d.fileCount ?? 0,
                                bytes: fmtBytes(d.archiveBytes),
                            }),
                        );
                        onOperationDone();
                    })
                    .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
            },
        );
    }, [zipPath, destDir, overwrite, onOperationDone, t, runJob]);

    const doVerifyHash = useCallback(() => {
        setBusy(true);
        setError(null);
        setVerifyResult(null);
        commands
            .integrityVerifyFile(hashPath.trim(), expectedHash.trim())
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setVerifyResult(
                    `${t(`archive.verify.${r.data.status}`)}` +
                        (r.data.actualHash ? ` · ${r.data.actualHash.slice(0, 24)}…` : ""),
                );
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [hashPath, expectedHash, t]);

    const doCreateManifest = useCallback(() => {
        setBusy(true);
        setError(null);
        setManifestResult(null);
        commands
            .integrityCreateManifest(manifestRoot.trim())
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setManifestText(r.data.text);
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [manifestRoot]);

    const doVerifyManifest = useCallback(() => {
        if (!manifestRoot.trim() || !manifestText.trim()) {
            setError(t("archive.error.manifestNeeds"));
            return;
        }
        setBusy(true);
        setError(null);
        setManifestResult(null);
        commands
            .integrityVerifyManifest({
                root: manifestRoot.trim(),
                manifestText: manifestText,
            })
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setManifestResult(
                    t("archive.manifest.result", {
                        same: r.data.same ?? 0,
                        modified: r.data.modified ?? 0,
                        added: r.data.added ?? 0,
                        removed: r.data.removed ?? 0,
                        unreadable: r.data.unreadable ?? 0,
                    }) +
                        "\n" +
                        r.data.items
                            .filter(([, kind]) => kind !== "same")
                            .map(([path, kind]) => `${kind}: ${path}`)
                            .join("\n"),
                );
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [manifestRoot, manifestText, t]);

    const tabs: [Tab, string][] = [
        ["create", t("archive.tab.create")],
        ["extract", t("archive.tab.extract")],
        ["integrity", t("archive.tab.integrity")],
    ];

    return (
        <section
            aria-label={t("archive.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <div style={rowStyle}>
                <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("archive.title")}</h2>
                {tabs.map(([id, label]) => (
                    <button
                        key={id}
                        type="button"
                        onClick={() => setTab(id)}
                        style={{
                            ...boxStyle,
                            padding: "0 var(--spacing-xs)",
                            fontWeight: tab === id ? 700 : 400,
                            borderColor: tab === id ? "var(--color-accent)" : "var(--color-border)",
                        }}
                    >
                        {label}
                    </button>
                ))}
            </div>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("archive.hint")}
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {tab === "create" ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
                    <textarea
                        aria-label={t("archive.sources")}
                        value={sources}
                        onChange={(e) => setSources(e.target.value)}
                        rows={4}
                        placeholder={"C:\\path\\file1.txt\nC:\\path\\folder"}
                        style={areaStyle}
                    />
                    <div style={rowStyle}>
                        <input
                            aria-label={t("archive.output")}
                            value={output}
                            onChange={(e) => setOutput(e.target.value)}
                            placeholder="D:\Archives\out.zip"
                            style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                        />
                        <select aria-label={t("archive.level")} value={level} onChange={(e) => setLevel(e.target.value)} style={boxStyle}>
                            <option value="store">{t("archive.level.store")}</option>
                            <option value="fast">{t("archive.level.fast")}</option>
                            <option value="normal">{t("archive.level.normal")}</option>
                            <option value="maximum">{t("archive.level.maximum")}</option>
                        </select>
                        <button type="button" disabled={busy} onClick={doCreate} style={boxStyle}>
                            {t("archive.create")}
                        </button>
                    </div>
                    {createResult ? (
                        <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                            {createResult}
                        </div>
                    ) : null}
                </div>
            ) : null}

            {tab === "extract" ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
                    <div style={rowStyle}>
                        <input
                            aria-label={t("archive.zipPath")}
                            value={zipPath}
                            onChange={(e) => setZipPath(e.target.value)}
                            placeholder="D:\Archives\out.zip"
                            style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                        />
                        <button type="button" disabled={busy || !zipPath.trim()} onClick={doInspect} style={boxStyle}>
                            {t("archive.inspect")}
                        </button>
                    </div>
                    {inspection ? (
                        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                            {t("archive.inspect.summary", {
                                entries: inspection.entryCount ?? 0,
                                files: inspection.fileCount ?? 0,
                                dirs: inspection.dirCount ?? 0,
                            })}
                            {" · "}
                            {fmtBytes(inspection.totalUncompressed)} → {fmtBytes(inspection.totalCompressed)}
                            {inspection.duplicateEntries.length > 0
                                ? ` · ${t("archive.inspect.duplicates")}: ${inspection.duplicateEntries.length}`
                                : ""}
                            {inspection.unsafeEntries.length > 0
                                ? ` · ⚠ ${t("archive.inspect.unsafe")}: ${inspection.unsafeEntries.length}`
                                : ""}
                            {inspection.bombSuspect ? ` · ⚠ ${t("archive.inspect.bomb")}` : ""}
                            <div style={{ maxHeight: "12em", overflowY: "auto", marginTop: "var(--spacing-xs)" }}>
                                {inspection.entries.slice(0, 100).map((e) => (
                                    <div key={e.name}>
                                        {e.isDir ? "📁" : "📄"} {e.name} · {fmtBytes(e.uncompressedSize)} →{" "}
                                        {fmtBytes(e.compressedSize)}
                                    </div>
                                ))}
                                {(inspection.entries.length ?? 0) > 100
                                    ? t("archive.inspect.truncated")
                                    : null}
                            </div>
                        </div>
                    ) : null}
                    <div style={rowStyle}>
                        <input
                            aria-label={t("archive.dest")}
                            value={destDir}
                            onChange={(e) => setDestDir(e.target.value)}
                            placeholder="C:\path\to\extract"
                            style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                        />
                        <select aria-label={t("archive.overwrite")} value={overwrite} onChange={(e) => setOverwrite(e.target.value)} style={boxStyle}>
                            <option value="skip">{t("archive.overwrite.skip")}</option>
                            <option value="overwrite">{t("archive.overwrite.overwrite")}</option>
                            <option value="rename">{t("archive.overwrite.rename")}</option>
                        </select>
                        <button type="button" disabled={busy} onClick={doExtract} style={boxStyle}>
                            {t("archive.extract")}
                        </button>
                    </div>
                    {extractResult ? (
                        <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
                            {extractResult}
                        </div>
                    ) : null}
                </div>
            ) : null}

            {tab === "integrity" ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
                        <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("archive.verifyFile")}</strong>
                        <div style={rowStyle}>
                            <input
                                aria-label={t("archive.filePath")}
                                value={hashPath}
                                onChange={(e) => setHashPath(e.target.value)}
                                placeholder="C:\path\file.txt"
                                style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                            />
                            <input
                                aria-label={t("archive.expectedHash")}
                                value={expectedHash}
                                onChange={(e) => setExpectedHash(e.target.value)}
                                placeholder="SHA-256…"
                                style={{ ...boxStyle, flex: 1, minWidth: "16em", fontFamily: "var(--typography-mono-family)" }}
                            />
                            <button type="button" disabled={busy || !hashPath.trim()} onClick={doVerifyHash} style={boxStyle}>
                                {t("archive.verify")}
                            </button>
                        </div>
                        {verifyResult ? (
                            <div role="status" style={{ fontSize: "var(--typography-size-sm)", fontWeight: 600 }}>
                                {verifyResult}
                            </div>
                        ) : null}
                    </div>

                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
                        <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("archive.manifest.title")}</strong>
                        <div style={rowStyle}>
                            <input
                                aria-label={t("archive.manifest.root")}
                                value={manifestRoot}
                                onChange={(e) => setManifestRoot(e.target.value)}
                                placeholder="C:\path\to\folder"
                                style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                            />
                            <button type="button" disabled={busy || !manifestRoot.trim()} onClick={doCreateManifest} style={boxStyle}>
                                {t("archive.manifest.create")}
                            </button>
                            <button type="button" disabled={busy || !manifestText.trim()} onClick={doVerifyManifest} style={boxStyle}>
                                {t("archive.manifest.verify")}
                            </button>
                        </div>
                        <textarea
                            aria-label={t("archive.manifest.text")}
                            value={manifestText}
                            onChange={(e) => setManifestText(e.target.value)}
                            rows={6}
                            placeholder={"sha256hash  relative/path.txt\n…"}
                            style={areaStyle}
                        />
                        {manifestResult ? (
                            <pre style={areaStyle}>{manifestResult}</pre>
                        ) : null}
                    </div>
                </div>
            ) : null}
        </section>
    );
}
