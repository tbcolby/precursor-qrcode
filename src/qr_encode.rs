//! QR Code encoder — from-scratch implementation for no_std environments.
//!
//! Supports QR Version 1-10, Error Correction Levels L/M/Q/H,
//! and Numeric/Alphanumeric/Byte encoding modes.
//!
//! Zero external dependencies. Pure Rust. Built for Precursor.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

// ─── Error Correction Levels ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EcLevel {
    L, // ~7% recovery
    M, // ~15% recovery
    Q, // ~25% recovery
    H, // ~30% recovery
}

impl EcLevel {
    pub fn label(&self) -> &'static str {
        match self {
            EcLevel::L => "L",
            EcLevel::M => "M",
            EcLevel::Q => "Q",
            EcLevel::H => "H",
        }
    }

    pub fn next(&self) -> EcLevel {
        match self {
            EcLevel::L => EcLevel::M,
            EcLevel::M => EcLevel::Q,
            EcLevel::Q => EcLevel::H,
            EcLevel::H => EcLevel::L,
        }
    }

    pub fn format_bits(&self) -> u8 {
        match self {
            EcLevel::L => 0b01,
            EcLevel::M => 0b00,
            EcLevel::Q => 0b11,
            EcLevel::H => 0b10,
        }
    }
}

// ─── Encoding Modes ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Numeric,      // 0-9
    Alphanumeric, // 0-9, A-Z, space, $%*+-./:
    Byte,         // Any UTF-8
}

impl Mode {
    pub fn indicator(&self) -> u8 {
        match self {
            Mode::Numeric => 0b0001,
            Mode::Alphanumeric => 0b0010,
            Mode::Byte => 0b0100,
        }
    }

    /// Character count indicator bit length for versions 1-9.
    pub fn cc_bits_small(&self) -> usize {
        match self {
            Mode::Numeric => 10,
            Mode::Alphanumeric => 9,
            Mode::Byte => 8,
        }
    }

    /// Character count indicator bit length for version 10.
    pub fn cc_bits_medium(&self) -> usize {
        match self {
            Mode::Numeric => 12,
            Mode::Alphanumeric => 11,
            Mode::Byte => 16,
        }
    }
}

/// Detect optimal encoding mode for the given text.
pub fn detect_mode(text: &str) -> Mode {
    if text.chars().all(|c| c.is_ascii_digit()) {
        Mode::Numeric
    } else if text.chars().all(|c| alphanumeric_value(c).is_some()) {
        Mode::Alphanumeric
    } else {
        Mode::Byte
    }
}

fn alphanumeric_value(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'A'..='Z' => Some(c as u8 - b'A' + 10),
        ' ' => Some(36),
        '$' => Some(37),
        '%' => Some(38),
        '*' => Some(39),
        '+' => Some(40),
        '-' => Some(41),
        '.' => Some(42),
        '/' => Some(43),
        ':' => Some(44),
        _ => None,
    }
}

// ─── Version/Capacity Tables ────────────────────────────────────────────────

/// Total codewords for each version (1-10).
const TOTAL_CODEWORDS: [usize; 10] = [26, 44, 70, 100, 134, 172, 196, 242, 292, 346];

/// EC codewords per block for [version-1][ec_level].
/// Order: L, M, Q, H
const EC_CODEWORDS_PER_BLOCK: [[usize; 4]; 10] = [
    [7, 10, 13, 17],   // V1
    [10, 16, 22, 28],  // V2
    [15, 26, 18, 22],  // V3
    [20, 18, 26, 16],  // V4
    [26, 24, 18, 22],  // V5
    [18, 16, 24, 28],  // V6
    [20, 18, 18, 26],  // V7
    [24, 22, 22, 26],  // V8
    [30, 22, 20, 24],  // V9
    [18, 26, 24, 28],  // V10
];

