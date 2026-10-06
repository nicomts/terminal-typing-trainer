//! Project DL palette and the styles built from it.
//!
//! This is the only file that defines colors. The raw palette is private on
//! purpose: UI code calls the style functions below, which name *what*
//! something is (a mistake, the cursor) rather than *which color* it is.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

// ---- Raw palette: Project DL, tuned for terminals ----
//
// Box-drawing borders are one cell thin and panels are separated only by
// their fill, so the dark tones are spread further apart than on the web.
// Original Project DL values are noted where a color changed.

const BACKGROUND: Color = Color::Rgb(0x0C, 0x0C, 0x10);
const SURFACE: Color = Color::Rgb(0x1A, 0x1A, 0x24); // was #13131A
const BORDER: Color = Color::Rgb(0x3A, 0x3A, 0x52); // was #1E1E2A
const BORDER_FOCUS: Color = Color::Rgb(0x62, 0x66, 0x8A); // new
const CYAN: Color = Color::Rgb(0x00, 0xD4, 0xFF); // Accent 1, unchanged
const MAGENTA: Color = Color::Rgb(0xFF, 0x2E, 0xA3); // Accent 2, was #F0008C
const TEXT: Color = Color::Rgb(0xEA, 0xEA, 0xF2);
const MUTED: Color = Color::Rgb(0x80, 0x83, 0xA2); // was #6E7191

// Magenta is reserved for mistakes. If it appeared anywhere else,
// a magenta title would read as an error.
const ERROR: Color = MAGENTA;

// ---- Screen and panels ----

/// Fill for the whole frame, so the app never shows the terminal's own background.
pub fn app() -> Style {
    Style::new().fg(TEXT).bg(BACKGROUND)
}

/// Inside of a bordered panel.
pub fn panel() -> Style {
    Style::new().fg(TEXT).bg(SURFACE)
}

/// Border of an inactive panel. Deliberately subtle.
pub fn border() -> Style {
    Style::new().fg(BORDER)
}

/// Border of the panel the user is working in.
pub fn border_focused() -> Style {
    Style::new().fg(BORDER_FOCUS)
}

/// The block every panel is built from: rounded corners, panel fill,
/// border colored by focus. Add a title with `.title(...)` at the call site.
pub fn panel_block(focused: bool) -> Block<'static> {
    let border_style = if focused { border_focused() } else { border() };
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(panel())
}

// ---- Typing area ----

/// Characters the user hasn't reached yet.
pub fn untyped() -> Style {
    Style::new().fg(MUTED)
}

/// Characters typed correctly.
pub fn correct() -> Style {
    Style::new().fg(TEXT)
}

/// A mistyped character. `expected` is the character that should have been typed.
///
/// A colored space is invisible, so a missed space gets a filled background.
/// Other mistakes are also underlined, so they don't rely on color alone.
pub fn incorrect(expected: char) -> Style {
    if expected == ' ' {
        Style::new().bg(ERROR)
    } else {
        Style::new().fg(ERROR).add_modifier(Modifier::UNDERLINED)
    }
}

/// The next character to type, drawn as a cyan block.
pub fn cursor() -> Style {
    Style::new().fg(BACKGROUND).bg(CYAN)
}

// ---- Stats, menus, hints ----

/// Live numbers such as WPM and accuracy.
pub fn stat_value() -> Style {
    Style::new().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// The label next to a stat ("wpm", "acc").
pub fn stat_label() -> Style {
    Style::new().fg(MUTED)
}

#[expect(dead_code, reason = "used by a later milestone")]
pub fn menu_item() -> Style {
    Style::new().fg(TEXT)
}

#[expect(dead_code, reason = "used by a later milestone")]
pub fn menu_selected() -> Style {
    Style::new().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// Key hints and the command explanation shown after each round.
pub fn hint() -> Style {
    Style::new().fg(MUTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missed_space_is_visible() {
        // A space has no glyph, so only a background makes the mistake visible.
        assert_eq!(incorrect(' ').bg, Some(ERROR));
    }

    #[test]
    fn mistakes_do_not_rely_on_color_alone() {
        assert!(incorrect('|').add_modifier.contains(Modifier::UNDERLINED));
    }
}
