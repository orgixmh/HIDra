// Tauri bridge. Every command goes through `callBackend`, which either calls the
// real Rust backend via `invoke` or, when running in a plain browser, a MOCK
// layer so `npm run build` and browser preview work without hardware.

import { invoke } from "@tauri-apps/api/core";
import { KEYBOARD_EFFECTS_FALLBACK } from "./constants";
import type {
  ButtonAction,
  ButtonRaw,
  CustomCell,
  DpiStage,
  KeyboardEffect,
  KeyboardState,
  LedEffect,
  ListDevices,
  MouseState,
  Profile,
  RemapKey,
  RemapKind,
  Rgb,
  SmartSettings,
  SmartView,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.__TAURI_INTERNALS__ !== "undefined"
  );
}

// ---------------------------------------------------------------------------
// MOCK backend — plausible fake data + no-op writes (logged to console)
// ---------------------------------------------------------------------------

function clampByte(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)));
}

const mockDevices: ListDevices = {
  keyboard: true,
  mouse: true,
  mouse_mode: "2.4g",
  keyboard_vendor: "Redragon",
  keyboard_model: "Brahma Pro (K586RGB-PRO)",
  mouse_vendor: "Redragon",
  mouse_model: "Cobra Pro (M711-PRO)",
  keyboard_untested: false,
  mouse_untested: false,
};

const mockMouse: MouseState = {
  present: true,
  mode: "2.4g",
  led: { effect: "steady", r: 216, g: 31, b: 38, speed: 128, brightness: 255 },
  dpi: [
    { x: 800, y: 800, color: [216, 31, 38], enabled: true },
    { x: 1600, y: 1600, color: [0, 190, 255], enabled: true },
    { x: 3200, y: 3200, color: [0, 230, 90], enabled: true },
    { x: 6400, y: 6400, color: [255, 170, 0], enabled: false },
    { x: 12000, y: 12000, color: [180, 0, 255], enabled: false },
  ],
  dpi_effect: true,
  polling: 1000,
  buttons: Array.from({ length: 16 }, () => [0, 0, 0] as ButtonRaw),
};

const mockKeyboards: Record<Profile, KeyboardState> = {
  1: {
    present: true,
    active: 1,
    effect: 6,
    brightness: 5,
    speed: 3,
    direction: 0,
    full_rgb: false,
    color: [216, 31, 38],
  },
  2: {
    present: true,
    active: 1,
    effect: 11,
    brightness: 4,
    speed: 2,
    direction: 0,
    full_rgb: true,
    color: [0, 180, 255],
  },
  3: {
    present: true,
    active: 1,
    effect: 5,
    brightness: 3,
    speed: 3,
    direction: 1,
    full_rgb: false,
    color: [120, 0, 255],
  },
};

let mockActiveProfile: Profile = 1;

const MOCK_REMAP_KEYS: RemapKey[] = [
  ...["Esc", "Tab", "Caps", "Enter", "Espaço", "Ctrl E", "Alt E", "Win", "Menu"],
  ..."QWERTYUIOPASDFGHJKLZXCVBNM".split(""),
  ..."1234567890".split(""),
  ..."F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12".split(" "),
].map((label, index) => ({
  label,
  index,
  raw: [0x02, 0x02, 0x04 + index] as ButtonRaw,
}));

function log(cmd: string, args?: Record<string, unknown>): void {
  // eslint-disable-next-line no-console
  console.info(`[mock] ${cmd}`, args ?? {});
}


const mockSmart: SmartView = { settings: { enabled: false, rules: [], background: [24, 24, 32] }, active: false,
  status: "off", error: null, recent: [], lit_keys: [], scripts: [] };
try {
  const saved = JSON.parse(localStorage.getItem("hidra.smartNotifications.preview") ?? "null") as SmartSettings | null;
  if (saved && typeof saved.enabled === "boolean" && Array.isArray(saved.rules)) {
    mockSmart.settings = { ...saved, background: saved.background ?? [24, 24, 32],
      rules: saved.rules.map((rule) => ({ ...rule, blinking: rule.blinking ?? false, source: rule.source ?? "notification", script_path: rule.script_path ?? "", interval_seconds: rule.interval_seconds ?? 30,
        duration_seconds: [0, 60, 300, 600].includes(rule.duration_seconds) ? rule.duration_seconds : 0 })) };
    mockSmart.active = saved.enabled;
    mockSmart.status = saved.enabled ? "listening" : "off";
  }
} catch { /* Preview preferences are optional. */ }

