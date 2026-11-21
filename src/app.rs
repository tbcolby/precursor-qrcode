//! Screens, key handling and persistence. The encoder and the saved-code
//! model are in `qrcode-app-core`; drawing is in `ui.rs`.

use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;
use qrcode_app_core::model::{self, MAX_NAME_LEN};
use qrcode_app_core::{encode, Codes, EcLevel, IndexRecord, LegacyCode, QrCode, Settings, Template, TextField};

/// PDDB dictionaries (same names as version 0.1, so existing data is found).
pub const SETTINGS_DICT: &str = "qrcode.settings";
pub const CODES_DICT: &str = "qrcode.codes";
const SETTINGS_KEY: &str = "config";
const CODES_KEY: &str = "index";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Menu,
    Input,
    Display,
    SaveName,
    Saved,
    Settings,
    Help,
}

pub const MENU_ITEMS: [&str; 4] = ["New QR code", "Saved codes", "Settings", "Help"];
pub const SETTINGS_ROWS: usize = 3;

pub struct App {
    pub(crate) screen: Screen,
    settings_store: Store,
    codes_store: Store,

    pub(crate) view: View,
    pub(crate) menu: ListState,
    pub(crate) saved_list: ListState,
    pub(crate) settings_row: usize,
    pub(crate) visible_rows: usize,

    pub(crate) settings: Settings,
    settings_dirty: bool,
    pub(crate) codes: Codes,
    codes_read_only: bool,

    pub(crate) input: TextField,
    pub(crate) name: TextField,
    /// The code on screen and the text it encodes.
    pub(crate) qr: Option<QrCode>,
    pub(crate) qr_text: String,
    display_return: View,
    pub(crate) confirm_delete: bool,
    pub(crate) status: String,
}

impl App {
    pub fn new(screen: Screen, settings_store: Store, codes_store: Store) -> Self {
        let mut app = Self {
            screen,
            settings_store,
            codes_store,
            view: View::Menu,
            menu: ListState::default(),
            saved_list: ListState::default(),
            settings_row: 0,
            visible_rows: 1,
            settings: Settings::default(),
            settings_dirty: false,
            codes: Codes::new(),
            codes_read_only: false,
            input: TextField::new(model::max_text_len()),
            name: TextField::new(MAX_NAME_LEN),
            qr: None,
            qr_text: String::new(),
            display_return: View::Input,
            confirm_delete: false,
            status: String::new(),
        };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    /// (Re)load from the PDDB. Called at start and whenever the set of open
    /// bases changes, so nothing from a just-locked basis stays on screen.
    pub fn reload(&mut self) {
        self.settings = match self.settings_store.load::<Settings>(SETTINGS_KEY) {
            Ok(Some(s)) => s.sanitized(),
            Ok(None) => Settings::default(),
            Err(e) => {
                log::warn!("settings unreadable ({:?}); using defaults", e.kind());
                Settings::default()
            }
        };
        self.settings_dirty = false;
        self.load_codes();
        self.clear_code();
        self.input.clear();
        self.name.clear();
        self.confirm_delete = false;
        self.saved_list = ListState::default();
        self.view = View::Menu;
    }

    fn clear_code(&mut self) {
        self.qr = None;
        self.qr_text.clear();
    }

    fn load_codes(&mut self) {
        self.codes_read_only = false;
        self.codes = match self.codes_store.load::<IndexRecord>(CODES_KEY) {
            Ok(None) => Codes::new(),
            Ok(Some(IndexRecord::Current(c))) => match c.check_version() {
                Ok(()) => c,
                Err(v) => {
                    log::warn!("saved codes are version {v}, newer than this app");
                    self.codes_read_only = true;
                    self.status = "Saved codes are from a newer app version".into();
                    Codes::new()
                }
            },
            Ok(Some(IndexRecord::Legacy(names))) => self.migrate_legacy(names),
            Err(e) => {
                log::error!("saved codes unreadable: {:?}", e.kind());
                self.codes_read_only = true;
                self.status = "Couldn't read saved codes".into();
                Codes::new()
            }
        };
    }

    /// Convert the 0.1 layout (index of names + one `code.<name>` key each)
    /// to one record, then delete every old payload key, including orphans
    /// left by 0.1's delete.
    fn migrate_legacy(&mut self, names: Vec<String>) -> Codes {
        let entries = names
            .into_iter()
            .map(|n| {
                let payload = self.codes_store.load::<LegacyCode>(&model::legacy_code_key(&n)).ok().flatten();
                (n, payload)
            })
            .collect();
        let codes = model::migrate_legacy(entries);
        if let Err(e) = self.codes_store.save(CODES_KEY, &codes) {
            log::error!("migration save failed: {:?}", e.kind());
            self.codes_read_only = true;
            return codes;
        }
        for key in self.codes_store.keys().unwrap_or_default() {
            if model::is_legacy_code_key(&key) {
                if let Err(e) = self.codes_store.delete(&key) {
                    log::warn!("couldn't delete old key: {:?}", e.kind());
                }
            }
        }
        log::info!("migrated {} saved codes to the v1 layout", codes.len());
        codes
    }

    fn save_codes(&mut self) -> bool {
        if self.codes_read_only {
            self.status = "Saved codes are read-only".into();
            return false;
        }
        match self.codes_store.save(CODES_KEY, &self.codes) {
            Ok(()) => true,
            Err(e) => {
                log::error!("save failed: {:?}", e.kind());
                self.status = "Save failed".into();
                false
            }
        }
    }

    fn save_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        match self.settings_store.save(SETTINGS_KEY, &self.settings) {
            Ok(()) => self.settings_dirty = false,
            Err(e) => log::error!("settings save failed: {:?}", e.kind()),
        }
    }

