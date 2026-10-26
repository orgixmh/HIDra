import { Settings } from "./Settings";
import { useTranslation } from "../i18n";
import type { DeviceKind, ListDevices, Profile } from "../types";
import { SegmentedControl } from "./ui/SegmentedControl";

interface SidebarProps {
  devices: ListDevices;
  lightingLocked?: boolean;
  version: string;
  checking: boolean;
  onCheckUpdates: () => void;
  selected: DeviceKind;
  onSelect: (kind: DeviceKind) => void;
  profile: Profile;
  onProfile: (profile: Profile) => void;
}

interface DeviceMeta {
  kind: DeviceKind;
  name: string;
  model: string;
  vendor: string | null;
  connected: boolean;
  untested: boolean;
  mode?: string;
}

export function Sidebar({
  devices,
  lightingLocked,
  version,
  checking,
  onCheckUpdates,
  selected,
  onSelect,
  profile,
  onProfile,
}: SidebarProps) {
  const { t } = useTranslation();
  const items: DeviceMeta[] = [
    {
      kind: "mouse",
      name: t("Mouse"),
      model: devices.mouse_model ?? t("Nenhum compatível"),
      vendor: devices.mouse_vendor,
      connected: devices.mouse,
      untested: devices.mouse_untested,
      mode:
        devices.mouse_mode === "2.4g"
          ? "2.4G"
          : devices.mouse_mode === "wired"
            ? t("Cabo")
            : undefined,
    },
    {
      kind: "keyboard",
      name: t("Teclado"),
      model: devices.keyboard_model ?? t("Nenhum compatível"),
      vendor: devices.keyboard_vendor,
      connected: devices.keyboard,
      untested: devices.keyboard_untested,
    },
  ];

  return (
    <aside className="sidebar">
      <div className="brand">
        <img className="brand__logo" src="/favicon.svg" alt="" aria-hidden="true" />
        <div className="brand__text">
          <strong>HIDra</strong>
          <span>CONFIG</span>
        </div>
      </div>

      <nav className="devicelist">
        {items.map((item) => (
          <div key={item.kind} className="devicelist__group">
            <button
              type="button"
              className={`device${item.kind === selected ? " is-active" : ""}`}
              onClick={() => onSelect(item.kind)}
            >
              <span
                className={`device__dot${item.connected ? " is-on" : ""}`}
                title={item.connected ? t("Conectado") : t("Ausente")}
              />
              <span className="device__info">
                <span className="device__name">{t(item.name)}</span>
                <span
                  className="device__model"
                  title={[item.vendor, item.model].filter(Boolean).join(" ")}
                >
                  {t(item.model)}
                </span>
              </span>
              {item.untested && item.connected && (
                <span
                  className="device__mode device__mode--warn"
                  title={t("Modelo adicionado pela comunidade e ainda não testado em hardware")}
                >
                  ?
                </span>
              )}
              {item.mode && item.connected && (
                <span className="device__mode">{item.mode}</span>
              )}
            </button>

            {item.kind === "keyboard" && selected === "keyboard" && devices.keyboard_capabilities?.profiles !== 1 && (
              <div className="profilepick">
                <span className="profilepick__label">{t("Perfil")}</span>
                <SegmentedControl<Profile>
                  size="sm"
                  disabled={lightingLocked}
                  value={profile}
                  onChange={onProfile}
                  options={[
                    { value: 1, label: "1" },
                    { value: 2, label: "2" },
                    { value: 3, label: "3" },
                  ]}
                />
              </div>
            )}
          </div>
        ))}
      </nav>

      <div className="sidebar__foot">
        <Settings />
        <span className="sidebar__hint">
          {t("Configuração local via 2.4G / cabo")}</span>
        <div className="sidebar__version">
          <span>{version && version !== "dev" ? `v${version}` : ""}</span>
          <button
            type="button"
            className="linkbtn"
            onClick={onCheckUpdates}
            disabled={checking}
          >
            {checking ? t("Procurando…") : t("Procurar atualizações")}
          </button>
        </div>
      </div>
    </aside>
  );
}
