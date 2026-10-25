import { en } from "./locales/en.ts";
import { ptBR } from "./locales/pt-BR.ts";

export type Language = "en" | "pt-BR";
export const DEFAULT_LANGUAGE: Language = "en";
export const LANGUAGE_STORAGE_KEY = "hidra.language";
export const LANGUAGES: { value: Language; label: string }[] = [
  { value: "en", label: "English" },
  { value: "pt-BR", label: "Português (Brasil)" },
];
export const catalogs: Record<Language, Record<string, string>> = { en, "pt-BR": ptBR };
export type TranslationParams = Record<string, string | number>;

export function isLanguage(value: unknown): value is Language {
  return value === "en" || value === "pt-BR";
}

export function readLanguage(storage?: Pick<Storage, "getItem">): Language {
  try {
    const saved: unknown = JSON.parse(storage?.getItem(LANGUAGE_STORAGE_KEY) ?? "null");
    return isLanguage(saved) ? saved : DEFAULT_LANGUAGE;
  } catch {
    return DEFAULT_LANGUAGE;
  }
}

// Backend diagnostics and notifications may already contain parameter values.
// Match complete catalog templates, preserving unknown driver/OS diagnostics.
const templates = Object.keys(en).filter((key) => /\{\w+\}/.test(key))
  .sort((a, b) => b.replace(/\{\w+\}/g, "").length - a.replace(/\{\w+\}/g, "").length)
  .map((key) => {
  const names: string[] = [];
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = escaped.replace(/\\\{(\w+)\\\}/g, (_match, name: string) => {
    names.push(name);
    return "(.+?)";
  });
  return { key, names, pattern: new RegExp(`^${pattern}$`, "s") };
});

export function translate(language: Language, message: string, params?: TranslationParams): string {
  const catalog = catalogs[language];
  let template = Object.hasOwn(catalog, message) ? catalog[message] : undefined;
  let values = params;
  if (template === undefined && params === undefined) {
    for (const entry of templates) {
      const match = entry.pattern.exec(message);
      if (!match) continue;
      template = catalog[entry.key];
      values = Object.fromEntries(entry.names.map((name, i) => [name,
        name === "reason" ? translate(language, match[i + 1]) : match[i + 1],
      ]));
      break;
    }
  }
  return (template ?? message).replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    values && Object.hasOwn(values, name) ? String(values[name]) : placeholder,
  );
}
