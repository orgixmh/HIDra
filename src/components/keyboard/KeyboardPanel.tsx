import { useTranslation } from "../../i18n";
import type { KeyboardCapabilities, KeyboardEffect, KeyboardState, Profile } from "../../types";
import { useLocalStorage } from "../../hooks";
import { Tabs } from "../ui/Tabs";
import type { TabItem } from "../ui/Tabs";
import { SmartNotifications } from "./SmartNotifications";
import type { SmartController } from "../../useSmartNotifications";
import { KeyboardLighting } from "./KeyboardLighting";
import { KeyboardPerKey } from "./KeyboardPerKey";
import { KeyboardRemap } from "./KeyboardRemap";
import { KeyboardSettings } from "./KeyboardSettings";
import { CustomWheel } from "./CustomWheel";
import { KeyboardProfiles } from "./KeyboardProfiles";

type KeyboardTab = "led" | "perkey" | "smart" | "wheel" | "remap" | "profiles" | "settings";

const TABS: TabItem<KeyboardTab>[] = [
  { id: "led", label: "Iluminação" },
  { id: "perkey", label: "Cores por tecla" },
  { id: "smart", label: "Smart Notifications" },
  { id: "wheel", label: "Custom Wheel" },
  { id: "remap", label: "Remapear teclas" },
  { id: "profiles", label: "Perfis" },
  { id: "settings", label: "Settings" },
];

interface KeyboardPanelProps {
  state: KeyboardState | null;
  smart: SmartController;
  capabilities?: KeyboardCapabilities | null;
  effects: KeyboardEffect[];
  /** Model of the keyboard that was found, or a generic label when none is. */
  name: string;
  present: boolean;
  profile: Profile;
  onProfile: (profile: Profile) => void;
  applied: string;
  onApplied: () => void;
}

export function KeyboardPanel({
  state,
  smart,
  capabilities,
  effects,
  name,
  present,
  profile,
  onProfile,
  applied,
  onApplied,
}: KeyboardPanelProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useLocalStorage<KeyboardTab>("hidra.keyboardTab", "led");

  const tabs = TABS.filter((t) =>
    t.id === "led" || t.id === "smart" || t.id === "settings" ||
    (t.id === "wheel" && !!capabilities?.custom_wheel) ||
    (t.id === "perkey" && (capabilities?.per_key ?? true)) ||
    (t.id === "remap" && (capabilities?.remap ?? true)) ||
    (t.id === "profiles" && (capabilities?.profiles ?? 3) > 1));
  const active = tabs.some((t) => t.id === tab) ? tab : "led";
  const features = [
    { label: "LIGHTING", available: present },
    { label: "PER KEY", available: present && (capabilities?.per_key ?? false) },
    { label: "SMART", available: present && (capabilities?.smart_notifications ?? false) },
  ];

  return (
    <div className="workspace">
      <div className="workspace__top">
        <div className="workspace__heading">
          <h1>{t(name)}</h1>
          {(capabilities?.profiles ?? 3) > 1 && <span className="chip chip--muted">{t("Perfil {profile}", { profile })}</span>}
          <span className={`chip${present ? " chip--on" : " chip--off"}`}>
            {present ? t("Conectado") : t("Desconectado")}
          </span>
          <span className="keyboard-capabilities" role="group" aria-label={t("Keyboard features")}>
            {features.map(({ label, available }) => {
              const description = `${t(label)}: ${t(available ? "Available" : "Unavailable")}`;
              return <span key={label}
                className={`keyboard-capability${available ? " keyboard-capability--on" : ""}`}
                role="img" aria-label={description} title={description}>
                <span className={`led${available ? " led--on" : ""}`} aria-hidden="true" />
                {t(label)}
              </span>;
            })}
          </span>
        </div>
        <Tabs tabs={tabs} active={active} onChange={setTab} />
      </div>

      {!present && (
        <p className="disconnected-note">
          {t("Teclado não detectado. Conecte o cabo para aplicar alterações — os controles abaixo continuam editáveis.")}</p>
      )}

      <div className="workspace__body">
        {active === "wheel" && capabilities?.custom_wheel ? <CustomWheel key={capabilities.custom_wheel.settings_key} capability={capabilities.custom_wheel} /> : active === "settings" ? <KeyboardSettings present={present} model={name} /> : active === "smart" ? <SmartNotifications controller={smart}
          layout={capabilities?.layout ?? undefined} aspect={capabilities?.layout_aspect ?? undefined}
          supported={capabilities?.smart_notifications ?? false} present={present} /> : <>
        {smart.view?.active && <p className="note">{t("Smart Notifications controls the lighting. Turn it off to use manual controls.")}</p>}
        <fieldset className="smart-manual-controls" disabled={smart.view?.active ?? false}>
        {active === "led" && (
          <KeyboardLighting
            state={state}
            capabilities={capabilities}
            present={present}
            effects={effects}
            profile={profile}
            applied={applied}
            onApplied={onApplied}
          />
        )}
        {active === "perkey" && <KeyboardPerKey key={name} profile={profile} applied={applied} layout={capabilities?.layout ?? undefined} aspect={capabilities?.layout_aspect ?? undefined} present={present} />}
        {active === "remap" && <KeyboardRemap applied={applied} />}
        {active === "profiles" && (
          <KeyboardProfiles
            profile={profile}
            onProfile={onProfile}
            applied={applied}
          />
        )}
        </fieldset></>}
      </div>
    </div>
  );
}
