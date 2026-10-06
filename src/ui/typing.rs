//! The Typing screen: the target command, coloured by what has been typed.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::session::Session;
use crate::theme;

/// Rows taken by the panel's top and bottom border.
const BORDER_ROWS: u16 = 2;
/// Rows taken by the hint under the panel.
const HINT_ROWS: u16 = 1;

/// Total rows this screen needs at `width` columns: the wrapped command,
/// the panel borders and the hint line. No empty rows.
pub fn height(session: &Session, width: u16) -> u16 {
    text_rows(session, width) + BORDER_ROWS + HINT_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, session: &Session) {
    let panel_height = text_rows(session, area.width) + BORDER_ROWS;
    let [panel_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let lines = target_lines(session, inner_width(area.width));
    let text = Paragraph::new(lines).block(theme::panel_block(true).title(" type "));
    frame.render_widget(text, panel_area);

    let hint = if session.is_finished() {
        "done · esc quit"
    } else {
        "esc quit"
    };
    frame.render_widget(Span::styled(hint, theme::hint()), hint_area);
}

/// Columns available for text inside a panel `width` wide (minus the two
/// side borders). At least 1, so a tiny terminal never divides by zero.
fn inner_width(width: u16) -> usize {
    usize::from(width.saturating_sub(2)).max(1)
}

/// How many lines the command wraps to inside a panel `width` wide.
/// Must match the line breaks made by `target_lines`.
fn text_rows(session: &Session, width: u16) -> u16 {
    let rows = session.target().len().div_ceil(inner_width(width)).max(1);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// The command as lines of exactly `inner_width` characters (the last one
/// may be shorter). We wrap by character ourselves instead of using
/// `Paragraph::wrap`, so the line count is simple arithmetic and the panel
/// height always matches what is drawn.
///
/// One span per character. A mistake still shows the *target* character,
/// so the user sees which symbol they should have typed.
fn target_lines(session: &Session, inner_width: usize) -> Vec<Line<'static>> {
    let target = session.target();
    let typed = session.typed();

    let mut lines = Vec::new();
    let mut spans = Vec::new();
    for (i, &expected) in target.iter().enumerate() {
        let style = if i < typed.len() {
            if typed[i] == expected {
                theme::correct()
            } else {
                theme::incorrect(expected)
            }
        } else if i == typed.len() {
            theme::cursor()
        } else {
            theme::untyped()
        };
        spans.push(Span::styled(expected.to_string(), style));

        if spans.len() == inner_width {
            // `spans` moves into the line; start a fresh Vec for the next one.
            lines.push(Line::from(spans));
            spans = Vec::new();
        }
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Cell;
    use ratatui::style::Style;

    /// Checks every colour/modifier that `style` sets. Fields the style leaves
    /// unset (like the background of a typed char) come from the panel instead.
    fn assert_cell_has(cell: &Cell, style: Style) {
        if let Some(fg) = style.fg {
            assert_eq!(cell.fg, fg);
        }
        if let Some(bg) = style.bg {
            assert_eq!(cell.bg, bg);
        }
        assert!(cell.modifier.contains(style.add_modifier));
    }

    #[test]
    fn chars_are_styled_by_typing_state() {
        let mut session = Session::new("ls | wc");
        for c in ['l', 'x'] {
            session.type_char(c);
        }

        let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &session))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // Text starts inside the border, at (1, 1).
        assert_cell_has(&buffer[(1, 1)], theme::correct());
        assert_cell_has(&buffer[(2, 1)], theme::incorrect('s'));
        assert_eq!(buffer[(2, 1)].symbol(), "s");
        assert_cell_has(&buffer[(3, 1)], theme::cursor());
        assert_cell_has(&buffer[(4, 1)], theme::untyped());
    }

    #[test]
    fn short_command_takes_one_row() {
        // Panel 20 wide leaves 18 columns for text.
        assert_eq!(text_rows(&Session::new("ls -l"), 20), 1);
    }

    #[test]
    fn command_that_fills_a_row_exactly_takes_one_row() {
        assert_eq!(text_rows(&Session::new("abcdefghijklmnopqr"), 20), 1);
    }

    #[test]
    fn long_command_wraps_onto_more_rows() {
        assert_eq!(text_rows(&Session::new("abcdefghijklmnopqrs"), 20), 2);
    }

    #[test]
    fn line_breaks_match_the_row_count() {
        let session = Session::new("abcdefghijklmnopqrstuvwxyz");
        let lines = target_lines(&session, inner_width(20));
        assert_eq!(lines.len(), usize::from(text_rows(&session, 20)));
        assert_eq!(lines[0].width(), 18);
    }

    #[test]
    fn height_counts_text_borders_and_hint() {
        assert_eq!(height(&Session::new("ls -l"), 20), 1 + 2 + 1);
    }

    #[test]
    fn tiny_width_does_not_divide_by_zero() {
        assert_eq!(inner_width(0), 1);
        assert_eq!(text_rows(&Session::new("ls"), 2), 2);
    }
}
