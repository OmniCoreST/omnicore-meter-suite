import { writable, derived } from "svelte/store";
import { tr } from "$lib/i18n/tr";
import { en } from "$lib/i18n/en";

export type Locale = "tr" | "en";

type TranslationKeys = keyof typeof tr;
type Translations = Record<TranslationKeys, string>;

const translations: Record<Locale, Translations> = {
  tr: tr as Translations,
  en: en as Translations,
};

function createLocaleStore() {
  const { subscribe, set } = writable<Locale>("tr");

  return {
    subscribe,
    setLocale: (locale: Locale) => {
      localStorage.setItem("locale", locale);
      set(locale);
    },
    init: () => {
      const stored = localStorage.getItem("locale") as Locale | null;
      if (stored && (stored === "tr" || stored === "en")) {
        set(stored);
      }
    },
  };
}

export const localeStore = createLocaleStore();

export const t = derived(localeStore, ($locale) => translations[$locale]);
