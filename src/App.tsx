import { useTranslation } from "./i18n";
import { useCallback, useEffect, useState } from "react";
import "./App.css";
import type {
  DeviceKind,
  KeyboardEffect,
  KeyboardState,
  ListDevices,
  MouseState,
  Profile,
} from "./types";
import {
  keyboardEffects,
  keyboardReadState,
  keyboardSetActiveProfile,
  listDevices,
  mouseReadState,
} from "./backend";
import { useLocalStorage, useUpdater } from "./hooks";
import { ToastProvider, useToast } from "./components/Toast";
import { useSmartNotifications } from "./useSmartNotifications";
import { CloseToTray } from "./components/CloseToTray";
import { Sidebar } from "./components/Sidebar";
import { UpdateBanner } from "./components/UpdateBanner";
import { SegmentedControl } from "./components/ui/SegmentedControl";
import { MousePanel } from "./components/mouse/MousePanel";
import { KeyboardPanel } from "./components/keyboard/KeyboardPanel";

const EMPTY_DEVICES: ListDevices = {
  keyboard: false,
  mouse: false,
  mouse_mode: null,
  keyboard_vendor: null,
  keyboard_model: null,
  mouse_vendor: null,
  mouse_model: null,
  keyboard_untested: false,
  mouse_untested: false,
};

function AppShell() {
  const { t } = useTranslation();
  const notify = useToast();
  const smart = useSmartNotifications();

  const [devices, setDevices] = useState<ListDevices>(EMPTY_DEVICES);
  const [selected, setSelected] = useLocalStorage<DeviceKind>("hidra.device", "mouse");
  const [profile, setProfile] = useLocalStorage<Profile>("hidra.profile", 1);

  const [mouseState, setMouseState] = useState<MouseState | null>(null);
  const [keyboardState, setKeyboardState] = useState<KeyboardState | null>(null);
  const [effects, setEffects] = useState<KeyboardEffect[]>([]);

  const refreshDevices = useCallback(() => {
    listDevices()
      .then(setDevices)
      .catch((e: unknown) => notify(String(e), "error"));
  }, [notify]);

  const loadMouse = useCallback(() => {
    mouseReadState()
      .then(setMouseState)
      .catch((e: unknown) => notify(String(e), "error"));
  }, [notify]);

  const loadKeyboard = useCallback(
    (p: Profile) => {
      keyboardReadState(p)
        .then(setKeyboardState)
        .catch((e: unknown) => notify(String(e), "error"));
    },
    [notify],
  );

  // Selecting a profile also switches the profile the keyboard displays, like
  // the official software — so what you edit is what lights up.
  const selectProfile = useCallback(
    (p: Profile) => {
      setProfile(p);
      if (devices.keyboard) {
        keyboardSetActiveProfile(p)
          .then(() => notify(`Perfil ${p} ativo`, "success"))
          .catch((e: unknown) => notify(String(e), "error"));
      }
    },
    [devices.keyboard, notify, setProfile],
  );

  const caps = devices.keyboard_capabilities;
  const keyboardProfile: Profile = caps?.profiles === 1 ? 1 : profile;
  const smartReady = smart.view !== null;
  const lightingLocked = smart.view?.active ?? false;

  // Initial load: devices + keyboard effect catalogue.
  useEffect(() => {
    refreshDevices();
  }, [refreshDevices]);

  useEffect(() => {
    if (!devices.keyboard) return;
    keyboardEffects()
      .then(setEffects)
      .catch((e: unknown) => notify(String(e), "error"));
  }, [devices.keyboard_model, devices.keyboard, notify]);

  // Load the selected device's state when device/profile changes.
  useEffect(() => {
    if (selected === "mouse") loadMouse();
    else {
      setKeyboardState(null);
      if (devices.keyboard && smartReady && !lightingLocked) loadKeyboard(keyboardProfile);
    }
  }, [selected, keyboardProfile, devices.keyboard_model, devices.keyboard, smartReady, lightingLocked, loadMouse, loadKeyboard]);

  // Device discovery sends a model query on some keyboards. Keep it explicit:
  // polling can interrupt their lighting even when no setting is being changed.

  const updater = useUpdater();

  const mode = mouseState?.mode ?? devices.mouse_mode;
  const mouseApplied =
    mode === "wired" ? "Aplicado via cabo" : "Aplicado via 2.4G";

  return (
    <div className="app">
      <CloseToTray />
      <Sidebar
        devices={devices}
        lightingLocked={smart.view?.active ?? false}
        version={updater.version}
        checking={updater.state.kind === "checking"}
        onCheckUpdates={() => void updater.check(true)}
        selected={selected}
        onSelect={setSelected}
        profile={profile}
        onProfile={selectProfile}
      />

      <main className="main">
        <UpdateBanner
          state={updater.state}
          support={updater.support}
          onInstall={(update) => void updater.install(update)}
          onDismiss={updater.dismiss}
        />

        <header className="topbar">
          <SegmentedControl<DeviceKind>
            value={selected}
            onChange={setSelected}
            options={[
              { value: "mouse", label: t("Mouse") },
              { value: "keyboard", label: t("Teclado") },
            ]}
          />
          <button
            type="button"
            className="btn"
            onClick={refreshDevices}
            title={t("Procurar dispositivos após conectar ou desconectar o cabo/dongle")}
          >
            {t("Atualizar dispositivos")}</button>
          <div className="topbar__status">
            <span className={`led${devices.mouse ? " led--on" : ""}`} />
            {t("Mouse")}<span className={`led${devices.keyboard ? " led--on" : ""}`} />
            {t("Teclado")}</div>
        </header>

        {selected === "mouse" ? (
          <MousePanel
            state={mouseState}
            name={devices.mouse_model ?? t("Mouse")}
            present={devices.mouse}
            applied={mouseApplied}
            onApplied={loadMouse}
          />
        ) : (
          <KeyboardPanel
            key={devices.keyboard_model ?? "none"}
            state={keyboardState}
            smart={smart}
            capabilities={caps}
            name={devices.keyboard_model ?? t("Teclado")}
            effects={effects}
            present={devices.keyboard}
            profile={keyboardProfile}
            onProfile={selectProfile}
            applied="Aplicado"
            onApplied={() => loadKeyboard(keyboardProfile)}
          />
        )}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <AppShell />
    </ToastProvider>
  );
}
