//! The Typing screen: the target command, coloured by what has been typed.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use super::{BORDER_ROWS, HINT_ROWS, command_rows, wrap_spans};
use crate::session::Session;
use crate::theme;

/// Total rows this screen needs at `width` columns: the wrapped command,
/// the panel borders and the hint line. No empty rows.
pub fn height(session: &Session, width: u16) -> u16 {
    command_rows(session.target().len(), width) + BORDER_ROWS + HINT_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, session: &Session) {
    let panel_height = command_rows(session.target().len(), area.width) + BORDER_ROWS;
    let [panel_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let lines = wrap_spans(target_spans(session), area.width);
    let text = Paragraph::new(lines).block(theme::panel_block(true).title(" type "));
    frame.render_widget(text, panel_area);

    frame.render_widget(Span::styled("esc quit", theme::hint()), hint_area);
}

/// One span per target character, styled by how far the user has got.
/// A mistake still shows the *target* character, so the user sees which
/// symbol they should have typed.
fn target_spans(session: &Session) -> Vec<Span<'static>> {
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
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Cell;
    use ratatui::style::Style;
    use std::time::Instant;

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
            session.type_char(c, Instant::now());
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
    fn height_counts_text_borders_and_hint() {
        assert_eq!(height(&Session::new("ls -l"), 20), 1 + 2 + 1);
    }
}
