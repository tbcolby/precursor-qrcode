# Precursor QR Code Generator

Shows QR codes on the Precursor's 1-bit display so a phone or scanner can read text off the device without a cable or radio. Type text, press Enter, scan.

Status: builds at the toolkit's xous-core anchor, clippy-clean, 26 host tests pass. **Not yet verified on Renode or hardware**: `capture.py` is written but the Renode capture hasn't been run, and no code has been scanned off a real device yet.

## Features

- **QR versions 1-10** (21×21 to 57×57 modules), smallest version chosen automatically, **error correction L / M / Q / H** (default M).
- **Modes**: numeric, alphanumeric (`0-9 A-Z space $ % * + - . / :`) or byte, whichever single mode fits the whole text. Maximum at EC-L: 652 digits, 395 alphanumeric characters, or 271 bytes; at EC-H: 288 / 174 / 119.
- **All eight masks** are scored with the ISO/IEC 18004 penalty rules and the lowest is used.
- **Live counter** while typing: "N of M bytes at EC-x", the version the text needs, or why it won't fit.
- **Cursor editing** (← → move, ↑ ↓ start / end) and **templates**: F1 WiFi (`WIFI:T:WPA;S:;P:;;`, cursor in the SSID), F2 URL (`https://`), F3 vCard 3.0 skeleton (cursor in the name).
- **Display**: the module size is the largest that fits, or a fixed 2-8 px (reduced automatically if the code wouldn't fit). 4-module quiet zone. ← → re-encodes the shown code at another EC level without changing your default.
- **Saved codes** with unique names; each keeps its own EC level. Delete asks for confirmation (`d` twice) and removes the payload.
- **Hide on focus loss** (on by default): when the app goes to the background, a displayed code is cleared and the app returns to its menu.

The encoder is in `core/` (`qrcode-app-core`, no Xous dependencies). Its tests rebuild every version 1-10 × EC level × mode × mask combination with the independent [`qrcode`](https://crates.io/crates/qrcode) crate and require identical data codewords, interleaved codewords and module matrices. They also check the ISO/IEC 18004 capacity table, format- and version-information values, and the Reed-Solomon worked example.

## Keys

Backspace or F4 leaves every screen. The `∴` key opens the system menu and never reaches the app.

| Screen | Key | Action |
|---|---|---|
| Menu | ↑ ↓, Enter | Choose New QR code / Saved codes / Settings / Help |
| | `n` | New QR code |
| | Backspace, F4 | Leave the app (it stays running in the background) |
| Input | type | Insert at the cursor (printable ASCII) |
| | ← → / ↑ ↓ | Move the cursor / jump to start or end |
| | Enter | Make the code, or show why it can't be made |
| | F1 / F2 / F3 | WiFi / URL / vCard template (replaces the text) |
| | Backspace | Delete before the cursor; on an empty field, back to the menu |
| | F4 | Clear the field; on an empty field, back to the menu |
| QR code | ↑ ↓ | Bigger / smaller modules (auto, 2-8 px) |
| | ← → | Re-encode at the next / previous EC level |
| | `s` | Save under a name |
| | `n` | New code |
| | Backspace, F4 | Back |
| Save | type, Enter | Name (up to 24 characters), save |
| | Backspace / F4 | Delete a character (empty: cancel) / cancel |
| Saved codes | ↑ ↓, Enter | Show the selected code at its saved EC level |
| | `d` `d` | Delete the selected code |
| | Backspace, F4 | Back |
| Settings | ↑ ↓, ← → / Enter | Choose a row (EC level, module size, hide on focus loss), change it |
| | Backspace, F4 | Back (saves changed settings) |

## Storage

| PDDB dictionary | Key | Contents |
|---|---|---|
| `qrcode.settings` | `config` | `{"version":1,"ec_level":"M","module_size":0,"hide_on_background":true}` |
| `qrcode.codes` | `index` | `{"version":1,"codes":[{"name":…,"text":…,"ec_level":…}]}` |

- Records are replaced with delete-then-write in the basis that already holds them; new records go to the most recently unlocked basis, and the save screen shows which basis the saved codes live in.
- Settings are written only when they changed (when leaving Settings or the code screen, or on focus loss). Saved codes are written when you add or delete one.
- When a PDDB basis is locked or unlocked the app reloads everything and returns to the menu, so codes from a locked basis don't stay on screen.
- A record that can't be read, or that a newer version wrote, is never overwritten; the app shows a message.
- **Upgrading from 0.1**: 0.1 kept one `code.<name>` key per code and its delete never removed them. On first load the app moves those codes into the single `index` record and deletes every `code.*` key, including ones 0.1 failed to delete.

## Limitations and security notes

- A QR code on screen can be read by any camera in view, including from a distance. Don't display secrets where others can see the screen.
- The PDDB encrypts saved codes at rest. The app adds no protection of its own.
- Versions above 10, mixed-mode segments, Kanji mode, ECI, and Micro QR are not implemented. Byte mode is plain ASCII (the keyboard input is ASCII).
- WiFi templates aren't escaped for you: put a backslash before `\ ; , : "` in an SSID or password.
- No import over the network (0.1 reserved TCP port 7880 for it; it was never implemented).
- Screen geometry hasn't been checked on a device yet. Renode capture not yet run.

## Building

The app builds inside a [xous-core](https://github.com/betrusted-io/xous-core) tree. The package is `qrcode-app` (not `qrcode`, which would collide with the crate of that name), so the manifest key and `gam::APP_NAME_QRCODE_APP` match. With [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit):

```bash
bin/integrate --xous-root ~/xous-core --grade ~/Dev/precursor/precursor-qrcode   # build, clippy, tests, lint
cd ~/xous-core && cargo xtask renode-image qrcode-app                              # emulator image
```

`bin/integrate` copies the app to `apps/qrcode-app`, merges `manifest-fragment.json` into `apps/manifest.json`, and adds the workspace member. Host tests alone: `cd core && cargo test`.

## Layout

```
Cargo.toml               app crate: gam, precursor-appkit, qrcode-app-core
core/src/encode.rs       QR encoder: modes, RS over GF(256), placement, masks, format/version info
core/src/model.rs        settings, saved codes, 0.1 migration, templates, text field
core/src/tests.rs        differential tests vs the qrcode crate, ISO vectors, regressions
src/main.rs              GAM registration and message loop
src/app.rs               screens, keys, persistence
src/ui.rs                drawing (one rectangle per dark run, batched)
capture.py               Renode capture sequence (not yet run)
```

## Author

Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).
