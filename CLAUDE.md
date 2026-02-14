# QR Code Generator — Agent Instructions

## App Identity
- **Package**: `qrcode`
- **SERVER_NAME**: `_QR Code Generator_`
- **APP_NAME**: `QR Code`
- **manifest context_name**: `QR Code`
- **TCP Port**: 7880 (import, not yet implemented)

## Architecture
- **UX Type**: `UxType::Chat` with raw keyboard input
- **State Machine**: 7 states (MainMenu, Input, Display, SavePrompt, SaveNameEntry, LoadList, Settings, Help)
- **Threading**: None — QR generation is synchronous (fast enough on 100MHz)
- **PDDB**: 2 dictionaries (`qrcode.settings`, `qrcode.codes`)

## QR Encoder (`qr_encode.rs`)
This module has **zero Xous dependencies**. It is a standalone QR encoder that could be extracted into a library crate.

### Key implementation details:
- GF(2^8) arithmetic with runtime-computed exp/log tables
- Reed-Solomon via polynomial division over GF(2^8)
- Three encoding modes: Numeric (3 digits → 10 bits), Alphanumeric (2 chars → 11 bits), Byte (1 char → 8 bits)
- QR Versions 1-10 (21x21 to 57x57 modules)
- All 8 mask patterns evaluated with full 4-rule penalty scoring
- Format info uses BCH(15,5) encoding XORed with 0x5412

### Critical constants:
- `TOTAL_CODEWORDS`: Total codewords per version
- `EC_CODEWORDS_PER_BLOCK`: EC codewords per RS block
- `BLOCK_INFO`: (group1_blocks, group1_data, group2_blocks, group2_data) per version/EC
- `DATA_CAPACITY`: Character capacity per version/EC/mode
- `ALIGNMENT_POSITIONS`: Alignment pattern centers per version

## Rendering Strategy
- QR modules are drawn as filled rectangles via `gam.draw_rectangle()`
- Module size auto-calculated: `min(336, 460) / (qr_modules + 8)`
- 4-module quiet zone (white border) per QR specification
- Status bar below QR code shows version, EC level, module size

## Patterns Established (for ecosystem)
1. **Standard key constants**: `KEY_UP`, `KEY_DOWN`, etc. — copy to all apps
2. **Header/footer drawing helpers**: Reusable across all Chat-mode apps
3. **JSON PDDB storage**: Settings via serde_json, index+items pattern for collections
4. **State machine with `needs_redraw` flag**: Avoids unnecessary redraws
5. **Focus-aware save**: Settings saved on background and quit

## Known Limitations
- Text input doesn't support cursor movement (append-only)
- No TCP import yet (port 7880 reserved)
- Version 10 max (271 bytes in Byte mode at EC-L)
- No kanji encoding mode

## Build
```bash
cargo build -p qrcode --target riscv32imac-unknown-xous-elf
cargo xtask renode-image qrcode
```
