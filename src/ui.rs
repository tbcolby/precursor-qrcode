//! Drawing for every screen. Sizes come from the canvas, never constants.

use gam::menu::{DrawStyle, Point, Rectangle};
use gam::{GamObjectList, GamObjectType, GlyphStyle, PixelColor};
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::MARGIN;
use qrcode_app_core::encode::{detect_mode, mode_len, select_version, symbol_size, QUIET_ZONE};
use qrcode_app_core::model;

use crate::app::{App, View, MENU_ITEMS};

pub fn draw(app: &mut App) {
    app.screen.clear();
    match app.view {
        View::Menu => draw_menu(app),
        View::Input => draw_input(app),
        View::Display => draw_display(app),
        View::SaveName => draw_save_name(app),
        View::Saved => draw_saved(app),
        View::Settings => draw_settings(app),
        View::Help => draw_help(app),
    }
    app.screen.flush();
}

/// Draw the status line (if any) just above `bottom`; returns the new bottom.
fn status_line(app: &App, bottom: isize) -> isize {
    if app.status.is_empty() {
        return bottom;
    }
    let h = app.screen.line_height(GlyphStyle::Small);
    app.screen.text(bottom - h, GlyphStyle::Small, &app.status);
    bottom - h
}

fn draw_menu(app: &mut App) {
    let top = app.screen.header("QR Code Generator");
    let bottom = app.screen.footer("↑↓ move · ⏎ select · n new · ⌫ exit");
    let bottom = status_line(app, bottom);
    let info_h = app.screen.line_height(GlyphStyle::Small);
    app.screen.list(top, bottom - info_h, &MENU_ITEMS, &app.menu);
    let info = match app.codes.len() {
        0 => String::from("No saved codes"),
        1 => String::from("1 saved code"),
        n => format!("{n} saved codes"),
    };
    app.screen.text(bottom - info_h, GlyphStyle::Small, &info);
}

fn draw_input(app: &mut App) {
    let s = &app.screen;
    let w = s.size().x;
    let mut y = s.header("New QR code");
    let bottom = s.footer("⏎ make · F1 WiFi F2 URL F3 vCard · F4 clear/back");
    let bottom = status_line(app, bottom);

    let ec = app.settings.ec_level;
    let text = app.input.as_str();
    let (used, max) = model::usage(text, ec);
    let unit = if detect_mode(text) == qrcode_app_core::Mode::Byte { "bytes" } else { "chars" };
    let counter = format!("{used} of {max} {unit} at EC-{} ({})", ec.label(), detect_mode(text).label());
    y += s.text(y, GlyphStyle::Small, &counter);

    let info_h = 2 * s.line_height(GlyphStyle::Small);
    let box_bottom = bottom - info_h - 4;
    s.frame(Point::new(MARGIN - 2, y), Point::new(w - MARGIN + 1, box_bottom), 1);
    let shown = if app.input.is_empty() { String::from("(type text; ←→ move cursor)") } else { app.input.with_cursor() };
    s.text_in(Point::new(MARGIN + 2, y + 4), Point::new(w - MARGIN - 2, box_bottom - 2), GlyphStyle::Monospace, &shown);

    let msg = if text.is_empty() {
        String::from("↑/↓ cursor to start/end")
    } else {
        // Version lookup only: the full encode (8 masks) runs on Enter.
        let mode = detect_mode(text);
        match select_version(mode, mode_len(text, mode), ec) {
            Some(v) if qrcode_app_core::module_px(symbol_size(v), app.code_area(), 1).is_some() => {
                format!("OK: version {v} ({0}x{0})", symbol_size(v))
            }
            Some(v) => format!("Version {v} doesn't fit the screen"),
            None => format!("Too long for EC-{}; try a lower level in Settings", ec.label()),
        }
    };
    s.text_in(Point::new(MARGIN, box_bottom + 4), Point::new(w - MARGIN, bottom), GlyphStyle::Small, &msg);
}