/// Number of EC blocks for [version-1][ec_level].
/// Format: (group1_blocks, group1_data_cw, group2_blocks, group2_data_cw)
const BLOCK_INFO: [[(usize, usize, usize, usize); 4]; 10] = [
    // V1:  L, M, Q, H
    [(1, 19, 0, 0), (1, 16, 0, 0), (1, 13, 0, 0), (1, 9, 0, 0)],
    // V2
    [(1, 34, 0, 0), (1, 28, 0, 0), (1, 22, 0, 0), (1, 16, 0, 0)],
    // V3
    [(1, 55, 0, 0), (1, 44, 0, 0), (2, 17, 0, 0), (2, 13, 0, 0)],
    // V4
    [(1, 80, 0, 0), (2, 32, 0, 0), (2, 24, 0, 0), (4, 9, 0, 0)],
    // V5
    [(1, 108, 0, 0), (2, 43, 0, 0), (2, 15, 2, 16), (2, 11, 2, 12)],
    // V6
    [(2, 68, 0, 0), (4, 27, 0, 0), (4, 19, 0, 0), (4, 15, 0, 0)],
    // V7
    [(2, 78, 0, 0), (4, 31, 0, 0), (2, 14, 4, 15), (4, 13, 1, 14)],
    // V8
    [(2, 97, 0, 0), (2, 38, 2, 39), (4, 18, 2, 19), (4, 14, 2, 15)],
    // V9
    [(2, 116, 0, 0), (3, 36, 2, 37), (4, 16, 4, 17), (4, 12, 4, 13)],
    // V10
    [(2, 68, 2, 69), (4, 43, 1, 44), (6, 19, 2, 20), (6, 15, 2, 16)],
];

/// Data capacity in characters for [version-1][ec_level][mode].
/// Mode order: Numeric, Alphanumeric, Byte
const DATA_CAPACITY: [[[usize; 3]; 4]; 10] = [
    // V1: L, M, Q, H × Num, Alnum, Byte
    [[41, 25, 17], [34, 20, 14], [27, 16, 11], [17, 10, 7]],
    // V2
    [[77, 47, 32], [63, 38, 26], [48, 29, 20], [34, 20, 14]],
    // V3
    [[127, 77, 53], [101, 61, 42], [77, 47, 32], [58, 35, 24]],
    // V4
    [[187, 114, 78], [149, 90, 62], [111, 67, 46], [82, 50, 34]],
    // V5
    [[255, 154, 106], [202, 122, 84], [144, 87, 60], [106, 64, 44]],
    // V6
    [[322, 195, 134], [255, 154, 106], [178, 108, 74], [139, 84, 58]],
    // V7
    [[370, 224, 154], [293, 178, 122], [207, 125, 86], [154, 93, 64]],
    // V8
    [[461, 279, 192], [365, 221, 152], [259, 157, 108], [202, 122, 84]],
    // V9
    [[552, 335, 230], [432, 262, 180], [312, 189, 130], [235, 143, 98]],
    // V10
    [[652, 395, 271], [513, 311, 213], [364, 221, 151], [288, 174, 119]],
];

/// Alignment pattern center coordinates for versions 2-10.
/// Version 1 has no alignment patterns.
const ALIGNMENT_POSITIONS: [&[usize]; 10] = [
    &[],             // V1
    &[6, 18],        // V2
    &[6, 22],        // V3
    &[6, 26],        // V4
    &[6, 30],        // V5
    &[6, 34],        // V6
    &[6, 22, 38],    // V7
    &[6, 24, 42],    // V8
    &[6, 26, 46],    // V9
    &[6, 28, 50],    // V10
];

/// Find the smallest version that can hold the given data.
pub fn select_version(text: &str, mode: Mode, ec: EcLevel) -> Option<u8> {
    let len = match mode {
        Mode::Byte => text.len(), // byte count
        _ => text.chars().count(), // character count
    };
    let ec_idx = ec_index(ec);
    let mode_idx = mode_index(mode);

    for v in 0..10 {
        if DATA_CAPACITY[v][ec_idx][mode_idx] >= len {
            return Some((v + 1) as u8);
        }
    }
    None // too long for version 1-10
}

fn ec_index(ec: EcLevel) -> usize {
    match ec {
        EcLevel::L => 0,
        EcLevel::M => 1,
        EcLevel::Q => 2,
        EcLevel::H => 3,
    }
}

fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::Numeric => 0,
        Mode::Alphanumeric => 1,
        Mode::Byte => 2,
    }
}

/// Size of the QR code matrix for a given version.
pub fn matrix_size(version: u8) -> usize {
    (version as usize) * 4 + 17
}

/// Maximum character capacity for a version/ec/mode combo.
pub fn capacity(version: u8, ec: EcLevel, mode: Mode) -> usize {
    DATA_CAPACITY[(version - 1) as usize][ec_index(ec)][mode_index(mode)]
}

// ─── Galois Field GF(2^8) Arithmetic ───────────────────────────────────────

/// GF(2^8) with primitive polynomial 0x11D (x^8 + x^4 + x^3 + x^2 + 1).
/// This is the field used by QR code Reed-Solomon encoding.

const GF_SIZE: usize = 256;

/// Precomputed exponential table: alpha^i for i in 0..255.
fn gf_exp_table() -> [u8; 512] {
    let mut table = [0u8; 512];
    let mut val: u16 = 1;
    for i in 0..255 {
        table[i] = val as u8;
        table[i + 255] = val as u8; // wrap for easy modular access
        val <<= 1;
        if val >= 256 {
            val ^= 0x11D;
        }
    }
    table
}

/// Precomputed log table: log_alpha(i) for i in 1..255.
fn gf_log_table() -> [u8; 256] {
    let exp = gf_exp_table();
    let mut table = [0u8; 256];
    for i in 0..255 {
        table[exp[i] as usize] = i as u8;
    }
    table
}

fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let log = gf_log_table();
    let exp = gf_exp_table();
    let log_sum = log[a as usize] as usize + log[b as usize] as usize;
    exp[log_sum]
}

fn gf_poly_mul(p: &[u8], q: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; p.len() + q.len() - 1];
    for (i, &a) in p.iter().enumerate() {
        for (j, &b) in q.iter().enumerate() {
            result[i + j] ^= gf_mul(a, b);
        }
    }
    result
}

/// Generate Reed-Solomon generator polynomial for n EC codewords.
fn rs_generator_poly(n: usize) -> Vec<u8> {
    let exp = gf_exp_table();
    let mut g = vec![1u8];
    for i in 0..n {
        g = gf_poly_mul(&g, &[1, exp[i]]);
    }
    g
}

/// Compute Reed-Solomon error correction codewords.
fn rs_encode(data: &[u8], ec_count: usize) -> Vec<u8> {
    let gen = rs_generator_poly(ec_count);
    let mut msg = Vec::with_capacity(data.len() + ec_count);
    msg.extend_from_slice(data);
    msg.resize(data.len() + ec_count, 0);

    for i in 0..data.len() {
        let coef = msg[i];
        if coef != 0 {
            for j in 1..gen.len() {
                msg[i + j] ^= gf_mul(gen[j], coef);
            }
        }
    }

    msg[data.len()..].to_vec()
}

// ─── Data Encoding ──────────────────────────────────────────────────────────

struct BitWriter {
    bits: Vec<bool>,
}

impl BitWriter {
    fn new() -> Self {
        Self { bits: Vec::new() }
    }

    fn write(&mut self, value: u32, count: usize) {
        for i in (0..count).rev() {
            self.bits.push((value >> i) & 1 != 0);
        }
    }

    fn len(&self) -> usize {
        self.bits.len()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for chunk in self.bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
        bytes
    }
}

fn encode_numeric(writer: &mut BitWriter, text: &str) {
    let chars: Vec<u32> = text.chars().map(|c| c as u32 - '0' as u32).collect();
    for chunk in chars.chunks(3) {
        match chunk.len() {
            3 => writer.write(chunk[0] * 100 + chunk[1] * 10 + chunk[2], 10),
            2 => writer.write(chunk[0] * 10 + chunk[1], 7),
            1 => writer.write(chunk[0], 4),
            _ => {}
        }
    }
}

fn encode_alphanumeric(writer: &mut BitWriter, text: &str) {
    let vals: Vec<u32> = text
        .chars()
        .map(|c| alphanumeric_value(c).unwrap_or(0) as u32)
        .collect();
    for chunk in vals.chunks(2) {
        match chunk.len() {
            2 => writer.write(chunk[0] * 45 + chunk[1], 11),
            1 => writer.write(chunk[0], 6),
            _ => {}
        }
    }
}

