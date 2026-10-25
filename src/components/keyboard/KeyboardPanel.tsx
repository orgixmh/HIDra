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
import { KeyboardProfiles } from "./KeyboardProfiles";

type KeyboardTab = "led" | "perkey" | "smart" | "remap" | "profiles";

const TABS: TabItem<KeyboardTab>[] = [
  { id: "led", label: "Iluminação" },
  { id: "perkey", label: "Cores por tecla" },
  { id: "smart", label: "Smart Notifications" },
  { id: "remap", label: "Remapear teclas" },
  { id: "profiles", label: "Perfis" },
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
    t.id === "led" || t.id === "smart" ||
    (t.id === "perkey" && (capabilities?.per_key ?? true)) ||
    (t.id === "remap" && (capabilities?.remap ?? true)) ||
    (t.id === "profiles" && (capabilities?.profiles ?? 3) > 1));
  const active = tabs.some((t) => t.id === tab) ? tab : "led";

  return (
    <div className="workspace">
      <div className="workspace__top">
        <div className="workspace__heading">
          <h1>{t(name)}</h1>
          {(capabilities?.profiles ?? 3) > 1 && <span className="chip chip--muted">{t("Perfil {profile}", { profile })}</span>}
          <span className={`chip${present ? " chip--on" : " chip--off"}`}>
            {present ? t("Conectado") : t("Desconectado")}
          </span>
        </div>
        <Tabs tabs={tabs} active={active} onChange={setTab} />
      </div>

      {!present && (
        <p className="disconnected-note">
          {t("Teclado não detectado. Conecte o cabo para aplicar alterações — os controles abaixo continuam editáveis.")}</p>
      )}

      {capabilities?.remap === false && <p className="note">{t("Lighting and per-key colours are available over USB. Key remapping and profile switching are not supported yet.")}</p>}
      <div className="workspace__body">
        {active === "smart" ? <SmartNotifications controller={smart}
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
