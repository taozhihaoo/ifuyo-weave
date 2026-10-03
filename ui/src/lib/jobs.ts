import { commands, type IpcError, type JobStatusDto } from "../generated/bindings";

export type JobTick =
  { kind: "status"; status: JobStatusDto } | { kind: "failed"; error: IpcError };

/**
 * 轮询任务状态直到结束；返回停止函数（组件卸载时调用，防泄漏）。
 * 400ms 是 M1 的诚实轮询间隔（M7 引擎落地后改事件推送）。
 */
export function pollJob(jobId: string, onTick: (tick: JobTick) => void): () => void {
  let stopped = false;
  const timer = setInterval(() => {
    if (stopped) {
      return;
    }
    void commands
      .getJob(jobId)
      .then((result) => {
        if (stopped) {
          return;
        }
        if (result.status === "ok") {
          onTick({ kind: "status", status: result.data });
        } else {
          stopped = true;
          onTick({ kind: "failed", error: result.error });
        }
      })
      .catch(() => {
        // invoke 层异常（应用退出等）：停止轮询，不伪造状态。
        stopped = true;
      });
  }, 400);
  return () => {
    stopped = true;
    clearInterval(timer);
  };
}

export function isTerminalState(status: JobStatusDto): boolean {
  return status.state !== "running";
}

export function formatTimestamp(epochMs: number | null, locale: string): string {
  if (epochMs === null || !Number.isFinite(epochMs)) {
    return "—";
  }
  return new Date(epochMs).toLocaleString(locale === "zh-CN" ? "zh-CN" : "en-US");
}