fn encode_byte(writer: &mut BitWriter, text: &str) {
    for byte in text.as_bytes() {
        writer.write(*byte as u32, 8);
    }
}

/// Encode the data payload (mode + character count + data + terminator + padding).
fn encode_data(text: &str, mode: Mode, version: u8, ec: EcLevel) -> Vec<u8> {
    let vi = (version - 1) as usize;
    let ei = ec_index(ec);

    // Total data codewords available
    let (g1_blocks, g1_data, g2_blocks, g2_data) = BLOCK_INFO[vi][ei];
    let total_data_cw = g1_blocks * g1_data + g2_blocks * g2_data;
    let total_data_bits = total_data_cw * 8;

    let mut writer = BitWriter::new();

    // Mode indicator (4 bits)
    writer.write(mode.indicator() as u32, 4);

    // Character count
    let cc_bits = if version <= 9 {
        mode.cc_bits_small()
    } else {
        mode.cc_bits_medium()
    };
    let char_count = match mode {
        Mode::Byte => text.len(),
        _ => text.chars().count(),
    };
    writer.write(char_count as u32, cc_bits);

    // Data encoding
    match mode {
        Mode::Numeric => encode_numeric(&mut writer, text),
        Mode::Alphanumeric => encode_alphanumeric(&mut writer, text),
        Mode::Byte => encode_byte(&mut writer, text),
    }

    // Terminator: up to 4 zero bits
    let remaining = total_data_bits.saturating_sub(writer.len());
    let terminator_bits = remaining.min(4);
    if terminator_bits > 0 {
        writer.write(0, terminator_bits);
    }

    // Pad to byte boundary
    while writer.len() % 8 != 0 {
        writer.write(0, 1);
    }

    // Pad with alternating 0xEC, 0x11
    let mut pad_idx = 0;
    while writer.len() < total_data_bits {
        writer.write(if pad_idx % 2 == 0 { 0xEC } else { 0x11 }, 8);
        pad_idx += 1;
    }

    writer.to_bytes()[..total_data_cw].to_vec()
}

// ─── Block Interleaving ─────────────────────────────────────────────────────

/// Split data into blocks, compute EC for each, interleave.
fn interleave_blocks(data: &[u8], version: u8, ec: EcLevel) -> Vec<u8> {
    let vi = (version - 1) as usize;
    let ei = ec_index(ec);
    let ec_per_block = EC_CODEWORDS_PER_BLOCK[vi][ei];
    let (g1_blocks, g1_data, g2_blocks, g2_data) = BLOCK_INFO[vi][ei];

    let mut data_blocks: Vec<Vec<u8>> = Vec::new();
    let mut ec_blocks: Vec<Vec<u8>> = Vec::new();
    let mut offset = 0;

    // Group 1 blocks
    for _ in 0..g1_blocks {
        let block = data[offset..offset + g1_data].to_vec();
        let ec = rs_encode(&block, ec_per_block);
        data_blocks.push(block);
        ec_blocks.push(ec);
        offset += g1_data;
    }

    // Group 2 blocks
    for _ in 0..g2_blocks {
        let block = data[offset..offset + g2_data].to_vec();
        let ec = rs_encode(&block, ec_per_block);
        data_blocks.push(block);
        ec_blocks.push(ec);
        offset += g2_data;
    }

    // Interleave data codewords
    let mut result = Vec::new();
    let max_data_len = data_blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    for i in 0..max_data_len {
        for block in &data_blocks {
            if i < block.len() {
                result.push(block[i]);
            }
        }
    }

    // Interleave EC codewords
    for i in 0..ec_per_block {
        for block in &ec_blocks {
            if i < block.len() {
                result.push(block[i]);
            }
        }
    }

    result
}

// ─── Matrix Construction ────────────────────────────────────────────────────

/// QR code matrix. true = dark module, false = light module.
#[derive(Clone)]
pub struct QrMatrix {
    pub size: usize,
    pub modules: Vec<Vec<bool>>,
    reserved: Vec<Vec<bool>>, // marks function pattern positions
}

