# Interface languages

HIDra defaults to **English**, independently of the operating system language.
The sidebar language selector also offers **Português (Brasil)**. The saved
`hidra.language` preference is reused on the next launch. Unsupported or corrupt
preferences, and blocked storage, fall back to English. Language changes update
`<html lang>` and text in place without remounting device panels or calling the
hardware backend. Device IDs, protocol values, key offsets and stored drafts
remain independent of the display language.

## Translate a message

All visible UI text goes through `useTranslation()` from `src/i18n.tsx`:

```tsx
const { t } = useTranslation();
return <button>{t("Aplicar")}</button>;
```

The catalogs in `src/locales/en.ts` and `src/locales/pt-BR.ts` use the original
source message as a stable key, in the style of gettext. This keeps existing
backend effect/key labels usable without moving UI translations into protocol
code. Add the same key to both catalogs. Portuguese is checked against the
English key set by TypeScript. Do not assemble sentences from translated words;
use complete messages and named parameters:

```tsx
t("Perfil {profile}", { profile: 2 })
```

Labels from shared lookup tables are translated where displayed, never when
constructing a HID command. Toasts translate stored source messages when
rendered, keeping the notification callback stable so changing language cannot
trigger device discovery effects. Known backend messages are translated using
exact keys or complete named templates; unknown OS/driver diagnostics are
preserved verbatim. Release notes and product names retain their source text.

## Add another language

1. Add a catalog under `src/locales/`, with exactly the English keys and named
   parameters. Follow the typed Portuguese catalog as the example.
2. Add its identifier to `Language`, `LANGUAGES`, `catalogs`, and `isLanguage` in
   `src/i18n-core.ts`. Use the language's native name in the selector.
3. Extend the catalog parity test in `scripts/tests/i18n.test.mjs` to include it.
4. Run `npm run test:i18n`, `npm run lint`, and `npm run build`. Translation
   tests use Node's TypeScript stripping (Node 22.6+).
5. Check all mouse and keyboard tabs, notifications, the update banner and
   tooltips. Change language with unsaved settings, restart to check persistence,
   and confirm there is no background device polling.
