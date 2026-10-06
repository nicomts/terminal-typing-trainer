//! The Typing screen: the target command, coloured by what has been typed.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::session::Session;
use crate::theme;

/// Panel with room for 3 lines of text (plus 2 border rows), then a hint line.
const PANEL_HEIGHT: u16 = 5;
pub const HEIGHT: u16 = PANEL_HEIGHT + 1;

pub fn render(frame: &mut Frame, area: Rect, session: &Session) {
    let [panel_area, hint_area] =
        Layout::vertical([Constraint::Length(PANEL_HEIGHT), Constraint::Length(1)]).areas(area);

    let text = Paragraph::new(target_line(session))
        .wrap(Wrap { trim: false })
        .block(theme::panel_block(true).title(" type "));
    frame.render_widget(text, panel_area);

    let hint = if session.is_finished() {
        "done · esc quit"
    } else {
        "esc quit"
    };
    frame.render_widget(Span::styled(hint, theme::hint()), hint_area);
}

/// One span per target character. A mistake still shows the *target*
/// character, so the user sees which symbol they should have typed.
fn target_line(session: &Session) -> Line<'static> {
    let target = session.target();
    let typed = session.typed();

    let mut spans = Vec::with_capacity(target.len());
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
    }
    Line::from(spans)
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
}
