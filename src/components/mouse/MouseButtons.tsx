import { useTranslation } from "../../i18n";
import { useState } from "react";
import type {
  ButtonAction,
  ButtonActionType,
  DpiOp,
  MouseButton,
  MouseState,
} from "../../types";
import { mouseSetButton } from "../../backend";
import {
  KEY_USAGES,
  MEDIA_USAGES,
  MODIFIERS,
  MOUSE_BUTTONS,
} from "../../constants";
import { useRun } from "../Toast";
import { Panel, Field } from "../ui/Panel";
import { MouseSilhouette } from "./MouseSilhouette";

interface MouseButtonsProps {
  state: MouseState | null;
  applied: string;
}

const CATEGORY_OPTIONS: { value: ButtonActionType; label: string }[] = [
  { value: "mouse", label: "Clique do mouse" },
  { value: "dpi", label: "DPI" },
  { value: "fire", label: "Disparo / Fire" },
  { value: "media", label: "Multimídia" },
  { value: "key", label: "Tecla" },
  { value: "combo", label: "Combo (modificador + tecla)" },
  { value: "three_click", label: "Três cliques" },
  { value: "rgb_toggle", label: "Ligar/desligar RGB" },
  { value: "polling_switch", label: "Trocar polling" },
  { value: "disabled", label: "Desativar" },
];

const MOUSE_BUTTON_OPTIONS: { value: MouseButton; label: string }[] = [
  { value: "left", label: "Esquerdo" },
  { value: "right", label: "Direito" },
  { value: "middle", label: "Meio" },
  { value: "back", label: "Voltar" },
  { value: "forward", label: "Avançar" },
];

const DPI_OP_OPTIONS: { value: DpiOp; label: string }[] = [
  { value: "up", label: "Aumentar" },
  { value: "down", label: "Diminuir" },
  { value: "cycle", label: "Ciclar" },
];

