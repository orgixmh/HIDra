<h1 align="center">HIDra</h1>

<p align="center">
  Configure gaming mice and keyboards from <b>Linux, Windows and macOS</b> —
  talking straight to the hardware over raw HID, with no vendor software.
  <br>
  Supports the <b>Redragon Cobra Pro (M711-PRO)</b> mouse and the
  <b>Redragon Brahma Pro (K586RGB-PRO)</b> keyboard, whose own tools are
  Windows-only.
</p>

<p align="center">
  <a href="https://github.com/luizjr/HIDra/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/luizjr/HIDra?display_name=tag"></a>
  <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
  <a href="https://github.com/luizjr/HIDra/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/luizjr/HIDra/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/luizjr/HIDra/releases/tag/nightly"><img alt="Nightly build" src="https://github.com/luizjr/HIDra/actions/workflows/nightly.yml/badge.svg"></a>
</p>

<p align="center">
  <b><a href="https://luizjr.github.io/HIDra/">luizjr.github.io/HIDra</a></b> ·
  <a href="README.pt-BR.md">Leia em português 🇧🇷</a>
</p>

---

Vendors ship their configuration tools for Windows only, one tool per product,
and they stop being updated. HIDra is one small app that speaks the devices'
own USB protocols, so your lighting, DPI and key remapping survive your choice
of operating system.

> Not affiliated with, endorsed by, or sponsored by any hardware manufacturer.
> Product names are used only to say which hardware is supported.

## Screenshots

| Mouse | Keyboard |
| --- | --- |
| ![Mouse lighting, DPI and buttons](docs/screenshots/mouse.png) | ![Keyboard lighting effects](docs/screenshots/keyboard.png) |

Per-key colours, painted key by key and stored on the keyboard itself:

![Per-key colour editor](docs/screenshots/perkey.png)

## Supported devices

<!-- devices:start -->
| Device | USB ID | Status | Features |
| --- | --- | --- | --- |
| Redragon Cobra Pro (M711-PRO) — 2.4 GHz receiver | `25A7:FA07` | ✅ verified | lighting, DPI, buttons, polling |
| Redragon Cobra Pro (M711-PRO) — cable | `25A7:FA08` | ✅ verified | lighting, DPI, buttons, polling |
| Redragon Brahma Pro (K586RGB-PRO) | `0C45:5004` | ✅ verified | 18 effects + per-key colour, 3 profiles, key remap |
| Redragon UCAL K673SP-RGB-PRO | `258A:010C` | ✅ verified | wired lighting, 18 effects + per-key colour (model identity checked); no remap/profiles |
<!-- devices:end -->

**Your device is not here?** HIDra is not a Redragon project — it started with
those two because they were on the author's desk. Any brand belongs in that
table, and many peripherals are rebadged siblings that speak a protocol HIDra
already knows, so adding one is often a single entry in
[`src-tauri/src/devices.rs`](src-tauri/src/devices.rs):

```sh
python3 scripts/probe-device.py     # finds the config endpoint, prints the entry
python3 scripts/sync-devices.py     # regenerates udev rules, tables, website
```

