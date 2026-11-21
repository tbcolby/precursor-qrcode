//! Tests (compiled only under #[cfg(test)], see lib.rs).
//!
//! The differential tests rebuild every symbol with the independent `qrcode`
//! crate (0.12, dev-dependency only) at the same version, EC level and mask,
//! and require identical data codewords and identical module matrices.

extern crate std;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::encode::*;
use crate::model::*;

// ─── Differential tests against the `qrcode` crate ─────────────────────────

fn ref_ec(ec: EcLevel) -> qrcode::EcLevel {
    match ec {
        EcLevel::L => qrcode::EcLevel::L,
        EcLevel::M => qrcode::EcLevel::M,
        EcLevel::Q => qrcode::EcLevel::Q,
        EcLevel::H => qrcode::EcLevel::H,
    }
}

fn ref_mask(m: u8) -> qrcode::canvas::MaskPattern {
    use qrcode::canvas::MaskPattern::*;
    [Checkerboard, HorizontalLines, VerticalLines, DiagonalLines, LargeCheckerboard, Fields, Diamonds, Meadow]
        [m as usize]
}

/// Reference data codewords for a single-mode segment.
fn ref_data(text: &str, mode: Mode, version: u8, ec: EcLevel) -> Vec<u8> {
    let mut b = qrcode::bits::Bits::new(qrcode::Version::Normal(version as i16));
    match mode {
        Mode::Numeric => b.push_numeric_data(text.as_bytes()).unwrap(),
        Mode::Alphanumeric => b.push_alphanumeric_data(text.as_bytes()).unwrap(),
        Mode::Byte => b.push_byte_data(text.as_bytes()).unwrap(),
    }
    b.push_terminator(ref_ec(ec)).unwrap();
    b.into_bytes()
}

/// Reference module matrix (row-major, true = dark).
fn ref_matrix(raw: &[u8], version: u8, ec: EcLevel, mask: u8) -> Vec<bool> {
    let v = qrcode::Version::Normal(version as i16);
    let (data, ecc) = qrcode::ec::construct_codewords(raw, v, ref_ec(ec)).unwrap();
    let mut canvas = qrcode::canvas::Canvas::new(v, ref_ec(ec));
    canvas.draw_all_functional_patterns();
    canvas.draw_data(&data, &ecc);
    canvas.apply_mask(ref_mask(mask));
    canvas.into_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect()
}

/// A text of `n` units that needs exactly `mode`.
fn sample(mode: Mode, n: usize, seed: usize) -> String {
    const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
    (0..n)
        .map(|i| {
            let k = (i * 7 + seed * 13 + i * i) % 1000;
            match mode {
                Mode::Numeric => (b'0' + (k % 10) as u8) as char,
                Mode::Alphanumeric => ALNUM[k % ALNUM.len()] as char,
                // lowercase guarantees byte mode
                Mode::Byte => {
                    if i == 0 {
                        'a'
                    } else {
                        (b' ' + (k % 95) as u8) as char
                    }
                }
            }
        })
        .collect()
}

fn check_against_reference(text: &str, ec: EcLevel, version: u8) {
    let mode = detect_mode(text);
    let ours_data = encode_data(text, mode, version, ec);
    let raw = ref_data(text, mode, version, ec);
    assert_eq!(ours_data, raw, "data codewords v{version} {ec:?} {mode:?} len {}", text.len());
    for mask in 0..8 {
        let ours = encode_with(text, ec, Some(version), Some(mask)).unwrap();
        assert_eq!(ours.version, version);
        assert_eq!(
            ours.modules,
            ref_matrix(&raw, version, ec, mask),
            "matrix v{version} {ec:?} mask {mask} {mode:?} len {}",
            text.len()
        );
    }
}

#[test]
fn matches_reference_for_every_version_level_and_mask() {
    // Regression: versions 7-10 lacked the version-information blocks and
    // differed from the reference under every mask.
    for version in MIN_VERSION..=MAX_VERSION {
        for ec in EcLevel::ALL {
            for (seed, mode) in [Mode::Numeric, Mode::Alphanumeric, Mode::Byte].into_iter().enumerate() {
                let cap = capacity(version, ec, mode);
                // full symbol, and a short text forced into this version
                check_against_reference(&sample(mode, cap, seed), ec, version);
                check_against_reference(&sample(mode, 1 + cap / 3, seed + 5), ec, version);
            }
        }
    }
}