impl QrMatrix {
    fn new(size: usize) -> Self {
        Self {
            size,
            modules: vec![vec![false; size]; size],
            reserved: vec![vec![false; size]; size],
        }
    }

    fn set(&mut self, row: usize, col: usize, dark: bool) {
        if row < self.size && col < self.size {
            self.modules[row][col] = dark;
        }
    }

    fn reserve(&mut self, row: usize, col: usize, dark: bool) {
        if row < self.size && col < self.size {
            self.modules[row][col] = dark;
            self.reserved[row][col] = true;
        }
    }

    fn is_reserved(&self, row: usize, col: usize) -> bool {
        row < self.size && col < self.size && self.reserved[row][col]
    }

    /// Place a finder pattern at (row, col) — top-left corner of the 7x7 pattern.
    fn place_finder(&mut self, row: isize, col: isize) {
        for dr in -1..=7isize {
            for dc in -1..=7isize {
                let r = row + dr;
                let c = col + dc;
                if r < 0 || r >= self.size as isize || c < 0 || c >= self.size as isize {
                    continue;
                }
                let dark = if dr < 0 || dr > 6 || dc < 0 || dc > 6 {
                    false // separator
                } else if dr == 0 || dr == 6 || dc == 0 || dc == 6 {
                    true // outer border
                } else if dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4 {
                    true // inner square
                } else {
                    false // gap
                };
                self.reserve(r as usize, c as usize, dark);
            }
        }
    }

    /// Place an alignment pattern centered at (row, col).
    fn place_alignment(&mut self, center_row: usize, center_col: usize) {
        for dr in -2..=2isize {
            for dc in -2..=2isize {
                let r = (center_row as isize + dr) as usize;
                let c = (center_col as isize + dc) as usize;
                if self.is_reserved(r, c) {
                    continue; // don't overlap finder patterns
                }
                let dark = dr.abs() == 2 || dc.abs() == 2 || (dr == 0 && dc == 0);
                self.reserve(r, c, dark);
            }
        }
    }

    /// Place timing patterns (row 6 and col 6).
    fn place_timing(&mut self) {
        for i in 8..self.size - 8 {
            let dark = i % 2 == 0;
            if !self.is_reserved(6, i) {
                self.reserve(6, i, dark);
            }
            if !self.is_reserved(i, 6) {
                self.reserve(i, 6, dark);
            }
        }
    }

    /// Reserve format info areas (filled later after masking).
    fn reserve_format_info(&mut self) {
        // Around top-left finder
        for i in 0..=8 {
            if i < self.size {
                self.reserved[8][i] = true;
                self.reserved[i][8] = true;
            }
        }
        // Around top-right finder
        for i in 0..8 {
            if self.size - 1 - i < self.size {
                self.reserved[8][self.size - 1 - i] = true;
            }
        }
        // Around bottom-left finder
        for i in 0..7 {
            if self.size - 7 + i < self.size {
                self.reserved[self.size - 7 + i][8] = true;
            }
        }
        // Dark module
        self.reserve(self.size - 8, 8, true);
    }

    /// Place data bits into the matrix using the standard QR zigzag pattern.
    fn place_data(&mut self, data_bits: &[bool]) {
        let mut bit_idx = 0;
        let mut col = self.size as isize - 1;

        while col > 0 {
            // Skip column 6 (timing pattern)
            if col == 6 {
                col -= 1;
            }

            // Two columns at a time, going up then down
            let going_up = ((self.size as isize - 1 - col) / 2) % 2 == 0;

            for step in 0..self.size {
                let row = if going_up {
                    self.size - 1 - step
                } else {
                    step
                };

                // Right column of the pair
                if !self.is_reserved(row, col as usize) {
                    if bit_idx < data_bits.len() {
                        self.modules[row][col as usize] = data_bits[bit_idx];
                        bit_idx += 1;
                    }
                }

                // Left column of the pair
                let left = (col - 1) as usize;
                if col > 0 && !self.is_reserved(row, left) {
                    if bit_idx < data_bits.len() {
                        self.modules[row][left] = data_bits[bit_idx];
                        bit_idx += 1;
                    }
                }
            }

            col -= 2;
        }
    }
}

