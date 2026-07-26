export type Locale = "en" | "zh-CN";

export const LOCALE_STORAGE_KEY = "rid.locale.v1";
const LOCALE_CHANGE_EVENT = "rid:locale-change";

export function detectWindowsLocale(languages: readonly string[]): Locale {
  const preferred = languages[0]?.trim().replaceAll("_", "-").toLowerCase();
  return preferred === "zh-cn" || preferred === "zh-hans-cn" ? "zh-CN" : "en";
}

export function resolveLocale(
  saved: string | null,
  languages: readonly string[],
): Locale {
  if (saved === "en" || saved === "zh-CN") return saved;
  return detectWindowsLocale(languages);
}

export function getLocaleSnapshot(): Locale {
  if (typeof window === "undefined") return "en";
  return resolveLocale(
    window.localStorage.getItem(LOCALE_STORAGE_KEY),
    window.navigator.languages,
  );
}

export function getServerLocaleSnapshot(): Locale {
  return "en";
}

export function subscribeLocale(onStoreChange: () => void) {
  window.addEventListener("storage", onStoreChange);
  window.addEventListener(LOCALE_CHANGE_EVENT, onStoreChange);
  return () => {
    window.removeEventListener("storage", onStoreChange);
    window.removeEventListener(LOCALE_CHANGE_EVENT, onStoreChange);
  };
}

export function setLocalePreference(locale: Locale) {
  window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  window.dispatchEvent(new Event(LOCALE_CHANGE_EVENT));
}
