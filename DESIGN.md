> Historical notes from version 0.1 (package `qrcode`). Some statements here (no version information needed, manifest key `qrcode`, Chat UX) were wrong or are outdated; see README.md for current behaviour.

# QR CODE GENERATOR
## Design Specification

---

## Problem Statement

You need to transfer a URL, WiFi password, bitcoin address, or text snippet from your Precursor to a phone. The Precursor has no Bluetooth. Its WiFi may not be connected. USB requires drivers. But it has a 336x536 high-contrast display and phones have cameras.

**QR codes are the air-gap bridge.**

Type text on the Precursor keyboard, generate a QR code, hold the screen up to any phone camera. Transfer complete. No network. No pairing. No drivers. No trust required.

This is the most natural data export mechanism for a security-focused device.

---

## Precursor Fit: Perfect

| Constraint | Advantage |
|-----------|-----------|
| 1-bit monochrome | QR codes ARE 1-bit — black modules on white. Zero color mapping needed. |
| 336x536 display | Enough for QR Version 10 (57x57 modules) at 5px/module = 285px. Plenty of room. |
| Physical keyboard | Direct text entry, no autocorrect |
| No network required | QR codes are the network — optical transfer |
| PDDB encrypted | Saved QR codes (WiFi passwords, keys) are encrypted at rest |
| TRNG available | Could generate random passwords/keys and immediately encode them |

---

## Features

### Must Have (P0)
- Text input via keyboard
- QR code generation (Version 1-10, Error Correction Level M)
- Display QR code centered on screen
- Multiple encoding modes: Numeric, Alphanumeric, Byte (auto-detected)
- Clear/new code

### Should Have (P1)
- Save codes to PDDB with custom names
- Load saved codes
- Delete saved codes
- Adjustable error correction level (L/M/Q/H)
- Adjustable module size (auto-fit, 3px, 4px, 5px, 6px)

### Could Have (P2)
- TCP import (receive text from desktop, generate QR)
- Pre-built templates: WiFi, URL, vCard, Bitcoin address
- QR code version info display
- Copy-to-clipboard via USB HID

### Won't Have (v1)
- QR code scanning/reading (no camera)
- Micro QR codes
- Color QR codes (1-bit display)
- Logo embedding

---

## Screen Flows

```
                    ┌──────────┐
                    │ MainMenu │
                    └─────┬────┘
                          │
              ┌───────────┼───────────┐
              ▼           ▼           ▼
        ┌──────────┐ ┌────────┐ ┌──────────┐
        │  Input   │ │  Load  │ │ Settings │
        │  (type)  │ │  List  │ │          │
        └─────┬────┘ └───┬────┘ └──────────┘
              │           │
              ▼           ▼
        ┌──────────┐ ┌────────┐
        │ Display  │ │ Display│
        │ QR Code  │ │ QR Code│
        └─────┬────┘ └────────┘
              │
              ▼
        ┌──────────┐
        │  Save?   │
        │ (prompt) │
        └──────────┘
```

### States
1. **MainMenu** — "New QR", "Saved Codes", "Settings", "Help"
2. **Input** — Text entry with live character count and version estimate
3. **Display** — QR code rendered, full screen, minimal chrome
4. **SavePrompt** — "Save this code? [Y/N]" with name entry
5. **LoadList** — Scrollable list of saved codes
6. **Settings** — Error correction level, module size
7. **Help** — Keyboard reference

---

## Keyboard Mapping

### MainMenu
| Key | Action |
|-----|--------|
| Up/Down | Navigate menu |
| Enter | Select item |
| Q | Quit app |
| N | Quick: jump to New QR |
| H | Help |

### Input Mode
| Key | Action |
|-----|--------|
| A-Z, 0-9, symbols | Type characters |
| Backspace | Delete last character |
| Enter | Generate QR code |
| Q | Back to menu (if input empty) / Clear input (if not empty, second Q = back) |
| F1 | Template: WiFi |
| F2 | Template: URL (pre-fills "https://") |
| F3 | Template: vCard |
| F4 | Clear all input |

### Display Mode
| Key | Action |
|-----|--------|
| S | Save this code |
| N | New code (back to input) |
| +/Up | Increase module size |
| -/Down | Decrease module size |
| Q | Back to menu |
| E | Export via TCP |

### LoadList
| Key | Action |
|-----|--------|
| Up/Down | Navigate list |
| Enter | Load and display |
| D | Delete selected |
| Q | Back to menu |