// ─── Masking ────────────────────────────────────────────────────────────────

fn mask_condition(mask_id: u8, row: usize, col: usize) -> bool {
    match mask_id {
        0 => (row + col) % 2 == 0,
        1 => row % 2 == 0,
        2 => col % 3 == 0,
        3 => (row + col) % 3 == 0,
        4 => (row / 2 + col / 3) % 2 == 0,
        5 => (row * col) % 2 + (row * col) % 3 == 0,
        6 => ((row * col) % 2 + (row * col) % 3) % 2 == 0,
        7 => ((row + col) % 2 + (row * col) % 3) % 2 == 0,
        _ => false,
    }
}

fn apply_mask(matrix: &mut QrMatrix, mask_id: u8) {
    for row in 0..matrix.size {
        for col in 0..matrix.size {
            if !matrix.is_reserved(row, col) && mask_condition(mask_id, row, col) {
                matrix.modules[row][col] = !matrix.modules[row][col];
            }
        }
    }
}

/// Evaluate penalty score for a masked matrix.
fn evaluate_penalty(matrix: &QrMatrix) -> u32 {
    let mut penalty = 0u32;
    let n = matrix.size;

    // Rule 1: Consecutive same-color modules in rows/columns (>= 5)
    for row in 0..n {
        let mut run = 1;
        for col in 1..n {
            if matrix.modules[row][col] == matrix.modules[row][col - 1] {
                run += 1;
            } else {
                if run >= 5 {
                    penalty += (run - 2) as u32;
                }
                run = 1;
            }
        }
        if run >= 5 {
            penalty += (run - 2) as u32;
        }
    }

    for col in 0..n {
        let mut run = 1;
        for row in 1..n {
            if matrix.modules[row][col] == matrix.modules[row - 1][col] {
                run += 1;
            } else {
                if run >= 5 {
                    penalty += (run - 2) as u32;
                }
                run = 1;
            }
        }
        if run >= 5 {
            penalty += (run - 2) as u32;
        }
    }

    // Rule 2: 2x2 same-color blocks
    for row in 0..n - 1 {
        for col in 0..n - 1 {
            let c = matrix.modules[row][col];
            if c == matrix.modules[row][col + 1]
                && c == matrix.modules[row + 1][col]
                && c == matrix.modules[row + 1][col + 1]
            {
                penalty += 3;
            }
        }
    }

    // Rule 3: Finder-like patterns (1011101 0000 or 0000 1011101)
    let pattern_a: [bool; 11] = [
        true, false, true, true, true, false, true, false, false, false, false,
    ];
    let pattern_b: [bool; 11] = [
        false, false, false, false, true, false, true, true, true, false, true,
    ];

    for row in 0..n {
        for col in 0..n.saturating_sub(10) {
            let slice: Vec<bool> = (0..11).map(|i| matrix.modules[row][col + i]).collect();
            if slice == pattern_a || slice == pattern_b {
                penalty += 40;
            }
        }
    }

    for col in 0..n {
        for row in 0..n.saturating_sub(10) {
            let slice: Vec<bool> = (0..11).map(|i| matrix.modules[row + i][col]).collect();
            if slice == pattern_a || slice == pattern_b {
                penalty += 40;
            }
        }
    }

    // Rule 4: Proportion of dark modules
    let total = (n * n) as u32;
    let dark: u32 = matrix
        .modules
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&m| m)
        .count() as u32;
    let percent = (dark * 100) / total;
    let prev_five = (percent / 5) * 5;
    let next_five = prev_five + 5;
    let dev_prev = if prev_five > 50 {
        prev_five - 50
    } else {
        50 - prev_five
    };
    let dev_next = if next_five > 50 {
        next_five - 50
    } else {
        50 - next_five
    };
    penalty += (dev_prev / 5).min(dev_next / 5) * 10;

    penalty
}

// ─── Format Information ─────────────────────────────────────────────────────

