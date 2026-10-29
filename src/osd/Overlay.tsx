import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { OSDDisplay } from "./types";

function Icon({ name }: { name?: string | null }) {
  const paths: Record<string, string> = {
    sun: "M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6 7 7m10 10 1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
    info: "M12 11v6m0-10v.1M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
    volume: "m11 4-6 5H2v6h3l6 5V4Zm4 4a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14",
    check: "m5 12 4 4L19 6",
    warning: "m12 3 10 18H2L12 3Zm0 6v5m0 3v.1",
    moon: "M20.5 13A9 9 0 0 1 11 3.5 9 9 0 1 0 20.5 13Z",
  };
  const icon = name?.toLowerCase();
  const path = icon && Object.prototype.hasOwnProperty.call(paths, icon) ? paths[icon] : undefined;
  if (!path) return null;
  return <svg className="osd-icon" width="30" height="30" viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={path} /></svg>;
}

export function Overlay() {
  const [display, setDisplay] = useState<OSDDisplay | null>(null);
  const latest = useRef(0);
  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    void (async () => {
      try {
        const unlisten = await listen<OSDDisplay>("hidra-osd-display", ({ payload }) => {
          if (!disposed && payload.id >= latest.current) { latest.current = payload.id; setDisplay(payload); }
        });
        if (disposed) { unlisten(); return; }
        stop = unlisten;
        await invoke("osd_ready");
      } catch (error) { console.error("OSD initialization failed", error); }
    })();
    return () => { disposed = true; stop?.(); };
  }, []);
  useLayoutEffect(() => {
    if (display) void invoke("osd_rendered", { id: display.id }).catch((error: unknown) => console.error("OSD render acknowledgement failed", error));
  }, [display]);
  if (!display) return null;
  const [r, g, b] = display.settings.backgroundColor;
  const alpha = 1 - display.settings.transparency / 100;
  const luminance = [r, g, b].map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  }).reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
  return <div className="osd-frame" style={{ backgroundColor: `rgba(${r}, ${g}, ${b}, ${alpha})`, color: luminance > 0.179 ? "#111318" : "#f8f8fa" }} role="status" aria-live="polite" aria-atomic="true">
    <Icon name={display.icon} />
    <span className="osd-text" style={{ fontSize: display.textSize }}>{display.text}</span>
  </div>;
}
