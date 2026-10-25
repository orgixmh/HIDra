import { useTranslation } from "../../i18n";
import { useEffect, useState } from "react";
import type { DpiStage, MouseState, Rgb } from "../../types";
import {
  mouseSetDpiAll,
  mouseSetDpiColor,
  mouseSetDpiEffect,
  mouseSetDpiStage,
} from "../../backend";
import { rgbToHex } from "../../hooks";
import { useRun } from "../Toast";
import { Panel } from "../ui/Panel";
import { Toggle } from "../ui/Toggle";
import { ColorPicker } from "../ui/ColorPicker";

interface MouseDpiProps {
  state: MouseState | null;
  applied: string;
  onApplied: () => void;
}

const DEFAULT_STAGES: DpiStage[] = [
  { x: 800, y: 800, color: [216, 31, 38], enabled: true },
  { x: 1600, y: 1600, color: [0, 190, 255], enabled: true },
  { x: 3200, y: 3200, color: [0, 230, 90], enabled: true },
  { x: 6400, y: 6400, color: [255, 170, 0], enabled: false },
  { x: 12000, y: 12000, color: [180, 0, 255], enabled: false },
];

/** DPI is stored in ~83-DPI steps; report what the hardware will actually use. */
function appliedDpi(dpi: number): number {
  const byte = Math.min(0xbd, Math.max(0, Math.round((dpi * 3) / 250) - 1));
  return Math.round(((byte + 1) * 250) / 3);
}

function clampDpi(v: number): number {
  if (Number.isNaN(v)) return 100;
  return Math.max(100, Math.min(16000, Math.round(v / 50) * 50));
}

export function MouseDpi({ state, applied, onApplied }: MouseDpiProps) {
  const { t } = useTranslation();
  const run = useRun();
  const [stages, setStages] = useState<DpiStage[]>(DEFAULT_STAGES);
  const [editColor, setEditColor] = useState<number | null>(null);
  const [dpiEffect, setDpiEffect] = useState(true);

  useEffect(() => {
    if (state?.dpi && state.dpi.length > 0) {
      setStages(state.dpi.slice(0, 5));
    }
    if (state?.dpi_effect != null) setDpiEffect(state.dpi_effect);
  }, [state]);

  const patch = (i: number, next: Partial<DpiStage>): DpiStage[] => {
    const updated = stages.map((s, idx) => (idx === i ? { ...s, ...next } : s));
    setStages(updated);
    return updated;
  };

  const applyStage = async (i: number) => {
    const s = stages[i];
    const ok = await run(async () => {
      await mouseSetDpiStage(i + 1, s.x, s.y);
      await mouseSetDpiColor(i + 1, s.color);
    }, applied);
    if (ok) onApplied();
  };

  const toggleEnabled = async (i: number, on: boolean) => {
    const updated = patch(i, { enabled: on });
    await run(() => mouseSetDpiAll(updated), applied);
  };

  const applyDpiEffect = async (on: boolean) => {
    setDpiEffect(on);
    await run(() => mouseSetDpiEffect(on), applied);
  };

  const applyAll = async () => {
    const ok = await run(() => mouseSetDpiAll(stages), applied);
    if (ok) onApplied();
  };

  return (
    <div className="tabgrid tabgrid--single">
      <Panel
        title="DPI"
        subtitle={t("Até 5 estágios. Passo de gravação ≈ 83 DPI, então o valor aplicado pode diferir do digitado.")}
        actions={
          <button type="button" className="btn" onClick={applyAll}>
            {t("Aplicar todos")}</button>
        }
      >
        <div className="dpi-effect-row">
          <Toggle
            checked={dpiEffect}
            onChange={applyDpiEffect}
            label={t("DPI Effect — LED indica o estágio ativo")}
          />
        </div>

        <div className="dpi-table">
          <div className="dpi-head">
            <span>{t("Estágio")}</span>
            <span>DPI (100–16000)</span>
            <span>{t("Cor")}</span>
            <span>{t("Ativo")}</span>
            <span />
          </div>

          {stages.map((s, i) => (
            <div className="dpi-row" key={i}>
              <span className="dpi-stage-badge">{i + 1}</span>

              <div className="dpi-input-wrap">
                <input
                  className="input dpi-input"
                  type="number"
                  min={100}
                  max={16000}
                  step={50}
                  value={s.x}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    patch(i, { x: v, y: v });
                  }}
                  onBlur={(e) => {
                    const v = clampDpi(Number(e.target.value));
                    patch(i, { x: v, y: v });
                  }}
                />
                <span className="dpi-applied">≈ {appliedDpi(s.x)} DPI</span>
              </div>

              <div className="dpi-color-cell">
                <button
                  type="button"
                  className="swatch swatch--lg"
                  style={{ background: rgbToHex(s.color) }}
                  title={t("Editar cor do estágio")}
                  onClick={() => setEditColor(editColor === i ? null : i)}
                />
                {editColor === i && (
                  <div className="popover">
                    <ColorPicker
                      value={s.color}
                      onChange={(c: Rgb) => patch(i, { color: c })}
                    />
                  </div>
                )}
              </div>

              <Toggle
                checked={s.enabled}
                onChange={(on) => toggleEnabled(i, on)}
              />

              <button
                type="button"
                className="btn btn--sm btn--primary"
                onClick={() => applyStage(i)}
              >
                {t("Aplicar")}</button>
            </div>
          ))}
        </div>
      </Panel>
    </div>
  );
}
