import { callBackend, isTauri } from "../backend";
import { DEFAULT_OSD_SETTINGS } from "./types";
import type { OSDMessage, OSDSettings, OSDSettingsView } from "./types";
export type { OSDMessage, OSDSettings, OSDPosition } from "./types";

/** Replace the current overlay. False means disabled or empty; failures reject. */
export async function showOSD(message: OSDMessage): Promise<boolean> {
  if (!isTauri()) throw new Error("The OSD is available in the desktop app.");
  return callBackend<boolean>("osd_show", { message });
}
export async function getOSDSettings(): Promise<OSDSettingsView> {
  if (!isTauri()) return { settings: { ...DEFAULT_OSD_SETTINGS }, error: null };
  return callBackend<OSDSettingsView>("osd_settings");
}
export async function saveOSDSettings(settings: OSDSettings): Promise<OSDSettingsView> {
  if (!isTauri()) throw new Error("The OSD is available in the desktop app.");
  return callBackend<OSDSettingsView>("osd_save_settings", { settings });
}
