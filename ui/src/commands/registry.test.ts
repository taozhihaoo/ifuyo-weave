import { beforeEach, describe, expect, it } from "vitest";
import { clearCommands, listCommands, registerCommand, runCommand } from "./registry";

beforeEach(() => {
  clearCommands();
});

describe("command registry", () => {
  it("lists commands in deterministic id order", () => {
    registerCommand({ id: "b.two", labelKey: "k", run: () => {} });
    registerCommand({ id: "a.one", labelKey: "k", run: () => {} });
    expect(listCommands().map((c) => c.id)).toEqual(["a.one", "b.two"]);
  });

  it("rejects duplicate ids", () => {
    registerCommand({ id: "app.ping", labelKey: "k", run: () => {} });
    expect(() => registerCommand({ id: "app.ping", labelKey: "k", run: () => {} })).toThrowError(
      /duplicate command id/,
    );
  });

  it("throws on unknown command instead of silent no-op", () => {
    expect(() => runCommand("missing.command")).toThrowError(/unknown command/);
  });

  it("runs a registered command", () => {
    let ran = false;
    registerCommand({
      id: "x.run",
      labelKey: "k",
      run: () => {
        ran = true;
      },
    });
    runCommand("x.run");
    expect(ran).toBe(true);
  });
});
