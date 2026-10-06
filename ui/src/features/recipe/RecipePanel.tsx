import { useCallback, useEffect, useState, type CSSProperties } from "react";
import type { RecipeRunRecordDto, RecipeRunResultDto } from "../../generated/bindings";
import { commands } from "../../generated/bindings";
import { useT } from "../../i18n";

/** M2.7：Recipe 自动化中心（§37/§57：列表→配置→Preview→Run→History）。 */

interface RecipePanelProps {
    onOperationDone: () => void;
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

interface ActionRow {
    actionType: "filterByExtension" | "renamePrefix" | "archiveCreate" | "hashManifest";
    parameter: string;
    enabled: boolean;
}

export function RecipePanel({ onOperationDone }: RecipePanelProps) {
    const t = useT();
    const [recipes, setRecipes] = useState<{ id: string; name: string; enabled: boolean; risk: string; trigger: string }[]>([]);
    const [runs, setRuns] = useState<RecipeRunRecordDto[]>([]);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // 编辑器状态
    const [name, setName] = useState("");
    const [inputFolder, setInputFolder] = useState("");
    const [actions, setActions] = useState<ActionRow[]>([]);
    const [runResult, setRunResult] = useState<RecipeRunResultDto | null>(null);
    const [savedId, setSavedId] = useState<string | null>(null);

    const load = useCallback(() => {
        commands
            .recipeList()
            .then((r) => {
                if (r.status === "ok") setRecipes(r.data);
            })
            .catch(() => {});
        commands
            .recipeRunHistory()
            .then((r) => {
                if (r.status === "ok") setRuns(r.data);
            })
            .catch(() => {});
    }, []);

    useEffect(() => {
        load();
    }, [load]);

    const saveRecipe = useCallback(() => {
        if (!name.trim() || actions.length === 0) {
            setError(t("recipe.error.needsNameActions"));
            return;
        }
        setBusy(true);
        setError(null);
        const id = savedId ?? `r_${Date.now()}`;
        commands
            .recipeSave({
                id,
                name: name.trim(),
                description: "",
                enabled: true,
                inputFolder: inputFolder.trim(),
                trigger: { type: "manual" },
                parameters: [],
                actions: actions.map((a, i) => ({
                    id: `a${i}`,
                    actionType: a.actionType,
                    enabled: a.enabled,
                    parameter: a.parameter || null,
                })),
            })
            .then((r) => {
                setBusy(false);
                if (r.status !== "ok") {
                    setError(r.error.code);
                    return;
                }
                setSavedId(id);
                load();
                onOperationDone();
            })
            .catch((e: unknown) => {
                setBusy(false);
                setError(e instanceof Error ? e.message : String(e));
            });
    }, [name, inputFolder, actions, savedId, load, onOperationDone, t]);

    const runRecipe = useCallback(
        (dryRun: boolean, recipeId?: string) => {
            const id = recipeId ?? savedId;
            if (!id || !inputFolder.trim()) {
                setError(t("recipe.error.needsIdFolder"));
                return;
            }
            setBusy(true);
            setError(null);
            setRunResult(null);
            commands
                .recipeRunWithHistory({
                    id,
                    inputFolder: inputFolder.trim(),
                    dryRun,
                })
                .then((r) => {
                    setBusy(false);
                    if (r.status !== "ok") {
                        setError(r.error.code);
                        return;
                    }
                    setRunResult(r.data);
                    load();
                })
                .catch((e: unknown) => {
                    setBusy(false);
                    setError(e instanceof Error ? e.message : String(e));
                });
        },
        [savedId, inputFolder, load],
    );

    const addAction = useCallback((actionType: ActionRow["actionType"]) => {
        setActions((prev) => [
            ...prev,
            {
                actionType,
                parameter: actionType === "renamePrefix" ? "" : actionType === "filterByExtension" ? "png,jpg" : "",
                enabled: true,
            },
        ]);
    }, []);

    return (
        <section
            aria-label={t("recipe.title")}
            style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
        >
            <h2 style={{ margin: 0, fontSize: "var(--typography-size-xl)" }}>{t("recipe.title")}</h2>
            <div style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
                {t("recipe.hint")}
            </div>

            {error ? (
                <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
                    {error}
                </div>
            ) : null}

            {/* Recipe 列表（§37） */}
            {recipes.length > 0 ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                    <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("recipe.list")}</strong>
                    {recipes.map((r) => (
                        <div key={r.id} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                            <span style={{ fontWeight: 600 }}>{r.name}</span>
                            <span style={{ color: "var(--color-text-muted)" }}>{r.trigger}</span>
                            <span style={{ color: "var(--color-text-muted)" }}>{r.risk}</span>
                            <button
                                type="button"
                                disabled={busy}
                                style={{ ...boxStyle, padding: "0 var(--spacing-xs)", marginLeft: "auto" }}
                                onClick={() => {
                                    setSavedId(r.id);
                                    runRecipe(false, r.id);
                                }}
                            >
                                {t("recipe.run")}
                            </button>
                            <button
                                type="button"
                                disabled={busy}
                                style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                                onClick={() => {
                                    commands
                                        .recipeDelete(r.id)
                                        .then((d) => {
                                            if (d.status === "ok") load();
                                        })
                                        .catch(() => {});
                                }}
                            >
                                ✕
                            </button>
                        </div>
                    ))}
                </div>
            ) : null}

