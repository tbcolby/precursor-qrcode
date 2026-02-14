//! UI rendering for the QR Code Generator.
//!
//! Draws all screens: menu, input, QR display, save prompt, load list, settings, help.

use crate::app::{AppState, MenuItem, QrApp};
use crate::qr_encode;

use gam::*;

// Layout constants (ecosystem standard)
const SCREEN_WIDTH: isize = 336;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 46;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = 536 - FOOTER_HEIGHT;
const CONTENT_HEIGHT: isize = CONTENT_BOTTOM - CONTENT_TOP;

// Font metrics
const REGULAR_HEIGHT: isize = 15;
const LINE_GAP: isize = 4;
const LINE_HEIGHT: isize = REGULAR_HEIGHT + LINE_GAP;

/// Draw the complete screen based on current app state.
pub fn draw(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    // Clear screen
    let clear_rect = graphics_server::Rectangle::new_coords_with_style(
        0,
        0,
        SCREEN_WIDTH,
        536,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Light,
            graphics_server::PixelColor::Light,
            0,
        ),
    );
    gam.draw_rectangle(canvas, clear_rect).ok();

    match app.state {
        AppState::MainMenu => draw_main_menu(app, gam, canvas),
        AppState::Input => draw_input(app, gam, canvas),
        AppState::Display => draw_display(app, gam, canvas),
        AppState::SavePrompt => draw_save_prompt(app, gam, canvas),
        AppState::SaveNameEntry => draw_save_name(app, gam, canvas),
        AppState::LoadList => draw_load_list(app, gam, canvas),
        AppState::Settings => draw_settings(app, gam, canvas),
        AppState::Help => draw_help(app, gam, canvas),
    }

    gam.redraw().ok();
}

// ─── Header / Footer helpers ────────────────────────────────────────────────

fn draw_header(gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    // Header background
    let bg = graphics_server::Rectangle::new_coords_with_style(
        0,
        0,
        SCREEN_WIDTH,
        HEADER_HEIGHT,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            0,
        ),
    );
    gam.draw_rectangle(canvas, bg).ok();

    // Title text (inverted)
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8,
            2,
            SCREEN_WIDTH - 8,
            HEADER_HEIGHT - 2,
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.invert = true;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", title).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, canvas: graphics_server::Gid, labels: &[&str; 4]) {
    let y = CONTENT_BOTTOM;
    let zone_width = SCREEN_WIDTH / 4;

    // Footer separator line
    let sep = graphics_server::Line::new_with_style(
        Point::new(0, y),
        Point::new(SCREEN_WIDTH, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            1,
        ),
    );
    gam.draw_line(canvas, sep).ok();

    for (i, label) in labels.iter().enumerate() {
        if label.is_empty() {
            continue;
        }
        let x = (i as isize) * zone_width;
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                x + 2,
                y + 4,
                x + zone_width - 2,
                y + FOOTER_HEIGHT - 4,
            )),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "F{}: {}", i + 1, label).ok();
        gam.post_textview(&mut tv).ok();
    }
}

// ─── Screen Renderers ───────────────────────────────────────────────────────

