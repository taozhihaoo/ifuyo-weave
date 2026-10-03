import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAppStore } from "../stores/appStore";

const mocks = vi.hoisted(() => ({
  ping: vi.fn(),
  getAppInfo: vi.fn(),
  getAppConfig: vi.fn(),
  setAppConfig: vi.fn(),
  inspectFile: vi.fn(),
  hashFile: vi.fn(),
  analyzeDirectory: vi.fn(),
  getJob: vi.fn(),
  cancelJob: vi.fn(),
  listTools: vi.fn(),
}));

vi.mock("../generated/bindings", () => ({ commands: mocks }));

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: () => Promise.resolve(() => {}),
  }),
}));

const { registerBuiltinCommands } = await import("../commands/builtin");
const { clearCommands } = await import("../commands/registry");
const { default: App } = await import("./App");

afterEach(() => {
  cleanup();
});

beforeEach(() => {
  vi.clearAllMocks();
  clearCommands();
  registerBuiltinCommands();
  useAppStore.setState({
    locale: "zh-CN",
    theme: "system",
    appInfo: {
      name: "Weave",
      vendor: "ifuyo · 伊芙游",
      site: "https://ifuyo.com",
      version: "0.1.0",
      environment: "development",
    },
    ipcStatus: "connected",
    lastError: null,
    inspection: null,
    inspectError: null,
    scanJob: null,
    scanStatus: null,
    hashJob: null,
    hashStatus: null,
    hashResult: null,
  });
});

describe("App shell (M0 §8.2)", () => {
  it("renders brand, version and milestone", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Weave" })).toBeTruthy();
    expect(screen.getByText(/ifuyo/)).toBeTruthy();
    expect(screen.getByText(/v0\.1\.0/)).toBeTruthy();
    expect(screen.getByText("M1 File Core")).toBeTruthy();
  });

  it("renders IPC connection status from real state", () => {
    render(<App />);
    expect(screen.getByRole("status").textContent).toContain("IPC 已连接");
  });

  it("exposes the two registry commands", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "运行 IPC Ping" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "显示应用信息" })).toBeTruthy();
  });

  it("toggles language via resources, not hardcoded copy", async () => {
    render(<App />);
    mocks.setAppConfig.mockResolvedValue({
      status: "ok",
      data: { version: 1, language: "en", theme: "system" },
    });
    fireEvent.click(screen.getByRole("button", { name: "English" }));
    await vi.waitFor(() => {
      expect(useAppStore.getState().locale).toBe("en");
    });
    expect(screen.getByText("M1 File Core")).toBeTruthy();
  });

  it("renders an error state with code and suggestion when present", () => {
    useAppStore.setState({
      lastError: {
        kind: "internal",
        code: "brand.parseFailed",
        message: "bad brand json",
        location: null,
        recoverability: "fatal",
        suggestion: "restore brand.json",
      },
    });
    render(<App />);
    expect(screen.getByRole("alert").textContent).toContain("brand.parseFailed");
    expect(screen.getByRole("alert").textContent).toContain("restore brand.json");
  });
});

// ─── M1：文件核心面板（store 驱动渲染；拖放桥由运行时冒烟覆盖）───

const BS_CONST = String.fromCharCode(92);
const fileInspection = {
  status: "complete",
  requested: "C:" + BS_CONST + "w" + BS_CONST + "a.txt",
  normalizedPath: "C:" + BS_CONST + "w" + BS_CONST + "a.txt",
  name: "a.txt",
  extension: "txt",
  kind: "file",
  size: 11,
  readonly: false,
  hidden: false,
  createdMs: 1000,
  modifiedMs: 2000,
  accessedMs: null,
  classification: { category: "text", evidence: "extension", mime: null },
  encoding: "ascii",
  warnings: [],
};