function mockBackend<T>(cmd: string, args?: Record<string, unknown>): T {
  const a = (args ?? {}) as Record<string, unknown>;
  switch (cmd) {
    case "smart_notifications_get":
      return structuredCloneSafe(mockSmart) as T;
    case "smart_notifications_save": {
      mockSmart.settings = structuredCloneSafe(a.settings as SmartSettings);
      mockSmart.active = mockSmart.settings.enabled;
      mockSmart.status = mockSmart.active ? "listening" : "off";
      mockSmart.lit_keys = [];
      try { localStorage.setItem("hidra.smartNotifications.preview", JSON.stringify(mockSmart.settings)); } catch { /* optional */ }
      return structuredCloneSafe(mockSmart) as T;
    }
    case "smart_notifications_background": {
      mockSmart.settings.background = structuredCloneSafe(a.color as Rgb);
      try { localStorage.setItem("hidra.smartNotifications.preview", JSON.stringify(mockSmart.settings)); } catch { /* optional */ }
      return structuredCloneSafe(mockSmart) as T;
    }
    case "smart_notifications_clear": {
      mockSmart.lit_keys = [];
      return structuredCloneSafe(mockSmart) as T;
    }
    case "smart_notifications_test": {
      if (!mockSmart.active) throw new Error("Turn on Smart Notifications before testing a rule");
      const rule = mockSmart.settings.rules.find((rule) => rule.id === a.id);
      if (!rule) throw new Error("Save the rule before testing it");
      mockSmart.lit_keys = [...rule.keys];
      mockSmart.recent.unshift({ app: "HIDra preview", title: rule.name, body: "Test notification", matched: true });
      mockSmart.recent = mockSmart.recent.slice(0, 12);
      setTimeout(() => {
        mockSmart.lit_keys = [];
        window.dispatchEvent(new CustomEvent("smart-notifications-mock", { detail: structuredCloneSafe(mockSmart) }));
      }, 10_000);
      return structuredCloneSafe(mockSmart) as T;
    }

    case "list_devices":
      return { ...mockDevices } as T;

    case "mouse_read_state":
      return structuredCloneSafe(mockMouse) as T;

    case "mouse_set_led": {
      log(cmd, a);
      mockMouse.led = {
        effect: a.effect as LedEffect,
        r: clampByte(a.r as number),
        g: clampByte(a.g as number),
        b: clampByte(a.b as number),
        speed: clampByte(a.speed as number),
        brightness: clampByte(a.brightness as number),
      };
      return undefined as T;
    }

    case "mouse_set_dpi_stage": {
      log(cmd, a);
      const stage = (a.stage as number) - 1;
      if (mockMouse.dpi && mockMouse.dpi[stage]) {
        mockMouse.dpi[stage].x = a.x as number;
        mockMouse.dpi[stage].y = a.y as number;
      }
      return undefined as T;
    }

    case "mouse_set_dpi_all": {
      log(cmd, a);
      mockMouse.dpi = a.dpi as DpiStage[];
      return undefined as T;
    }

    case "mouse_set_dpi_color": {
      log(cmd, a);
      const stage = (a.stage as number) - 1;
      if (mockMouse.dpi && mockMouse.dpi[stage]) {
        mockMouse.dpi[stage].color = [
          clampByte(a.r as number),
          clampByte(a.g as number),
          clampByte(a.b as number),
        ];
      }
      return undefined as T;
    }

    case "mouse_set_dpi_effect": {
      log(cmd, a);
      mockMouse.dpi_effect = Boolean(a.on);
      return undefined as T;
    }

    case "mouse_set_polling": {
      log(cmd, a);
      mockMouse.polling = a.hz as number;
      return undefined as T;
    }

    case "mouse_set_button": {
      log(cmd, a);
      return undefined as T;
    }

    case "keyboard_read_state": {
      const profile = (a.profile as Profile) ?? 1;
      return { ...mockKeyboards[profile], active: mockActiveProfile } as T;
    }

    case "keyboard_effects":
      return KEYBOARD_EFFECTS_FALLBACK.map((e) => ({ ...e })) as T;

    case "keyboard_set_effect": {
      log(cmd, a);
      const profile = (a.profile as Profile) ?? mockActiveProfile;
      mockKeyboards[profile] = {
        present: true,
        active: mockActiveProfile,
        effect: a.effect as number,
        brightness: a.brightness as number,
        speed: a.speed as number,
        direction: a.direction as number,
        full_rgb: Boolean(a.full_rgb),
        color: [
          clampByte(a.r as number),
          clampByte(a.g as number),
          clampByte(a.b as number),
        ],
      };
      return undefined as T;
    }

    case "keyboard_set_brightness":
    case "keyboard_set_speed":
    case "keyboard_set_direction":
    case "keyboard_set_full_rgb":
    case "keyboard_set_color":
    case "keyboard_set_mode": {
      log(cmd, a);
      const profile = (a.profile as Profile) ?? mockActiveProfile;
      const s = mockKeyboards[profile];
      if (cmd === "keyboard_set_brightness") s.brightness = a.value as number;
      if (cmd === "keyboard_set_speed") s.speed = a.value as number;
      if (cmd === "keyboard_set_direction") s.direction = a.left ? 1 : 0;
      if (cmd === "keyboard_set_full_rgb") s.full_rgb = Boolean(a.on);
      if (cmd === "keyboard_set_color") {
        s.color = [
          clampByte(a.r as number),
          clampByte(a.g as number),
          clampByte(a.b as number),
        ];
      }
      if (cmd === "keyboard_set_mode") s.effect = a.effect as number;
      return undefined as T;
    }

    case "keyboard_set_active_profile": {
      log(cmd, a);
      mockActiveProfile = a.profile as Profile;
      return undefined as T;
    }

    case "keyboard_remap_keys":
      return MOCK_REMAP_KEYS.map((k) => ({ ...k })) as T;

    case "keyboard_remap":
    case "keyboard_restore_keymap":
    case "keyboard_set_custom_colors": {
      log(cmd, a);
      return undefined as T;
    }

    default:
      throw new Error(`Comando desconhecido no mock: ${cmd}`);
  }
}

