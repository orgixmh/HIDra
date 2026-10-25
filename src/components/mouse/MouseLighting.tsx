import { useTranslation } from "../../i18n";
import { useEffect, useState } from "react";
import type { LedEffect, MouseState, Rgb } from "../../types";
import { mouseSetLed } from "../../backend";
import { useRun } from "../Toast";
import { Panel, Field } from "../ui/Panel";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Slider } from "../ui/Slider";
import { ColorPicker } from "../ui/ColorPicker";
import { MouseSilhouette } from "./MouseSilhouette";

interface MouseLightingProps {
  state: MouseState | null;
  applied: string;
  onApplied: () => void;
}

const EFFECT_OPTIONS: { value: LedEffect; label: string }[] = [
  { value: "steady", label: "Sólido" },
  { value: "breathing", label: "Respiração" },
  { value: "cycle", label: "Ciclo" },
  { value: "off", label: "Desligado" },
];

export function MouseLighting({ state, applied, onApplied }: MouseLightingProps) {
  const { t } = useTranslation();
  const run = useRun();
  const led = state?.led ?? null;

  const [effect, setEffect] = useState<LedEffect>("steady");
  const [color, setColor] = useState<Rgb>([216, 31, 38]);
  const [brightness, setBrightness] = useState(255);
  const [speed, setSpeed] = useState(128);

  useEffect(() => {
    if (!led) return;
    setEffect(led.effect);
    setColor([led.r, led.g, led.b]);
    setBrightness(led.brightness);
    setSpeed(led.speed);
  }, [led]);

  const hasColor = effect === "steady" || effect === "breathing";
  const hasSpeed = effect === "breathing" || effect === "cycle";

  const apply = async () => {
    const ok = await run(
      () =>
        mouseSetLed({
          effect,
          r: color[0],
          g: color[1],
          b: color[2],
          speed,
          brightness,
        }),
      applied,
    );
    if (ok) onApplied();
  };

  return (
    <div className="tabgrid">
      <Panel
        title={t("Iluminação")}
        subtitle={t("Efeito do LED, cor e brilho do mouse")}
        actions={
          <button type="button" className="btn btn--primary" onClick={apply}>
            {t("Aplicar")}</button>
        }
      >
        <Field label={t("Efeito")}>
          <SegmentedControl<LedEffect>
            options={EFFECT_OPTIONS}
            value={effect}
            onChange={setEffect}
          />
        </Field>

        <Field
          label={t("Cor")}
          hint={hasColor ? undefined : "Este efeito ignora a cor selecionada"}
        >
          <ColorPicker
            value={color}
            onChange={setColor}
            disabled={!hasColor}
          />
        </Field>

        <Field label={t("Brilho")}>
          <Slider
            min={0}
            max={255}
            value={brightness}
            onChange={setBrightness}
            format={(v) => `${Math.round((v / 255) * 100)}%`}
            disabled={effect === "off"}
          />
        </Field>

        {hasSpeed && (
          <Field label={t("Velocidade")} hint={t("Menor = mais rápido")}>
            <Slider
              min={0}
              max={255}
              value={speed}
              onChange={setSpeed}
              format={(v) => String(v)}
            />
          </Field>
        )}
      </Panel>

      <Panel title={t("Pré-visualização")} className="preview-panel">
        <div className="preview-stage">
          <MouseSilhouette
            color={color}
            brightness={brightness}
            lit={effect !== "off"}
          />
        </div>
      </Panel>
    </div>
  );
}
