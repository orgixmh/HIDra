import { useEffect, useState } from "react";
import { callBackend, isTauri } from "../../backend";
import { useTranslation } from "../../i18n";
import { showOSD } from "../../osd";
import type { CustomWheelCapability, CustomWheelPreferences, CustomWheelView, WheelMode } from "../../types";
import { Panel, Field } from "../ui/Panel";
import { Toggle } from "../ui/Toggle";

const MODES: { id: WheelMode; label: string; message: string; icon: string }[] = [
  { id: "volume", label: "Volume Control", message: "Volume Mode", icon: "volume" },
  { id: "brightness", label: "Brightness Control", message: "Brightness Mode", icon: "sun" },
  { id: "zoom", label: "Zoom Control", message: "Zoom Mode", icon: "info" },
];

export function CustomWheel({ capability }: { capability: CustomWheelCapability }) {
  const { t } = useTranslation();
  const [preferences, setPreferences] = useState<CustomWheelPreferences | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [previewMode, setPreviewMode] = useState<WheelMode>("brightness");
  const desktop = isTauri();
  useEffect(() => {
    let disposed = false;
    void callBackend<CustomWheelView>("custom_wheel_get", { device_key: capability.settings_key })
      .then((view) => { if (!disposed) { setPreferences(view.preferences); setError(view.error); } })
      .catch((err: unknown) => { if (!disposed) setError(String(err)); });
    return () => { disposed = true; };
  }, [capability.settings_key]);
  const save = async (next: CustomWheelPreferences) => {
    setBusy(true); setError(null); setNotice(null);
    try {
      const view = await callBackend<CustomWheelView>("custom_wheel_save", { device_key: capability.settings_key, preferences: next });
      setPreferences(view.preferences); setError(view.error); setNotice("Custom Wheel settings saved");
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  const setMode = (mode: WheelMode, enabled: boolean) => {
    if (!preferences || mode === "volume") return;
    const modes = enabled ? [...preferences.modes, mode] : preferences.modes.filter((item) => item !== mode);
    void save({ ...preferences, modes });
  };
  const move = (index: number, offset: number) => {
    if (!preferences) return;
    const modes = [...preferences.modes];
    const other = index + offset;
    if (other < 0 || other >= modes.length) return;
    [modes[index], modes[other]] = [modes[other], modes[index]];
    void save({ ...preferences, modes });
  };
  const preview = async () => {
    const mode = MODES.find((item) => item.id === previewMode);
    if (!mode) return;
    setBusy(true); setError(null); setNotice(null);
    try {
      const accepted = await showOSD({ text: t(mode.message), icon: mode.icon });
      if (!accepted) setNotice("Enable the global OSD in Settings to preview.");
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  const disabled = !desktop || !preferences || busy;
  const orderedModes = [...(preferences?.modes ?? []), ...MODES.map(({ id }) => id).filter((mode) => !preferences?.modes.includes(mode))];
  return <Panel title={t("Custom Wheel")} subtitle={t("Configure wheel modes and the planned press gestures.")}>
    <p className="note">{t("Phase 1: preferences only. Actual wheel remapping and gesture detection will be implemented in Phase 2. The wheel keeps its normal system behaviour for now.")}</p>
    <Field label={t("Enable Custom Wheel")}>
      <Toggle checked={preferences?.enabled ?? false} disabled={disabled}
        label={t(preferences?.enabled ? "Ligado" : "Desligado")}
        onChange={(enabled) => { if (preferences) void save({ ...preferences, enabled }); }} />
    </Field>
    <h3 className="custom-wheel__heading">{t("Available modes and cycle order")}</h3>
    <p className="note">{t("Volume is always available as the fallback. Move enabled modes up or down to set their future cycle order.")}</p>
    <div className="custom-wheel__modes">
      {orderedModes.map((mode) => {
        const details = MODES.find((item) => item.id === mode)!;
        const index = preferences?.modes.indexOf(mode) ?? -1;
        const enabled = index >= 0;
        return <div className="custom-wheel__mode" key={mode}>
          <Toggle checked={enabled} disabled={disabled || mode === "volume"} label={t(details.label)} onChange={(value) => setMode(mode, value)} />
          {enabled && <div className="btnrow">
            <span className="chip chip--muted" aria-label={t("Cycle position {position}", { position: index + 1 })}>{index + 1}</span>
            <button className="btn btn--sm" type="button" disabled={disabled || index === 0} onClick={() => move(index, -1)}
              aria-label={t("Move {mode} up", { mode: t(details.label) })}>{t("Move up")}</button>
            <button className="btn btn--sm" type="button" disabled={disabled || index === (preferences?.modes.length ?? 0) - 1} onClick={() => move(index, 1)}
              aria-label={t("Move {mode} down", { mode: t(details.label) })}>{t("Move down")}</button>
          </div>}
        </div>;
      })}
    </div>
    <h3 className="custom-wheel__heading">{t("Planned wheel gestures")}</h3>
    <dl className="custom-wheel__gestures">
      <dt>{t("Single press")}</dt><dd>{t("Switch to next enabled mode")}</dd>
      <dt>{t("Double press")}</dt><dd>{t("Toggle system mute")}</dd>
      <dt>{t("Long press")}</dt><dd>{t("Return to Volume mode")}</dd>
    </dl>
    <p className="note">{t("Planned timing: double press window 300 ms; long press threshold 700 ms. These assignments are read-only.")}</p>
    <Field label={t("Show OSD when switching modes")}>
      <Toggle checked={preferences?.show_osd ?? true} disabled={disabled}
        label={t(preferences?.show_osd !== false ? "Ligado" : "Desligado")}
        onChange={(show_osd) => { if (preferences) void save({ ...preferences, show_osd }); }} />
    </Field>
    <div className="custom-wheel__preview">
      <Field label={t("Preview mode")}>
        <select className="input" value={previewMode} disabled={disabled}
          aria-label={t("Preview mode")} onChange={(event) => setPreviewMode(event.target.value as WheelMode)}>
          {MODES.map((mode) => <option key={mode.id} value={mode.id}>{t(mode.label)}</option>)}
        </select>
      </Field>
      <button className="btn" type="button" disabled={disabled || !preferences?.show_osd} onClick={() => void preview()}>{t("Preview mode OSD")}</button>
    </div>
    <p className="note">{t("Preview uses the global OSD appearance settings and does not change the hardware mode.")}</p>
    {notice && <p className="note" role="status">{t(notice)}</p>}
    {error && <p className="smart-error" role="alert">{t(error)}</p>}
  </Panel>;
}
