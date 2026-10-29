export type OSDPosition = "top" | "bottom" | "top-left" | "top-right" | "bottom-left" | "bottom-right";
export interface OSDSettings {
  enabled: boolean;
  position: OSDPosition;
  width: number;
  height: number;
  textSize: number;
  duration: number;
  edgeMargin: number;
}
export interface OSDMessage {
  text: string;
  icon?: string;
  textSize?: number;
  duration?: number;
  width?: number;
  height?: number;
  position?: OSDPosition;
}
export interface OSDSettingsView { settings: OSDSettings; error: string | null }
export interface OSDDisplay { id: number; text: string; icon?: string | null; textSize: number; settings: OSDSettings }
export const DEFAULT_OSD_SETTINGS: OSDSettings = {
  enabled: true, position: "bottom", width: 280, height: 100, textSize: 16, duration: 1500, edgeMargin: 32,
};
export const OSD_POSITIONS: { value: OSDPosition; label: string }[] = [
  { value: "top", label: "Top" }, { value: "bottom", label: "Bottom" },
  { value: "top-left", label: "Top Left" }, { value: "top-right", label: "Top Right" },
  { value: "bottom-left", label: "Bottom Left" }, { value: "bottom-right", label: "Bottom Right" },
];
export function validOSDSettings(settings: OSDSettings): boolean {
  return Number.isInteger(settings.duration) && settings.duration >= 100 && settings.duration <= 30000 &&
    [[settings.width, 120, 1200], [settings.height, 64, 600], [settings.textSize, 10, 48], [settings.edgeMargin, 0, 256]]
      .every(([value, min, max]) => Number.isFinite(value) && value >= min && value <= max) &&
    OSD_POSITIONS.some(({ value }) => value === settings.position);
}
