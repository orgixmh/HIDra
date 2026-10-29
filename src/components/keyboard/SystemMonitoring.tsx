import { useEffect, useRef, useState } from "react";
import { useTranslation } from "../../i18n";
import type { MonitoringSettings, Rgb } from "../../types";
import { DEFAULT_MONITORING } from "../../types";
import type { SmartController } from "../../useSmartNotifications";
import { ColorPicker } from "../ui/ColorPicker";
import { Field } from "../ui/Panel";
import { Toggle } from "../ui/Toggle";
import { useToast } from "../Toast";

export function SystemMonitoring({ controller, available }: { controller: SmartController; available: boolean }) {
  const { t } = useTranslation();
  const notify = useToast();
  const { view, setMonitoring } = controller;
  const [settings, setSettings] = useState<MonitoringSettings>(DEFAULT_MONITORING);
  const pending = useRef<MonitoringSettings | null>(null);
  const acknowledged = useRef<MonitoringSettings>(DEFAULT_MONITORING);
  const sending = useRef(false);
  const saved = JSON.stringify(view?.settings.monitoring ?? DEFAULT_MONITORING);
  useEffect(() => {
    acknowledged.current = JSON.parse(saved) as MonitoringSettings;
    if (!sending.current) setSettings(acknowledged.current);
  }, [saved]);
  const update = (kind: keyof MonitoringSettings, change: { enabled?: boolean; color?: Rgb; bar?: boolean }) => {
    const next = { ...settings, [kind]: { ...settings[kind], ...change } };
    setSettings(next); pending.current = next;
    if (sending.current) return;
    sending.current = true;
    void (async () => {
      try {
        while (pending.current) {
          const value = pending.current; pending.current = null;
          const next = await setMonitoring(value);
          acknowledged.current = next.settings.monitoring;
        }
      } catch (err) { pending.current = null; setSettings(acknowledged.current); notify(String(err), "error"); }
      finally { sending.current = false; }
    })();
  };
  return <section className="system-monitoring" aria-label={t("System monitoring")}>
    <h3>{t("System monitoring")}</h3>
    <p className="note">{t("Choose a single key or a filled bar for each row. Above 90% blinks. Notifications take priority over monitoring keys.")}</p>
    <div className="system-monitoring__grid">
      {([ ["cpu", "CPU monitoring"], ["ram", "RAM monitoring"], ["disk", "Disk space (/)"] ] as const).map(([kind, label]) => {
        const reading = view?.metrics?.[kind];
        return <div className="system-monitoring__meter" key={kind}>
          <div className="system-monitoring__heading">
            <Toggle checked={settings[kind].enabled} disabled={!view || !available}
              onChange={(enabled) => update(kind, { enabled })} label={t(label)} />
            <output className={`chip${(reading?.percent ?? 0) > 90 ? " chip--off" : " chip--muted"}`}>
              {!settings[kind].enabled ? t("Desligado") : !view?.active ? "—" : reading?.percent != null ? `${Math.round(reading.percent)}%` : "—"}
            </output>
          </div>
          <Field label={t("Display mode")}>
            <select className="input" value={settings[kind].bar ? "bar" : "single"} disabled={!view || !available}
              onChange={(event) => update(kind, { bar: event.target.value === "bar" })}>
              <option value="single">{t("Single key")}</option>
              <option value="bar">{t("Bar")}</option>
            </select>
          </Field>
          <ColorPicker value={settings[kind].color} disabled={!view || !available}
            onChange={(color) => update(kind, { color })} />
          {settings[kind].enabled && reading?.error && <p className="smart-error" role="status">{t(reading.error)}</p>}
        </div>;
      })}
    </div>
    {!view?.active && <p className="note">{t("Turn on Smart Notifications to display system usage on your keyboard.")}</p>}
  </section>;
}