export function MouseButtons({ state, applied }: MouseButtonsProps) {
  const { t } = useTranslation();
  const run = useRun();
  const [uiIndex, setUiIndex] = useState(0);

  const [category, setCategory] = useState<ButtonActionType>("mouse");
  const [mouseBtn, setMouseBtn] = useState<MouseButton>("left");
  const [dpiOp, setDpiOp] = useState<DpiOp>("cycle");
  const [fireSpeed, setFireSpeed] = useState(40);
  const [fireClicks, setFireClicks] = useState(3);
  const [mediaUsage, setMediaUsage] = useState(MEDIA_USAGES[0].usage);
  const [keyUsage, setKeyUsage] = useState(KEY_USAGES[0].code);
  const [comboMods, setComboMods] = useState(0);
  const [comboUsage, setComboUsage] = useState(KEY_USAGES[0].code);

  const slot = MOUSE_BUTTONS[uiIndex];
  const raw = state?.buttons?.[slot.index];

  const buildAction = (): ButtonAction => {
    switch (category) {
      case "mouse":
        return { type: "mouse", button: mouseBtn };
      case "dpi":
        return { type: "dpi", op: dpiOp };
      case "fire":
        return { type: "fire", speed: fireSpeed, clicks: fireClicks };
      case "media":
        return { type: "media", usage: mediaUsage };
      case "key":
        return { type: "key", usage: keyUsage };
      case "combo":
        return { type: "combo", modifiers: comboMods, usage: comboUsage };
      case "three_click":
        return { type: "three_click" };
      case "rgb_toggle":
        return { type: "rgb_toggle" };
      case "polling_switch":
        return { type: "polling_switch" };
      case "disabled":
        return { type: "disabled" };
    }
  };

  const apply = () => {
    void run(() => mouseSetButton(slot.index, buildAction()), applied);
  };

  const toggleMod = (bit: number) => {
    setComboMods((m) => m ^ bit);
  };

  return (
    <div className="tabgrid">
      <Panel title={t("Botões")} subtitle={t("Clique em um botão do diagrama para reprogramá-lo")}>
        <div className="mouse-map">
          <MouseSilhouette color={[70, 74, 82]} lit={false} />
          {MOUSE_BUTTONS.map((b, i) => (
            <button
              key={b.num}
              type="button"
              className={`mouse-hot${i === uiIndex ? " is-active" : ""}`}
              style={{ left: `${b.x}%`, top: `${b.y}%` }}
              title={t(b.label)}
              onClick={() => setUiIndex(i)}
            >
              {b.num}
            </button>
          ))}
        </div>
        <ul className="legend">
          {MOUSE_BUTTONS.map((b, i) => (
            <li key={b.num}>
              <button
                type="button"
                className={`legend__item${i === uiIndex ? " is-active" : ""}`}
                onClick={() => setUiIndex(i)}
              >
                <span className="legend__num">{b.num}</span>
                {t(b.label)}
              </button>
            </li>
          ))}
        </ul>
      </Panel>

      <Panel
        title={t("Ação — Botão {number}", { number: slot.num })}
        subtitle={
          raw
            ? t("Tabela #{index} · atual [{value}]", { index: slot.index, value: raw.join(", ") })
            : t("Tabela #{index}", { index: slot.index })
        }
        actions={
          <button type="button" className="btn btn--primary" onClick={apply}>
            {t("Aplicar")}</button>
        }
      >
        <Field label={t("Categoria")}>
          <select
            className="input select"
            value={category}
            onChange={(e) => setCategory(e.target.value as ButtonActionType)}
          >
            {CATEGORY_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {t(o.label)}
              </option>
            ))}
          </select>
        </Field>

        {category === "mouse" && (
          <Field label={t("Botão emulado")}>
            <select
              className="input select"
              value={mouseBtn}
              onChange={(e) => setMouseBtn(e.target.value as MouseButton)}
            >
              {MOUSE_BUTTON_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {t(o.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {category === "dpi" && (
          <Field label={t("Operação de DPI")}>
            <select
              className="input select"
              value={dpiOp}
              onChange={(e) => setDpiOp(e.target.value as DpiOp)}
            >
              {DPI_OP_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {t(o.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {category === "fire" && (
          <>
            <Field label={t("Velocidade (ms entre cliques)")}>
              <input
                className="input"
                type="number"
                min={0}
                max={255}
                value={fireSpeed}
                onChange={(e) => setFireSpeed(Number(e.target.value))}
              />
            </Field>
            <Field label={t("Nº de cliques")}>
              <input
                className="input"
                type="number"
                min={1}
                max={255}
                value={fireClicks}
                onChange={(e) => setFireClicks(Number(e.target.value))}
              />
            </Field>
          </>
        )}

        {category === "media" && (
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

        {category === "key" && (
          <Field label={t("Tecla")}>
            <select
              className="input select"
              value={keyUsage}
              onChange={(e) => setKeyUsage(Number(e.target.value))}
            >
              {KEY_USAGES.map((k) => (
                <option key={k.code} value={k.code}>
                  {t(k.label)}
                </option>
              ))}
            </select>
          </Field>
        )}

        {category === "combo" && (
          <>
            <Field label={t("Modificadores")}>
              <div className="checkrow">
                {MODIFIERS.map((m) => (
                  <label key={m.code} className="checkchip">
                    <input
                      type="checkbox"
                      checked={(comboMods & m.code) !== 0}
                      onChange={() => toggleMod(m.code)}
                    />
                    {t(m.label)}
                  </label>
                ))}
              </div>
            </Field>
            <Field label={t("Tecla")}>
              <select
                className="input select"
                value={comboUsage}
                onChange={(e) => setComboUsage(Number(e.target.value))}
              >
                {KEY_USAGES.map((k) => (
                  <option key={k.code} value={k.code}>
                    {t(k.label)}
                  </option>
                ))}
              </select>
            </Field>
          </>
        )}

        {(category === "three_click" ||
          category === "rgb_toggle" ||
          category === "polling_switch" ||
          category === "disabled") && (
          <p className="note">
            {category === "three_click" && t("Executa três cliques do botão esquerdo.")}
            {category === "rgb_toggle" && t("Alterna a iluminação RGB do mouse.")}
            {category === "polling_switch" && t("Alterna a taxa de polling atual.")}
            {category === "disabled" && t("O botão fica sem função.")}
          </p>
        )}
      </Panel>
    </div>
  );
}
