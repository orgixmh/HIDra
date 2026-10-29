import { useEffect, useId, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "../backend";
import { useTranslation } from "../i18n";
import { getOSDSettings, saveOSDSettings, showOSD } from "../osd";
import { DEFAULT_OSD_SETTINGS, OSD_POSITIONS, validOSDSettings } from "../osd/types";
import type { OSDSettings as Config, OSDPosition } from "../osd/types";
import { Field } from "./ui/Panel";
import { Toggle } from "./ui/Toggle";
import { ColorPicker } from "./ui/ColorPicker";

const fields = [
  { key: "duration", label: "Display duration (ms)", min: 100, max: 30000 },
  { key: "textSize", label: "Text size (px)", min: 10, max: 48 },
  { key: "width", label: "Width (px)", min: 120, max: 1200 },
  { key: "height", label: "Height (px)", min: 64, max: 600 },
  { key: "edgeMargin", label: "Edge margin (px)", min: 0, max: 256 },
  { key: "transparency", label: "Background transparency (%)", min: 0, max: 100 },
] as const;

export function OSDSettings({ open }: { open: boolean }) {
  const { t } = useTranslation();
  const id = useId();
  const [draft, setDraft] = useState<Config>({ ...DEFAULT_OSD_SETTINGS });
  const [saved, setSaved] = useState<Config | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const desktop = isTauri();
  useEffect(() => {
    if (!open) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    void (async () => {
      try {
        if (desktop) {
          const unlisten = await listen<string>("hidra-osd-error", ({ payload }) => { if (!disposed) setError(payload); });
          if (disposed) { unlisten(); return; }
          stop = unlisten;
        }
        const next = await getOSDSettings();
        if (!disposed) { setDraft(next.settings); setSaved(next.settings); setError(next.error); }
      } catch (err) { if (!disposed) setError(String(err)); }
    })();
    return () => { disposed = true; stop?.(); };
  }, [open, desktop]);
  const save = async (preview: boolean) => {
    setBusy(true); setError(null); setNotice(null);
    try {
      const next = await saveOSDSettings(draft);
      setSaved(next.settings); setDraft(next.settings);
      if (preview) await showOSD({ text: t("HIDra OSD Test"), icon: "info" });
      else setNotice("OSD settings saved");
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  const toggle = async (enabled: boolean) => {
    if (!saved) return;
    setBusy(true); setError(null); setNotice(null);
    try {
      const next = await saveOSDSettings({ ...saved, enabled });
      setSaved(next.settings); setDraft((current) => ({ ...current, enabled }));
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  const valid = validOSDSettings(draft);
  const dirty = saved !== null && JSON.stringify(draft) !== JSON.stringify(saved);
  return <section className="settings-osd" aria-labelledby={`${id}-heading`}>
    <div className="settings-osd__heading">
      <h3 id={`${id}-heading`}>{t("On-Screen Display (OSD)")}</h3>
      <Toggle checked={draft.enabled} onChange={(enabled) => void toggle(enabled)} disabled={!desktop || !saved || busy}
        label={t(draft.enabled ? "Ligado" : "Desligado")} />
    </div>
    <p className="note">{t("Short, non-interactive messages above your other windows, including while HIDra is in the tray.")}</p>
    <div className="osd-settings-grid">
      <Field label={t("Position")} htmlFor={`${id}-position`}>
        <select className="input" id={`${id}-position`} value={draft.position} disabled={!desktop || !saved || busy}
          onChange={(event) => setDraft({ ...draft, position: event.target.value as OSDPosition })}>
          {OSD_POSITIONS.map((position) => <option key={position.value} value={position.value}>{t(position.label)}</option>)}
        </select>
      </Field>
      {fields.map(({ key, label, min, max }) => <Field key={key} label={t(label)} htmlFor={`${id}-${key}`} hint={`${min}–${max}`}>
        <input className="input" id={`${id}-${key}`} type="number" min={min} max={max} step={1} value={draft[key]}
          disabled={!desktop || !saved || busy} onChange={(event) => setDraft({ ...draft, [key]: Number(event.target.value) })} />
      </Field>)}
    </div>
    <Field label={t("Background colour")}>
      <ColorPicker value={draft.backgroundColor} disabled={!desktop || !saved || busy}
        onChange={(backgroundColor) => setDraft({ ...draft, backgroundColor })} />
    </Field>
    <p className="note">{t("0% is opaque; 100% is fully transparent. Text and icons stay opaque.")}</p>
    <p className="note">{t("The OSD appears on the monitor containing the pointer. Preview also saves your current settings.")}</p>
    <div className="btnrow">
      <button className="btn" type="button" disabled={!desktop || !saved || busy || !dirty || !valid}
        onClick={() => void save(false)}>{t("Save OSD settings")}</button>
      <button className="btn" type="button" disabled={!desktop || !saved || busy || !draft.enabled || !valid}
        onClick={() => void save(true)}>{t("Preview OSD")}</button>
    </div>
    {!desktop && <p className="note">{t("The OSD is available in the desktop app.")}</p>}
    {!valid && <p className="smart-error" role="alert">{t("Invalid OSD settings. Check the numeric limits shown by each input.")}</p>}
    {notice && <p className="note" role="status">{t(notice)}</p>}
    {error && <p className="smart-error" role="alert">{t(error)}</p>}
  </section>;
}