fn draw_display(app: &mut App) {
    let s = &app.screen;
    let w = s.size().x;
    let Some(q) = app.qr.as_ref() else {
        s.text(4, GlyphStyle::Regular, "No code");
        return;
    };
    let bottom = s.footer("↑↓ size · ←→ EC · s save · n new · ⌫ back");
    let bottom = status_line(app, bottom);
    let info_h = s.line_height(GlyphStyle::Small);

    let area = app.code_area();
    let px = qrcode_app_core::module_px(q.size, area, app.settings.module_size as usize).unwrap_or(1);
    let total = ((q.size + 2 * QUIET_ZONE) * px) as isize;
    let x0 = ((w - total) / 2).max(0) + (QUIET_ZONE * px) as isize;
    let y0 = ((bottom - info_h - total) / 2).max(0) + (QUIET_ZONE * px) as isize;
    let px_i = px as isize;

    // One rectangle per horizontal dark run, sent 32 at a time.
    let gid = s.gid();
    let style = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0);
    let mut list = GamObjectList::new(gid);
    for row in 0..q.size {
        let y = y0 + row as isize * px_i;
        for (col, len) in q.row_runs(row) {
            let x = x0 + col as isize * px_i;
            let rect = Rectangle::new_with_style(
                Point::new(x, y),
                Point::new(x + len as isize * px_i - 1, y + px_i - 1),
                style,
            );
            if let Err(obj) = list.push(GamObjectType::Rect(rect)) {
                s.gam().draw_list(list).ok();
                list = GamObjectList::new(gid);
                list.push(obj).ok();
            }
        }
    }
    s.gam().draw_list(list).ok();

    let narrowed = if app.settings.module_size as usize > px { " (reduced to fit)" } else { "" };
    let info = format!(
        "V{} {}x{} · EC-{} · mask {} · {} px{}",
        q.version,
        q.size,
        q.size,
        q.ec.label(),
        q.mask,
        px,
        narrowed
    );
    s.text(bottom - info_h, GlyphStyle::Small, &info);
}

fn draw_save_name(app: &mut App) {
    let s = &app.screen;
    let w = s.size().x;
    let mut y = s.header("Save QR code");
    let bottom = s.footer("⏎ save · ⌫ delete · F4 cancel");
    status_line(app, bottom);
    y += s.text(y, GlyphStyle::Regular, "Name:");
    let line_h = s.line_height(GlyphStyle::Monospace);
    s.frame(Point::new(MARGIN - 2, y), Point::new(w - MARGIN + 1, y + line_h + 6), 1);
    let shown = if app.name.is_empty() { String::from("(type a name)") } else { app.name.with_cursor() };
    s.text_in(Point::new(MARGIN + 2, y + 3), Point::new(w - MARGIN - 2, y + line_h + 4), GlyphStyle::Monospace, &shown);
    y += line_h + 12;
    y += s.text(y, GlyphStyle::Small, &format!("Stored in PDDB basis: {}", app.codes_basis()));
    s.text_in(
        Point::new(MARGIN, y),
        Point::new(w - MARGIN, bottom),
        GlyphStyle::Small,
        "Anyone who can see the screen can scan a displayed code.",
    );
}

fn draw_saved(app: &mut App) {
    let title = format!("Saved codes ({})", app.codes.len());
    let top = app.screen.header(&title);
    let bottom = app.screen.footer("⏎ show · d delete · ⌫ back");
    let bottom = status_line(app, bottom);
    if app.codes.is_empty() {
        app.screen.text(top + 8, GlyphStyle::Regular, "No saved codes.");
        return;
    }
    let labels: Vec<String> = (0..app.codes.len()).map(|i| app.codes.label(i)).collect();
    app.visible_rows = app.screen.list(top, bottom, &labels, &app.saved_list);
}

fn draw_settings(app: &mut App) {
    let top = app.screen.header("Settings");
    let bottom = app.screen.footer("↑↓ select · ←→ change · ⌫ back");
    let st = &app.settings;
    let rows = [
        format!("Error correction: {} ({})", st.ec_level.label(), st.ec_level.recovery()),
        format!("Module size: {}", st.module_label()),
        format!("Hide code on focus loss: {}", if st.hide_on_background { "on" } else { "off" }),
    ];
    let state = ListState { selected: app.settings_row, top: 0 };
    app.screen.list(top, bottom, &rows, &state);
}

const HELP: &[&str] = &[
    "Versions 1-10, EC L/M/Q/H.",
    "Max at EC-L: 652 digits, 395",
    "  A-Z0-9 $%*+-./: or 271 bytes.",
    "Input: ⏎ make, ←→ ↑↓ cursor,",
    "  F1 WiFi, F2 URL, F3 vCard,",
    "  F4 clear (again: back),",
    "  ⌫ delete (empty: back)",
    "WiFi: escape ; , : \\ \" in the",
    "  SSID/password with \\",
    "Code: ↑↓ module size,",
    "  ←→ EC level for this code,",
    "  s save, n new, ⌫ back",
    "Saved: ⏎ show, d d delete",
    "Anyone in view can scan",
    "  a code on the screen.",
];

fn draw_help(app: &mut App) {
    let s = &app.screen;
    let mut y = s.header("Help");
    let bottom = s.footer("⌫ back");
    let h = s.line_height(GlyphStyle::Small);
    for line in HELP {
        if y + h > bottom {
            break;
        }
        y += s.text(y, GlyphStyle::Small, line);
    }
}
