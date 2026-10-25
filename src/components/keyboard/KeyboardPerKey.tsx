import { useTranslation } from "../../i18n";
import { useState } from "react";
import type { CustomCell, Profile, Rgb } from "../../types";
import { keyboardSetCustomColors } from "../../backend";
import type { KeyCell } from "../../keyboardLayout";
import { KEYBOARD_ASPECT, KEYBOARD_LAYOUT } from "../../keyboardLayout";
import { rgbCss } from "../../hooks";
import { useRun } from "../Toast";
import { Panel, Field } from "../ui/Panel";
import { ColorPicker } from "../ui/ColorPicker";

interface KeyboardPerKeyProps {
  profile: Profile;
  layout?: KeyCell[];
  aspect?: number;
  present: boolean;
  applied: string;
}

const OFF: Rgb = [26, 28, 33];

export function KeyboardPerKey({ profile, applied, layout, aspect, present }: KeyboardPerKeyProps) {
  const { t } = useTranslation();
  const keys = layout ?? KEYBOARD_LAYOUT;
  const run = useRun();
  const [paint, setPaint] = useState<Rgb>([216, 31, 38]);
  // colour by LED offset; absent = key is off (black)
  const [painted, setPainted] = useState<Record<number, Rgb>>({});

  const setKey = (off: number, rgb: Rgb | null) =>
    setPainted((prev) => {
      const next = { ...prev };
      if (rgb) next[off] = rgb;
      else delete next[off];
      return next;
    });

  const fillAll = () => {
    const all: Record<number, Rgb> = {};
    for (const k of keys) all[k.off] = paint;
    setPainted(all);
  };
  const clearAll = () => setPainted({});

  const apply = () => {
    // Send every key: painted ones get their colour, the rest go dark, so the
    // keyboard ends up exactly like the preview.
    const cells: CustomCell[] = keys.map((k) => {
      const c = painted[k.off] ?? [0, 0, 0];
      return { off: k.off, r: c[0], g: c[1], b: c[2] };
    });
    void run(() => keyboardSetCustomColors(profile, cells), applied);
  };

  return (
    <div className="tabgrid tabgrid--single">
      <Panel
        title={t("Cores por tecla")}
        subtitle={t("Pinte cada tecla individualmente (efeito Customizável)")}
        actions={
          <div className="btnrow">
            <button type="button" className="btn" onClick={fillAll}>
              {t("Preencher tudo")}</button>
            <button type="button" className="btn" onClick={clearAll}>
              {t("Limpar")}</button>
            <button type="button" className="btn btn--primary" onClick={apply} disabled={!present}>
              {t("Aplicar")}</button>
          </div>
        }
      >
        <Field label={t("Cor do pincel")}>
          <ColorPicker value={paint} onChange={setPaint} />
        </Field>

        <p className="note">
          {t("Clique numa tecla para pintá-la; clique com o botão direito para apagá-la. Aplicar ativa o efeito Customizável e envia o padrão para o teclado.")}
        </p>

        <div
          className="kbd-stage"
          style={{ aspectRatio: String(aspect ?? KEYBOARD_ASPECT) }}
        >
          {keys.map((k) => {
            const c = painted[k.off] ?? OFF;
            const lit = k.off in painted;
            return (
              <button
                type="button"
                key={k.off}
                className={`kbd-key${lit ? " kbd-key--lit" : ""}`}
                title={t(k.label)}
                style={{
                  left: `${k.x}%`,
                  top: `${k.y}%`,
                  width: `${k.w}%`,
                  height: `${k.h}%`,
                  background: rgbCss(c),
                }}
                onClick={() => setKey(k.off, paint)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setKey(k.off, null);
                }}
              >
                <span>{t(k.label)}</span>
              </button>
            );
          })}
        </div>
      </Panel>
    </div>
  );
}