fn draw_main_menu(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "QR Code Generator");

    let items = MenuItem::all();
    for (i, item) in items.iter().enumerate() {
        let y = CONTENT_TOP + 20 + (i as isize) * (LINE_HEIGHT + 8);
        let selected = i == app.menu_index;

        if selected {
            // Highlight bar
            let hl = graphics_server::Rectangle::new_coords_with_style(
                8,
                y - 2,
                SCREEN_WIDTH - 8,
                y + LINE_HEIGHT + 2,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Dark,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, hl).ok();
        }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16,
                y,
                SCREEN_WIDTH - 16,
                y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", item.label()).ok();
        gam.post_textview(&mut tv).ok();
    }

    // Saved count indicator
    if !app.saved_codes.is_empty() {
        let y = CONTENT_TOP + 20 + (LINE_HEIGHT + 8) * 4 + 20;
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16,
                y,
                SCREEN_WIDTH - 16,
                y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{} saved codes", app.saved_codes.len()).ok();
        gam.post_textview(&mut tv).ok();
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_input(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Enter Text");

    // Instructions
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8,
            CONTENT_TOP + 8,
            SCREEN_WIDTH - 8,
            CONTENT_TOP + 8 + LINE_HEIGHT * 2,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Type text, press Enter to generate QR code").ok();
    gam.post_textview(&mut tv).ok();

    // Input box
    let input_top = CONTENT_TOP + 40;
    let input_bottom = CONTENT_BOTTOM - 80;

    let border = graphics_server::Rectangle::new_coords_with_style(
        8,
        input_top,
        SCREEN_WIDTH - 8,
        input_bottom,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 1,
        },
    );
    gam.draw_rectangle(canvas, border).ok();

    // Input text
    let display_text = if app.input_text.is_empty() {
        "(empty)"
    } else {
        &app.input_text
    };

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16,
            input_top + 8,
            SCREEN_WIDTH - 16,
            input_bottom - 8,
        )),
    );
    tv.style = GlyphStyle::Monospace;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", display_text).ok();
    gam.post_textview(&mut tv).ok();

    // Status line: character count, mode, estimated version
    let y_status = input_bottom + 8;
    let mode = qr_encode::detect_mode(&app.input_text);
    let version = if app.input_text.is_empty() {
        None
    } else {
        qr_encode::select_version(&app.input_text, mode, app.settings.ec_level)
    };
    let cap = version
        .map(|v| qr_encode::capacity(v, app.settings.ec_level, mode))
        .unwrap_or(0);

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8,
            y_status,
            SCREEN_WIDTH - 8,
            y_status + LINE_HEIGHT * 2,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    if let Some(v) = version {
        write!(
            tv,
            "{}ch | {:?} | V{} | EC-{} | max {}",
            app.input_text.len(),
            mode,
            v,
            app.settings.ec_level.label(),
            cap,
        )
        .ok();
    } else if app.input_text.is_empty() {
        write!(tv, "EC-{} | Ready", app.settings.ec_level.label()).ok();
    } else {
        write!(tv, "{}ch | Too long for V1-10!", app.input_text.len()).ok();
    }
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["WiFi", "URL", "vCard", "Clear"]);
}

fn draw_display(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    if let Some(ref matrix) = app.qr_matrix {
        let module_px = app.effective_module_size() as isize;
        let qr_modules = matrix.size as isize;
        let quiet_zone = 4isize; // QR spec: 4-module quiet zone
        let total_modules = qr_modules + quiet_zone * 2;
        let total_px = total_modules * module_px;

        // Center QR code on screen
        let x_offset = (SCREEN_WIDTH - total_px) / 2;
        let y_offset = (536 - total_px) / 2;

        // Draw quiet zone (white background)
        let bg = graphics_server::Rectangle::new_coords_with_style(
            x_offset,
            y_offset,
            x_offset + total_px,
            y_offset + total_px,
            graphics_server::DrawStyle::new(
                graphics_server::PixelColor::Light,
                graphics_server::PixelColor::Light,
                0,
            ),
        );
        gam.draw_rectangle(canvas, bg).ok();

        // Draw QR modules
        for row in 0..matrix.size {
            for col in 0..matrix.size {
                if matrix.modules[row][col] {
                    let x = x_offset + (col as isize + quiet_zone) * module_px;
                    let y = y_offset + (row as isize + quiet_zone) * module_px;
                    let rect = graphics_server::Rectangle::new_coords_with_style(
                        x,
                        y,
                        x + module_px,
                        y + module_px,
                        graphics_server::DrawStyle::new(
                            graphics_server::PixelColor::Dark,
                            graphics_server::PixelColor::Dark,
                            0,
                        ),
                    );
                    gam.draw_rectangle(canvas, rect).ok();
                }
            }
        }

        // Status bar at bottom
        let status_y = y_offset + total_px + 4;
        if status_y < 536 - 20 {
            let mode = qr_encode::detect_mode(&app.qr_text);
            let version = qr_encode::select_version(&app.qr_text, mode, app.settings.ec_level)
                .unwrap_or(0);

            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    4,
                    status_y,
                    SCREEN_WIDTH - 4,
                    status_y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Small;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);
            write!(
                tv,
                "V{} EC-{} {}px  S:save N:new Q:back",
                version,
                app.settings.ec_level.label(),
                module_px,
            )
            .ok();
            gam.post_textview(&mut tv).ok();

            // Truncated text preview
            let preview_y = status_y + LINE_HEIGHT + 2;
            if preview_y < 536 - 16 {
                let preview: String = if app.qr_text.len() > 40 {
                    let mut s: String = app.qr_text.chars().take(37).collect();
                    s.push_str("...");
                    s
                } else {
                    app.qr_text.clone()
                };

                let mut tv = TextView::new(
                    canvas,
                    TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                        4,
                        preview_y,
                        SCREEN_WIDTH - 4,
                        preview_y + LINE_HEIGHT,
                    )),
                );
                tv.style = GlyphStyle::Small;
                tv.draw_border = false;
                tv.margin = Point::new(0, 0);
                write!(tv, "{}", preview).ok();
                gam.post_textview(&mut tv).ok();
            }
        }
    }
}

