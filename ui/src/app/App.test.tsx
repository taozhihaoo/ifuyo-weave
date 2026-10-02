import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAppStore } from "../stores/appStore";

const mocks = vi.hoisted(() => ({
  ping: vi.fn(),
  getAppInfo: vi.fn(),
  getAppConfig: vi.fn(),
  setAppConfig: vi.fn(),
  inspectPath: vi.fn(),
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
    probe: null,
    probeError: null,
  });
});

describe("App shell (M0 §8.2)", () => {
  it("renders brand, version and milestone", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Weave" })).toBeTruthy();
    expect(screen.getByText(/ifuyo/)).toBeTruthy();
    expect(screen.getByText(/v0\.1\.0/)).toBeTruthy();
    expect(screen.getByText("M0 Foundation")).toBeTruthy();
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
    expect(screen.getByText("M0 Foundation")).toBeTruthy();
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
