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
const HIGHLIGHT: Color = Color::Rgb(0x28, 0x28, 0x3A); // new: selected menu row

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

/// Border right after a mistyped key. `t` goes from 0.0 (just mistyped:
/// magenta) to 1.0 (back to the normal focused border). This is a mistake
/// signal, which is what magenta is reserved for.
pub fn mistake_border(t: f64) -> Style {
    Style::new().fg(mix(ERROR, BORDER_FOCUS, t))
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

/// The cursor during the "off" half of its blink: an underlined cyan
/// character instead of a block, so you never lose track of where it is.
pub fn cursor_dim() -> Style {
    Style::new().fg(CYAN).add_modifier(Modifier::UNDERLINED)
}

/// The filled part of a progress or accuracy bar.
pub fn bar_filled() -> Style {
    Style::new().fg(CYAN)
}

/// The part of a bar not filled yet.
pub fn bar_empty() -> Style {
    Style::new().fg(BORDER)
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

/// A menu row that isn't selected.
pub fn menu_item() -> Style {
    Style::new().fg(TEXT)
}

/// The selected menu row: cyan on a slightly lighter band.
pub fn menu_selected() -> Style {
    Style::new()
        .fg(CYAN)
        .bg(HIGHLIGHT)
        .add_modifier(Modifier::BOLD)
}

/// Cyan text that brightens towards white as `amount` goes from 0.0 to 1.0.
/// Used for the shimmer on the logo and the pulsing "new best".
pub fn glow(amount: f64) -> Style {
    Style::new()
        .fg(mix(CYAN, TEXT, amount))
        .add_modifier(Modifier::BOLD)
}

/// Key hints and the command explanation shown after each round.
pub fn hint() -> Style {
    Style::new().fg(MUTED)
}

// ---- Animation ----

/// `color` partway through fading in: at `t` = 0.0 it matches the
/// background (invisible), at 1.0 it is `color` itself.
pub fn fade_in(color: Color, t: f64) -> Color {
    mix(BACKGROUND, color, t)
}

/// A color between `from` (`t` = 0.0) and `to` (`t` = 1.0).
///
/// Only RGB colors can be blended. Our palette is all RGB, but a cell can
/// also hold `Color::Reset` (the terminal's default), which is left as is.
fn mix(from: Color, to: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            mix_channel(r1, r2, t),
            mix_channel(g1, g2, t),
            mix_channel(b1, b2, t),
        ),
        _ => to,
    }
}

/// One of the red, green or blue parts of `mix`.
fn mix_channel(from: u8, to: u8, t: f64) -> u8 {
    let value = f64::from(from) + (f64::from(to) - f64::from(from)) * t;
    // `value` is always within 0..=255 here, because it lies between two
    // u8 values; `as u8` would also clamp it if it weren't.
    value.round() as u8
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
    fn mix_keeps_its_ends() {
        assert_eq!(mix(BACKGROUND, CYAN, 0.0), BACKGROUND);
        assert_eq!(mix(BACKGROUND, CYAN, 1.0), CYAN);
    }

    #[test]
    fn mix_halfway_is_the_average() {
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(200, 100, 50);
        assert_eq!(mix(black, white, 0.5), Color::Rgb(100, 50, 25));
    }

    #[test]
    fn fade_in_starts_at_the_background_and_ends_at_the_color() {
        assert_eq!(fade_in(TEXT, 0.0), BACKGROUND);
        assert_eq!(fade_in(TEXT, 1.0), TEXT);
    }

    #[test]
    fn non_rgb_colors_are_not_blended() {
        assert_eq!(fade_in(Color::Reset, 0.3), Color::Reset);
    }

    #[test]
    fn mistake_border_settles_on_the_normal_border() {
        assert_eq!(mistake_border(0.0).fg, Some(ERROR));
        assert_eq!(mistake_border(1.0), border_focused());
    }

    #[test]
    fn mistakes_do_not_rely_on_color_alone() {
        assert!(incorrect('|').add_modifier.contains(Modifier::UNDERLINED));
    }
}
