//! Settings, saved codes, templates and the text field: what the app
//! persists and edits, with validation and the migration from 0.1.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::encode::{capacity, detect_mode, EcLevel, MAX_VERSION};

/// Bump when the stored format changes.
pub const STATE_VERSION: u32 = 1;
/// 0 = largest that fits; otherwise pixels per module.
pub const MIN_MODULE_PX: u8 = 2;
pub const MAX_MODULE_PX: u8 = 8;
/// Longest name for a saved code.
pub const MAX_NAME_LEN: usize = 24;

/// Longest input the field accepts: byte-mode capacity of version 10 at EC-L.
pub fn max_text_len() -> usize { capacity(MAX_VERSION, EcLevel::L, crate::encode::Mode::Byte) }

/// How much of the capacity `text` uses at `ec`: (used, max) in the units
/// of the mode the text needs. Drives the live "N of M" counter.
pub fn usage(text: &str, ec: EcLevel) -> (usize, usize) {
    let mode = detect_mode(text);
    (crate::encode::mode_len(text, mode), capacity(MAX_VERSION, ec, mode))
}

fn default_ec() -> EcLevel { EcLevel::M }
fn default_true() -> bool { true }

/// User settings. Field names match the 0.1 layout; a missing `version`
/// means 0.1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub version: u32,
    #[serde(default = "default_ec")]
    pub ec_level: EcLevel,
    /// 0 = auto (largest that fits), else 2-8 px, reduced if it won't fit.
    #[serde(default)]
    pub module_size: u8,
    /// Leave the code screen when the app loses focus, so a payload isn't
    /// left on screen to reappear later.
    #[serde(default = "default_true")]
    pub hide_on_background: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: STATE_VERSION, ec_level: default_ec(), module_size: 0, hide_on_background: true }
    }
}

impl Settings {
    /// Clamp values read from storage.
    pub fn sanitized(mut self) -> Self {
        self.version = STATE_VERSION;
        if self.module_size != 0 {
            self.module_size = self.module_size.clamp(MIN_MODULE_PX, MAX_MODULE_PX);
        }
        self
    }

    /// Auto → 2 → 3 … → 8 (stays at 8).
    pub fn bigger(&mut self) {
        self.module_size = match self.module_size {
            0 => MIN_MODULE_PX,
            n => (n + 1).min(MAX_MODULE_PX),
        };
    }

    /// 8 → … → 2 → Auto (stays at Auto).
    pub fn smaller(&mut self) {
        self.module_size = match self.module_size {
            0 | MIN_MODULE_PX => 0,
            n => n - 1,
        };
    }

    pub fn module_label(&self) -> String {
        match self.module_size {
            0 => String::from("auto"),
            n => alloc::format!("{n} px"),
        }
    }
}

/// One saved code. `Debug` hides the payload (WiFi passwords, keys…).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedCode {
    pub name: String,
    pub text: String,
    pub ec_level: EcLevel,
}

impl fmt::Debug for SavedCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SavedCode")
            .field("name", &self.name)
            .field("text", &"<redacted>")
            .field("ec_level", &self.ec_level)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    EmptyName,
    /// Another saved code already has this name (case-insensitive).
    Duplicate,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::EmptyName => write!(f, "Name can't be empty"),
            SaveError::Duplicate => write!(f, "A code with that name exists"),
        }
    }
}

/// All saved codes in one record, so a delete rewrites the record without
/// the payload instead of leaving it in its own key.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Codes {
    pub version: u32,
    pub codes: Vec<SavedCode>,
}

impl Codes {
    pub fn new() -> Self { Self { version: STATE_VERSION, codes: Vec::new() } }

    pub fn len(&self) -> usize { self.codes.len() }

    pub fn is_empty(&self) -> bool { self.codes.is_empty() }

    pub fn get(&self, i: usize) -> Option<&SavedCode> { self.codes.get(i) }

    pub fn contains_name(&self, name: &str) -> bool {
        self.codes.iter().any(|c| c.name.eq_ignore_ascii_case(name.trim()))
    }

