//! QR Code encoder (ISO/IEC 18004): versions 1-10, error correction L/M/Q/H,
//! numeric / alphanumeric / byte mode, all eight masks with penalty scoring.
//!
//! Single-segment encoding only (the whole text in one mode); mixed-mode
//! segmentation would save a few modules and isn't worth the complexity here.

use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

pub const MIN_VERSION: u8 = 1;
pub const MAX_VERSION: u8 = 10;
/// Quiet zone on each side, in modules (ISO/IEC 18004 §6.3.8).
pub const QUIET_ZONE: usize = 4;

/// Error correction level. Serde names match what version 0.1 stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EcLevel {
    L,
    M,
    Q,
    H,
}

impl EcLevel {
    pub const ALL: [EcLevel; 4] = [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H];

    pub fn label(self) -> &'static str {
        match self {
            EcLevel::L => "L",
            EcLevel::M => "M",
            EcLevel::Q => "Q",
            EcLevel::H => "H",
        }
    }

    /// Approximate share of the symbol that can be damaged and still decode.
    pub fn recovery(self) -> &'static str {
        match self {
            EcLevel::L => "7%",
            EcLevel::M => "15%",
            EcLevel::Q => "25%",
            EcLevel::H => "30%",
        }
    }

    pub fn next(self) -> EcLevel {
        match self {
            EcLevel::L => EcLevel::M,
            EcLevel::M => EcLevel::Q,
            EcLevel::Q => EcLevel::H,
            EcLevel::H => EcLevel::L,
        }
    }

    pub fn prev(self) -> EcLevel {
        match self {
            EcLevel::L => EcLevel::H,
            EcLevel::M => EcLevel::L,
            EcLevel::Q => EcLevel::M,
            EcLevel::H => EcLevel::Q,
        }
    }

    /// The two EC bits of the format information.
    pub fn format_bits(self) -> u16 {
        match self {
            EcLevel::L => 0b01,
            EcLevel::M => 0b00,
            EcLevel::Q => 0b11,
            EcLevel::H => 0b10,
        }
    }

    fn index(self) -> usize {
        match self {
            EcLevel::L => 0,
            EcLevel::M => 1,
            EcLevel::Q => 2,
            EcLevel::H => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Numeric => "numeric",
            Mode::Alphanumeric => "alphanumeric",
            Mode::Byte => "byte",
        }
    }

    fn indicator(self) -> u32 {
        match self {
            Mode::Numeric => 0b0001,
            Mode::Alphanumeric => 0b0010,
            Mode::Byte => 0b0100,
        }
    }

    /// Character-count field width.
    pub fn count_bits(self, version: u8) -> usize {
        let small = version <= 9;
        match self {
            Mode::Numeric => {
                if small {
                    10
                } else {
                    12
                }
            }
            Mode::Alphanumeric => {
                if small {
                    9
                } else {
                    11
                }
            }
            Mode::Byte => {
                if small {
                    8
                } else {
                    16
                }
            }
        }
    }

    /// Bits for `n` characters of data (excluding header).
    pub fn data_bits(self, n: usize) -> usize {
        match self {
            Mode::Numeric => 10 * (n / 3) + [0, 4, 7][n % 3],
            Mode::Alphanumeric => 11 * (n / 2) + 6 * (n % 2),
            Mode::Byte => 8 * n,
        }
    }
}

/// Alphanumeric-mode value of a character.
pub fn alphanumeric_value(c: char) -> Option<u32> {
    Some(match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='Z' => c as u32 - 'A' as u32 + 10,
        ' ' => 36,
        '$' => 37,
        '%' => 38,
        '*' => 39,
        '+' => 40,
        '-' => 41,
        '.' => 42,
        '/' => 43,
        ':' => 44,
        _ => return None,
    })
}

/// The densest single mode that can carry `text`.
pub fn detect_mode(text: &str) -> Mode {
    if text.bytes().all(|b| b.is_ascii_digit()) {
        Mode::Numeric
    } else if text.chars().all(|c| alphanumeric_value(c).is_some()) {
        Mode::Alphanumeric
    } else {
        Mode::Byte
    }
}

