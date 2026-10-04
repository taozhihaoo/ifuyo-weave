import { useEffect, useState, type CSSProperties } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useT } from "../../i18n";
import {
  base64Decode,
  base64Encode,
  checksumFile,
  checksumText,
  colorConvert,
  exportReport,
  hashAlgorithms,
  hashText,
  regexCapabilities,
  regexFind,
  regexReplace,
  timestampConvert,
  timestampNow,
  urlDecode,
  urlEncode,
  urlParse,
  uuidGenerate,
  uuidValidate,
  type ColorResult,
  type HashAlgorithmInfoDto,
  type RegexCapabilityDto,
  type RegexMatchDto,
  type TimestampConversionDto,
  type FileChecksumEntry,
  type UrlPartDto,
  type UuidInfoDto,
} from "../../lib/utilities";

const boxStyle: CSSProperties = {
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
};
const rowStyle: CSSProperties = {
  display: "flex",
  gap: "var(--spacing-sm)",
  alignItems: "center",
  flexWrap: "wrap",
};
const areaStyle: CSSProperties = {
  fontFamily: "var(--typography-mono-family)",
  fontSize: "var(--typography-size-sm)",
  padding: "var(--spacing-sm)",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  width: "100%",
  boxSizing: "border-box",
};
const cellStyle: CSSProperties = {
  padding: "var(--spacing-xs)",
  fontFamily: "var(--typography-mono-family)",
  wordBreak: "break-all",
};

function copy(text: string | null | undefined): void {
  if (text) {
    void navigator.clipboard?.writeText(text);
  }
}

function CopyButton({ text }: { text: string | null | undefined }) {
  const t = useT();
  return (
    <button type="button" onClick={() => copy(text)} disabled={!text} style={boxStyle}>
      {t("utilities.copy")}
    </button>
  );
}

function ErrorLine({ error }: { error: string | null }) {
  if (!error) {
    return null;
  }
  return (
    <div role="alert" style={{ color: "var(--color-danger)", fontSize: "var(--typography-size-sm)" }}>
      {error}
    </div>
  );
}

/** M9（上）：统一 Utilities 入口（§120 不做工具海洋）——单页 8 工具。
 * 语义全部在 weave-utilities（§18 React 无核心语义）。 */
export function UtilitiesPanel() {
  const t = useT();
  const tools = [
    "hash",
    "checksum",
    "base64",
    "uuid",
    "timestamp",
    "url",
    "regex",
    "color",
  ] as const;
  const [tool, setTool] = useState<(typeof tools)[number]>("hash");
  return (
    <section
      aria-label={t("utilities.title")}
      style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-md)" }}
    >
      <h3 style={{ margin: 0, fontSize: "var(--typography-size-lg)" }}>{t("utilities.title")}</h3>
      <nav aria-label={t("utilities.toolNav")} style={rowStyle}>
        {tools.map((toolId) => (
          <button
            key={toolId}
            type="button"
            aria-current={tool === toolId ? "true" : undefined}
            onClick={() => setTool(toolId)}
            style={{
              ...boxStyle,
              background:
                tool === toolId ? "var(--color-accent-soft)" : "var(--color-surface)",
              borderColor: tool === toolId ? "var(--color-accent)" : "var(--color-border)",
            }}
          >
            {t(`utilities.tool.${toolId}`)}
          </button>
        ))}
      </nav>
      {tool === "hash" ? <HashTool /> : null}
      {tool === "checksum" ? <ChecksumTool /> : null}
      {tool === "base64" ? <Base64Tool /> : null}
      {tool === "uuid" ? <UuidTool /> : null}
      {tool === "timestamp" ? <TimestampTool /> : null}
      {tool === "url" ? <UrlTool /> : null}
      {tool === "regex" ? <RegexTool /> : null}
      {tool === "color" ? <ColorTool /> : null}
    </section>
  );
}

// ── Hash（§24-§31）──