function structuredCloneSafe<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

// ---------------------------------------------------------------------------
// Single dispatch helper
// ---------------------------------------------------------------------------

export async function callBackend<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauri()) {
    return invoke<T>(cmd, args);
  }
  // Simulate a little latency so the UI/toasts behave like the real thing.
  await new Promise((resolve) => setTimeout(resolve, 90));
  return mockBackend<T>(cmd, args);
}

// ---------------------------------------------------------------------------
// Typed command wrappers
// ---------------------------------------------------------------------------

export const listDevices = (): Promise<ListDevices> =>
  callBackend<ListDevices>("list_devices");

export const mouseReadState = (): Promise<MouseState> =>
  callBackend<MouseState>("mouse_read_state");

export const mouseSetLed = (led: {
  effect: LedEffect;
  r: number;
  g: number;
  b: number;
  speed: number;
  brightness: number;
}): Promise<void> => callBackend<void>("mouse_set_led", { ...led });

export const mouseSetDpiStage = (stage: number, x: number, y: number): Promise<void> =>
  callBackend<void>("mouse_set_dpi_stage", { stage, x, y });

export const mouseSetDpiAll = (dpi: DpiStage[]): Promise<void> =>
  callBackend<void>("mouse_set_dpi_all", { dpi });

export const mouseSetDpiColor = (stage: number, [r, g, b]: Rgb): Promise<void> =>
  callBackend<void>("mouse_set_dpi_color", { stage, r, g, b });

export const mouseSetDpiEffect = (on: boolean): Promise<void> =>
  callBackend<void>("mouse_set_dpi_effect", { on });

export const mouseSetPolling = (hz: number): Promise<void> =>
  callBackend<void>("mouse_set_polling", { hz });

export const mouseSetButton = (index: number, action: ButtonAction): Promise<void> =>
  callBackend<void>("mouse_set_button", { index, action });

export const keyboardReadState = (profile: Profile): Promise<KeyboardState> =>
  callBackend<KeyboardState>("keyboard_read_state", { profile });

export const keyboardEffects = (): Promise<KeyboardEffect[]> =>
  callBackend<KeyboardEffect[]>("keyboard_effects");

export interface KeyboardEffectArgs {
  profile: Profile;
  effect: number;
  r: number;
  g: number;
  b: number;
  brightness: number;
  speed: number;
  direction: number;
  full_rgb: boolean;
}

export const keyboardSetEffect = (args: KeyboardEffectArgs): Promise<void> =>
  callBackend<void>("keyboard_set_effect", { ...args });

export const keyboardSetBrightness = (profile: Profile, value: number): Promise<void> =>
  callBackend<void>("keyboard_set_brightness", { profile, value });

export const keyboardSetSpeed = (profile: Profile, value: number): Promise<void> =>
  callBackend<void>("keyboard_set_speed", { profile, value });

export const keyboardSetDirection = (profile: Profile, left: boolean): Promise<void> =>
  callBackend<void>("keyboard_set_direction", { profile, left });

export const keyboardSetFullRgb = (profile: Profile, on: boolean): Promise<void> =>
  callBackend<void>("keyboard_set_full_rgb", { profile, on });

export const keyboardSetColor = (profile: Profile, [r, g, b]: Rgb): Promise<void> =>
  callBackend<void>("keyboard_set_color", { profile, r, g, b });

export const keyboardSetMode = (profile: Profile, effect: number): Promise<void> =>
  callBackend<void>("keyboard_set_mode", { profile, effect });

export const keyboardSetActiveProfile = (profile: Profile): Promise<void> =>
  callBackend<void>("keyboard_set_active_profile", { profile });

export const keyboardRemapKeys = (): Promise<RemapKey[]> =>
  callBackend<RemapKey[]>("keyboard_remap_keys");

export const keyboardRemap = (args: {
  index: number;
  kind: RemapKind;
  code: number;
  code2: number;
}): Promise<void> => callBackend<void>("keyboard_remap", { ...args });

export const keyboardRestoreKeymap = (): Promise<void> =>
  callBackend<void>("keyboard_restore_keymap");

export const keyboardSetCustomColors = (
  profile: Profile,
  cells: CustomCell[],
): Promise<void> =>
  callBackend<void>("keyboard_set_custom_colors", { profile, cells });
