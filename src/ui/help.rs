//! The help screen: how the trainer works and which keys do what. Drawn
//! instead of the Menu or Results screen while `App::show_help` is set.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{BORDER_ROWS, centered_area};
use crate::app::CAN_QUIT;
use crate::theme;

/// How the trainer works, one short sentence per row. Each fits in the
/// narrowest panel (56 columns of text).
const ABOUT: [&str; 4] = [
    "Type the command shown. Nothing you type is ever run.",
    "Mistakes are marked; fixed ones still cost accuracy.",
    "Commands with your weak symbols come up more often.",
    "Old mistakes fade as you start getting a symbol right.",
];

/// The menu's keys. Only the terminal app can quit (see `app::CAN_QUIT`).
const MENU_KEYS: &str = if CAN_QUIT {
    "↑↓ or j k choose · enter start · esc quit"
} else {
    "↑↓ or j k choose · enter start"
};

/// Each screen with its keys.
const KEYS: [(&str, &str); 3] = [
    ("menu    ", MENU_KEYS),
    ("typing  ", "backspace delete · esc back to the menu"),
    ("results ", "enter next · tab retry · esc menu"),
];

/// Draws the help panel in the middle of `area`.
pub fn render(frame: &mut Frame, area: Rect, width: u16) {
    let lines = lines();
    // A handful of fixed rows, so the conversion can't fail in practice.
    let rows = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let panel_area = centered_area(area, width, rows + BORDER_ROWS);

    let panel = Paragraph::new(lines).block(theme::panel_block(true).title(" help "));
    frame.render_widget(panel, panel_area);
}

fn lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for sentence in ABOUT {
        lines.push(Line::styled(sentence, theme::menu_item()));
    }
    lines.push(Line::default());
    for (screen, keys) in KEYS {
        lines.push(Line::from(vec![
            Span::styled(screen, theme::stat_value()),
            Span::styled(keys, theme::stat_label()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::styled("any key to close", theme::hint()));
    lines
}
