//! Application state machine and input handling for the QR Code Generator.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::qr_encode::{self, EcLevel, QrMatrix};
use crate::storage::Storage;

// Standard key codes (ecosystem standard)
const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

/// Maximum text length we support (Version 10, EC-L, Byte mode = 271 chars).
const MAX_TEXT_LEN: usize = 271;

/// App states
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    MainMenu,
    Input,
    Display,
    SavePrompt,
    SaveNameEntry,
    LoadList,
    Settings,
    Help,
}

/// Menu items for the main menu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    NewQr,
    SavedCodes,
    Settings,
    Help,
}

impl MenuItem {
    pub fn label(&self) -> &'static str {
        match self {
            MenuItem::NewQr => "New QR Code",
            MenuItem::SavedCodes => "Saved Codes",
            MenuItem::Settings => "Settings",
            MenuItem::Help => "Help",
        }
    }

    pub fn all() -> &'static [MenuItem] {
        &[
            MenuItem::NewQr,
            MenuItem::SavedCodes,
            MenuItem::Settings,
            MenuItem::Help,
        ]
    }
}

/// Settings for the QR encoder
#[derive(Debug, Clone)]
pub struct QrSettings {
    pub ec_level: EcLevel,
    pub module_size: u8, // 0 = auto, 3-6 = fixed
}

impl Default for QrSettings {
    fn default() -> Self {
        Self {
            ec_level: EcLevel::M,
            module_size: 0,
        }
    }
}

/// Saved QR code entry
#[derive(Debug, Clone)]
pub struct SavedCode {
    pub name: String,
    pub text: String,
    pub ec_level: EcLevel,
}

/// The main application state
pub struct QrApp {
    pub state: AppState,
    pub menu_index: usize,
    pub input_text: String,
    pub qr_matrix: Option<QrMatrix>,
    pub qr_text: String, // the text that was encoded
    pub settings: QrSettings,
    pub saved_codes: Vec<SavedCode>,
    pub load_index: usize,
    pub save_name: String,
    pub settings_index: usize,
    pub needs_redraw: bool,
    storage: Option<Storage>,
}

impl QrApp {
    pub fn new() -> Self {
        Self {
            state: AppState::MainMenu,
            menu_index: 0,
            input_text: String::new(),
            qr_matrix: None,
            qr_text: String::new(),
            settings: QrSettings::default(),
            saved_codes: Vec::new(),
            load_index: 0,
            save_name: String::new(),
            settings_index: 0,
            needs_redraw: true,
            storage: None,
        }
    }

    /// Initialize PDDB storage and load saved data.
    pub fn init_storage(&mut self) {
        match Storage::new() {
            Ok(mut s) => {
                // Load settings
                if let Some(settings) = s.load_settings() {
                    self.settings = settings;
                }
                // Load saved codes
                self.saved_codes = s.load_codes();
                self.storage = Some(s);
            }
            Err(e) => {
                log::warn!("Failed to initialize storage: {:?}", e);
            }
        }
    }

    /// Save current settings to PDDB.
    pub fn save_settings(&mut self) {
        if let Some(ref mut s) = self.storage {
            s.save_settings(&self.settings);
        }
    }

    /// Save state when backgrounding or quitting.
    pub fn save_state(&mut self) {
        self.save_settings();
    }