#[test]
fn reference_codewords_include_ec_interleaving() {
    for version in MIN_VERSION..=MAX_VERSION {
        for ec in EcLevel::ALL {
            let text = sample(Mode::Byte, capacity(version, ec, Mode::Byte), version as usize);
            let raw = encode_data(&text, Mode::Byte, version, ec);
            let (d, e) =
                qrcode::ec::construct_codewords(&raw, qrcode::Version::Normal(version as i16), ref_ec(ec)).unwrap();
            let mut want = d.clone();
            want.extend_from_slice(&e);
            assert_eq!(interleave(&raw, version, ec), want, "v{version} {ec:?}");
            assert_eq!(want.len(), total_codewords(version, ec));
        }
    }
}

// ─── Known values from ISO/IEC 18004 ────────────────────────────────────────

#[test]
fn version_information_values() {
    // ISO/IEC 18004 Annex D, Table D.1
    assert_eq!(version_info(7), 0x07C94);
    assert_eq!(version_info(8), 0x085BC);
    assert_eq!(version_info(9), 0x09A99);
    assert_eq!(version_info(10), 0x0A4D3);
}

#[test]
fn version_information_is_placed_for_v7_plus() {
    for version in 7..=MAX_VERSION {
        let q = encode_with(&sample(Mode::Byte, 10, 1), EcLevel::M, Some(version), Some(0)).unwrap();
        let bits = version_info(version);
        let n = q.size;
        for i in 0..18 {
            let want = bits >> i & 1 == 1;
            assert_eq!(q.get(i / 3, n - 11 + i % 3), want, "v{version} top-right bit {i}");
            assert_eq!(q.get(n - 11 + i % 3, i / 3), want, "v{version} bottom-left bit {i}");
        }
    }
}

#[test]
fn format_information_values() {
    // ISO/IEC 18004 Annex C, Table C.1
    assert_eq!(format_info(EcLevel::M, 0), 0x5412);
    assert_eq!(format_info(EcLevel::L, 0), 0x77C4);
    assert_eq!(format_info(EcLevel::L, 7), 0x6976);
    assert_eq!(format_info(EcLevel::H, 7), 0x083B);
    assert_eq!(format_info(EcLevel::Q, 3), 0x3A06);
}

