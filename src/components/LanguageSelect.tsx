import { LANGUAGES, isLanguage } from "../i18n-core";
import { useTranslation } from "../i18n";

export function LanguageSelect() {
  const { language, setLanguage, t } = useTranslation();
  return (
    <label className="language-setting">
      <span>{t("Language")}</span>
      <select
        className="input select"
        aria-label={t("Language")}
        value={language}
        onChange={(event) => {
          const next = event.target.value;
          if (isLanguage(next)) setLanguage(next);
        }}
      >
        {LANGUAGES.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
      </select>
    </label>
  );
}
