import { useTranslation } from "../../i18n";
import { useEffect, useState } from "react";
import type {
  KeyboardCapabilities,
  KeyboardEffect,
  KeyboardState,
  Profile,
  Rgb,
} from "../../types";
import { keyboardSetEffect } from "../../backend";
import { CUSTOM_EFFECT_ID } from "../../constants";
import { rgbCss } from "../../hooks";
import { useRun } from "../Toast";
import { Panel, Field } from "../ui/Panel";
import { Slider } from "../ui/Slider";
import { Toggle } from "../ui/Toggle";
import { ColorPicker } from "../ui/ColorPicker";
import { SegmentedControl } from "../ui/SegmentedControl";
import { KeyboardDiagram } from "./KeyboardDiagram";

interface KeyboardLightingProps {
  state: KeyboardState | null;
  capabilities?: KeyboardCapabilities | null;
  present: boolean;
  effects: KeyboardEffect[];
  profile: Profile;
  applied: string;
  onApplied: () => void;
}

function scale([r, g, b]: Rgb, factor: number): string {
  return rgbCss([
    Math.round(r * factor),
    Math.round(g * factor),
    Math.round(b * factor),
  ]);
}

function rainbow(slot: number, factor: number): string {
  const hue = (slot * 11) % 360;
  return `hsl(${hue} 90% ${Math.round(30 + 30 * factor)}%)`;
}

export function KeyboardLighting({
  state,
  capabilities,
  present,
  effects,
  profile,
  applied,
  onApplied,
}: KeyboardLightingProps) {
  const { t } = useTranslation();
  const run = useRun();

  const [effect, setEffect] = useState<number>(6);
  const [color, setColor] = useState<Rgb>([216, 31, 38]);
  const [brightness, setBrightness] = useState(5);
  const [speed, setSpeed] = useState(3);
  const [direction, setDirection] = useState(0);
  const [fullRgb, setFullRgb] = useState(false);

  useEffect(() => {
    if (!state) return;
    if (state.effect != null) setEffect(state.effect);
    if (state.color) setColor(state.color);
    if (state.brightness != null) setBrightness(state.brightness);
    if (state.speed != null) setSpeed(state.speed);
    if (state.direction != null) setDirection(state.direction);
    if (state.full_rgb != null) setFullRgb(state.full_rgb);
  }, [state, profile]);

  const brightnessMax = capabilities?.brightness_max ?? 5;
  const speedMin = capabilities?.speed_min ?? 1;
  const speedMax = capabilities?.speed_max ?? 5;
  const meta = effects.find((e) => e.id === effect);
  const hasColor = meta?.has_color ?? false;
  const hasDirection = meta?.has_direction ?? false;
  const isCustom = effect === (capabilities?.custom_effect ?? CUSTOM_EFFECT_ID);
  const factor = brightness / brightnessMax;

  const colorAt = (slot: number): string => {
    if (brightness === 0) return "#141519";
    if (hasColor && !fullRgb) return scale(color, 0.25 + 0.75 * factor);
    return rainbow(slot, factor);
  };

  const apply = async () => {
    const ok = await run(
      () =>
        keyboardSetEffect({
          profile,
          effect,
          r: color[0],
          g: color[1],
          b: color[2],
          brightness: Math.min(brightness, brightnessMax),
          speed: Math.min(Math.max(speed, speedMin), speedMax),
          direction: hasDirection ? direction : 0,
          full_rgb: fullRgb,
        }),
      applied,
    );
    if (ok) onApplied();
  };

  return (
    <div className="tabgrid tabgrid--kbd">
      <Panel
        title={t("Iluminação")}
        subtitle={capabilities?.profiles === 1 ? t("USB keyboard lighting") : t("Efeito de luz do Perfil {profile}", { profile })}
        actions={
          <button type="button" className="btn btn--primary" onClick={apply} disabled={!present || !state?.present || !effects.some((e) => e.id === effect)}>
            {t("Aplicar")}</button>
        }
      >
        <Field label={t("Efeito")}>
          <select
            className="input select"
            value={effect}
            onChange={(e) => setEffect(Number(e.target.value))}
          >
            {effects.map((e) => (
              <option key={e.id} value={e.id}>
                {t(e.name)}
              </option>
            ))}
          </select>
        </Field>

        {isCustom && (
          <p className="note">
            {t("Modo Customizável: pinte cada tecla na aba Cores por tecla.")}
          </p>
        )}

        {hasColor && (
          <Field label={t("Multicolor (Full RGB)")}>
            <Toggle
              checked={fullRgb}
              onChange={setFullRgb}
              label={fullRgb ? t("Ligado") : t("Desligado")}
            />
          </Field>
        )}

        {hasColor && !fullRgb && (
          <Field label={t("Cor")}>
            <ColorPicker value={color} onChange={setColor} />
          </Field>
        )}

        {hasDirection && (
          <Field label={t("Direção")}>
            <SegmentedControl<number>
              value={direction}
              onChange={setDirection}
              options={[
                { value: 0, label: t("→ Direita") },
                { value: 1, label: t("← Esquerda") },
              ]}
            />
          </Field>
        )}

        <Field label={t("Brilho")}>
          <Slider
            min={0}
            max={brightnessMax}
            value={Math.min(brightness, brightnessMax)}
            onChange={setBrightness}
            format={(v) => `${v}/${brightnessMax}`}
          />
        </Field>

        <Field label={t("Velocidade")} hint={t("Menor = mais rápido")}>
          <Slider
            min={speedMin}
            max={speedMax}
            value={Math.min(Math.max(speed, speedMin), speedMax)}
            onChange={setSpeed}
            format={(v) => `${v}/${speedMax}`}
          />
        </Field>
      </Panel>

      <Panel title={t("Pré-visualização")} className="preview-panel">
        <div className="preview-stage preview-stage--kbd">
          <KeyboardDiagram colorAt={colorAt} layout={capabilities?.layout ?? undefined} aspect={capabilities?.layout_aspect ?? undefined} />
        </div>
      </Panel>
    </div>
  );
}
