import { commands } from "../generated/bindings";
import { useAppStore } from "../stores/appStore";
import { registerCommand } from "./registry";

/** M0 的两个验证性命令（M0 §35）：走真实 IPC，不做本地 mock。 */
export function registerBuiltinCommands(): void {
  registerCommand({
    id: "app.ping",
    labelKey: "command.ping",
    run: async () => {
      const pong = await commands.ping();
      useAppStore.getState().setIpcStatus(pong.message === "pong" ? "connected" : "failed");
    },
  });

  registerCommand({
    id: "app.showInfo",
    labelKey: "command.showInfo",
    run: async () => {
      const result = await commands.getAppInfo();
      if (result.status === "ok") {
        useAppStore.getState().setAppInfo(result.data);
      } else {
        useAppStore.getState().setError(result.error);
      }
    },
  });
}
