# Precursor QR Code Generator

**The air gap is the bridge.**

```
"The most secure channel between two devices
is the one that never touches a wire.
Light travels from screen to camera.
No packets. No protocols. No trust required.
Just photons and geometry."
```

---

## What This Is

Precursor QR Code Generator creates QR codes on the Precursor's 1-bit monochrome display. Type text on the physical keyboard, press Enter, and a standards-compliant QR code appears on screen. Point any phone camera at it. Transfer complete.

This is a from-scratch QR code encoder written in Rust with zero external dependencies. It implements the full ISO 18004 encoding pipeline: mode detection, Reed-Solomon error correction over GF(2^8), matrix construction with finder/alignment/timing patterns, all eight data masking patterns with penalty evaluation, and format information encoding. Versions 1 through 10. Error correction levels L, M, Q, and H.

No crate pulled from the internet. No supply chain. Just math.

---

## Why This Project

Every modern device has a dozen ways to send data to another device. Bluetooth. WiFi. NFC. AirDrop. QR codes. USB. Each one requires trust — trust that the protocol isn't compromised, trust that the radio isn't being intercepted, trust that the receiving device isn't logging the transfer.

Precursor was built to minimize trust. Its CPU is open-source. Its storage is hardware-encrypted. Its radio can be physically disabled. But until now, getting data *out* of the device meant either TCP over WiFi (requires network connectivity and trust in the network) or USB HID autotype (requires physical cable and trust in the USB stack).

QR codes require trusting only physics.

Light leaves the display. It enters a camera. The geometry of the pattern encodes the data. There is no radio to intercept, no protocol to compromise, no handshake to man-in-the-middle. The air gap between screen and camera is the most auditable communication channel in computing.

This is not a convenience feature. On a security-focused device, it is an *architectural primitive*.

---

## Why Precursor

The match between QR codes and Precursor hardware is almost suspicious in its perfection.

**1-bit display IS the QR spec.** QR codes are black and white modules. The Precursor display is black and white pixels. There is no color mapping, no dithering, no approximation. Every dark module is a block of `PixelColor::Dark` pixels. Every light module is `PixelColor::Light`. The display *is* the encoding.

**336x536 resolution is generous.** A Version 6 QR code (41x41 modules) at 6 pixels per module with 4-module quiet zone is 294x294 pixels — fits with room for a status bar. Even Version 10 (57x57 modules) at 5px/module is 325x325 — still fits. The display is large enough for meaningful data capacity.

**Physical keyboard for direct entry.** No on-screen keyboard stealing display real estate. No autocorrect mangling URLs. No predictive text leaking your input to a language model. You type what you mean. The QR code encodes what you typed.

**PDDB for encrypted storage.** WiFi passwords, bitcoin addresses, API keys — the kind of data you encode into QR codes is often sensitive. On Precursor, saved codes live in hardware-encrypted flash with plausible deniability. Your QR code library is private by design.

**No network dependency.** The entire point is that QR codes work without a network. The app works without WiFi. There is nothing to connect to, nothing to authenticate, nothing that can fail because a server is down.

---

## How It Works

### Features

- **Text input** with live character count, mode detection, and version preview
- **QR encoding** from scratch: Numeric, Alphanumeric, and Byte modes
- **Versions 1-10** (21x21 to 57x57 modules) — auto-selected for minimum size
- **Error correction** L/M/Q/H — configurable, defaults to M (~15% recovery)
- **Auto-fit display** — maximizes module size for readability
- **Adjustable module size** — 3px to 8px per module, or auto
- **Save/load codes** to encrypted PDDB storage
- **Templates** — WiFi network, URL, vCard pre-fills
- **Full quiet zone** — 4-module white border per QR specification

### Keyboard Controls

#### Main Menu
| Key | Action |
|-----|--------|
| Up/Down | Navigate menu |
| Enter | Select |
| N | New QR code |
| Q | Quit |

