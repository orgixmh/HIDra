// Shared domain types for the HIDra frontend.
// Mirrors the Tauri command payloads described in PROTOCOL.md.

export type DeviceKind = "mouse" | "keyboard";

export type Profile = 1 | 2 | 3;

export type MouseMode = "2.4g" | "wired";

export interface ListDevices {
  keyboard: boolean;
  keyboard_capabilities?: KeyboardCapabilities | null;
  mouse: boolean;
  mouse_mode: MouseMode | null;
  /** What was found, from the backend device table. */
  keyboard_vendor: string | null;
  keyboard_model: string | null;
  mouse_vendor: string | null;
  mouse_model: string | null;
  /** Set when the model was added to the table but never proven on hardware. */
  keyboard_untested: boolean;
  mouse_untested: boolean;
}

export type LedEffect = "steady" | "breathing" | "cycle" | "off";

export interface MouseLed {
  effect: LedEffect;
  r: number;
  g: number;
  b: number;
  speed: number;
  brightness: number;
}

export type Rgb = [number, number, number];

export interface DpiStage {
  x: number;
  y: number;
  color: Rgb;
  enabled: boolean;
}

/** Raw button table entry: [TYPE, P1, P2] */
export type ButtonRaw = [number, number, number];

export interface MouseState {
  present: boolean;
  mode: MouseMode | null;
  led: MouseLed | null;
  dpi: DpiStage[] | null;
  dpi_effect: boolean | null;
  polling: number | null;
  buttons: ButtonRaw[] | null;
}

export type MouseButton = "left" | "right" | "middle" | "back" | "forward";
export type DpiOp = "up" | "down" | "cycle";

/** Discriminated union of every button action the mouse understands. */
export type ButtonAction =
  | { type: "disabled" }
  | { type: "mouse"; button: MouseButton }
  | { type: "dpi"; op: DpiOp }
  | { type: "fire"; speed: number; clicks: number }
  | { type: "three_click" }
  | { type: "rgb_toggle" }
  | { type: "polling_switch" }
  | { type: "media"; usage: number }
  | { type: "key"; usage: number }
  | { type: "combo"; modifiers: number; usage: number };

export type ButtonActionType = ButtonAction["type"];

export interface KeyboardState {
  present: boolean;
  active: number | null;
  effect: number | null;
  brightness: number | null;
  speed: number | null;
  direction: number | null;
  full_rgb: boolean | null;
  color: Rgb | null;
}

export interface KeyboardEffect {
  id: number;
  name: string;
  has_color: boolean;
  has_direction: boolean;
}

export interface RemapKey {
  index: number;
  label: string;
  raw: ButtonRaw;
}

export type RemapKind = "key" | "modifier" | "media" | "disable";

export interface CustomCell {
  off: number;
  r: number;
  g: number;
  b: number;
}

/** Supported controls and geometry supplied by the selected device's driver. */
export interface KeyboardCapabilities {
  profiles: number;
  remap: boolean;
  per_key: boolean;
  smart_notifications?: boolean;
  brightness_max: number;
  speed_min: number;
  speed_max: number;
  custom_effect: number;
  layout_aspect: number | null;
  layout: import("./keyboardLayout").KeyCell[] | null;
}

export interface NotificationRule {
  id: string;
  name: string;
  enabled: boolean;
  app: string;
  text: string;
  keys: number[];
  color: Rgb;
  duration_seconds: number;
}
export interface SmartSettings { enabled: boolean; rules: NotificationRule[] }
export interface SmartView {
  settings: SmartSettings;
  active: boolean;
  status: string;
  error: string | null;
  recent: { app: string; title: string; body: string; matched: boolean }[];
  lit_keys: number[];
}