/// Length of `text` in `mode`'s units (bytes for byte mode, else characters).
pub fn mode_len(text: &str, mode: Mode) -> usize {
    match mode {
        Mode::Byte => text.len(),
        _ => text.chars().count(),
    }
}

// ─── Version tables (ISO/IEC 18004 Table 9) ────────────────────────────────

/// EC codewords per block, [version-1][L, M, Q, H].
const EC_PER_BLOCK: [[usize; 4]; 10] = [
    [7, 10, 13, 17],
    [10, 16, 22, 28],
    [15, 26, 18, 22],
    [20, 18, 26, 16],
    [26, 24, 18, 22],
    [18, 16, 24, 28],
    [20, 18, 18, 26],
    [24, 22, 22, 26],
    [30, 22, 20, 24],
    [18, 26, 24, 28],
];

/// (group-1 blocks, data codewords each, group-2 blocks, data codewords each).
const BLOCKS: [[(usize, usize, usize, usize); 4]; 10] = [
    [(1, 19, 0, 0), (1, 16, 0, 0), (1, 13, 0, 0), (1, 9, 0, 0)],
    [(1, 34, 0, 0), (1, 28, 0, 0), (1, 22, 0, 0), (1, 16, 0, 0)],
    [(1, 55, 0, 0), (1, 44, 0, 0), (2, 17, 0, 0), (2, 13, 0, 0)],
    [(1, 80, 0, 0), (2, 32, 0, 0), (2, 24, 0, 0), (4, 9, 0, 0)],
    [(1, 108, 0, 0), (2, 43, 0, 0), (2, 15, 2, 16), (2, 11, 2, 12)],
    [(2, 68, 0, 0), (4, 27, 0, 0), (4, 19, 0, 0), (4, 15, 0, 0)],
    [(2, 78, 0, 0), (4, 31, 0, 0), (2, 14, 4, 15), (4, 13, 1, 14)],
    [(2, 97, 0, 0), (2, 38, 2, 39), (4, 18, 2, 19), (4, 14, 2, 15)],
    [(2, 116, 0, 0), (3, 36, 2, 37), (4, 16, 4, 17), (4, 12, 4, 13)],
    [(2, 68, 2, 69), (4, 43, 1, 44), (6, 19, 2, 20), (6, 15, 2, 16)],
];

/// Alignment pattern centre coordinates.
const ALIGNMENT: [&[usize]; 10] = [
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
];

/// Modules per side for a version.
pub fn symbol_size(version: u8) -> usize { version as usize * 4 + 17 }

/// Data codewords (before EC) for a version and level.
pub fn data_codewords(version: u8, ec: EcLevel) -> usize {
    let (b1, d1, b2, d2) = BLOCKS[version as usize - 1][ec.index()];
    b1 * d1 + b2 * d2
}

/// Total codewords (data + EC) for a version: the same for every EC level.
pub fn total_codewords(version: u8, ec: EcLevel) -> usize {
    let (b1, _, b2, _) = BLOCKS[version as usize - 1][ec.index()];
    data_codewords(version, ec) + (b1 + b2) * EC_PER_BLOCK[version as usize - 1][ec.index()]
}

/// Bits a text of `n` units needs in `mode` at `version`, header included.
pub fn bits_needed(mode: Mode, n: usize, version: u8) -> usize { 4 + mode.count_bits(version) + mode.data_bits(n) }

/// Longest text (in `mode` units) that fits in `version` at `ec`.
pub fn capacity(version: u8, ec: EcLevel, mode: Mode) -> usize {
    let avail = data_codewords(version, ec) * 8;
    let max_count = (1usize << mode.count_bits(version)) - 1;
    let mut n = 0;
    while n < max_count && bits_needed(mode, n + 1, version) <= avail {
        n += 1;
    }
    n
}

/// Smallest version that holds `n` units in `mode` at `ec`.
pub fn select_version(mode: Mode, n: usize, ec: EcLevel) -> Option<u8> {
    (MIN_VERSION..=MAX_VERSION).find(|&v| n <= capacity(v, ec, mode))
}

// ─── GF(256) and Reed-Solomon ───────────────────────────────────────────────