fn draw_save_prompt(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    // Draw the QR code in background (dimmed effect via overlay)
    draw_display(app, gam, canvas);

    // Dialog box overlay
    let dialog_y = 200isize;
    let dialog_h = 80isize;

    let bg = graphics_server::Rectangle::new_coords_with_style(
        30,
        dialog_y,
        SCREEN_WIDTH - 30,
        dialog_y + dialog_h,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 2,
        },
    );
    gam.draw_rectangle(canvas, bg).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            44,
            dialog_y + 12,
            SCREEN_WIDTH - 44,
            dialog_y + dialog_h - 12,
        )),
    );
    tv.style = GlyphStyle::Regular;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Save this QR code?\n\nY: Yes  N: No").ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_save_name(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Save QR Code");

    let y = CONTENT_TOP + 30;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16,
            y,
            SCREEN_WIDTH - 16,
            y + LINE_HEIGHT,
        )),
    );
    tv.style = GlyphStyle::Regular;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Enter a name:").ok();
    gam.post_textview(&mut tv).ok();

    // Name input box
    let box_y = y + LINE_HEIGHT + 16;
    let border = graphics_server::Rectangle::new_coords_with_style(
        16,
        box_y,
        SCREEN_WIDTH - 16,
        box_y + LINE_HEIGHT + 16,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 1,
        },
    );
    gam.draw_rectangle(canvas, border).ok();

    let display = if app.save_name.is_empty() {
        "(type a name)"
    } else {
        &app.save_name
    };
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            24,
            box_y + 4,
            SCREEN_WIDTH - 24,
            box_y + LINE_HEIGHT + 12,
        )),
    );
    tv.style = GlyphStyle::Monospace;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", display).ok();
    gam.post_textview(&mut tv).ok();

    // Instructions
    let instr_y = box_y + LINE_HEIGHT + 30;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16,
            instr_y,
            SCREEN_WIDTH - 16,
            instr_y + LINE_HEIGHT,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Enter: save | Q: cancel").ok();
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_load_list(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Saved Codes");

    if app.saved_codes.is_empty() {
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16,
                CONTENT_TOP + 30,
                SCREEN_WIDTH - 16,
                CONTENT_TOP + 30 + LINE_HEIGHT * 2,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "No saved codes.\n\nPress Q to go back.").ok();
        gam.post_textview(&mut tv).ok();
    } else {
        let max_visible = ((CONTENT_HEIGHT - 20) / (LINE_HEIGHT + 6)) as usize;
        let scroll_offset = if app.load_index >= max_visible {
            app.load_index - max_visible + 1
        } else {
            0
        };

        for (vi, i) in (scroll_offset..app.saved_codes.len())
            .take(max_visible)
            .enumerate()
        {
            let code = &app.saved_codes[i];
            let y = CONTENT_TOP + 12 + (vi as isize) * (LINE_HEIGHT + 6);
            let selected = i == app.load_index;

            if selected {
                let hl = graphics_server::Rectangle::new_coords_with_style(
                    4,
                    y - 2,
                    SCREEN_WIDTH - 4,
                    y + LINE_HEIGHT + 2,
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Dark,
                        0,
                    ),
                );
                gam.draw_rectangle(canvas, hl).ok();
            }

            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    12,
                    y,
                    SCREEN_WIDTH - 12,
                    y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Regular;
            tv.invert = selected;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);

            let preview: String = if code.text.len() > 20 {
                let mut s: String = code.text.chars().take(17).collect();
                s.push_str("...");
                s
            } else {
                code.text.clone()
            };
            write!(tv, "{}: {}", code.name, preview).ok();
            gam.post_textview(&mut tv).ok();
        }
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_settings(app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Settings");

    let settings_items = [
        (
            "Error Correction",
            app.settings.ec_level.label(),
        ),
        (
            "Module Size",
            if app.settings.module_size == 0 {
                "Auto"
            } else {
                // We can't easily return a dynamic string here, so use a match
                match app.settings.module_size {
                    3 => "3px",
                    4 => "4px",
                    5 => "5px",
                    6 => "6px",
                    7 => "7px",
                    8 => "8px",
                    _ => "Auto",
                }
            },
        ),
    ];

    for (i, (label, value)) in settings_items.iter().enumerate() {
        let y = CONTENT_TOP + 30 + (i as isize) * (LINE_HEIGHT + 16);
        let selected = i == app.settings_index;

        if selected {
            let hl = graphics_server::Rectangle::new_coords_with_style(
                8,
                y - 4,
                SCREEN_WIDTH - 8,
                y + LINE_HEIGHT + 4,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Dark,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, hl).ok();
        }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16,
                y,
                200,
                y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", label).ok();
        gam.post_textview(&mut tv).ok();

        // Value on right side
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                210,
                y,
                SCREEN_WIDTH - 16,
                y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Bold;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "<  {}  >", value).ok();
        gam.post_textview(&mut tv).ok();
    }

    // Instructions
    let y = CONTENT_TOP + 30 + (settings_items.len() as isize) * (LINE_HEIGHT + 16) + 20;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16,
            y,
            SCREEN_WIDTH - 16,
            y + LINE_HEIGHT * 2,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Up/Down: select | Left/Right: change\nQ: back").ok();
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_help(_app: &QrApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Help");

    let help_text = [
        "QR Code Generator v0.1",
        "",
        "MAIN MENU",
        "  Up/Down: Navigate",
        "  Enter: Select",
        "  N: New QR code",
        "  Q: Quit",
        "",
        "INPUT",
        "  Type text, Enter to generate",
        "  F1: WiFi template",
        "  F2: URL template",
        "  F3: vCard template",
        "  F4: Clear input",
        "",
        "QR DISPLAY",
        "  S: Save code",
        "  N: New code",
        "  Up/Down: Adjust size",
        "  Q: Back to menu",
        "",
        "SAVED CODES",
        "  Enter: Load and display",
        "  D: Delete selected",
    ];

    for (i, line) in help_text.iter().enumerate() {
        let y = CONTENT_TOP + 4 + (i as isize) * (REGULAR_HEIGHT + 2);
        if y + REGULAR_HEIGHT > CONTENT_BOTTOM {
            break;
        }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                8,
                y,
                SCREEN_WIDTH - 8,
                y + REGULAR_HEIGHT,
            )),
        );
        tv.style = if line.starts_with("QR") || line.starts_with("MAIN") || line.starts_with("INPUT") || line.starts_with("SAVED") {
            GlyphStyle::Bold
        } else {
            GlyphStyle::Small
        };
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", line).ok();
        gam.post_textview(&mut tv).ok();
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}
