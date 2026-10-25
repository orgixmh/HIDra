# HIDra — notes for AI assistants

HIDra configures gaming mice and keyboards by writing to their vendor HID
configuration channel: RGB lighting, DPI, polling rate, key and button
remapping. Rust (Tauri v2) backend, React + TypeScript UI, no daemon, no kernel
module. It started with two Redragon devices but is deliberately brand-agnostic:
**growing the device table is the point of the project.**

Read this before changing anything; it is short on purpose.

## Map

```
src/                          React UI. English default, English / Portuguese (pt-BR) catalogs.
  App.tsx                     Device selection, state loading, toasts
  backend.ts                  Tauri bridge + a mock backend so the UI runs in a browser
  components/{mouse,keyboard} One panel per device kind
src-tauri/src/
  devices.rs                  THE DEVICE TABLE — data, not code
  hid.rs                      Discovery and transport; per-platform endpoint picking
  mouse.rs                    Protocol::Compx17   (17-byte feature reports)
  keyboard.rs                 Protocol::Sonix64   (64-byte reports)
  lib.rs                      Tauri commands, one per user action
scripts/probe-device.py       Reads /sys, prints a ready device-table entry
scripts/sync-devices.py       Regenerates everything derived from the table
site/index.html               Landing page (GitHub Pages)
PROTOCOL.md                   Byte-level notes for both protocols
docs/ADDING-DEVICES.md        The human version of this file
```

## Invariants

1. **Everything model-specific lives in `devices.rs`.** `hid.rs`, `mouse.rs` and
   `keyboard.rs` must never test for a specific vid/pid. Adding a rebadged
   sibling of a supported device is one entry in `SUPPORTED` and nothing else.
2. **Generated files are generated.** `packaging/70-hidra.rules`, the device
   tables in `README.md` / `README.pt-BR.md`, and the table in `site/index.html`
   all come from `devices.rs` via `scripts/sync-devices.py`. Never hand-edit
   them; run the script. CI fails on drift.
3. **No CSS or SVG filters.** `filter`, `backdrop-filter` and SVG `<filter>`
   force compositing layers that WebKitGTK paints in the wrong place on some
   Linux GPUs — black rectangles over the UI. Glows are gradients and stacked
   strokes. This is a real bug that shipped once; do not reintroduce it.
4. **Protocol facts get written down.** A new byte, offset or quirk goes into
   `PROTOCOL.md` in the same change that uses it.
5. **Configuration registers only.** The same ones the vendor tool writes. Never
   bootloader or firmware commands — nothing here may be able to brick a device.

## Commands

```sh
npm install
npm run tauri dev                                 # run the app
npm run lint && npm run build                     # oxlint + tsc + vite
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
python3 scripts/sync-devices.py --check           # generated files up to date?
python3 scripts/probe-device.py                   # what is plugged in, in table form
```

Rust artifacts are large; `export CARGO_TARGET_DIR=/tmp/hidra-target` if disk is
tight. CI runs lint, build, fmt, clippy and the sync check on Linux, Windows and
macOS.

## Adding a device (the common task)

There is a `/add-device` command with the full flow. In brief:

1. `python3 scripts/probe-device.py` — on Linux, this gives you vid, pid,
   interface, both usage pages and a pasteable entry. Elsewhere, the numbers
   have to come from the user; ask for `--json` output from a Linux machine or
   the equivalent from USBView / ioreg.
2. Paste into `SUPPORTED` in `devices.rs`; choose the `Protocol` variant whose
   report shape matches. Set `support: Support::Reported` unless the user has
   confirmed it works on their hardware.
3. `python3 scripts/sync-devices.py`.
4. `npm run build` and `cargo clippy … -D warnings`.
5. **Ask the user to test it.** Read a setting, change a setting, watch the
   hardware. You cannot verify this yourself, and claiming otherwise is the
   worst thing you can do here. Only after they confirm does `support` become
   `Support::Verified` (then re-run the sync script).
6. Commit, and say in the message which OS and device the user tested on.

If no existing `Protocol` fits, the device needs a capture — see
`docs/ADDING-DEVICES.md`, "Adding a new protocol". That is a much larger change:
a new module next to `mouse.rs`, a `Protocol` variant, an entry in `FEATURES` in
`scripts/sync-devices.py`, and notes in `PROTOCOL.md`.

## Conventions

- Comments explain **why**, not what. Match the density of the surrounding code.
- Code, comments, docs, commit messages and issue templates: English. UI strings
  use `useTranslation()` and the catalogs under `src/locales/` (English default,
  pt-BR also supported). Both READMEs change together. See `docs/TRANSLATING.md`.
- Tauri commands are one-per-user-action and live in `lib.rs`; device I/O never
  happens in the UI layer.
- A Tauri command whose JS caller passes a multi-word argument needs
  `#[tauri::command(rename_all = "snake_case")]`.
- Releases: `sh scripts/bump-version.sh X.Y.Z`, update `CHANGELOG.md`, tag
  `vX.Y.Z`, push. CI builds and publishes every installer. Never publish by hand.

## What only a human can do

Confirm that hardware reacted. Every device entry, every protocol change and
every UI change to a device panel needs somebody with that device on their desk
to look at it. Write the code, prepare the test steps, then ask — and report what
they saw, not what you expect they will see.