const fn build_exp() -> [u8; 512] {
    let mut t = [0u8; 512];
    let mut v: u16 = 1;
    let mut i = 0;
    while i < 255 {
        t[i] = v as u8;
        t[i + 255] = v as u8;
        v <<= 1;
        if v & 0x100 != 0 {
            v ^= 0x11D;
        }
        i += 1;
    }
    t
}

const fn build_log(exp: &[u8; 512]) -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 255 {
        t[exp[i] as usize] = i as u8;
        i += 1;
    }
    t
}

/// α^i in GF(256) with the QR polynomial x^8+x^4+x^3+x^2+1, built at compile time.
static GF_EXP: [u8; 512] = build_exp();
static GF_LOG: [u8; 256] = build_log(&GF_EXP);

pub fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        GF_EXP[GF_LOG[a as usize] as usize + GF_LOG[b as usize] as usize]
    }
}

/// Generator polynomial ∏(x - α^i), i < n, highest coefficient first.
fn rs_generator(n: usize) -> Vec<u8> {
    let mut g = vec![1u8];
    for &root in GF_EXP.iter().take(n) {
        let mut next = vec![0u8; g.len() + 1];
        for (j, &c) in g.iter().enumerate() {
            next[j] ^= c;
            next[j + 1] ^= gf_mul(c, root);
        }
        g = next;
    }
    g
}

/// Reed-Solomon EC codewords for one block.
pub fn rs_ec(data: &[u8], n: usize) -> Vec<u8> {
    let gen = rs_generator(n);
    let mut rem = vec![0u8; n];
    for &d in data {
        let factor = d ^ rem[0];
        rem.remove(0);
        rem.push(0);
        for (r, &g) in rem.iter_mut().zip(&gen[1..]) {
            *r ^= gf_mul(g, factor);
        }
    }
    rem
}

// ─── Data encoding ──────────────────────────────────────────────────────────

struct Bits(Vec<u8>, usize);

impl Bits {
    fn push(&mut self, value: u32, n: usize) {
        for i in (0..n).rev() {
            if self.1 % 8 == 0 {
                self.0.push(0);
            }
            if value >> i & 1 == 1 {
                *self.0.last_mut().unwrap() |= 0x80 >> (self.1 % 8);
            }
            self.1 += 1;
        }
    }
}

/// The data codewords (header, data, terminator, padding) for `text` in a
/// single `mode` segment. Caller guarantees it fits.
pub fn encode_data(text: &str, mode: Mode, version: u8, ec: EcLevel) -> Vec<u8> {
    let cap_bits = data_codewords(version, ec) * 8;
    let mut b = Bits(Vec::with_capacity(cap_bits / 8), 0);
    b.push(mode.indicator(), 4);
    b.push(mode_len(text, mode) as u32, mode.count_bits(version));
    match mode {
        Mode::Numeric => {
            for chunk in text.as_bytes().chunks(3) {
                let v = chunk.iter().fold(0u32, |a, &d| a * 10 + (d - b'0') as u32);
                b.push(v, [0, 4, 7, 10][chunk.len()]);
            }
        }
        Mode::Alphanumeric => {
            let vals: Vec<u32> = text.chars().filter_map(alphanumeric_value).collect();
            for pair in vals.chunks(2) {
                if pair.len() == 2 {
                    b.push(pair[0] * 45 + pair[1], 11);
                } else {
                    b.push(pair[0], 6);
                }
            }
        }
        Mode::Byte => {
            for &byte in text.as_bytes() {
                b.push(byte as u32, 8);
            }
        }
    }
    let term = (cap_bits - b.1).min(4);
    b.push(0, term);
    let pad = (8 - b.1 % 8) % 8;
    b.push(0, pad);
    let mut fill = [0xEC, 0x11].iter().cycle();
    while b.0.len() < cap_bits / 8 {
        b.0.push(*fill.next().unwrap());
    }
    b.0
}

