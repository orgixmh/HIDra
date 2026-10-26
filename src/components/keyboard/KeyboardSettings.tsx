import { useEffect, useState } from "react";
import { callBackend } from "../../backend";
import { useTranslation } from "../../i18n";
import type { Rgb } from "../../types";
import { Panel, Field } from "../ui/Panel";
import { Toggle } from "../ui/Toggle";

type Profile = { enabled: boolean; white: Rgb };
type Calibration = { profile: Profile; suggested_white: Rgb; error: string | null };

export function KeyboardSettings({ present, model }: { present: boolean; model: string }) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Calibration | null>(null);
  const [white, setWhite] = useState<Rgb>([255, 255, 255]);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let disposed = false;
    setBusy(true); setError(null); setSettings(null);
    void callBackend<Calibration | null>("keyboard_calibration_get").then((next) => {
      if (disposed) return;
      setSettings(next); setWhite(next?.profile.white ?? [255, 255, 255]); setError(next?.error ?? null);
    }).catch((err) => { if (!disposed) setError(String(err)); })
      .finally(() => { if (!disposed) setBusy(false); });
    return () => { disposed = true; };
  }, [present, model]);
  const save = async (profile: Profile) => {
    setBusy(true); setError(null);
    try {
      const next = await callBackend<Calibration>("keyboard_calibration_save", { profile });
      setSettings(next); setWhite(next.profile.white);
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  const dirty = settings && white.some((value, i) => value !== settings.profile.white[i]);
  return <Panel title={t("RGB calibration")} subtitle={t("Balance a colour cast so that white and your chosen colours look more natural.")}
    actions={<Toggle checked={settings?.profile.enabled ?? false} disabled={busy || !present || !settings}
      label={t(settings?.profile.enabled ? "Ligado" : "Desligado")}
      onChange={(enabled) => void save({ enabled, white })} />}>
    <p className="note">{t("LED channels can have different strengths. Calibration scales each channel before sending colours to the keyboard, while keeping your chosen colours unchanged in HIDra.")}</p>
    {!busy && !settings && <p className="note">{t("Calibration requires a connected keyboard with calibration support.")}</p>}
    {settings && <>
      <Field label={t("Calibrated white")} hint={t("Enter the RGB values that look white on your keyboard. HIDra maps white (255/255/255) to these values and scales other colours by the same ratios.")}>
        <div className="calibration-channels">
          {(["R", "G", "B"] as const).map((channel, i) => <label key={channel}>
            <span>{channel}</span>
            <input className="input" type="number" min={1} max={255} step={1} value={white[i]}
              disabled={busy || !present} onChange={(event) => {
                const next: Rgb = [...white]; next[i] = Math.max(1, Math.min(255, Math.round(Number(event.target.value)) || 1)); setWhite(next);
              }} />
          </label>)}
        </div>
      </Field>
      <p className="note">{t("Applies to chosen Lighting colours, saved per-key colours and Smart Notifications. Built-in rainbow effects generate their own colours and cannot be corrected here.")}</p>
      <p className="note">{t("Smart Notifications updates immediately. For Lighting and per-key colours, apply your chosen colours again after changing calibration. Turning calibration off sends colours unchanged.")}</p>
      <div className="btnrow">
        <button className="btn btn--primary" type="button" disabled={busy || !present || !dirty}
          onClick={() => void save({ enabled: settings.profile.enabled, white })}>{t("Apply calibration")}</button>
        <button className="btn" type="button" disabled={busy || !present}
          onClick={() => setWhite(settings.suggested_white)}>{t("Use suggested white")}</button>
        <button className="btn" type="button" disabled={busy || !present}
          onClick={() => setWhite([255, 255, 255])}>{t("Reset channels")}</button>
      </div>
    </>}
    {error && <p className="smart-error" role="alert">{t(error)}</p>}
  </Panel>;
}