    pub(crate) fn codes_basis(&self) -> String {
        self.codes_store
            .basis_of(CODES_KEY)
            .ok()
            .flatten()
            .or_else(|| self.codes_store.target_basis())
            .unwrap_or_else(|| "?".into())
    }

    /// Smaller canvas side: the square the code must fit in, minus room for
    /// the info line and footer.
    pub(crate) fn code_area(&self) -> usize {
        let s = self.screen.size();
        let reserved = 3 * self.screen.line_height(gam::GlyphStyle::Small) + 12;
        s.x.min(s.y - reserved).max(0) as usize
    }

    pub fn on_foreground(&mut self) {
        self.screen.refresh_size();
        self.draw();
    }

    /// Save changed settings; optionally take the code off the screen.
    pub fn on_background(&mut self) {
        self.save_settings();
        if self.settings.hide_on_background && matches!(self.view, View::Display | View::SaveName) {
            self.clear_code();
            self.view = View::Menu;
        }
    }

    pub fn draw(&mut self) { crate::ui::draw(self); }

    pub fn on_key(&mut self, key: Key) -> Action {
        self.status.clear();
        if !matches!(key, Key::Char('d') | Key::Char('D')) {
            self.confirm_delete = false;
        }
        match self.view {
            View::Menu => return self.key_menu(key),
            View::Input => self.key_input(key),
            View::Display => self.key_display(key),
            View::SaveName => self.key_save_name(key),
            View::Saved => self.key_saved(key),
            View::Settings => self.key_settings(key),
            View::Help => {
                if key.is_back() || key == Key::Enter {
                    self.view = View::Menu;
                }
            }
        }
        Action::Stay
    }

    fn key_menu(&mut self, key: Key) -> Action {
        match key {
            Key::Up => self.menu.step_wrapping(-1, MENU_ITEMS.len(), MENU_ITEMS.len()),
            Key::Down => self.menu.step_wrapping(1, MENU_ITEMS.len(), MENU_ITEMS.len()),
            Key::Enter => match self.menu.selected {
                0 => self.start_input(),
                1 => {
                    self.saved_list.clamp(self.codes.len(), self.visible_rows);
                    self.view = View::Saved;
                }
                2 => {
                    self.settings_row = 0;
                    self.view = View::Settings;
                }
                _ => self.view = View::Help,
            },
            Key::Char('n') | Key::Char('N') => self.start_input(),
            k if k.is_back() => {
                self.save_settings();
                return Action::Leave;
            }
            _ => {}
        }
        Action::Stay
    }

    fn start_input(&mut self) {
        self.input.clear();
        self.view = View::Input;
    }

    fn template(&mut self, t: Template) {
        let (text, cursor) = t.text_and_cursor();
        self.input.set(text, cursor);
    }

    fn key_input(&mut self, key: Key) {
        match key {
            Key::Enter => self.generate(),
            Key::Backspace => {
                if self.input.is_empty() {
                    self.view = View::Menu;
                } else {
                    self.input.backspace();
                }
            }
            Key::F(4) => {
                // First press clears, second (on an empty field) goes back.
                if self.input.is_empty() {
                    self.view = View::Menu;
                } else {
                    self.input.clear();
                }
            }
            Key::F(1) => self.template(Template::Wifi),
            Key::F(2) => self.template(Template::Url),
            Key::F(3) => self.template(Template::VCard),
            Key::Left => self.input.left(),
            Key::Right => self.input.right(),
            Key::Up => self.input.home(),
            Key::Down => self.input.end(),
            Key::Char(c) => {
                if !self.input.insert(c) && self.input.len() >= self.input.max() {
                    self.status = format!("Limit is {} characters", self.input.max());
                }
            }
            _ => {}
        }
    }

    fn generate(&mut self) {
        let text = self.input.as_str().to_string();
        self.show(&text, self.settings.ec_level, View::Input);
    }