/// Split data into blocks, add EC, interleave: the final codeword sequence.
pub fn interleave(data: &[u8], version: u8, ec: EcLevel) -> Vec<u8> {
    let (b1, d1, b2, d2) = BLOCKS[version as usize - 1][ec.index()];
    let n_ec = EC_PER_BLOCK[version as usize - 1][ec.index()];
    let mut blocks: Vec<&[u8]> = Vec::with_capacity(b1 + b2);
    let mut off = 0;
    for len in core::iter::repeat_n(d1, b1).chain(core::iter::repeat_n(d2, b2)) {
        blocks.push(&data[off..off + len]);
        off += len;
    }
    let ecs: Vec<Vec<u8>> = blocks.iter().map(|b| rs_ec(b, n_ec)).collect();
    let mut out = Vec::with_capacity(total_codewords(version, ec));
    for i in 0..d1.max(d2) {
        out.extend(blocks.iter().filter_map(|b| b.get(i)));
    }
    for i in 0..n_ec {
        out.extend(ecs.iter().map(|e| e[i]));
    }
    out
}

// ─── Format and version information ─────────────────────────────────────────

/// 15-bit format information: BCH(15,5), generator 0x537, XOR mask 0x5412.
pub fn format_info(ec: EcLevel, mask: u8) -> u16 {
    let data = ec.format_bits() << 3 | mask as u16;
    let mut rem = data << 10;
    for i in (10..15).rev() {
        if rem >> i & 1 == 1 {
            rem ^= 0x537 << (i - 10);
        }
    }
    (data << 10 | rem) ^ 0x5412
}

/// 18-bit version information for versions 7+: BCH(18,6), generator 0x1F25.
pub fn version_info(version: u8) -> u32 {
    let data = version as u32;
    let mut rem = data << 12;
    for i in (12..18).rev() {
        if rem >> i & 1 == 1 {
            rem ^= 0x1F25 << (i - 12);
        }
    }
    data << 12 | rem
}

// ─── Matrix ─────────────────────────────────────────────────────────────────

/// An encoded symbol. `modules` is row-major, `true` = dark. No quiet zone.
#[derive(Clone, PartialEq, Eq)]
pub struct QrCode {
    pub version: u8,
    pub ec: EcLevel,
    pub mask: u8,
    pub mode: Mode,
    pub size: usize,
    pub modules: Vec<bool>,
}

impl fmt::Debug for QrCode {
    /// Omits the modules: they encode the (possibly secret) payload.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrCode")
            .field("version", &self.version)
            .field("ec", &self.ec)
            .field("mask", &self.mask)
            .field("size", &self.size)
            .finish()
    }
}

impl QrCode {
    pub fn get(&self, row: usize, col: usize) -> bool { self.modules[row * self.size + col] }

    /// Dark runs in a row as `(first_col, length)`, for drawing one rectangle
    /// per run rather than per module.
    pub fn row_runs(&self, row: usize) -> Vec<(usize, usize)> {
        let r = &self.modules[row * self.size..(row + 1) * self.size];
        let mut runs = Vec::new();
        let mut c = 0;
        while c < self.size {
            if r[c] {
                let s = c;
                while c < self.size && r[c] {
                    c += 1;
                }
                runs.push((s, c - s));
            } else {
                c += 1;
            }
        }
        runs
    }
}

struct Grid {
    size: usize,
    dark: Vec<bool>,
    function: Vec<bool>,
}

impl Grid {
    fn new(version: u8) -> Self {
        let size = symbol_size(version);
        Self { size, dark: vec![false; size * size], function: vec![false; size * size] }
    }

    fn set_fn(&mut self, row: usize, col: usize, dark: bool) {
        let i = row * self.size + col;
        self.dark[i] = dark;
        self.function[i] = true;
    }

    fn finder(&mut self, row: isize, col: isize) {
        for dr in -1..=7isize {
            for dc in -1..=7isize {
                let (r, c) = (row + dr, col + dc);
                if r < 0 || c < 0 || r >= self.size as isize || c >= self.size as isize {
                    continue;
                }
                let inside = (0..=6).contains(&dr) && (0..=6).contains(&dc);
                let border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                let core = (2..=4).contains(&dr) && (2..=4).contains(&dc);
                let dark = inside && (border || core);
                self.set_fn(r as usize, c as usize, dark);
            }
        }
    }

