import { describe, expect, it } from "vitest";
import { LOCALES, isLocale, translate } from "./index";
import en from "./en.json";
import zhCN from "./zh-CN.json";

describe("i18n resources", () => {
  it("zh-CN and en expose identical key sets (charter #34)", () => {
    expect(Object.keys(zhCN).sort()).toEqual(Object.keys(en).sort());
  });

  it("resolves every key in both locales without falling back", () => {
    for (const key of Object.keys(zhCN)) {
      for (const locale of LOCALES) {
        expect(translate(locale, key)).not.toBe(key);
      }
    }
  });

  it("falls back to en for missing keys, then to the key itself", () => {
    expect(translate("en", "nonexistent.key")).toBe("nonexistent.key");
  });

  it("validates locale values", () => {
    expect(isLocale("zh-CN")).toBe(true);
    expect(isLocale("en")).toBe(true);
    expect(isLocale("fr")).toBe(false);
  });
});