    /// Encode and show `text` at `ec`, or put the reason in the status line.
    fn show(&mut self, text: &str, ec: EcLevel, back_to: View) {
        match self.encode_fitting(text, ec) {
            Ok(q) => {
                self.qr = Some(q);
                self.qr_text = text.to_string();
                self.display_return = back_to;
                self.view = View::Display;
            }
            Err(msg) => self.status = msg,
        }
    }

    pub(crate) fn encode_fitting(&self, text: &str, ec: EcLevel) -> Result<QrCode, String> {
        let q = encode(text, ec).map_err(|e| e.to_string())?;
        let area = self.code_area();
        if qrcode_app_core::module_px(q.size, area, 1).is_none() {
            return Err(format!("Version {} doesn't fit the screen", q.version));
        }
        Ok(q)
    }

    fn key_display(&mut self, key: Key) {
        match key {
            Key::Up => {
                self.settings.bigger();
                self.settings_dirty = true;
            }
            Key::Down => {
                self.settings.smaller();
                self.settings_dirty = true;
            }
            Key::Left | Key::Right => {
                // Re-encode this code at another EC level (doesn't change the default).
                if let Some(q) = self.qr.as_ref() {
                    let ec = if key == Key::Right { q.ec.next() } else { q.ec.prev() };
                    let text = self.qr_text.clone();
                    match self.encode_fitting(&text, ec) {
                        Ok(q) => self.qr = Some(q),
                        Err(msg) => self.status = msg,
                    }
                }
            }
            Key::Char('s') | Key::Char('S') => {
                self.name.clear();
                self.view = View::SaveName;
            }
            Key::Char('n') | Key::Char('N') => self.start_input(),
            k if k.is_back() => {
                self.save_settings();
                self.clear_code();
                self.view = self.display_return;
            }
            _ => {}
        }
    }

    fn key_save_name(&mut self, key: Key) {
        match key {
            Key::Enter => {
                let Some(ec) = self.qr.as_ref().map(|q| q.ec) else {
                    self.view = View::Menu;
                    return;
                };
                let text = self.qr_text.clone();
                match self.codes.add(self.name.as_str(), &text, ec) {
                    Ok(()) => {
                        if self.save_codes() {
                            self.status = format!("Saved \"{}\"", self.name.as_str().trim());
                        } else {
                            self.codes.remove(self.codes.len() - 1);
                        }
                        self.view = View::Display;
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
            Key::Backspace => {
                if !self.name.backspace() {
                    self.view = View::Display;
                }
            }
            Key::F(4) => self.view = View::Display,
            Key::Left => self.name.left(),
            Key::Right => self.name.right(),
            Key::Char(c) => {
                self.name.insert(c);
            }
            _ => {}
        }
    }

    fn key_saved(&mut self, key: Key) {
        let len = self.codes.len();
        match key {
            Key::Up => self.saved_list.step(-1, len, self.visible_rows),
            Key::Down => self.saved_list.step(1, len, self.visible_rows),
            Key::Enter => {
                if let Some(c) = self.codes.get(self.saved_list.selected) {
                    // Use the code's own EC level; the default setting is untouched.
                    let (text, ec) = (c.text.clone(), c.ec_level);
                    self.show(&text, ec, View::Saved);
                }
            }
            Key::Char('d') | Key::Char('D') if len > 0 => {
                if !self.confirm_delete {
                    self.confirm_delete = true;
                    self.status = "Press d again to delete".into();
                } else {
                    self.confirm_delete = false;
                    let i = self.saved_list.selected;
                    if let Some(removed) = self.codes.remove(i) {
                        let name = removed.name.clone();
                        if self.save_codes() {
                            self.status = format!("Deleted \"{name}\"");
                        } else {
                            self.codes.restore(i, removed);
                        }
                    }
                    self.saved_list.clamp(self.codes.len(), self.visible_rows);
                }
            }
            k if k.is_back() => self.view = View::Menu,
            _ => {}
        }
    }

    fn key_settings(&mut self, key: Key) {
        let forward = matches!(key, Key::Right | Key::Enter);
        match key {
            Key::Up => self.settings_row = self.settings_row.checked_sub(1).unwrap_or(SETTINGS_ROWS - 1),
            Key::Down => self.settings_row = (self.settings_row + 1) % SETTINGS_ROWS,
            Key::Left | Key::Right | Key::Enter => {
                match self.settings_row {
                    0 => {
                        self.settings.ec_level =
                            if forward { self.settings.ec_level.next() } else { self.settings.ec_level.prev() }
                    }
                    1 => {
                        if forward {
                            self.settings.bigger()
                        } else {
                            self.settings.smaller()
                        }
                    }
                    _ => self.settings.hide_on_background = !self.settings.hide_on_background,
                }
                self.settings_dirty = true;
            }
            k if k.is_back() => {
                self.save_settings();
                self.view = View::Menu;
            }
            _ => {}
        }
    }
}
