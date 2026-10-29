# On-Screen Display (OSD)

The OSD is a hardware-independent service. It is not connected to wheel actions,
remapping, HID interception or input listeners. Only the Settings preview consumes
it in this phase. It continues to work with the main window hidden in the tray.

## API

```typescript
import { showOSD } from "./osd";

await showOSD({ text: "Brightness Mode", icon: "sun" });
await showOSD({ text: "Custom message", position: "top-right", duration: 2500,
  width: 320, height: 100, textSize: 18 });
```

`text` is required; `icon`, `textSize`, `duration`, `width`, `height` and `position`
are optional. Defaults come from the global settings at the time of the call.
Positions are `top`, `bottom`, `top-left`, `top-right`, `bottom-left`, `bottom-right`.
The Promise resolves to false for disabled OSD or blank text, true for an accepted
request, and rejects invalid parameters. Window/render failures are logged and
reported as `hidra-osd-error` to the main window; the global Settings section shows
them. Accepted requests are asynchronous, not proof of physical screen rendering.

Rust modules can call `app.state::<osd::Osd>().show(osd::Message { text: ...,
..Default::default() })` using `tauri::Manager`. They do not need the main window.

Named icons: `sun`, `info`, `volume`, `check`, `warning`, `moon`. The SVG styling
matches existing HIDra inline icons; no icon dependency is introduced. Missing,
unknown or unsupported icon names fall back to text only. Text is escaped React
content, capped at 2048 Unicode characters, wrapped and clipped to three lines.

## Settings

Preferences live in the existing app config directory as `osd-settings.json`,
written atomically. Defaults: enabled, bottom, 280×100 logical pixels, 16px text,
1500ms duration, 32px margin, dark RGB (30,32,36) background and 4% transparency.
Background colour uses the custom RGB/HEX palette; transparency accepts 0–100%
(0 is opaque, 100 is invisible background). Only the background uses alpha;
text/icons remain opaque and choose dark/light foreground from the background
luminance. Existing settings files retain their values and load these appearance
defaults for missing fields. Limits: width 120–1200px, height 64–600px, text
10–48px, duration 100–30000ms, margin 0–256px. Invalid files fall back to defaults
and show their load error; explicit Save repairs preferences. Overrides do not
modify persisted settings. Disabling hides any existing message immediately.
Preview saves the current controls and invokes the real `showOSD` API.

## Lifecycle and positioning

`src-tauri/src/osd.rs` owns one lazily created webview labeled `osd`. Its separate
Vite entry `osd.html` only renders the overlay and never mounts the main app or its
HID hooks. It is transparent, undecorated, non-resizable, always on top, absent
from the taskbar, non-focusable and click-through. Failure to enable click-through
leaves the window hidden and reports an error rather than showing an interactive
window over the user's work. On Linux, the hidden GTK window is realized on the
main thread before installing click-through: Tao otherwise panics when it tries
to access the absent GDK window. Realization does not show or focus the overlay.
The native geometry limits match the requested viewport, preventing GTK from
retaining a larger natural WebKit height for a non-resizable window. The overlay has isolated CSS and
its frame fits the viewport without adding padding outside the requested size.
No CSS/SVG filters are used.

One coalescing wake slot retains the most recent message. A sleeping worker owns
one expiry deadline: replacement requests reset it, not another timer or window.
The renderer subscribes before announcing readiness and acknowledges after React
commits the new content; only then is a hidden overlay revealed. Readiness has a
bounded grace period. There is no polling loop, periodic task or input daemon.
Shutdown stops the worker; Tauri destroys the native overlay with the app.
Closing the main window with Smart off explicitly exits so the hidden reusable
OSD cannot accidentally keep an orphan process alive.

For each request, native Tauri cursor/monitor APIs select the pointer's monitor
(primary monitor fallback if cursor lookup is unavailable). Physical size, position
and scaled margins are clamped to its work area, including negative monitor
origins. Monitor choice is recalculated on the next OSD request, not continuously
while a message is already visible. The main window is never shown or focused.

Target: Ubuntu/GNOME on X11. Other compositors/platforms may impose their own
always-on-top, work-area, scaling or click-through restrictions; unsupported
native operations report an error without affecting HIDra's other facilities.
No automated tests or visual/runtime preview validation were performed for this
change, as requested; validation consists of JavaScript/TypeScript static/build
checks and native compilation. Multi-monitor/focus/tray behavior still needs a
manual desktop check using the actual Preview button.