The full walkthrough is [docs/ADDING-DEVICES.md](docs/ADDING-DEVICES.md)
([em português](docs/ADDING-DEVICES.pt-BR.md)). If you would rather not touch
code, open a
[device request](https://github.com/luizjr/HIDra/issues/new?template=device-request.yml)
with the output of `python3 scripts/probe-device.py --issue`.

## Smart Notifications (Linux)

On the wired UCAL K673SP-RGB-PRO, open **Smart Notifications** beside **Per-key
colours**. Add a rule, select keys on the keyboard, choose a colour, optional blinking and duration,
and save. Match an application (for example Viber), title/description text, or
both. Leave Application empty to match website notification text from any
browser. Select **Notification source → Gmail (Browser)** to match
`mail.google.com` automatically in the notification title or description, from
any browser. This does not filter by the mail sender. For custom rules,
**Use description** copies the third field from a recent notification.
The browser or application must actually send a desktop notification.

Turn the switch on to let notifications control the lighting. Other lighting
effects cannot run simultaneously. Choose a solid **Background colour** and save
it with the rules to keep the other keys lit. Switching off or closing HIDra restores
the captured lighting mode. The switch and rules survive restarts. HIDra must
remain running to receive notifications. Recent notification content stays in
memory and is never saved. **Test rule** checks the selected lighting assignment.

New rules default to **Until notification is dismissed**. This follows desktop
notification removal, not whether a message has been read inside its app. The
other choices keep an alert for **1, 5 or 10 minutes**. Notification keys can
blink between their alert colour and the background. Each notification is tracked
separately, including updates/replacements, so dismissing one does not clear
another notification on the same key. **Clear alerts** clears keyboard alerts
without dismissing desktop notifications. **Test rule** runs for ten seconds.

FDO notifications use the server's returned ID and `NotificationClosed` signal.
On Ubuntu/GNOME this keeps alerts active after a banner hides if the notification
remains in the notification centre. GTK applications are tracked by app and
notification ID; app-requested removal is observed, but GTK provides no standard
user-dismissal signal. If an app does not report removal, use **Clear alerts** or
a fixed duration. Only notifications arriving while Smart mode is listening are
tracked; existing notification history is not imported on startup. Existing rules
with second-based durations migrate to the new dismissal default.

Custom scripts are another **Notification source**. Enter the absolute path to
an executable script (with a shebang) and an interval of 5–86400 seconds. While
Smart mode is on, the script runs immediately and again after the selected delay
following completion. Exit code **1** triggers the assigned keys; **0** clears
that script's alert. The default duration is **Until script returns 0**. Other
exit codes, launch errors and the 10-second timeout show a status error and
preserve the last alert state. Runs never overlap for the same rule, use no
terminal and discard stdout/stderr. Off, rule changes and app exit stop running
scripts. Use the script file for arguments or environment setup. **Test rule**
previews lighting only; it does not execute the script.

The in-app **Custom colour** palette offers a saturation/brightness plane, hue,
saturation and brightness sliders, RGB channels and direct HEX entry.

The listener uses the session D-Bus notification service without root or replacing
your notification daemon. If the session disallows monitoring, the panel displays
an error. Smart mode uses temporary RGB frames and keeps one HID handle open;
its idle background frame needs periodic refreshes, without repeated device discovery
or configuration reads. Other devices/platforms currently show this mode as
unsupported.

## Install

Grab the installer for your system from the
[latest release](https://github.com/luizjr/HIDra/releases/latest).

### Linux — `.deb`, `.rpm` or AppImage

```sh
sudo dpkg -i HIDra_*_amd64.deb        # Debian / Ubuntu / Mint / Pop!_OS
sudo rpm -i HIDra-*.x86_64.rpm        # Fedora / openSUSE
chmod +x HIDra_*.AppImage && ./HIDra_*.AppImage
```

The binary is `hidra`, so the app also runs from a terminal by that name.

The `.deb` and `.rpm` install the udev rule themselves — just **replug the
device** after installing. With the AppImage, or when running from source, add
the rule once:

```sh
sudo sh packaging/install-udev.sh          # --uninstall reverts it
```

The rule is tagged `uaccess`, which hands the `hidraw` nodes to whoever is
logged in locally — no root, no setuid, nothing running in the background.

### Windows — `.exe`

Run the `.exe` installer. It is not code-signed, so SmartScreen will show
"Windows protected your PC": click **More info → Run anyway**. No driver is
needed; Windows already exposes the vendor HID collection.

### macOS — `.dmg`

Open the `.dmg` and drag HIDra to Applications. It is not notarized, so the
first launch needs **right-click → Open**, or:

```sh
xattr -dr com.apple.quarantine /Applications/HIDra.app
```

> Windows and macOS builds are produced by CI but have had far less testing than
> Linux. If something misbehaves there, please
> [open an issue](https://github.com/luizjr/HIDra/issues/new/choose) — that
> feedback is exactly what those builds need.

### Development builds

Every push to `main` rebuilds all three installers and replaces the rolling
[`nightly` pre-release](https://github.com/luizjr/HIDra/releases/tag/nightly).
Use it to try a fix before it is tagged; use the
[latest release](https://github.com/luizjr/HIDra/releases/latest) otherwise.

## Staying up to date

HIDra checks for a new release a few seconds after it starts, and whenever you
click **Procurar atualizações** in the sidebar. When one is out, a strip appears
above the panels offering to install it.

How it installs depends on how you installed HIDra:

| Install | What happens |
| --- | --- |
| Windows, macOS, Linux AppImage | HIDra downloads the new build, verifies its signature and restarts into it |
| Linux `.deb` / `.rpm` | HIDra downloads the signed package and hands it to `dpkg`/`rpm` through `pkexec`, so your system asks for authorisation the way it does for any package |
| Built from source, or packaged by someone else | HIDra tells you a release is out and links to it; it never touches a build it did not make |

Every artifact is signed with the project's release key, and the app verifies
the download against it before installing anything — so a tampered file is
refused rather than installed. Nightly builds are never offered as an update.

The check is one HTTPS request to GitHub for the release metadata. There is no
telemetry, no account, and nothing is sent about you or your hardware.


## What it does

**Mice (Cobra Pro family)**
- Lighting: steady, breathing, cycle, off — colour, brightness, speed
- DPI: 5 stages with independent X/Y, a colour per stage, stage indicator
- Buttons: remap all 8 — mouse clicks, DPI ±/cycle, a key, a combo (Ctrl+C),
  media keys, fire/rapid-fire, RGB toggle, polling switch
- Polling rate: 125 / 250 / 500 / 1000 Hz
- Reads back whatever is actually stored on the device

**Keyboards (Brahma Pro family)**
- 18 lighting effects plus **per-key** custom colours
- Colour, brightness (0–5), speed (1–5), direction, full-RGB mode
- 3 independent onboard profiles, switched from the app
- Key remapping: single key, modifier, media key, disable — plus restore factory

Everything is stored on the device itself, so it keeps working after you close
the app, reboot, or plug the device into another machine.

## Building from source

Requirements: [Rust](https://rustup.rs) ≥ 1.77, Node.js ≥ 18, and the Tauri
system dependencies for your OS
([official list](https://tauri.app/start/prerequisites/)). On Debian/Ubuntu:

```sh
sudo apt install libwebkit2gtk-4.1-dev libudev-dev build-essential curl file \
                 libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

```sh
npm install
npm run tauri dev      # run the app in development mode
npm run tauri build    # installers land in src-tauri/target/release/bundle
```

## Troubleshooting

**Linux: black rectangles painted over parts of the window.** WebKitGTK puts
filtered elements on their own compositing layer and, on some GPU setups
(hybrid Intel + NVIDIA under Wayland, reported on WebKitGTK 2.52), blits those
layers to the wrong place. HIDra therefore uses no CSS or SVG filters at all. If
you still hit it — in this app or another Tauri one — the blunt escape hatch is
`WEBKIT_DISABLE_COMPOSITING_MODE=1 hidra`, which turns GPU compositing off
entirely.

**A setting does not stick on a wireless mouse.** The 2.4 GHz radio sleeps when
the mouse is idle. Move the mouse while pressing *Apply*, or connect the cable.

**The device shows as disconnected on Linux.** The udev rule is missing or the
device was not replugged after installing it: `sudo sh packaging/install-udev.sh`
from a source checkout (the `.deb`/`.rpm` ship it), then unplug and replug.


## How it works

The app is [Tauri](https://tauri.app): a Rust backend that owns all device I/O
and a React/TypeScript UI. There is no daemon and no kernel module — the backend
opens the device's vendor HID collection with
[`hidapi`](https://github.com/libusb/hidapi) and writes the same packets the
official tool writes.

```
src/                    React UI (one panel per device kind)
src-tauri/src/
  devices.rs            table of supported devices  ← add yours here
  hid.rs                discovery + transport, per-platform endpoint picking
  mouse.rs              Compx 17-byte protocol
  keyboard.rs           SONiX 64-byte protocol
PROTOCOL.md             byte-level notes for both protocols
```

The keyboard protocol was reverse-engineered by capturing the official Windows
tool's USB traffic; every byte of it is written down in
[`PROTOCOL.md`](PROTOCOL.md) so nobody has to do that work twice.

The interface defaults to **English**. Choose **English** or **Português (Brasil)**
from **Language** at the bottom of the sidebar; the choice is saved locally.
Switching languages keeps your current edits and does not query or write to
connected devices. See [docs/TRANSLATING.md](docs/TRANSLATING.md) to add a language.

## Contributing

Bug reports, new device entries, protocol captures and UI work are all welcome —
see [CONTRIBUTING.md](CONTRIBUTING.md). The most valuable contribution is a new
device: [docs/ADDING-DEVICES.md](docs/ADDING-DEVICES.md) walks through it.

Working with an AI assistant? [`CLAUDE.md`](CLAUDE.md) documents the
architecture and the invariants for agents ([`AGENTS.md`](AGENTS.md) points
other tools at it), and Claude Code users get an `/add-device` command that runs
the whole flow. The hardware test still has to come from a person who owns the
device.

## Credits

- Mouse protocol: [`open-m711pro`](https://github.com/mateusands/open-m711pro),
  [`m913-ctl`](https://github.com/Qehbr/m913-ctl) and
  [`mouse_m908`](https://github.com/dokutan/mouse_m908) — all GPL.
- Keyboard protocol: captured from the vendor's own tool, with reference to
  [`rgb_keyboard`](https://github.com/dokutan/rgb_keyboard) and the OpenRGB
  community.

## License

[GPL-3.0-or-later](LICENSE). Use at your own risk: every panel has a
**restore defaults** button, and nothing here can be flashed to firmware.