    fn function_patterns(&mut self, version: u8) {
        let n = self.size as isize;
        self.finder(0, 0);
        self.finder(0, n - 7);
        self.finder(n - 7, 0);
        for i in 8..self.size - 8 {
            self.set_fn(6, i, i % 2 == 0);
            self.set_fn(i, 6, i % 2 == 0);
        }
        let pos = ALIGNMENT[version as usize - 1];
        for &r in pos {
            for &c in pos {
                // the three positions under the finder patterns
                let last = *pos.last().unwrap();
                let corner = [(6, 6), (6, last), (last, 6)].contains(&(r, c));
                if corner {
                    continue;
                }
                for dr in -2..=2isize {
                    for dc in -2..=2isize {
                        let dark = dr.abs() == 2 || dc.abs() == 2 || (dr == 0 && dc == 0);
                        self.set_fn((r as isize + dr) as usize, (c as isize + dc) as usize, dark);
                    }
                }
            }
        }
        // Reserve format areas (written per mask) and the dark module.
        self.format(0);
        self.set_fn(self.size - 8, 8, true);
        if version >= 7 {
            let bits = version_info(version);
            for i in 0..18 {
                let dark = bits >> i & 1 == 1;
                let (a, b) = (i / 3, self.size - 11 + i % 3);
                self.set_fn(a, b, dark); // top right
                self.set_fn(b, a, dark); // bottom left
            }
        }
    }

    /// Write both copies of the 15 format bits.
    fn format(&mut self, bits: u16) {
        let n = self.size;
        let bit = |i: usize| bits >> i & 1 == 1;
        for i in 0..6 {
            self.set_fn(i, 8, bit(i));
        }
        self.set_fn(7, 8, bit(6));
        self.set_fn(8, 8, bit(7));
        self.set_fn(8, 7, bit(8));
        for i in 9..15 {
            self.set_fn(8, 14 - i, bit(i));
        }
        for i in 0..8 {
            self.set_fn(8, n - 1 - i, bit(i));
        }
        for i in 8..15 {
            self.set_fn(n - 15 + i, 8, bit(i));
        }
    }

    /// Place codewords in the two-column zigzag, skipping function modules.
    fn place(&mut self, codewords: &[u8]) {
        let n = self.size;
        let total_bits = codewords.len() * 8;
        let mut k = 0;
        let mut right = n as isize - 1;
        let mut upward = true;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for step in 0..n {
                let row = if upward { n - 1 - step } else { step };
                for dc in 0..2 {
                    let col = right as usize - dc;
                    let i = row * n + col;
                    if !self.function[i] {
                        self.dark[i] = k < total_bits && codewords[k / 8] >> (7 - k % 8) & 1 == 1;
                        k += 1;
                    }
                }
            }
            upward = !upward;
            right -= 2;
        }
    }

    fn apply_mask(&mut self, mask: u8) {
        let n = self.size;
        for r in 0..n {
            for c in 0..n {
                let i = r * n + c;
                if !self.function[i] && mask_bit(mask, r, c) {
                    self.dark[i] = !self.dark[i];
                }
            }
        }
    }
}

/// The eight data-mask conditions (ISO/IEC 18004 Table 10).
pub fn mask_bit(mask: u8, r: usize, c: usize) -> bool {
    match mask {
        0 => (r + c) % 2 == 0,
        1 => r % 2 == 0,
        2 => c % 3 == 0,
        3 => (r + c) % 3 == 0,
        4 => (r / 2 + c / 3) % 2 == 0,
        5 => (r * c) % 2 + (r * c) % 3 == 0,
        6 => ((r * c) % 2 + (r * c) % 3) % 2 == 0,
        _ => ((r + c) % 2 + (r * c) % 3) % 2 == 0,
    }
}

