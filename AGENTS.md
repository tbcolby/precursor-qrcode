> Historical notes from version 0.1 (package `qrcode`). Some statements here (no version information needed, manifest key `qrcode`, Chat UX) were wrong or are outdated; see README.md for current behaviour.

# QR Code Generator — Agent Evolution Report

## Agents Used

### From xous-dev-toolkit:
1. **ideation.md** — Used for DESIGN.md, feature prioritization, screen flows
2. **architecture.md** — State machine design, opcode layout, module organization
3. **graphics.md** — Screen layout, text rendering, rectangle drawing
4. **storage.md** — PDDB schema design, JSON serialization patterns
5. **build.md** — Cargo.toml, manifest integration, dependency versions
6. **review.md** — Standards compliance checking

### Not Used:
- **networking.md** — TCP import deferred to v1.1
- **system.md** — No hardware access needed
- **testing.md** — Renode capture to be added

## New Specialist Agent: `encoding.md`

**Born from this build.** The QR encoder revealed patterns applicable to barcode generation (App #2) and potentially other encoding apps.

### Agent Definition: `agents/encoding.md`

**Role**: Implement data encoding algorithms (QR codes, barcodes, checksums, error correction) optimized for Precursor's 1-bit display.

**Expertise**:
- Galois field arithmetic (GF(2^8) and variants)
- Reed-Solomon error correction
- BCH codes (format/version info in QR)
- Polynomial arithmetic over finite fields
- Bit stream construction and padding
- Matrix/grid construction with reserved regions
- Mask pattern evaluation and selection
- Module-to-pixel rendering for 1-bit displays
- Auto-fit sizing calculations

**Patterns**:
- Precomputed exp/log tables for field arithmetic
- Generator polynomial construction via iterative multiplication
- Systematic encoding (data + remainder)
- Block interleaving for multi-block EC
- Zigzag data placement with reserved region skipping
- Penalty scoring for mask optimization
- Quiet zone enforcement

**Quality Criteria**:
- Encoder module has ZERO Xous dependencies (extractable as library)
- All encoding tables are compile-time or deterministic runtime
- Auto-mode detection (choose optimal encoding for input)
- Auto-version selection (choose minimum version for data)
- Rendering uses only GAM rectangle primitives

**Handoffs**:
- FROM ideation.md: "What encoding standard to implement"
- TO graphics.md: "How to render the encoded matrix"
- TO storage.md: "How to persist encoded data"
- TO build.md: "Whether to extract as library crate"

## Ecosystem Standards Confirmed

This build validated the following ecosystem standards:
- Keyboard constants (`KEY_UP`, etc.) — worked perfectly
- Header/footer layout pattern — reusable
- PDDB naming convention (`appname.category`) — clean
- JSON settings storage — good for small data
- Focus change handling — essential
- Menu navigation pattern (highlight bar + invert text)

## Recommended Toolkit Updates

1. **Add encoding.md** to `agents/` directory
2. **Add UI helpers** to graphics.md: `draw_header()`, `draw_footer()`, `draw_highlight_bar()` as common patterns
3. **Add to discoveries.md**: "JSON vs binary for PDDB — JSON recommended for settings, binary for large data"
4. **Add to STANDARDS.md**: Confirmed menu navigation pattern (dark rectangle + inverted text)

## Metrics

| Metric | Value |
|--------|-------|
| Source files | 4 |
| Estimated LOC | ~1,800 |
| PDDB dictionaries | 2 |
| States | 7+1 (Help) |
| External dependencies | 0 (QR encoder) |
| Toolkit agents used | 6 of 11 |
| New agents created | 1 (encoding.md) |