describe("M1 file core panels", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearCommands();
    registerBuiltinCommands();
    useAppStore.setState({
      locale: "zh-CN",
      theme: "system",
      appInfo: {
        name: "Weave",
        vendor: "ifuyo · 伊芙游",
        site: "https://ifuyo.com",
        version: "0.1.0",
        environment: "development",
      },
      ipcStatus: "connected",
      lastError: null,
      inspection: null,
      inspectError: null,
      scanJob: null,
      scanStatus: null,
      hashJob: null,
      hashStatus: null,
      hashResult: null,
    });
  });

  it("renders inspector panel with facts from real inspection DTO", () => {
    useAppStore.getState().setInspection(fileInspection);
    render(<App />);
    expect(screen.getByText("a.txt")).toBeTruthy();
    expect(screen.getByText("Path").textContent).toBeTruthy();
    // 编码与类型来自资源文件，不是硬编码文案
    expect(screen.getByText("ASCII")).toBeTruthy();
    expect(screen.getByRole("button", { name: "计算 SHA-256" })).toBeTruthy();
  });

  it("scan panel shows progress while running and cancel affordance", () => {
    useAppStore.getState().setScanJob({ jobId: "job_test0001", path: "C:" + BS_CONST + "w" });
    useAppStore.getState().setScanStatus({
      jobId: "job_test0001",
      state: "running",
      progressCurrent: 512,
      hash: null,
      scan: null,
      plan: null,
      undo: null,
      error: null,
    });
    render(<App />);
    expect(screen.getByText(/512 项/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "取消扫描" })).toBeTruthy();
  });

  it("scan panel renders completed report with distribution and top lists", () => {
    useAppStore.getState().setScanJob({ jobId: "job_test0002", path: "C:" + BS_CONST + "w" });
    useAppStore.getState().setScanStatus({
      jobId: "job_test0002",
      state: "completed",
      progressCurrent: null,
      hash: null,
      plan: null,
      undo: null,
      scan: {
        scanId: "op_test",
        root: "C:" + BS_CONST + "w",
        status: "completedWithWarnings",
        startedAtMs: 1000,
        finishedAtMs: 1500,
        durationMs: 500,
        directoriesScanned: 2,
        filesScanned: 3,
        otherEntries: 0,
        errorCount: 1,
        entriesProcessed: 6,
        totalSize: 100,
        maxDepth: 1,
        emptyDirectories: ["empty"],
        emptyDirectoriesTruncated: false,
        fileTypeDistribution: [
          { label: "text", count: 2 },
          { label: "unknown", count: 1 },
        ],
        largestFiles: [{ relativePath: "big.bin", size: 90, modifiedMs: 1000 }],
        oldestFiles: [],
        newestFiles: [],
        warnings: ["depth limit reached"],
        errors: [{ relativePath: "denied", code: "path.permissionDenied", message: "no" }],
        errorsTruncated: false,
        limited: true,
        limitedReason: "max depth reached (8)",
      },
      error: null,
    });
    render(<App />);
    expect(screen.getByText("完成（有警告）")).toBeTruthy();
    expect(screen.getByText("类型分布")).toBeTruthy();
    expect(screen.getByText("big.bin")).toBeTruthy();
    expect(screen.getByText(/扫描被资源上限截断/)).toBeTruthy();
    expect(screen.getByText("空目录 (1)")).toBeTruthy();
  });

  it("shows cancelled summary with processed count", () => {
    useAppStore.getState().setScanJob({ jobId: "job_test0003", path: "C:" + BS_CONST + "w" });
    useAppStore.getState().setScanStatus({
      jobId: "job_test0003",
      state: "completed",
      progressCurrent: null,
      hash: null,
      plan: null,
      undo: null,
      scan: {
        scanId: "op_c",
        root: "C:" + BS_CONST + "w",
        status: "cancelled",
        startedAtMs: 1000,
        finishedAtMs: 1500,
        durationMs: 500,
        directoriesScanned: 0,
        filesScanned: 0,
        otherEntries: 0,
        errorCount: 0,
        entriesProcessed: 1823,
        totalSize: 0,
        maxDepth: 0,
        emptyDirectories: [],
        emptyDirectoriesTruncated: false,
        fileTypeDistribution: [],
        largestFiles: [],
        oldestFiles: [],
        newestFiles: [],
        warnings: [],
        errors: [],
        errorsTruncated: false,
        limited: false,
        limitedReason: null,
      },
      error: null,
    });
    render(<App />);
    expect(screen.getByText(/共处理 1823 项/)).toBeTruthy();
  });
});