#[test]
fn reed_solomon_hello_world_1m() {
    // "HELLO WORLD" at 1-M, the worked example in ISO/IEC 18004 Annex I.
    let data = encode_data("HELLO WORLD", Mode::Alphanumeric, 1, EcLevel::M);
    assert_eq!(data, vec![32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17]);
    assert_eq!(rs_ec(&data, 10), vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
}

#[test]
fn gf_tables() {
    assert_eq!(gf_mul(0, 7), 0);
    assert_eq!(gf_mul(1, 0x53), 0x53);
    assert_eq!(gf_mul(2, 0x80), 0x1D); // reduction by 0x11D
    for a in 1..=255u8 {
        for b in [1u8, 2, 3, 0x53, 0xCA, 0xFF] {
            assert_eq!(gf_mul(a, b), gf_mul(b, a));
        }
    }
}

#[test]
fn computed_capacity_matches_iso_table() {
    // ISO/IEC 18004 Table 7 (numeric, alphanumeric, byte) for L, M, Q, H.
    const TABLE: [[[usize; 3]; 4]; 10] = [
        [[41, 25, 17], [34, 20, 14], [27, 16, 11], [17, 10, 7]],
        [[77, 47, 32], [63, 38, 26], [48, 29, 20], [34, 20, 14]],
        [[127, 77, 53], [101, 61, 42], [77, 47, 32], [58, 35, 24]],
        [[187, 114, 78], [149, 90, 62], [111, 67, 46], [82, 50, 34]],
        [[255, 154, 106], [202, 122, 84], [144, 87, 60], [106, 64, 44]],
        [[322, 195, 134], [255, 154, 106], [178, 108, 74], [139, 84, 58]],
        [[370, 224, 154], [293, 178, 122], [207, 125, 86], [154, 93, 64]],
        [[461, 279, 192], [365, 221, 152], [259, 157, 108], [202, 122, 84]],
        [[552, 335, 230], [432, 262, 180], [312, 189, 130], [235, 143, 98]],
        [[652, 395, 271], [513, 311, 213], [364, 221, 151], [288, 174, 119]],
    ];
    for v in 1..=10u8 {
        for (e, ec) in EcLevel::ALL.into_iter().enumerate() {
            for (m, mode) in [Mode::Numeric, Mode::Alphanumeric, Mode::Byte].into_iter().enumerate() {
                assert_eq!(capacity(v, ec, mode), TABLE[v as usize - 1][e][m], "v{v} {ec:?} {mode:?}");
            }
        }
    }
}

// ─── Encoder behaviour ──────────────────────────────────────────────────────

#[test]
fn picks_smallest_version() {
    assert_eq!(encode("HELLO WORLD", EcLevel::M).unwrap().version, 1);
    // 135 bytes at L overflows V6 (134) and lands in V7.
    let text = sample(Mode::Byte, 135, 3);
    let q = encode(&text, EcLevel::L).unwrap();
    assert_eq!((q.version, q.size), (7, 45));
    check_against_reference(&text, EcLevel::L, 7);
}

#[test]
fn wifi_with_long_passphrase_uses_v7_plus_and_matches_reference() {
    // The audit's example: ~90 bytes at EC-Q/H landed in V7+ and didn't decode.
    let pass: String = sample(Mode::Byte, 63, 9).replace([';', ',', ':', '\\', '"'], "x");
    let text = std::format!("WIFI:T:WPA;S:HomeNet;P:{pass};;");
    for ec in [EcLevel::Q, EcLevel::H] {
        let q = encode(&text, ec).unwrap();
        assert!(q.version >= 7, "{ec:?} -> v{}", q.version);
        check_against_reference(&text, ec, q.version);
    }
}

#[test]
fn best_mask_has_lowest_penalty() {
    for text in ["HELLO WORLD", "https://betrusted.io", "0123456789012345678901234567890"] {
        let q = encode(text, EcLevel::M).unwrap();
        let p = |m| {
            let c = encode_with(text, EcLevel::M, Some(q.version), Some(m)).unwrap();
            penalty(c.size, &c.modules)
        };
        let best = p(q.mask);
        assert!((0..8).all(|m| p(m) >= best), "{text}");
    }
}

#[test]
fn too_long_and_empty() {
    assert_eq!(encode("", EcLevel::L), Err(QrError::Empty));
    let text = sample(Mode::Byte, 120, 0);
    let e = encode(&text, EcLevel::H).unwrap_err();
    assert_eq!(e, QrError::TooLong { len: 120, max: 119, mode: Mode::Byte, ec: EcLevel::H });
    assert_eq!(e.to_string(), "Too long: 120 bytes, EC-H holds 119");
    assert!(encode(&sample(Mode::Byte, 119, 0), EcLevel::H).is_ok());
    // forcing a version that's too small is refused
    assert!(encode_with("HELLO WORLD HELLO WORLD", EcLevel::H, Some(1), None).is_err());
}

#[test]
fn mode_detection() {
    assert_eq!(detect_mode("0123"), Mode::Numeric);
    assert_eq!(detect_mode("HELLO WORLD:$%"), Mode::Alphanumeric);
    assert_eq!(detect_mode("hello"), Mode::Byte);
    assert_eq!(detect_mode("A\nB"), Mode::Byte);
}

#[test]
fn module_size_fits_canvas() {
    // Regression: a manual 8 px module at V6 was 392 px wide and got clipped.
    let v6 = symbol_size(6); // 41 modules + 8 quiet = 49
    assert_eq!(module_px(v6, 330, 8), Some(6));
    assert_eq!(module_px(v6, 330, 3), Some(3));
    assert_eq!(module_px(v6, 330, 0), Some(6));
    assert_eq!(module_px(symbol_size(10), 330, 0), Some(5));
    assert_eq!(module_px(symbol_size(10), 60, 0), None);
}

#[test]
fn row_runs_cover_dark_modules() {
    let q = encode("RUNS", EcLevel::L).unwrap();
    for r in 0..q.size {
        let mut row = vec![false; q.size];
        for (s, l) in q.row_runs(r) {
            row[s..s + l].iter_mut().for_each(|m| *m = true);
        }
        assert_eq!(row, q.modules[r * q.size..(r + 1) * q.size]);
    }
    // a finder row is one run of 7, gap, ..., one run of 7
    assert_eq!(q.row_runs(0).first(), Some(&(0, 7)));
}

#[test]
fn qrcode_debug_hides_modules() {
    let q = encode("secret", EcLevel::L).unwrap();
    let d = std::format!("{q:?}");
    assert!(d.contains("version") && !d.contains("true"));
}

// ─── Model ──────────────────────────────────────────────────────────────────

#[test]
fn settings_legacy_record_and_clamp() {
    let s: Settings = serde_json::from_str(r#"{"ec_level":"H","module_size":200}"#).unwrap();
    assert_eq!(s.version, 0);
    assert!(s.hide_on_background);
    let s = s.sanitized();
    assert_eq!((s.ec_level, s.module_size, s.version), (EcLevel::H, MAX_MODULE_PX, 1));
    let s: Settings = serde_json::from_str(r#"{"ec_level":"L","module_size":1}"#).unwrap();
    assert_eq!(s.sanitized().module_size, MIN_MODULE_PX);
    let s = Settings::default();
    assert_eq!(serde_json::from_str::<Settings>(&serde_json::to_string(&s).unwrap()).unwrap(), s);
}

#[test]
fn module_size_steps() {
    let mut s = Settings::default();
    s.bigger();
    assert_eq!(s.module_size, 2);
    for _ in 0..10 {
        s.bigger();
    }
    assert_eq!(s.module_size, 8);
    for _ in 0..10 {
        s.smaller();
    }
    assert_eq!(s.module_size, 0);
    assert_eq!(s.module_label(), "auto");
}

#[test]
fn duplicate_names_rejected() {
    // Regression: saving "wifi" twice made both entries load the second text.
    let mut c = Codes::new();
    c.add("wifi", "one", EcLevel::M).unwrap();
    assert_eq!(c.add(" WIFI", "two", EcLevel::M), Err(SaveError::Duplicate));
    assert_eq!(c.add("", "x", EcLevel::M), Err(SaveError::EmptyName));
    assert_eq!(c.get(0).unwrap().text, "one");
}

#[test]
fn delete_removes_payload_from_record() {
    // Regression: delete rewrote the index but left `code.<name>` in the PDDB.
    let mut c = Codes::new();
    c.add("a", "WIFI:S:x;P:SECRET;;", EcLevel::Q).unwrap();
    c.add("b", "B", EcLevel::L).unwrap();
    let removed = c.remove(0).unwrap();
    assert!(!serde_json::to_string(&c).unwrap().contains("SECRET"));
    c.restore(0, removed);
    assert_eq!(c.get(0).unwrap().name, "a");
}

#[test]
fn saved_code_keeps_its_own_ec_level() {
    // Regression: loading a code overwrote the global EC setting.
    let mut c = Codes::new();
    c.add("h", "x", EcLevel::H).unwrap();
    let json = serde_json::to_string(&c).unwrap();
    let back: Codes = serde_json::from_str(&json).unwrap();
    assert_eq!(back.get(0).unwrap().ec_level, EcLevel::H);
}

#[test]
fn debug_redacts_payload() {
    let mut c = Codes::new();
    c.add("pw", "hunter2", EcLevel::M).unwrap();
    assert!(!std::format!("{c:?}").contains("hunter2"));
}

#[test]
fn legacy_layout_migrates() {
    let idx: IndexRecord = serde_json::from_str(r#"["wifi","url"]"#).unwrap();
    assert_eq!(idx, IndexRecord::Legacy(vec!["wifi".to_string(), "url".to_string()]));
    let p: LegacyCode = serde_json::from_str(r#"{"text":"https://x","ec_level":"Q"}"#).unwrap();
    let codes = migrate_legacy(vec![("url".into(), Some(p)), ("wifi".into(), None)]);
    assert_eq!(codes.len(), 1);
    assert_eq!(codes.get(0).unwrap().ec_level, EcLevel::Q);
    assert_eq!(legacy_code_key("url"), "code.url");
    assert!(is_legacy_code_key("code.orphan") && !is_legacy_code_key("index"));
    let cur: IndexRecord = serde_json::from_str(&serde_json::to_string(&codes).unwrap()).unwrap();
    assert_eq!(cur, IndexRecord::Current(codes));
    assert_eq!(Codes { version: 9, codes: vec![] }.check_version(), Err(9));
}

#[test]
fn templates_put_cursor_in_first_field() {
    let mut f = TextField::new(max_text_len());
    let (t, cur) = Template::Wifi.text_and_cursor();
    f.set(t, cur);
    for c in "MyNet".chars() {
        f.insert(c);
    }
    assert_eq!(f.as_str(), "WIFI:T:WPA;S:MyNet;P:;;");
    let (t, cur) = Template::VCard.text_and_cursor();
    f.set(t, cur);
    for c in "Doe".chars() {
        f.insert(c);
    }
    assert_eq!(f.as_str(), "BEGIN:VCARD\nVERSION:3.0\nN:Doe;;\nEND:VCARD");
    let (t, cur) = Template::Url.text_and_cursor();
    f.set(t, cur);
    assert_eq!(f.cursor(), 8);
}

#[test]
fn text_field_editing() {
    let mut f = TextField::new(4);
    assert!(f.insert('a') && f.insert('c'));
    f.left();
    assert!(f.insert('b'));
    assert_eq!(f.with_cursor(), "ab|c");
    f.right();
    f.right();
    assert!(f.insert('d'));
    assert!(!f.insert('e')); // limit
    assert!(!f.insert('\u{2190}'));
    assert!(f.backspace());
    assert_eq!(f.as_str(), "abc");
    f.home();
    assert_eq!(f.with_cursor(), "|abc");
    f.end();
    assert_eq!(f.cursor(), 3);
    f.set("", 9);
    assert!(!f.backspace());
}

#[test]
fn input_limit_and_usage() {
    assert_eq!(max_text_len(), 271);
    assert_eq!(usage("HELLO", EcLevel::H), (5, 174));
    assert_eq!(usage("hello", EcLevel::H), (5, 119));
}