            {/* Builder（§38 列表式） */}
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
                <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("recipe.builder")}</strong>
                <div style={rowStyle}>
                    <input
                        aria-label={t("recipe.name")}
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder={t("recipe.namePlaceholder")}
                        style={{ ...boxStyle, flex: 1, minWidth: "10em" }}
                    />
                    <input
                        aria-label={t("recipe.inputFolder")}
                        value={inputFolder}
                        onChange={(e) => setInputFolder(e.target.value)}
                        placeholder="C:\path\to\folder"
                        style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
                    />
                </div>
                <div style={rowStyle}>
                    <select
                        aria-label={t("recipe.addAction")}
                        value=""
                        onChange={(e) => {
                            if (e.target.value) addAction(e.target.value as ActionRow["actionType"]);
                        }}
                        style={boxStyle}
                    >
                        <option value="">+ {t("recipe.addAction")}</option>
                        <option value="filterByExtension">{t("recipe.action.filter")}</option>
                        <option value="renamePrefix">{t("recipe.action.rename")}</option>
                        <option value="archiveCreate">{t("recipe.action.archive")}</option>
                        <option value="hashManifest">{t("recipe.action.manifest")}</option>
                    </select>
                </div>
                {actions.map((a, i) => (
                    <div key={i} style={{ ...rowStyle, fontSize: "var(--typography-size-sm)" }}>
                        <span style={{ fontWeight: 600 }}>{t(`recipe.action.${a.actionType}`)}</span>
                        <input
                            aria-label={t("recipe.actionParam")}
                            value={a.parameter}
                            onChange={(e) => {
                                const next = [...actions];
                                next[i] = { ...a, parameter: e.target.value };
                                setActions(next);
                            }}
                            placeholder={t("recipe.actionParam")}
                            style={{ ...boxStyle, flex: 1, minWidth: "10em" }}
                        />
                        <button
                            type="button"
                            aria-label={t("recipe.removeAction")}
                            onClick={() => setActions(actions.filter((_, j) => j !== i))}
                            style={{ ...boxStyle, padding: "0 var(--spacing-xs)" }}
                        >
                            ✕
                        </button>
                    </div>
                ))}
                <div style={rowStyle}>
                    <button type="button" disabled={busy} onClick={saveRecipe} style={boxStyle}>
                        {t("recipe.save")}
                    </button>
                    <button
                        type="button"
                        disabled={busy || !savedId}
                        onClick={() => runRecipe(true)}
                        style={boxStyle}
                    >
                        {t("recipe.dryRun")}
                    </button>
                    <button
                        type="button"
                        disabled={busy || !savedId}
                        onClick={() => runRecipe(false)}
                        style={boxStyle}
                    >
                        {t("recipe.run")}
                    </button>
                </div>
            </div>

            {runResult ? (
                <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
                    <div>
                        {t("recipe.result.status")}: {runResult.status}
                        {runResult.dryRun ? ` · ${t("recipe.result.dryRun")}` : ""}
                    </div>
                    <div style={{ color: "var(--color-text-muted)" }}>
                        {t("recipe.result.counts", {
                            matched: runResult.matchedFiles ?? 0,
                            processed: runResult.totalProcessed ?? 0,
                            skipped: runResult.totalSkipped ?? 0,
                            failed: runResult.totalFailed ?? 0,
                        })}
                    </div>
                    {runResult.blockedReason ? (
                        <div style={{ color: "var(--color-danger)" }}>⚠ {runResult.blockedReason}</div>
                    ) : null}
                    {runResult.actions.map((a) => (
                        <div key={a[0]}>
                            {a[1]}: {a[2]} ({a[3]})
                        </div>
                    ))}
                </div>
            ) : null}

            {/* Run History（§21） */}
            {runs.length > 0 ? (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
                    <strong style={{ fontSize: "var(--typography-size-sm)" }}>{t("recipe.history")}</strong>
                    {runs.slice(0, 20).map((r) => (
                        <div key={r.runId} style={{ ...rowStyle, fontSize: "var(--typography-size-xs)", color: "var(--color-text-muted)" }}>
                            <span>{r.recipeName}</span>
                            <span>{r.trigger}</span>
                            <span>{r.status}</span>
                            <span>{r.durationMs}ms</span>
                            {r.dryRun ? <span>{t("recipe.result.dryRun")}</span> : null}
                        </div>
                    ))}
                </div>
            ) : null}
        </section>
    );
}