    pub fn add(&mut self, name: &str, text: &str, ec_level: EcLevel) -> Result<(), SaveError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(SaveError::EmptyName);
        }
        if self.contains_name(name) {
            return Err(SaveError::Duplicate);
        }
        self.codes.push(SavedCode { name: String::from(name), text: String::from(text), ec_level });
        Ok(())
    }

    pub fn remove(&mut self, i: usize) -> Option<SavedCode> {
        if i < self.codes.len() {
            Some(self.codes.remove(i))
        } else {
            None
        }
    }

    /// Put a code back at `i` (undo a remove whose save failed).
    pub fn restore(&mut self, i: usize, code: SavedCode) { self.codes.insert(i.min(self.codes.len()), code); }

    /// One list line: name, EC level, short single-line preview.
    pub fn label(&self, i: usize) -> String {
        let c = &self.codes[i];
        let flat: String = c.text.chars().map(|ch| if ch == '\n' { ' ' } else { ch }).collect();
        let mut preview: String = flat.chars().take(14).collect();
        if flat.chars().count() > 14 {
            preview.push('…');
        }
        alloc::format!("{} [{}] {}", c.name, c.ec_level.label(), preview)
    }

    pub fn check_version(&self) -> Result<(), u32> {
        if self.version > STATE_VERSION {
            Err(self.version)
        } else {
            Ok(())
        }
    }
}

/// The `index` key holds the current record, or 0.1's list of names (each
/// payload in its own `code.<name>` key).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum IndexRecord {
    Current(Codes),
    Legacy(Vec<String>),
}

/// A 0.1 `code.<name>` record.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct LegacyCode {
    pub text: String,
    #[serde(default = "default_ec")]
    pub ec_level: EcLevel,
}

pub fn legacy_code_key(name: &str) -> String { alloc::format!("code.{name}") }

/// Every key 0.1 used for payloads, including orphans its delete left behind.
pub fn is_legacy_code_key(key: &str) -> bool { key.starts_with("code.") }

/// Build the current record from 0.1 names and loaded payloads. Missing
/// payloads are dropped; duplicate names keep the first.
pub fn migrate_legacy(entries: Vec<(String, Option<LegacyCode>)>) -> Codes {
    let mut codes = Codes::new();
    for (name, payload) in entries {
        if let Some(p) = payload {
            let _ = codes.add(&name, &p.text, p.ec_level);
        }
    }
    codes
}

/// Input templates: text and where the cursor goes (the first field to fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Wifi,
    Url,
    VCard,
}

impl Template {
    pub fn text_and_cursor(self) -> (&'static str, usize) {
        match self {
            // WIFI:T:<auth>;S:<ssid>;P:<password>;;  cursor in the SSID
            Template::Wifi => ("WIFI:T:WPA;S:;P:;;", 13),
            Template::Url => ("https://", 8),
            // cursor after "N:" (family name)
            Template::VCard => ("BEGIN:VCARD\nVERSION:3.0\nN:;;\nEND:VCARD", 26),
        }
    }
}

/// A text field with a cursor. Typing accepts printable ASCII; templates may
/// also contain newlines. The cursor is a byte index (content is ASCII).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    cursor: usize,
    max: usize,
}

impl TextField {
    pub fn new(max: usize) -> Self { Self { text: String::new(), cursor: 0, max } }

    pub fn as_str(&self) -> &str { &self.text }

    pub fn cursor(&self) -> usize { self.cursor }

    pub fn is_empty(&self) -> bool { self.text.is_empty() }

    pub fn len(&self) -> usize { self.text.len() }

    pub fn max(&self) -> usize { self.max }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Replace the content (keeping printable ASCII and newlines, up to the
    /// limit) and put the cursor at `cursor` (clamped).
    pub fn set(&mut self, s: &str, cursor: usize) {
        self.text = s
            .chars()
            .filter(|&c| c.is_ascii_graphic() || c == ' ' || c == '\n')
            .take(self.max)
            .collect();
        self.cursor = cursor.min(self.text.len());
    }

    /// Insert at the cursor. Returns false if refused (limit or character).
    pub fn insert(&mut self, c: char) -> bool {
        if (c.is_ascii_graphic() || c == ' ') && self.text.len() < self.max {
            self.text.insert(self.cursor, c);
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    /// Delete before the cursor. Returns false if nothing was deleted.
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.text.remove(self.cursor);
        true
    }

    pub fn left(&mut self) { self.cursor = self.cursor.saturating_sub(1); }

    pub fn right(&mut self) { self.cursor = (self.cursor + 1).min(self.text.len()); }

    pub fn home(&mut self) { self.cursor = 0; }

    pub fn end(&mut self) { self.cursor = self.text.len(); }

    /// Text with a `|` at the cursor, for display.
    pub fn with_cursor(&self) -> String {
        let mut s = String::with_capacity(self.text.len() + 1);
        s.push_str(&self.text[..self.cursor]);
        s.push('|');
        s.push_str(&self.text[self.cursor..]);
        s
    }
}
