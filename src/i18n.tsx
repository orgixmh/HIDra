import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { DEFAULT_LANGUAGE, isLanguage, LANGUAGE_STORAGE_KEY, readLanguage, translate } from "./i18n-core.ts";
import type { Language, TranslationParams } from "./i18n-core.ts";

type TranslationContext = {
  language: Language;
  setLanguage: (language: Language) => void;
  t: (message: string, params?: TranslationParams) => string;
};
const Context = createContext<TranslationContext>({
  language: DEFAULT_LANGUAGE,
  setLanguage: () => {},
  t: (message, params) => translate(DEFAULT_LANGUAGE, message, params),
});

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setValue] = useState<Language>(() => {
    try { return readLanguage(window.localStorage); }
    catch { return DEFAULT_LANGUAGE; }
  });
  const setLanguage = useCallback((next: Language) => {
    if (!isLanguage(next)) return;
    setValue(next);
    try { localStorage.setItem(LANGUAGE_STORAGE_KEY, JSON.stringify(next)); }
    catch { /* Keep language switching usable when storage is blocked. */ }
  }, []);
  useEffect(() => { document.documentElement.lang = language; }, [language]);
  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key === LANGUAGE_STORAGE_KEY || event.key === null) {
        try { setValue(readLanguage(window.localStorage)); }
        catch { setValue(DEFAULT_LANGUAGE); }
      }
    };
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);
  const context = useMemo(() => ({
    language, setLanguage,
    t: (message: string, params?: TranslationParams) => translate(language, message, params),
  }), [language, setLanguage]);
  return <Context.Provider value={context}>{children}</Context.Provider>;
}

export function useTranslation() { return useContext(Context); }