#### Text Input
| Key | Action |
|-----|--------|
| A-Z, 0-9, symbols | Type text |
| Backspace | Delete last character |
| Enter | Generate QR code |
| F1 | WiFi template |
| F2 | URL template (pre-fills https://) |
| F3 | vCard template |
| F4 | Clear all input |
| Q | Back (when input is empty) |

#### QR Display
| Key | Action |
|-----|--------|
| S | Save this code |
| N | New code |
| Up | Increase module size |
| Down | Decrease module size |
| Q | Back to menu |

#### Saved Codes
| Key | Action |
|-----|--------|
| Up/Down | Navigate list |
| Enter | Load and display |
| D | Delete selected |
| Q | Back to menu |

### Templates

**WiFi**: Generates `WIFI:T:WPA;S:;P:;;` — fill in SSID and password. Most phones auto-connect when they scan this.

**URL**: Pre-fills `https://` — just type the domain and path.

**vCard**: Generates a vCard 3.0 skeleton — fill in name and contact info.

---

## Screenshots

*Captured via headless Renode emulation on macOS ARM64. The Precursor display is 336x536 pixels, 1-bit monochrome.*

*(Screenshots will be captured after Renode testing)*

---

## Technical Architecture

```
apps/qrcode/
├── Cargo.toml        # Dependencies: xous, gam, pddb, serde
└── src/
    ├── main.rs       # Entry point, event loop, GAM registration
    ├── app.rs        # State machine, input handling, settings
    ├── qr_encode.rs  # Complete QR encoder: GF(2^8), Reed-Solomon,
    │                  # data encoding, matrix, masking, format info
    ├── ui.rs         # Screen rendering: menu, input, QR display,
    │                  # save/load, settings, help
    └── storage.rs    # PDDB persistence for codes and settings
```

### Design Decisions

**From-scratch QR encoder.** No external QR crate. The entire encoding pipeline — Galois field arithmetic, Reed-Solomon error correction, data mode encoding, matrix construction, mask pattern evaluation, format information — is implemented in `qr_encode.rs`. This eliminates supply chain risk, ensures `no_std` compatibility, and gives us full control over the rendering pipeline. The encoder has zero Xous dependencies and could be extracted as a standalone library.

**GF(2^8) with precomputed tables.** Reed-Solomon encoding requires Galois field multiplication. We precompute exp and log tables at runtime (512 + 256 bytes) for O(1) field operations. The generator polynomial for each EC block count is computed fresh — no static tables needed for the polynomial itself.

**Eight-mask evaluation with full penalty scoring.** The QR spec defines four penalty rules for evaluating mask patterns. We apply all eight masks to a cloned matrix, score each, and select the minimum penalty. This is ~8x the work of a fixed mask but produces optimally scannable codes.

**Chat UX with raw keys.** Like the calculator, we register for `UxType::Chat` with raw keyboard input. Every keypress goes through our own input handler — no GAM text pipeline interference.

**JSON for PDDB storage.** Settings and saved codes use serde JSON. This is slightly larger than binary encoding but infinitely easier to debug, migrate, and inspect. For the small data sizes involved (a few KB total), the overhead is negligible.

### PDDB Storage Layout

| Dictionary | Key | Contents |
|-----------|-----|----------|
| `qrcode.settings` | `config` | `{ "ec_level": "M", "module_size": 0 }` |
| `qrcode.codes` | `index` | JSON array of saved code names |
| `qrcode.codes` | `code.{name}` | `{ "text": "...", "ec_level": "M" }` |

### QR Encoding Pipeline

```
Text Input
    │
    ▼
Mode Detection (Numeric > Alphanumeric > Byte)
    │
    ▼
Version Selection (smallest V1-V10 that fits)
    │
    ▼
Data Encoding (mode indicator + char count + payload)
    │
    ▼
Reed-Solomon EC (GF(2^8) polynomial division)
    │
    ▼
Block Interleaving (data + EC codewords)
    │
    ▼
Matrix Construction (finder + alignment + timing patterns)
    │
    ▼
Data Placement (zigzag traversal)
    │
    ▼
Mask Evaluation (8 patterns × 4 penalty rules)
    │
    ▼
Format Info (EC level + mask, BCH encoded)
    │
    ▼
Final QR Matrix (ready to render)
```

### Dependencies

```toml
[dependencies]
xous = "0.9.69"
xous-ipc = "0.10.9"
gam = { path = "../../services/gam" }
pddb = { path = "../../services/pddb" }
ticktimer-server = { package = "xous-api-ticktimer", version = "0.9.68" }
log-server = { package = "xous-api-log", version = "0.1.68" }
xous-names = { package = "xous-api-names", version = "0.9.70" }
num-derive = { version = "0.4.2", default-features = false }
num-traits = { version = "0.2.14", default-features = false }
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
log = "0.4.14"
```

---

## Building

Precursor QR Code Generator is a Xous app. It builds as part of the [xous-core](https://github.com/betrusted-io/xous-core) workspace.

### Integration

1. Copy `src/` and `Cargo.toml` to `xous-core/apps/qrcode/`

2. Add to workspace `Cargo.toml`:
   ```toml
   "apps/qrcode",
   ```

3. Add to `apps/manifest.json`:
   ```json
   "qrcode": {
       "context_name": "QR Code",
       "menu_name": {
           "appmenu.qrcode": {
               "en": "QR Code",
               "en-tts": "QR Code"
           }
       }
   }
   ```

4. Build for Renode emulator:
   ```bash
   cargo xtask renode-image qrcode
   ```

5. Build for hardware:
   ```bash
   cargo xtask app-image qrcode
   ```

---

## Technical Notes

- QR encoding is pure Rust with zero external dependencies beyond std/alloc
- Reed-Solomon uses GF(2^8) with primitive polynomial 0x11D
- All eight mask patterns evaluated with full four-rule penalty scoring
- Supports Numeric (0-9), Alphanumeric (0-9, A-Z, symbols), and Byte (UTF-8) modes
- Version auto-selection picks the smallest version that fits the data
- Module size auto-fit maximizes readability on the 336px display
- Saved codes are encrypted at rest via PDDB
- The QR encoder module (`qr_encode.rs`) has zero Xous dependencies and could be extracted as a standalone `no_std` library

---

## Development

This app was developed using the methodology described in [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) — an LLM-assisted approach to Precursor app development on macOS ARM64.

App #1 in the 32-app Precursor ecosystem campaign.

---

## Author

Made by Tyler Colby — [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: [tyler@colbysdatamovers.com](mailto:tyler@colbysdatamovers.com) | [GitHub Issues](https://github.com/tbcolby/precursor-qrcode/issues)

---

## License

Licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full text.
