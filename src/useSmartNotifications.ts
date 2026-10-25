import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { callBackend, isTauri } from "./backend";
import type { SmartSettings, SmartView } from "./types";

export function useSmartNotifications() {
  const [view, setView] = useState<SmartView | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    const receive = (next: SmartView) => { if (!disposed) setView(next); };
    const load = async () => {
      try {
        if (isTauri()) {
          const unlisten = await listen<SmartView>("smart-notifications-changed", (event) => receive(event.payload));
          if (disposed) { unlisten(); return; }
          stop = unlisten;
        } else {
          const mock = (event: Event) => receive((event as CustomEvent<SmartView>).detail);
          window.addEventListener("smart-notifications-mock", mock);
          stop = () => window.removeEventListener("smart-notifications-mock", mock);
        }
        receive(await callBackend<SmartView>("smart_notifications_get"));
      } catch (err) { if (!disposed) setError(String(err)); }
    };
    void load();
    return () => { disposed = true; stop?.(); };
  }, []);
  const save = useCallback(async (settings: SmartSettings) => {
    const next = await callBackend<SmartView>("smart_notifications_save", { settings });
    setView(next); setError(null); return next;
  }, []);
  const test = useCallback(async (id: string) => {
    const next = await callBackend<SmartView>("smart_notifications_test", { id });
    setView(next); return next;
  }, []);
  const clear = useCallback(async () => {
    const next = await callBackend<SmartView>("smart_notifications_clear");
    setView(next); return next;
  }, []);
  return { view, error, save, test, clear };
}
export type SmartController = ReturnType<typeof useSmartNotifications>;
