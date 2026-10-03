import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAppStore } from "../stores/appStore";

const mocks = vi.hoisted(() => ({
  ping: vi.fn<() => Promise<{ message: string }>>(),
  getAppInfo: vi.fn<() => Promise<unknown>>(),
  getAppConfig: vi.fn<() => Promise<unknown>>(),
}));

vi.mock("../generated/bindings", () => ({ commands: mocks }));

const { bootstrapApp } = await import("./bootstrap");

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({
    locale: "zh-CN",
    theme: "system",
    appInfo: null,
    ipcStatus: "connecting",
    lastError: null,
  });
});

describe("bootstrapApp", () => {
  it("marks IPC connected on a real pong contract", async () => {
    mocks.ping.mockResolvedValue({ message: "pong" });
    mocks.getAppInfo.mockResolvedValue({
      status: "ok",
      data: {
        name: "Weave",
        vendor: "ifuyo",
        site: "https://ifuyo.com",
        version: "0.1.0",
        environment: "development",
      },
    });
    mocks.getAppConfig.mockResolvedValue({
      status: "ok",
      data: { version: 1, language: "en", theme: "dark" },
    });

    await bootstrapApp();

    expect(useAppStore.getState().ipcStatus).toBe("connected");
    expect(useAppStore.getState().appInfo?.name).toBe("Weave");
    expect(useAppStore.getState().locale).toBe("en");
    expect(useAppStore.getState().theme).toBe("dark");
    expect(useAppStore.getState().lastError).toBeNull();
  });

  it("marks IPC failed when ping rejects — never fake success", async () => {
    mocks.ping.mockRejectedValue(new Error("no backend"));
    await bootstrapApp();
    expect(useAppStore.getState().ipcStatus).toBe("failed");
  });

  it("surfaces structured errors instead of swallowing them", async () => {
    mocks.ping.mockResolvedValue({ message: "pong" });
    mocks.getAppInfo.mockResolvedValue({
      status: "error",
      error: {
        kind: "internal",
        code: "brand.parseFailed",
        message: "bad brand",
        location: null,
        recoverability: "fatal",
        suggestion: null,
      },
    });
    mocks.getAppConfig.mockResolvedValue({
      status: "error",
      error: {
        kind: "unsupported",
        code: "config.unsupportedVersion",
        message: "v99",
        location: null,
        recoverability: "fatal",
        suggestion: null,
      },
    });

    await bootstrapApp();

    const { lastError } = useAppStore.getState();
    expect(lastError?.code).toBe("config.unsupportedVersion");
  });
});