    /// Handle a key press. Returns false if the app should quit.
    pub fn handle_key(&mut self, key: char) -> bool {
        self.needs_redraw = true;

        match self.state {
            AppState::MainMenu => self.handle_menu_key(key),
            AppState::Input => self.handle_input_key(key),
            AppState::Display => self.handle_display_key(key),
            AppState::SavePrompt => self.handle_save_prompt_key(key),
            AppState::SaveNameEntry => self.handle_save_name_key(key),
            AppState::LoadList => self.handle_load_key(key),
            AppState::Settings => self.handle_settings_key(key),
            AppState::Help => self.handle_help_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: char) -> bool {
        let items = MenuItem::all();
        match key {
            KEY_UP => {
                if self.menu_index > 0 {
                    self.menu_index -= 1;
                }
            }
            KEY_DOWN => {
                if self.menu_index < items.len() - 1 {
                    self.menu_index += 1;
                }
            }
            KEY_ENTER | KEY_MENU => {
                match items[self.menu_index] {
                    MenuItem::NewQr => {
                        self.input_text.clear();
                        self.state = AppState::Input;
                    }
                    MenuItem::SavedCodes => {
                        self.load_index = 0;
                        self.state = AppState::LoadList;
                    }
                    MenuItem::Settings => {
                        self.settings_index = 0;
                        self.state = AppState::Settings;
                    }
                    MenuItem::Help => {
                        self.state = AppState::Help;
                    }
                }
            }
            'n' | 'N' => {
                self.input_text.clear();
                self.state = AppState::Input;
            }
            'q' | 'Q' => return false,
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_input_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER => {
                if !self.input_text.is_empty() {
                    self.generate_qr();
                }
            }
            KEY_BACKSPACE => {
                self.input_text.pop();
            }
            'q' | 'Q' => {
                if self.input_text.is_empty() {
                    self.state = AppState::MainMenu;
                } else {
                    // Q in input mode types 'q' if there's text
                    if self.input_text.len() < MAX_TEXT_LEN {
                        self.input_text.push(key);
                    }
                }
            }
            // F4: Clear all input
            '\u{F004}' => {
                self.input_text.clear();
            }
            // F1: WiFi template
            '\u{F001}' => {
                self.input_text = String::from("WIFI:T:WPA;S:;P:;;");
            }
            // F2: URL template
            '\u{F002}' => {
                self.input_text = String::from("https://");
            }
            // F3: vCard template
            '\u{F003}' => {
                self.input_text = String::from("BEGIN:VCARD\nVERSION:3.0\nN:;;\nEND:VCARD");
            }
            _ => {
                // Accept printable ASCII and common symbols
                if key.is_ascii_graphic() || key == ' ' {
                    if self.input_text.len() < MAX_TEXT_LEN {
                        self.input_text.push(key);
                    }
                } else {
                    self.needs_redraw = false;
                }
            }
        }
        true
    }

    fn generate_qr(&mut self) {
        match qr_encode::encode(&self.input_text, self.settings.ec_level) {
            Some(matrix) => {
                self.qr_matrix = Some(matrix);
                self.qr_text = self.input_text.clone();
                self.state = AppState::Display;
            }
            None => {
                log::warn!("Text too long for QR version 1-10");
                // Stay on input screen — text too long
            }
        }
    }

    fn handle_display_key(&mut self, key: char) -> bool {
        match key {
            'q' | 'Q' => {
                self.state = AppState::MainMenu;
            }
            'n' | 'N' => {
                self.input_text.clear();
                self.state = AppState::Input;
            }
            's' | 'S' => {
                self.save_name.clear();
                self.state = AppState::SavePrompt;
            }
            KEY_UP => {
                // Increase module size
                if self.settings.module_size == 0 {
                    self.settings.module_size = 3;
                } else if self.settings.module_size < 8 {
                    self.settings.module_size += 1;
                }
                self.save_settings();
            }
            KEY_DOWN => {
                // Decrease module size
                if self.settings.module_size <= 3 {
                    self.settings.module_size = 0; // auto
                } else {
                    self.settings.module_size -= 1;
                }
                self.save_settings();
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_save_prompt_key(&mut self, key: char) -> bool {
        match key {
            'y' | 'Y' | KEY_ENTER => {
                self.save_name.clear();
                self.state = AppState::SaveNameEntry;
            }
            'n' | 'N' | 'q' | 'Q' => {
                self.state = AppState::Display;
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_save_name_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER => {
                if !self.save_name.is_empty() {
                    let code = SavedCode {
                        name: self.save_name.clone(),
                        text: self.qr_text.clone(),
                        ec_level: self.settings.ec_level,
                    };
                    self.saved_codes.push(code);
                    if let Some(ref mut s) = self.storage {
                        s.save_codes(&self.saved_codes);
                    }
                    self.state = AppState::Display;
                }
            }
            KEY_BACKSPACE => {
                self.save_name.pop();
            }
            'q' | 'Q' if self.save_name.is_empty() => {
                self.state = AppState::Display;
            }
            _ => {
                if key.is_ascii_graphic() || key == ' ' {
                    if self.save_name.len() < 30 {
                        self.save_name.push(key);
                    }
                } else {
                    self.needs_redraw = false;
                }
            }
        }
        true
    }

    fn handle_load_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.load_index > 0 {
                    self.load_index -= 1;
                }
            }
            KEY_DOWN => {
                if !self.saved_codes.is_empty() && self.load_index < self.saved_codes.len() - 1 {
                    self.load_index += 1;
                }
            }
            KEY_ENTER => {
                if !self.saved_codes.is_empty() {
                    let code = &self.saved_codes[self.load_index];
                    self.input_text = code.text.clone();
                    self.settings.ec_level = code.ec_level;
                    self.generate_qr();
                }
            }
            'd' | 'D' => {
                if !self.saved_codes.is_empty() {
                    self.saved_codes.remove(self.load_index);
                    if let Some(ref mut s) = self.storage {
                        s.save_codes(&self.saved_codes);
                    }
                    if self.load_index > 0 && self.load_index >= self.saved_codes.len() {
                        self.load_index = self.saved_codes.len().saturating_sub(1);
                    }
                }
            }
            'q' | 'Q' => {
                self.state = AppState::MainMenu;
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_settings_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.settings_index > 0 {
                    self.settings_index -= 1;
                }
            }
            KEY_DOWN => {
                if self.settings_index < 1 {
                    // 2 settings: EC level, module size
                    self.settings_index += 1;
                }
            }
            KEY_LEFT | KEY_RIGHT | KEY_ENTER => {
                match self.settings_index {
                    0 => {
                        // Cycle EC level
                        self.settings.ec_level = self.settings.ec_level.next();
                    }
                    1 => {
                        // Cycle module size
                        if key == KEY_RIGHT || key == KEY_ENTER {
                            self.settings.module_size = match self.settings.module_size {
                                0 => 3,
                                3 => 4,
                                4 => 5,
                                5 => 6,
                                6 => 7,
                                7 => 8,
                                _ => 0,
                            };
                        } else {
                            self.settings.module_size = match self.settings.module_size {
                                0 => 8,
                                3 => 0,
                                4 => 3,
                                5 => 4,
                                6 => 5,
                                7 => 6,
                                8 => 7,
                                _ => 0,
                            };
                        }
                    }
                    _ => {}
                }
                self.save_settings();
            }
            'q' | 'Q' => {
                self.state = AppState::MainMenu;
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_help_key(&mut self, key: char) -> bool {
        match key {
            'q' | 'Q' | KEY_ENTER | KEY_BACKSPACE => {
                self.state = AppState::MainMenu;
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    /// Compute the effective module pixel size.
    pub fn effective_module_size(&self) -> usize {
        if self.settings.module_size > 0 {
            return self.settings.module_size as usize;
        }
        // Auto-fit: maximize module size for display
        if let Some(ref matrix) = self.qr_matrix {
            let qr_modules = matrix.size + 8; // +8 for quiet zone (4 each side)
            let avail = 336usize.min(460); // content area
            let size = avail / qr_modules;
            size.max(2).min(8)
        } else {
            5
        }
    }
}