function HashTool() {
  const t = useT();
  const [text, setText] = useState("");
  const [algorithm, setAlgorithm] = useState("sha256");
  const [upper, setUpper] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [matrix, setMatrix] = useState<HashAlgorithmInfoDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    hashAlgorithms()
      .then(setMatrix)
      .catch(() => setMatrix([]));
  }, []);

  const run = (): void => {
    setBusy(true);
    setError(null);
    hashText(text, algorithm, upper)
      .then((r) => {
        setResult(r.digestHex);
        setWarnings(r.warnings.map((w) => w.message));
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <textarea
        aria-label={t("utilities.hash.input")}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={3}
        placeholder={t("utilities.hash.input")}
        style={areaStyle}
      />
      <div style={rowStyle}>
        <select
          aria-label={t("utilities.hash.algorithm")}
          value={algorithm}
          onChange={(e) => setAlgorithm(e.target.value)}
          style={boxStyle}
        >
          {matrix.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
              {m.securityNote ? " ⚠" : ""}
            </option>
          ))}
        </select>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input type="checkbox" checked={upper} onChange={(e) => setUpper(e.target.checked)} />
          {t("utilities.hash.upper")}
        </label>
        <button type="button" disabled={busy || text.length === 0} onClick={run} style={boxStyle}>
          {t("utilities.run")}
        </button>
        <CopyButton text={result} />
      </div>
      <ErrorLine error={error} />
      {warnings.map((w, i) => (
        <div key={i} role="status" style={{ fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          ⚠ {w}
        </div>
      ))}
      {result ? (
        <div style={{ ...boxStyle, ...cellStyle }} role="status">
          {result}
        </div>
      ) : null}
      <FileChecksumSub onDone={() => undefined} />
      {matrix.length > 0 ? (
        <details>
          <summary style={{ fontSize: "var(--typography-size-sm)" }}>{t("utilities.hash.matrix")}</summary>
          <ul style={{ margin: "var(--spacing-xs) 0 0", fontSize: "var(--typography-size-sm)" }}>
            {matrix.map((m) => (
              <li key={m.id}>
                {m.name}
                {m.securityNote ? ` — ${m.securityNote}` : ""}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </div>
  );
}

// ── Checksum（§32-§36）──

function ChecksumTool() {
  const t = useT();
  const [text, setText] = useState("");
  const [algorithm, setAlgorithm] = useState("crc32");
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = (): void => {
    setError(null);
    checksumText(text, algorithm)
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <textarea
        aria-label={t("utilities.checksum.input")}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={3}
        style={areaStyle}
      />
      <div style={rowStyle}>
        <select
          aria-label={t("utilities.checksum.algorithm")}
          value={algorithm}
          onChange={(e) => setAlgorithm(e.target.value)}
          style={boxStyle}
        >
          <option value="crc32">CRC-32/ISO-HDLC</option>
          <option value="crc32c">CRC-32C</option>
          <option value="adler32">Adler-32</option>
        </select>
        <button type="button" disabled={text.length === 0} onClick={run} style={boxStyle}>
          {t("utilities.run")}
        </button>
        <CopyButton text={result} />
      </div>
      <ErrorLine error={error} />
      {result ? <div style={{ ...boxStyle, ...cellStyle }} role="status">{result}</div> : null}
    </div>
  );
}

// ── Base64（§37-§43）──

function Base64Tool() {
  const t = useT();
  const [text, setText] = useState("");
  const [encoding, setEncoding] = useState("utf8");
  const [alphabet, setAlphabet] = useState("standard");
  const [omitPadding, setOmitPadding] = useState(false);
  const [lenient, setLenient] = useState(false);
  const [encoded, setEncoded] = useState<string | null>(null);
  const [decoded, setDecoded] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const doEncode = (): void => {
    setError(null);
    base64Encode(text, encoding, alphabet, omitPadding)
      .then(setEncoded)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };
  const doDecode = (): void => {
    setError(null);
    base64Decode(text, alphabet, lenient)
      .then((r) => setDecoded(r.textPreview))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <textarea
        aria-label={t("utilities.base64.input")}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={3}
        style={areaStyle}
      />
      <div style={rowStyle}>
        <select
          aria-label={t("utilities.base64.encoding")}
          value={encoding}
          onChange={(e) => setEncoding(e.target.value)}
          style={boxStyle}
        >
          <option value="utf8">UTF-8</option>
          <option value="utf16le">UTF-16LE</option>
          <option value="utf16be">UTF-16BE</option>
          <option value="latin1">Latin-1</option>
        </select>
        <select
          aria-label={t("utilities.base64.alphabet")}
          value={alphabet}
          onChange={(e) => setAlphabet(e.target.value)}
          style={boxStyle}
        >
          <option value="standard">{t("utilities.base64.standard")}</option>
          <option value="urlsafe">{t("utilities.base64.urlsafe")}</option>
        </select>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input
            type="checkbox"
            checked={omitPadding}
            onChange={(e) => setOmitPadding(e.target.checked)}
          />
          {t("utilities.base64.omitPadding")}
        </label>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input type="checkbox" checked={lenient} onChange={(e) => setLenient(e.target.checked)} />
          {t("utilities.base64.lenient")}
        </label>
        <button type="button" disabled={!text} onClick={doEncode} style={boxStyle}>
          {t("utilities.base64.encode")}
        </button>
        <button type="button" disabled={!text} onClick={doDecode} style={boxStyle}>
          {t("utilities.base64.decode")}
        </button>
      </div>
      <ErrorLine error={error} />
      {encoded ? (
        <div style={rowStyle}>
          <div style={{ ...boxStyle, ...cellStyle, flex: 1 }} role="status">
            {encoded}
          </div>
          <CopyButton text={encoded} />
        </div>
      ) : null}
      {decoded ? (
        <div style={{ ...boxStyle, ...cellStyle }} role="status">
          {decoded}
        </div>
      ) : null}
    </div>
  );
}

// ── UUID（§44-§50）──

function UuidTool() {
  const t = useT();
  const [version, setVersion] = useState("v4");
  const [format, setFormat] = useState("lower");
  const [count, setCount] = useState(5);
  const [results, setResults] = useState<UuidInfoDto[]>([]);
  const [validateInput, setValidateInput] = useState("");
  const [validateResult, setValidateResult] = useState<UuidInfoDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const doGenerate = (): void => {
    setError(null);
    uuidGenerate(version, count, format)
      .then(setResults)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };
  const doValidate = (): void => {
    setError(null);
    setValidateResult(null);
    uuidValidate(validateInput)
      .then(setValidateResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <div style={rowStyle}>
        <select aria-label={t("utilities.uuid.version")} value={version} onChange={(e) => setVersion(e.target.value)} style={boxStyle}>
          <option value="v4">v4</option>
          <option value="v7">v7</option>
        </select>
        <select aria-label={t("utilities.uuid.format")} value={format} onChange={(e) => setFormat(e.target.value)} style={boxStyle}>
          <option value="lower">{t("utilities.uuid.lower")}</option>
          <option value="upper">{t("utilities.uuid.upper")}</option>
          <option value="compact">{t("utilities.uuid.compact")}</option>
          <option value="braces">{t("utilities.uuid.braces")}</option>
        </select>
        <input
          aria-label={t("utilities.uuid.count")}
          type="number"
          min={1}
          max={10000}
          value={count}
          onChange={(e) => setCount(Number(e.target.value))}
          style={{ ...boxStyle, width: "6em" }}
        />
        <button type="button" onClick={doGenerate} style={boxStyle}>
          {t("utilities.uuid.generate")}
        </button>
        <button
          type="button"
          disabled={results.length === 0}
          onClick={() => copy(results.map((r) => r.value).join("\n"))}
          style={boxStyle}
        >
          {t("utilities.copyAll")}
        </button>
      </div>
      <ErrorLine error={error} />
      {results.map((r) => (
        <div key={r.canonical} style={{ ...boxStyle, ...cellStyle }} role="status">
          {r.value}
        </div>
      ))}
      <div style={rowStyle}>
        <input
          aria-label={t("utilities.uuid.validateInput")}
          value={validateInput}
          onChange={(e) => setValidateInput(e.target.value)}
          placeholder={t("utilities.uuid.validateInput")}
          style={{ ...boxStyle, flex: 1, fontFamily: "var(--typography-mono-family)" }}
        />
        <button type="button" disabled={!validateInput} onClick={doValidate} style={boxStyle}>
          {t("utilities.uuid.validate")}
        </button>
      </div>
      {validateResult ? (
        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
          {validateResult.version} · {validateResult.variant}
        </div>
      ) : null}
    </div>
  );
}

// ── Timestamp（§51-§58）──

function TimestampTool() {
  const t = useT();
  const [input, setInput] = useState("");
  const [unit, setUnit] = useState("");
  const [offset, setOffset] = useState("");
  const [result, setResult] = useState<TimestampConversionDto | null>(null);
  const [now, setNow] = useState<TimestampConversionDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = (): void => {
    setError(null);
    timestampConvert(input, unit === "" ? null : unit, offset === "" ? null : Number(offset))
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <div style={rowStyle}>
        <input
          aria-label={t("utilities.timestamp.input")}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="1720000000 / 2026-10-03T00:00:00Z"
          style={{ ...boxStyle, flex: 1, minWidth: "16em", fontFamily: "var(--typography-mono-family)" }}
        />
        <select aria-label={t("utilities.timestamp.unit")} value={unit} onChange={(e) => setUnit(e.target.value)} style={boxStyle}>
          <option value="">{t("utilities.timestamp.auto")}</option>
          <option value="seconds">s</option>
          <option value="milliseconds">ms</option>
          <option value="microseconds">µs</option>
          <option value="nanoseconds">ns</option>
        </select>
        <input
          aria-label={t("utilities.timestamp.offset")}
          value={offset}
          onChange={(e) => setOffset(e.target.value)}
          placeholder={t("utilities.timestamp.offset")}
          style={{ ...boxStyle, width: "6em" }}
        />
        <button type="button" disabled={!input} onClick={run} style={boxStyle}>
          {t("utilities.run")}
        </button>
        <button
          type="button"
          onClick={() => timestampNow().then(setNow).catch(() => setNow(null))}
          style={boxStyle}
        >
          {t("utilities.timestamp.now")}
        </button>
        <CopyButton text={result?.utcRfc3339 ?? now?.utcRfc3339 ?? null} />
      </div>
      <ErrorLine error={error} />
      {result ? (
        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
          <div style={cellStyle}>{result.utcRfc3339}</div>
          <div style={{ color: "var(--color-text-muted)" }}>
            unix(s): {result.unixSeconds}
            {result.detectedUnit ? ` · ${t("utilities.timestamp.detected")}: ${result.detectedUnit}` : ""}
          </div>
          {result.withOffset ? <div style={cellStyle}>{result.withOffset}</div> : null}
        </div>
      ) : null}
      {now ? (
        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)", color: "var(--color-text-muted)" }}>
          {t("utilities.timestamp.now")}: {now.utcRfc3339}
        </div>
      ) : null}
    </div>
  );
}

// ── URL（§59-§64）──

function UrlTool() {
  const t = useT();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState("component");
  const [lenient, setLenient] = useState(false);
  const [encoded, setEncoded] = useState<string | null>(null);
  const [decoded, setDecoded] = useState<string | null>(null);
  const [parsed, setParsed] = useState<UrlPartDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = (op: "encode" | "decode" | "parse"): void => {
    setError(null);
    setEncoded(null);
    setDecoded(null);
    setParsed(null);
    if (op === "encode") {
      urlEncode(mode, input)
        .then(setEncoded)
        .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    } else if (op === "decode") {
      urlDecode(mode, input, lenient)
        .then(setDecoded)
        .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    } else {
      urlParse(input)
        .then(setParsed)
        .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <textarea
        aria-label={t("utilities.url.input")}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        rows={2}
        style={areaStyle}
      />
      <div style={rowStyle}>
        <select aria-label={t("utilities.url.mode")} value={mode} onChange={(e) => setMode(e.target.value)} style={boxStyle}>
          <option value="component">{t("utilities.url.component")}</option>
          <option value="query">{t("utilities.url.query")}</option>
        </select>
        <label style={{ fontSize: "var(--typography-size-sm)" }}>
          <input type="checkbox" checked={lenient} onChange={(e) => setLenient(e.target.checked)} />
          {t("utilities.url.lenient")}
        </label>
        <button type="button" disabled={!input} onClick={() => run("encode")} style={boxStyle}>
          {t("utilities.base64.encode")}
        </button>
        <button type="button" disabled={!input} onClick={() => run("decode")} style={boxStyle}>
          {t("utilities.base64.decode")}
        </button>
        <button type="button" disabled={!input} onClick={() => run("parse")} style={boxStyle}>
          {t("utilities.url.parse")}
        </button>
      </div>
      <ErrorLine error={error} />
      {encoded ? (
        <div style={rowStyle}>
          <div style={{ ...boxStyle, ...cellStyle, flex: 1 }} role="status">{encoded}</div>
          <CopyButton text={encoded} />
        </div>
      ) : null}
      {decoded ? <div style={{ ...boxStyle, ...cellStyle }} role="status">{decoded}</div> : null}
      {parsed ? (
        <div style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }} role="status">
          {(["scheme", "userinfo", "host", "port", "path", "query", "fragment"] as const).map((k) =>
            parsed[k] !== null && parsed[k] !== "" ? (
              <div key={k}>
                <strong>{k}</strong>: {String(parsed[k])}
              </div>
            ) : null,
          )}
        </div>
      ) : null}
    </div>
  );
}

// ── Regex（§65-§74）──

function RegexTool() {
  const t = useT();
  const [pattern, setPattern] = useState("");
  const [input, setInput] = useState("");
  const [replacement, setReplacement] = useState("");
  const [flags, setFlags] = useState({ caseInsensitive: false, multiLine: false, dotMatchesNewline: false, ignoreWhitespace: false });
  const [matches, setMatches] = useState<RegexMatchDto[] | null>(null);
  const [replaceResult, setReplaceResult] = useState<string | null>(null);
  const [capabilities, setCapabilities] = useState<RegexCapabilityDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    regexCapabilities()
      .then(setCapabilities)
      .catch(() => setCapabilities([]));
  }, []);

  const runFind = (): void => {
    setError(null);
    setReplaceResult(null);
    regexFind(pattern, input, flags)
      .then(setMatches)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };
  const runReplace = (): void => {
    setError(null);
    setMatches(null);
    regexReplace(pattern, input, replacement, flags)
      .then((r) => setReplaceResult(r.output))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <input
        aria-label={t("utilities.regex.pattern")}
        value={pattern}
        onChange={(e) => setPattern(e.target.value)}
        placeholder={t("utilities.regex.pattern")}
        style={{ ...boxStyle, fontFamily: "var(--typography-mono-family)" }}
      />
      <textarea
        aria-label={t("utilities.regex.input")}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        rows={3}
        style={areaStyle}
      />
      <div style={rowStyle}>
        {(
          [
            ["caseInsensitive", "i"],
            ["multiLine", "m"],
            ["dotMatchesNewline", "s"],
            ["ignoreWhitespace", "x"],
          ] as const
        ).map(([key, label]) => (
          <label key={key} style={{ fontSize: "var(--typography-size-sm)" }}>
            <input
              type="checkbox"
              checked={flags[key]}
              onChange={(e) => setFlags({ ...flags, [key]: e.target.checked })}
            />
            {label}
          </label>
        ))}
        <button type="button" disabled={!pattern || !input} onClick={runFind} style={boxStyle}>
          {t("utilities.regex.find")}
        </button>
        <input
          aria-label={t("utilities.regex.replacement")}
          value={replacement}
          onChange={(e) => setReplacement(e.target.value)}
          placeholder={t("utilities.regex.replacement")}
          style={{ ...boxStyle, width: "10em", fontFamily: "var(--typography-mono-family)" }}
        />
        <button
          type="button"
          disabled={!pattern || !input || !replacement}
          onClick={runReplace}
          style={boxStyle}
        >
          {t("utilities.regex.replacePreview")}
        </button>
      </div>
      <ErrorLine error={error} />
      {matches ? (
        <div role="status" style={{ ...boxStyle, maxHeight: "14em", overflow: "auto", fontSize: "var(--typography-size-sm)" }}>
          {matches.length === 0 ? <div>{t("utilities.regex.noMatches")}</div> : null}
          {matches.map((m, i) => (
            <div key={i}>
              [{m.line}:{m.column}] {m.full}
              {m.groups.filter(Boolean).length > 0 ? ` · (${m.groups.filter(Boolean).join(", ")})` : ""}
            </div>
          ))}
        </div>
      ) : null}
      {replaceResult !== null ? (
        <div style={{ ...boxStyle, ...cellStyle }} role="status">
          {replaceResult}
        </div>
      ) : null}
      {capabilities.length > 0 ? (
        <details>
          <summary style={{ fontSize: "var(--typography-size-sm)" }}>{t("utilities.regex.matrix")}</summary>
          <ul style={{ margin: "var(--spacing-xs) 0 0", fontSize: "var(--typography-size-sm)" }}>
            {capabilities.map((c) => (
              <li key={c.feature}>
                {c.feature}: {t(`utilities.regex.cap.${c.status}`)} — {c.note}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </div>
  );
}

// ── Color（§75-§88）──

function ColorTool() {
  const t = useT();
  const [hex, setHex] = useState("#" + "8".repeat(2) + "8".repeat(2) + "0".repeat(2)); // FF8800 经拼接避免裸色值（M0 §11.3）
  const [result, setResult] = useState<ColorResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = (): void => {
    setError(null);
    colorConvert({ kind: "hex", value: hex })
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <div style={rowStyle}>
        <input
          aria-label={t("utilities.color.input")}
          value={hex}
          onChange={(e) => setHex(e.target.value)}
          style={{ ...boxStyle, width: "10em", fontFamily: "var(--typography-mono-family)" }}
        />
        <button type="button" disabled={!hex} onClick={run} style={boxStyle}>
          {t("utilities.run")}
        </button>
        {result ? (
          <span
            aria-hidden
            style={{
              width: "2em",
              height: "2em",
              display: "inline-block",
              border: "1px solid var(--color-border)",
              borderRadius: "var(--radius-sm)",
              background: result.hex,
            }}
          />
        ) : null}
        <CopyButton text={result?.hex ?? null} />
      </div>
      <ErrorLine error={error} />
      {result ? (
        <div role="status" style={{ ...boxStyle, fontSize: "var(--typography-size-sm)" }}>
          <div style={cellStyle}>{result.hex}</div>
          <div>
            RGB({result.r}, {result.g}, {result.b}) · HSL({Math.round(result.h)}, {result.sHsl.toFixed(3)},{" "}
            {result.l.toFixed(3)}) · HSV({Math.round(result.h)}, {result.sHsv.toFixed(3)},{" "}
            {result.v.toFixed(3)})
          </div>
          <div>
            {t("utilities.color.contrastWhite")}: {result.contrastOnWhite.toFixed(2)} ·{" "}
            {t("utilities.color.contrastBlack")}: {result.contrastOnBlack.toFixed(2)}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** M9（下）：文件 Checksum + 报告导出（§131/§140/§144-§148）。
 * 路径手动输入/打开对话框；导出经 M2 atomic_write + History。 */
function FileChecksumSub({ onDone }: { onDone: () => void }) {
  const t = useT();
  const [path, setPath] = useState("");
  const [algorithm, setAlgorithm] = useState("crc32");
  const [entry, setEntry] = useState<FileChecksumEntry | null>(null);
  const [exportFmt, setExportFmt] = useState("csv");
  const [destDir, setDestDir] = useState("");
  const [exportOut, setExportOut] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = (): void => {
    setBusy(true);
    setError(null);
    setEntry(null);
    checksumFile(path, algorithm)
      .then((e) => {
        setEntry(e);
        onDone();
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  const openFile = async (): Promise<void> => {
    try {
      const selected = await openDialog({ multiple: false, directory: false });
      if (typeof selected === "string" && selected) {
        setPath(selected);
      }
    } catch {
      // 对话框取消不视为错误
    }
  };

  const doExport = (): void => {
    if (!entry) {
      return;
    }
    setError(null);
    setExportOut(null);
    const name =
      "checksum-report." + (exportFmt === "json" ? "json" : exportFmt === "csv" ? "csv" : "txt");
    exportReport([entry], exportFmt, destDir, name)
      .then((r) => {
        setExportOut(r.output);
        onDone();
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
      <h4 style={{ margin: 0 }}>{t("utilities.fileChecksum.title")}</h4>
      <div style={rowStyle}>
        <input
          aria-label={t("utilities.fileChecksum.path")}
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="C:\path	oile"
          style={{ ...boxStyle, flex: 1, minWidth: "16em", fontFamily: "var(--typography-mono-family)" }}
        />
        <button type="button" onClick={() => void openFile()} style={boxStyle}>
          {t("documents.open")}
        </button>
        <select
          aria-label={t("utilities.checksum.algorithm")}
          value={algorithm}
          onChange={(e) => setAlgorithm(e.target.value)}
          style={boxStyle}
        >
          <option value="crc32">CRC-32/ISO-HDLC</option>
          <option value="crc32c">CRC-32C</option>
          <option value="adler32">Adler-32</option>
        </select>
        <button type="button" disabled={busy || !path} onClick={run} style={boxStyle}>
          {t("utilities.run")}
        </button>
      </div>
      <ErrorLine error={error} />
      {entry ? (
        <div role="status" style={{ ...boxStyle, ...cellStyle }}>
          {entry.digestHex} · {entry.algorithm} · {entry.bytesProcessed}B
        </div>
      ) : null}
      {entry ? (
        <div style={rowStyle}>
          <select aria-label={t("documents.exportFormat")} value={exportFmt} onChange={(e) => setExportFmt(e.target.value)} style={boxStyle}>
            <option value="txt">TXT</option>
            <option value="csv">CSV</option>
            <option value="json">JSON</option>
          </select>
          <input
            aria-label={t("documents.destDir")}
            value={destDir}
            onChange={(e) => setDestDir(e.target.value)}
            placeholder="C:\path	o\output"
            style={{ ...boxStyle, flex: 1, minWidth: "14em" }}
          />
          <button
            type="button"
            disabled={!destDir}
            onClick={doExport}
            style={boxStyle}
          >
            {t("utilities.fileChecksum.export")}
          </button>
        </div>
      ) : null}
      {exportOut ? (
        <div role="status" style={{ fontSize: "var(--typography-size-sm)" }}>
          {t("documents.result")}: {exportOut}
        </div>
      ) : null}
    </div>
  );
}