/// Mask penalty score (ISO/IEC 18004 §7.8.3): runs, 2x2 blocks, finder-like
/// patterns, dark/light balance. Lower is better. Allocation-free.
pub fn penalty(size: usize, dark: &[bool]) -> u32 {
    let n = size;
    let at = |r: usize, c: usize| dark[r * n + c];
    let mut score = 0u32;
    // N1: runs of 5+ in rows and columns; N3: 1011101 with 4 light on one side.
    const FINDER: [bool; 7] = [true, false, true, true, true, false, true];
    for line in 0..n {
        for horizontal in [true, false] {
            let get = |i: usize| if horizontal { at(line, i) } else { at(i, line) };
            let mut run = 1;
            for i in 1..n {
                if get(i) == get(i - 1) {
                    run += 1;
                } else {
                    if run >= 5 {
                        score += run - 2;
                    }
                    run = 1;
                }
            }
            if run >= 5 {
                score += run - 2;
            }
            for i in 0..=n.saturating_sub(7) {
                if (0..7).all(|k| get(i + k) == FINDER[k]) {
                    let light = |a: isize, b: isize| (a..b).all(|j| j < 0 || j >= n as isize || !get(j as usize));
                    let i = i as isize;
                    if light(i - 4, i) || light(i + 7, i + 11) {
                        score += 40;
                    }
                }
            }
        }
    }
    // N2: 2x2 blocks of one colour.
    for r in 0..n - 1 {
        for c in 0..n - 1 {
            let v = at(r, c);
            if v == at(r, c + 1) && v == at(r + 1, c) && v == at(r + 1, c + 1) {
                score += 3;
            }
        }
    }
    // N4: 10 points per 5% deviation from 50% dark.
    let total = (n * n) as u32;
    let dark_count = dark.iter().filter(|&&d| d).count() as u32;
    let dev = (dark_count * 20).abs_diff(total * 10); // |dark/total - 1/2| * 20 * total
    score += dev / total * 10;
    score
}

/// Why a text can't be encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrError {
    Empty,
    /// Longer than version 10 holds at this level.
    TooLong { len: usize, max: usize, mode: Mode, ec: EcLevel },
}

impl fmt::Display for QrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QrError::Empty => write!(f, "Nothing to encode"),
            QrError::TooLong { len, max, mode, ec } => {
                let unit = if *mode == Mode::Byte { "bytes" } else { "characters" };
                write!(f, "Too long: {len} {unit}, EC-{} holds {max}", ec.label())
            }
        }
    }
}

/// Encode `text` at `ec`, choosing the smallest version and the best mask.
pub fn encode(text: &str, ec: EcLevel) -> Result<QrCode, QrError> { encode_with(text, ec, None, None) }

/// Encode with an optional forced version (must fit) and mask (0-7).
pub fn encode_with(text: &str, ec: EcLevel, version: Option<u8>, mask: Option<u8>) -> Result<QrCode, QrError> {
    if text.is_empty() {
        return Err(QrError::Empty);
    }
    let mode = detect_mode(text);
    let len = mode_len(text, mode);
    let too_long = || QrError::TooLong { len, max: capacity(MAX_VERSION, ec, mode), mode, ec };
    let version = match version {
        Some(v) if (MIN_VERSION..=MAX_VERSION).contains(&v) && len <= capacity(v, ec, mode) => v,
        Some(_) => return Err(too_long()),
        None => select_version(mode, len, ec).ok_or_else(too_long)?,
    };
    let codewords = interleave(&encode_data(text, mode, version, ec), version, ec);
    let mut grid = Grid::new(version);
    grid.function_patterns(version);
    grid.place(&codewords);

    let pick = |m: u8, g: &Grid| {
        let mut cand = Grid { size: g.size, dark: g.dark.clone(), function: g.function.clone() };
        cand.apply_mask(m);
        cand.format(format_info(ec, m));
        cand
    };
    let (mask, best) = match mask {
        Some(m) => (m & 7, pick(m & 7, &grid)),
        None => (0..8u8)
            .map(|m| (m, pick(m, &grid)))
            .min_by_key(|(_, g)| penalty(g.size, &g.dark))
            .unwrap(),
    };
    Ok(QrCode { version, ec, mask, mode, size: best.size, modules: best.dark })
}

/// Pixels per module so the symbol plus quiet zones fits in `avail_px`
/// (the smaller canvas side). `preferred = 0` means as large as fits.
/// `None` if it doesn't fit at 1 px.
pub fn module_px(size: usize, avail_px: usize, preferred: usize) -> Option<usize> {
    let max = avail_px / (size + 2 * QUIET_ZONE);
    if max == 0 {
        None
    } else if preferred == 0 {
        Some(max)
    } else {
        Some(preferred.min(max))
    }
}
