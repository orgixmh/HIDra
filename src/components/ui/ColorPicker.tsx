import { useEffect, useRef, useState } from "react";
import type { PointerEvent } from "react";
import type { Rgb } from "../../types";
import { hexToRgb, rgbToHex } from "../../hooks";
import { useTranslation } from "../../i18n";

const DEFAULT_PRESETS: Rgb[] = [[216,31,38], [255,120,0], [255,214,0], [0,214,90],
  [0,190,255], [40,90,255], [150,0,255], [255,0,180], [255,255,255], [0,0,0]];
type Hsv = [number, number, number];
function toHsv(rgb: Rgb, fallbackHue = 0): Hsv {
  const [r,g,b] = rgb.map((c) => c / 255), max = Math.max(r,g,b), min = Math.min(r,g,b), d = max - min;
  const hue = d === 0 ? fallbackHue : max === r ? ((g-b)/d + (g < b ? 6 : 0))*60 : max === g ? ((b-r)/d+2)*60 : ((r-g)/d+4)*60;
  return [hue, max === 0 ? 0 : d/max, max];
}
function toRgb([h,s,v]: Hsv): Rgb {
  const c = v*s, x = c*(1-Math.abs((h/60)%2-1)), m = v-c;
  const rgb = h < 60 ? [c,x,0] : h < 120 ? [x,c,0] : h < 180 ? [0,c,x] : h < 240 ? [0,x,c] : h < 300 ? [x,0,c] : [c,0,x];
  return rgb.map((n) => Math.round((n+m)*255)) as Rgb;
}
interface ColorPickerProps { value: Rgb; onChange: (value: Rgb) => void; onCommit?: (value: Rgb) => void; presets?: Rgb[]; disabled?: boolean }
export function ColorPicker({ value, onChange, onCommit, presets = DEFAULT_PRESETS, disabled }: ColorPickerProps) {
  const { t } = useTranslation();
  const hex = rgbToHex(value);
  const [hexDraft, setHexDraft] = useState(hex);
  const [expanded, setExpanded] = useState(false);
  const [hsv, setHsv] = useState<Hsv>(() => toHsv(value));
  const latest = useRef(value);
  useEffect(() => {
    setHexDraft(hex); latest.current = value;
    setHsv((previous) => rgbToHex(toRgb(previous)) === hex ? previous : toHsv(value, previous[0]));
  }, [hex, value]);
  const change = (rgb: Rgb, commit = false) => {
    latest.current = rgb; onChange(rgb); if (commit) onCommit?.(rgb);
  };
  const changeHsv = (next: Hsv, commit = false) => { setHsv(next); change(toRgb(next), commit); };
  const commitHex = () => {
    const parsed = hexToRgb(hexDraft);
    if (parsed) { setHsv(toHsv(parsed, hsv[0])); change(parsed, true); } else setHexDraft(hex);
  };
  const plane = (event: PointerEvent<HTMLDivElement>) => {
    if (disabled || event.currentTarget.closest("fieldset:disabled")) return;
    const box = event.currentTarget.getBoundingClientRect();
    changeHsv([hsv[0], Math.max(0,Math.min(1,(event.clientX-box.left)/box.width)), 1-Math.max(0,Math.min(1,(event.clientY-box.top)/box.height))]);
  };
  return <div className={`colorpicker${disabled ? " colorpicker--disabled" : ""}`}>
    <div className="colorpicker__top">
      <button type="button" className="colorpicker__swatch" style={{ background: hex }} disabled={disabled}
        title={t("Choose a custom colour")} aria-label={t("Choose a custom colour")} aria-expanded={expanded} onClick={() => setExpanded(!expanded)} />
      <div className="colorpicker__hexwrap"><span className="colorpicker__hash">#</span>
        <input className="colorpicker__hex" aria-label={t("HEX colour")} type="text" maxLength={7} spellCheck={false}
          value={hexDraft.replace(/^#/, "")} disabled={disabled}
          onChange={(event) => setHexDraft(`#${event.target.value.replace(/[^0-9a-fA-F]/g, "").slice(0, 6)}`)} onBlur={commitHex}
          onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); commitHex(); } }} />
      </div>
      <button className="btn btn--sm" type="button" disabled={disabled} aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>{t(expanded ? "Close palette" : "Custom colour")}</button>
    </div>
    {expanded && <div className="colorpicker__palette">
      <div className="colorpicker__plane" style={{ backgroundColor: `hsl(${hsv[0]},100%,50%)` }}
        role="group" aria-label={t("Saturation and brightness")}
        onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); plane(event); }}
        onPointerMove={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) plane(event); }}
        onPointerUp={(event) => { plane(event); event.currentTarget.releasePointerCapture(event.pointerId); onCommit?.(latest.current); }}
        onPointerCancel={() => onCommit?.(latest.current)}>
        <span className="colorpicker__cursor" style={{ left: `${hsv[1]*100}%`, top: `${(1-hsv[2])*100}%` }} />
      </div>
      {([ ["Hue",0,360,1], ["Saturation",1,100,1], ["Brightness",2,100,1] ] as const).map(([label,index,max,step]) => <label className="colorpicker__channel" key={label}>
        <span>{t(label)}</span><input type="range" min={0} max={max} step={step} disabled={disabled} value={index === 0 ? hsv[0] : Math.round(hsv[index]*100)}
          className={index === 0 ? "colorpicker__hue" : ""}
          onChange={(event) => { const next: Hsv = [...hsv]; next[index] = Number(event.target.value)/(index === 0 ? 1 : 100); changeHsv(next); }}
          onPointerUp={() => onCommit?.(latest.current)} onKeyUp={() => onCommit?.(latest.current)} onBlur={() => onCommit?.(latest.current)} />
        <output>{Math.round(index === 0 ? hsv[0] : hsv[index]*100)}{index === 0 ? "°" : "%"}</output>
      </label>)}
      <div className="colorpicker__rgb">{(["R","G","B"] as const).map((label,index) => <label key={label}>{label}
        <input className="input" type="number" min={0} max={255} step={1} disabled={disabled} value={value[index]} aria-label={`${label} (0–255)`}
          onChange={(event) => { const next: Rgb = [...value]; next[index] = Math.max(0,Math.min(255,Math.round(Number(event.target.value)))); setHsv(toHsv(next,hsv[0])); change(next); }}
          onBlur={() => onCommit?.(latest.current)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); onCommit?.(latest.current); } }} />
      </label>)}</div>
    </div>}
    <div className="colorpicker__swatches">{presets.map((preset) => { const pHex = rgbToHex(preset); return <button key={pHex} type="button" className={`swatch${pHex === hex ? " is-active" : ""}`}
      style={{ background: pHex }} title={pHex} aria-label={pHex} disabled={disabled} onClick={() => { setHsv(toHsv(preset,hsv[0])); change(preset,true); }} />; })}</div>
  </div>;
}