### Settings
| Key | Action |
|-----|--------|
| Up/Down | Navigate options |
| Left/Right | Change value |
| Enter | Confirm |
| Q | Back to menu |

---

## PDDB Schema

### Dictionary: `qrcode.codes`
Saved QR codes.

| Key | Format | Description |
|-----|--------|-------------|
| `index` | JSON array of strings | Ordered list of code names |
| `code.{name}` | JSON object | `{ "text": "...", "ec_level": "M", "created": "2026-02-13" }` |

### Dictionary: `qrcode.settings`
| Key | Format | Description |
|-----|--------|-------------|
| `config` | JSON | `{ "ec_level": "M", "module_size": 0, "last_template": null }` |

Module size 0 = auto-fit (maximize for display).

---

## QR Code Generation Algorithm

### Overview
QR codes encode data in a 2D grid of black/white modules. The generation process:

1. **Mode selection**: Detect optimal encoding (Numeric, Alphanumeric, Byte)
2. **Version selection**: Choose smallest version that fits data + EC
3. **Data encoding**: Convert text to bit stream with mode indicators
4. **Error correction**: Generate Reed-Solomon EC codewords
5. **Matrix construction**: Place data + EC in module grid
6. **Masking**: Apply 8 mask patterns, evaluate, choose best
7. **Format info**: Encode EC level + mask pattern

### Implementation Strategy
Implement QR code generation from scratch in Rust. No external crate.

**Why from scratch?**
- `no_std` environment — most QR crates depend on `std`
- Educational value — understanding the algorithm
- Zero dependencies — no supply chain risk on a security device
- Full control over rendering pipeline

### Supported Versions
- Version 1 (21x21) through Version 10 (57x57)
- Version 10 at 5px/module = 285x285px — fits comfortably in 336px width
- Higher versions possible but modules get small

### Error Correction Levels
| Level | Recovery | Use Case |
|-------|----------|----------|
| L | ~7% | Maximum data capacity |
| M | ~15% | Default — good balance |
| Q | ~25% | Outdoor/poor conditions |
| H | ~30% | Maximum reliability |

---

## Rendering

### QR Code Display
- Center QR code in content area (336 x ~460px)
- 4-module quiet zone (white border) required by spec
- Module size = auto-fit: `(min(336, 460) - 8*module_px) / (version*4 + 17)`
- Black modules = PixelColor::Dark
- White modules = PixelColor::Light
- Quiet zone = PixelColor::Light

### Status Display (below QR)
- Text preview (truncated to 1 line)
- Version + EC level
- Character count / capacity

### UX Type
`UxType::Chat` — standard layout. QR code drawn as filled rectangles via GAM.

---

## Complexity Estimate

| Metric | Estimate |
|--------|----------|
| Total LOC | ~1,500-2,000 |
| Modules | 6-7 (main, app, qr_encode, qr_render, storage, ui, templates) |
| Threading | None needed (generation is fast enough synchronously) |
| PDDB dictionaries | 2 (codes, settings) |
| TCP | Optional (port 7880) |
| Pump thread | Not needed |
| Build complexity | Medium — QR encoding from scratch is the bulk |

### Key Technical Challenges
1. **Reed-Solomon error correction** — GF(2^8) arithmetic, generator polynomials
2. **Data masking evaluation** — 8 patterns, penalty scoring
3. **Bit stream construction** — mode indicators, character counts, padding
4. **Module placement** — function patterns, alignment, timing, format info

---

## Implementation Phases

### Phase 1: QR Engine
- Galois field GF(2^8) arithmetic
- Reed-Solomon encoder
- Data encoding (numeric, alphanumeric, byte modes)
- Version/capacity tables
- Module matrix construction
- Masking and penalty evaluation
- Format and version info encoding

### Phase 2: Rendering
- Matrix to pixel conversion
- Auto-fit module sizing
- Centered display
- Status bar with metadata

### Phase 3: Input & Navigation
- Text input with keyboard
- State machine (menu, input, display, load, settings)
- Character count / version preview

### Phase 4: Storage
- Save/load/delete codes via PDDB
- Settings persistence

### Phase 5: Polish
- Templates (WiFi, URL, vCard)
- Adjustable module size
- Help screen
- TCP import (optional)

---

## TCP Import Protocol (Port 7880)

```
echo "Hello World" | nc <precursor-ip> 7880
```

Receives raw text, generates QR, displays immediately. Simple.
