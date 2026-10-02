import { useAppStore, type Locale } from "../stores/appStore";
import en from "./en.json";
import zhCN from "./zh-CN.json";

export const LOCALES: readonly Locale[] = ["zh-CN", "en"] as const;

type ResourceTable = Record<string, string>;

const resources: Record<Locale, ResourceTable> = {
  "zh-CN": zhCN,
  en,
};

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value);
}

/** 翻译：locale 缺 key 时回退 en，再缺则原样返回 key（禁止散落文案，charter #34）。 */
export function translate(locale: Locale, key: string): string {
  return resources[locale][key] ?? resources.en[key] ?? key;
}

export function useT(): (key: string) => string {
  const locale = useAppStore((state) => state.locale);
  return (key: string) => translate(locale, key);
}
