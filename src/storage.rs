//! PDDB storage for QR Code Generator.
//!
//! Dictionaries:
//!   - qrcode.settings: App configuration (EC level, module size)
//!   - qrcode.codes: Saved QR code collection

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::app::{QrSettings, SavedCode};
use crate::qr_encode::EcLevel;

const DICT_SETTINGS: &str = "qrcode.settings";
const DICT_CODES: &str = "qrcode.codes";
const KEY_CONFIG: &str = "config";
const KEY_INDEX: &str = "index";

pub struct Storage {
    pddb: pddb::Pddb,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    pub fn load_settings(&mut self) -> Option<QrSettings> {
        let mut key = self
            .pddb
            .get(DICT_SETTINGS, KEY_CONFIG, None, false, false, None, None::<fn()>)
            .ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        key.read_to_end(&mut buf).ok()?;
        let json: serde_json::Value = serde_json::from_slice(&buf).ok()?;

        let ec = match json.get("ec_level").and_then(|v| v.as_str()) {
            Some("L") => EcLevel::L,
            Some("Q") => EcLevel::Q,
            Some("H") => EcLevel::H,
            _ => EcLevel::M,
        };
        let module_size = json
            .get("module_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;

        Some(QrSettings {
            ec_level: ec,
            module_size,
        })
    }

    pub fn save_settings(&mut self, settings: &QrSettings) {
        let json = serde_json::json!({
            "ec_level": settings.ec_level.label(),
            "module_size": settings.module_size,
        });
        let data = serde_json::to_vec(&json).unwrap_or_default();

        if let Ok(mut key) = self.pddb.get(
            DICT_SETTINGS,
            KEY_CONFIG,
            None,
            true,
            true,
            Some(data.len()),
            None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            key.seek(std::io::SeekFrom::Start(0)).ok();
            key.write_all(&data).ok();
            key.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    pub fn load_codes(&mut self) -> Vec<SavedCode> {
        let mut codes = Vec::new();

        // Load index
        let names: Vec<String> = match self.pddb.get(
            DICT_CODES,
            KEY_INDEX,
            None,
            false,
            false,
            None,
            None::<fn()>,
        ) {
            Ok(mut key) => {
                let mut buf = Vec::new();
                use std::io::Read;
                if key.read_to_end(&mut buf).is_ok() {
                    serde_json::from_slice(&buf).unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };

        // Load each code
        for name in &names {
            let key_name = alloc::format!("code.{}", name);
            if let Ok(mut key) = self.pddb.get(
                DICT_CODES,
                &key_name,
                None,
                false,
                false,
                None,
                None::<fn()>,
            ) {
                let mut buf = Vec::new();
                use std::io::Read;
                if key.read_to_end(&mut buf).is_ok() {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&buf) {
                        let text = json
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let ec = match json.get("ec_level").and_then(|v| v.as_str()) {
                            Some("L") => EcLevel::L,
                            Some("Q") => EcLevel::Q,
                            Some("H") => EcLevel::H,
                            _ => EcLevel::M,
                        };
                        codes.push(SavedCode {
                            name: name.clone(),
                            text: String::from(text),
                            ec_level: ec,
                        });
                    }
                }
            }
        }

        codes
    }

    pub fn save_codes(&mut self, codes: &[SavedCode]) {
        // Save index
        let names: Vec<&str> = codes.iter().map(|c| c.name.as_str()).collect();
        let index_data = serde_json::to_vec(&names).unwrap_or_default();

        if let Ok(mut key) = self.pddb.get(
            DICT_CODES,
            KEY_INDEX,
            None,
            true,
            true,
            Some(index_data.len()),
            None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            key.seek(std::io::SeekFrom::Start(0)).ok();
            key.write_all(&index_data).ok();
            key.set_len(index_data.len() as u64).ok();
        }

        // Save each code
        for code in codes {
            let key_name = alloc::format!("code.{}", code.name);
            let json = serde_json::json!({
                "text": code.text,
                "ec_level": code.ec_level.label(),
            });
            let data = serde_json::to_vec(&json).unwrap_or_default();

            if let Ok(mut key) = self.pddb.get(
                DICT_CODES,
                &key_name,
                None,
                true,
                true,
                Some(data.len()),
                None::<fn()>,
            ) {
                use std::io::{Seek, Write};
                key.seek(std::io::SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
                key.set_len(data.len() as u64).ok();
            }
        }

        self.pddb.sync().ok();
    }
}
