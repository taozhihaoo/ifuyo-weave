# PRIVACY

> Applies to: **2.0.0-rc.1 (Release Candidate)**

Weave's privacy boundary (charter #19 / M0 §14). This document is also an acceptance checklist (charter #69). Because M2.10 Web / M2.11 Monitor introduced user-initiated outbound requests in 2.0, the commitments below distinguish "default behavior" from "explicitly initiated optional features" instead of claiming that every feature works offline.

## Commitments

- **No Telemetry** — no usage data is collected or reported
- **No Account** — no account system
- **No Cloud Requirement** — all **core** features work offline; only Web fetching and website monitoring inherently require network access, and only when explicitly initiated by the user
- **No File Upload** — user files are never uploaded. Web / Monitor only **download** remote content to the local machine; local file contents are never sent
- **Local-first** — offline by default; the UI capability layer grants no network permission, and the single Rust-side egress endpoint is subject to SSRF protection (see [SECURITY.md](SECURITY.md))

## M0 Implementation Status

| Item | Status | Location |
| --- | --- | --- |
| Capability minimization: `core:default` + user-initiated `dialog:default`; no fs/shell/process/network | ✅ | src-tauri/capabilities/default.json |
| CSP: self + IPC only; no remote JS/CSS/fonts | ✅ | src-tauri/tauri.conf.json |
| Logs written locally to `%APPDATA%/ifuyo/Weave/logs` | ✅ | src-tauri/src/logging.rs |
| Logs must not record file contents / passwords / tokens | ✅ (policy + review; M0 had no sensitive-path writes) | logging.rs doc comments |
| Config stored locally only (language/theme/favorites + clipboard/wisp/web/monitor sections) | ✅ | src-tauri/src/config.rs:39-48 |
| Crash reporting / analytics SDK | Absent | Guaranteed by dependency audit |

## 2.0 Network Egress Boundary (M2.10 / M2.11)

- The only egress path is the Rust-side `ureq` client, triggered explicitly by Web / Monitor commands (src-tauri/src/web/fetch.rs:192, src-tauri/src/monitor/check.rs); it **does not pass through Tauri capabilities**, so "capabilities grant no network" is not equivalent to "no network behavior".
- Local / private / link-local targets are denied by default (SSRF classification at the DNS resolution layer); `allow_private` permits them only when explicitly enabled by the user (src-tauri/src/web/target.rs:1-7,90-130).
- Fetch results are stored only on the local machine; requests carry no user file contents, account credentials or telemetry identifiers.
- JavaScript-rendered pages and authenticated sessions are not supported (2.1 backlog, stated honestly).

## Acceptance Checklist (2.0 RC review)

- [x] No telemetry by default
- [x] No forced network access (everything except Web / Monitor works offline)
- [x] Files processed locally by default
- [x] User files never uploaded (outbound requests only download remote content)
- [x] Logs contain no sensitive content (ids / counts only; paths appear only in error messages shown in the local UI, never in log macros)
- [x] Paths are validated (roots pass validate_absolute_path; selections must belong to scan results)
- [x] Deletion goes to the recycle bin by default (since M3: the only delete action)
- [x] Destructive operations have Preview / Confirm (since M2; M3 adds Plan Preview + confirmation before recycling)
- [x] Outbound targets deny private/local networks by default (M2.10 SSRF classification + regression tests)
- [x] Config, history, Job Journal and clipboard history are local only, with no cloud sync

## M3 Duplicate Finder Implementation Status

| Item | Status | Location |
| --- | --- | --- |
| All hashes computed locally (no remote hash, no content leaving the machine) | ✅ | weave-files duplicates pipeline |
| Logs never record file contents; only operation_id / job_id / counts | ✅ | duplicates_service / m3_commands |
| History (transactions/entries) stores only metadata such as path, size and time — never content | ✅ | weave-history (reused from M2) |
| The only delete action is the OS recycle bin; no permanent-delete path, no fs::remove fallback | ✅ | crates/weave-files/src/recycle.rs (§126 P0 audit item) |
| ScanCache / PlanCache are in-memory only, disappear on process exit, never persisted | ✅ | src-tauri/src/duplicates_service.rs |
| "Recent scan roots" are pure session state (appStore), never written to config | ✅ | ui/src/stores/appStore.ts |
| Network capability: none (this module makes no network calls; see the global egress boundary above) | ✅ | src-tauri/src/duplicates_service.rs |
| M4: text decoded/formatted/compared entirely locally; regex runs in-process (no remote hash / remote formatting API); history records paths and counts only | ✅ | weave-text / text_service |
| M4: Regex engine = Rust regex (in-process, linear time), no external service | ✅ | crates/weave-text |
| M5: datasets parsed/cleaned/converted entirely locally; sessions are ephemeral and never persisted; history records operation facts only | ✅ | weave-data / data_service |

## M7 Batch Engine Implementation Status

- Job Journal (`app-data/jobs/*.jsonl`): local only, records paths/sizes/status — same tier as History (local files, no upload).
- No telemetry / no upload / no cloud / no network calls: batch execution is local end to end (§243 Final Audit N: PASS).
- Logging: tracing uses a local non-blocking writer only; no path-level sensitive content is sent out.

## Additional M2.x Facts

- **M2.3 Clipboard**: clipboard history is stored locally only; it can be paused, capped and marked sensitive; nothing is transmitted (`weave-clipboard`).
- **M2.4 Privacy**: share preflight exports a **safe copy** and leaves the original untouched; detection and response are separated (`crates/weave-privacy/src/cleanup.rs`).
- **M2.8 Glint**: capture and OCR both run locally with bounded storage; deletion uses the recycle bin.
- **M2.9 Wisp**: sensitive-data detection for paste guard is **Limited** and declared as such — it does not claim to recognize every kind of sensitive content.
- **M2.12 Smart**: produces suggestions only and never executes automatically; file contents are never sent to an external AI service.
