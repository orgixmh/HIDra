import { useTranslation } from "../../i18n";
import { useEffect, useMemo, useState } from "react";
import type { RemapKey, RemapKind } from "../../types";
import { keyboardRemap, keyboardRemapKeys, keyboardRestoreKeymap } from "../../backend";
import { KEY_USAGES, MEDIA_USAGES, MODIFIERS } from "../../constants";
import { useRun, useToast } from "../Toast";
import { Panel, Field } from "../ui/Panel";

interface KeyboardRemapProps {
  applied: string;
}

type FnType = Extract<RemapKind, "key" | "modifier" | "media" | "disable">;

const FN_OPTIONS: { value: FnType; label: string }[] = [
  { value: "key", label: "Tecla" },
  { value: "modifier", label: "Modificador" },
  { value: "media", label: "Multimídia" },
  { value: "disable", label: "Desativar" },
];

export function KeyboardRemap({ applied }: KeyboardRemapProps) {
  const { t } = useTranslation();
  const run = useRun();
  const notify = useToast();

  const [keys, setKeys] = useState<RemapKey[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [search, setSearch] = useState("");

  const [fnType, setFnType] = useState<FnType>("key");
  const [keyCode, setKeyCode] = useState(KEY_USAGES[0].code);
  const [modCode, setModCode] = useState(MODIFIERS[0].code);
  const [mediaUsage, setMediaUsage] = useState(MEDIA_USAGES[0].usage);

  const loadKeys = () => {
    keyboardRemapKeys()
      .then(setKeys)
      .catch((e: unknown) => notify(String(e), "error"));
  };

  useEffect(() => {
    let alive = true;
    keyboardRemapKeys()
      .then((k) => {
        if (alive) setKeys(k);
      })
      .catch((e: unknown) => {
        if (alive) notify(String(e), "error");
      });
    return () => {
      alive = false;
    };
  }, [notify]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? keys.filter((k) => t(k.label).toLowerCase().includes(q)) : keys;
  }, [keys, search, t]);

  const selectedKey = keys.find((k) => k.index === selected) ?? null;

  const buildRemap = (): { kind: RemapKind; code: number; code2: number } => {
    switch (fnType) {
      case "key":
        return { kind: "key", code: keyCode, code2: 0 };
      case "modifier":
        return { kind: "modifier", code: modCode, code2: 0 };
      case "media":
        return {
          kind: "media",
          code: mediaUsage & 0xff,
          code2: (mediaUsage >> 8) & 0xff,
        };
      case "disable":
        return { kind: "disable", code: 0, code2: 0 };
    }
  };

  const apply = () => {
    if (selected === null) return;
    const { kind, code, code2 } = buildRemap();
    void run(() => keyboardRemap({ index: selected, kind, code, code2 }), applied);
  };

  const restore = async () => {
    const ok = await run(() => keyboardRestoreKeymap(), "Keymap restaurado ao padrão");
    if (ok) loadKeys();
  };

  return (
    <div className="tabgrid tabgrid--remap">
      <Panel
        title={t("Tecla física")}
        subtitle={t("Escolha a tecla que deseja reprogramar")}
      >
        <input
          className="input"
          type="search"
          placeholder={t("Buscar tecla…")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <ul className="keylist">
          {filtered.map((k) => (
            <li key={k.index}>
              <button
                type="button"
                className={`keylist__item${k.index === selected ? " is-active" : ""}`}
                onClick={() => setSelected(k.index)}
              >
                <span className="keylist__label">{t(k.label)}</span>
                <span className="keylist__idx">#{k.index}</span>
              </button>
            </li>
          ))}
          {filtered.length === 0 && (
            <li className="keylist__empty">{t("Nenhuma tecla encontrada")}</li>
          )}
        </ul>
      </Panel>

      <Panel
        title={t("Nova função")}
        subtitle={
          selectedKey
            ? t("Reprogramando “{label}” (#{index})", { label: t(selectedKey.label), index: selectedKey.index })
            : t("Selecione uma tecla à esquerda")
        }
        actions={
          <div className="btnrow">
            <button type="button" className="btn" onClick={restore}>
              {t("Restaurar padrão")}</button>
            <button
              type="button"
              className="btn btn--primary"
              disabled={selected === null}
              onClick={apply}
            >
              {t("Aplicar")}</button>
          </div>
        }
      >
        <Field label={t("Tipo de função")}>
          <select
            className="input select"
            value={fnType}
            onChange={(e) => setFnType(e.target.value as FnType)}
          >
            {FN_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {t(o.label)}
              </option>
            ))}
          </select>
        </Field>

        {fnType === "key" && (
          <Field label={t("Tecla de destino")}>
            <select
              className="input select"
              value={keyCode}
              onChange={(e) => setKeyCode(Number(e.target.value))}
            >
              {KEY_USAGES.map((k) => (
                <option key={k.code} value={k.code}>
                  {t(k.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {fnType === "modifier" && (
          <Field label={t("Modificador")}>
            <select
              className="input select"
              value={modCode}
              onChange={(e) => setModCode(Number(e.target.value))}
            >
              {MODIFIERS.map((m) => (
                <option key={m.code} value={m.code}>
                  {t(m.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {fnType === "media" && (
          <Field label={t("Tecla multimídia")}>
            <select
              className="input select"
              value={mediaUsage}
              onChange={(e) => setMediaUsage(Number(e.target.value))}
            >
              {MEDIA_USAGES.map((m) => (
                <option key={m.usage} value={m.usage}>
                  {t(m.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {fnType === "disable" && (
          <p className="note">{t("A tecla ficará sem função até ser reprogramada.")}</p>
        )}
      </Panel>
    </div>
  );
}
