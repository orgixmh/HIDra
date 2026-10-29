# Custom Wheel — Phase 1

This phase registers hardware capabilities and saves host preferences. It does
not open Linux input devices, intercept events, remap actions, process gestures,
start a background input worker, or change permissions/udev rules.

## Registry and discovery

`devices.rs` adds `Device.custom_wheel: Option<&'static CustomWheelSpec>`.
`CustomWheelSpec` describes vendor/product, optional name and bus, required
`InputCapability { event_type, code }` pairs, and clockwise/counterclockwise/press
keycodes. Only the RF K673SP entry has a wheel specification. Wired K673SP and
other existing entries keep `None`; their protocols and HID interfaces are unchanged.

User-supplied Linux input identity:

- CX 2.4G Wireless Receiver Consumer Control
- USB bus 0x0003, VID 0x3554, PID 0xfa09
- EV_KEY (0x01): clockwise KEY_VOLUMEUP 115 (scan c00e9), counterclockwise
  KEY_VOLUMEDOWN 114 (scan c00ea), press KEY_MUTE 113 (scan c00e2)

This Consumer Control event source is separate from the existing vendor HID
configuration endpoint. No eventN path is stored or assumed. Phase 2 must locate
an input device matching the identity and every required event capability.

The existing `keyboard_driver::Capabilities` serializes an optional
`custom_wheel::Capability { settings_key, input }` during normal device discovery.
The frontend mirrors it in `CustomWheelCapability`. Tab visibility uses this
capability only, with no frontend model names or USB IDs.

## Preferences and UI

`custom_wheel.rs` defines `Mode` (Volume, Brightness, Zoom), `Preferences`
(enabled, ordered modes, show_osd), `View`, and a managed `Store`. The store follows
existing native configuration persistence: app config directory, mutex-serialized
saves, temporary JSON, sync and atomic rename, Unix mode 0600. Preferences live
in `custom-wheel.json`, keyed by registry vendor/model/VID/PID/interface/connection,
so unrelated definitions/transports do not share a profile. The key is validated
against wheel-capable registry entries; it is never used as a filesystem path.
Missing files use defaults; invalid files expose a load error and defaults until
an explicit save repairs them. Stored modes must contain Volume and no duplicates.

The Custom Wheel tab follows Smart Notifications. Its controls remain independent
of Smart's lighting lock. Settings save on change:

- Enable Custom Wheel: off by default, explicitly marked as preferences only.
- Volume, Brightness and Zoom: initially all enabled. Volume cannot be disabled;
  at least one mode therefore always remains. Up/down buttons save cycle ordering.
- Read-only planned gestures: single press cycles modes, double press toggles
  system mute, long press returns to Volume; thresholds 300 ms and 700 ms.
- Show OSD when switching modes: on by default, reserved for Phase 2 consumers.
- A sample-mode selector and preview call the existing `showOSD` API. Preview
  does not switch modes or write hardware. It uses global OSD appearance and
  reports if global OSD is disabled. No second overlay is created.

English and Portuguese catalogs include the controls and errors.

## Files and validation

Changed: `devices.rs`, `keyboard_driver.rs`, `lib.rs`, frontend `types.ts`,
`KeyboardPanel.tsx`, `App.css`, both locale catalogs, and both README introductions.
Added: `custom_wheel.rs`, `CustomWheel.tsx`, and this document.

No tests, lint, builds, runtime preview or hardware validation were performed,
as explicitly requested. Source formatting and device-derived file generation
are maintenance steps only. No screenshot was captured.

Phase 2 remains responsible for input discovery/access, wheel interception,
actual volume/brightness/zoom actions, gesture recognition, cycling, and automatic
OSD messages. The enabled preference has no effect on wheel behaviour in Phase 1.