fn encode_format_info(ec: EcLevel, mask: u8) -> u16 {
    let data = ((ec.format_bits() as u16) << 3) | (mask as u16);
    let mut remainder = data << 10;

    // Divide by generator polynomial 0x537 (10100110111)
    for i in (0..5).rev() {
        if remainder & (1 << (i + 10)) != 0 {
            remainder ^= 0x537 << i;
        }
    }

    let format = (data << 10) | remainder;
    format ^ 0x5412 // XOR with mask pattern
}

fn place_format_info(matrix: &mut QrMatrix, ec: EcLevel, mask: u8) {
    let info = encode_format_info(ec, mask);
    let n = matrix.size;

    // Place format bits around top-left finder
    let positions_a: [(usize, usize); 15] = [
        (0, 8),
        (1, 8),
        (2, 8),
        (3, 8),
        (4, 8),
        (5, 8),
        (7, 8),
        (8, 8),
        (8, 7),
        (8, 5),
        (8, 4),
        (8, 3),
        (8, 2),
        (8, 1),
        (8, 0),
    ];

    for (i, &(r, c)) in positions_a.iter().enumerate() {
        let dark = (info >> i) & 1 != 0;
        matrix.modules[r][c] = dark;
    }

    // Place format bits in secondary positions
    let positions_b: [(usize, usize); 15] = [
        (8, n - 1),
        (8, n - 2),
        (8, n - 3),
        (8, n - 4),
        (8, n - 5),
        (8, n - 6),
        (8, n - 7),
        (8, n - 8),
        (n - 7, 8),
        (n - 6, 8),
        (n - 5, 8),
        (n - 4, 8),
        (n - 3, 8),
        (n - 2, 8),
        (n - 1, 8),
    ];

    for (i, &(r, c)) in positions_b.iter().enumerate() {
        let dark = (info >> i) & 1 != 0;
        matrix.modules[r][c] = dark;
    }
}

// ─── Public API ─────────────────────────────────────────────────────────────

/// Generate a QR code matrix from text.
///
/// Returns None if the text is too long for Version 1-10.
pub fn encode(text: &str, ec: EcLevel) -> Option<QrMatrix> {
    if text.is_empty() {
        return None;
    }

    let mode = detect_mode(text);
    let version = select_version(text, mode, ec)?;
    let size = matrix_size(version);

    // Step 1: Encode data into codeword stream
    let data = encode_data(text, mode, version, ec);

    // Step 2: Generate EC and interleave blocks
    let final_data = interleave_blocks(&data, version, ec);

    // Step 3: Convert to bit stream
    let data_bits: Vec<bool> = final_data
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |i| (byte >> i) & 1 != 0))
        .collect();

    // Step 4: Build base matrix with function patterns
    let mut matrix = QrMatrix::new(size);

    // Finder patterns
    matrix.place_finder(0, 0);
    matrix.place_finder(0, size as isize - 7);
    matrix.place_finder(size as isize - 7, 0);

    // Alignment patterns
    let positions = ALIGNMENT_POSITIONS[(version - 1) as usize];
    for &r in positions {
        for &c in positions {
            // Skip if overlapping with finder patterns
            if (r <= 8 && c <= 8)
                || (r <= 8 && c >= size - 8)
                || (r >= size - 8 && c <= 8)
            {
                continue;
            }
            matrix.place_alignment(r, c);
        }
    }

    // Timing patterns
    matrix.place_timing();

    // Reserve format info areas
    matrix.reserve_format_info();

    // Step 5: Place data
    matrix.place_data(&data_bits);

    // Step 6: Try all 8 masks, pick lowest penalty
    let mut best_mask = 0u8;
    let mut best_penalty = u32::MAX;

    for mask_id in 0..8u8 {
        let mut candidate = matrix.clone();
        apply_mask(&mut candidate, mask_id);
        place_format_info(&mut candidate, ec, mask_id);
        let penalty = evaluate_penalty(&candidate);
        if penalty < best_penalty {
            best_penalty = penalty;
            best_mask = mask_id;
        }
    }

    // Step 7: Apply best mask and format info
    apply_mask(&mut matrix, best_mask);
    place_format_info(&mut matrix, ec, best_mask);

    Some(matrix)
}
