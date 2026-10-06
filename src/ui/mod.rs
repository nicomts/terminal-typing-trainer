//! Top-level layout. Reads `App`, never changes it.

mod typing;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::Block;

use crate::app::App;
use crate::theme;

const MIN_WIDTH: u16 = 60;
const MAX_WIDTH: u16 = 110;

pub fn render(frame: &mut Frame, app: &App) {
    // Paint the whole screen so the terminal's own background never shows.
    // This block has no borders; bordered panels come from `theme::panel_block`.
    frame.render_widget(Block::new().style(theme::app()), frame.area());

    // The width comes first because the height depends on it:
    // a narrower panel wraps the command onto more lines.
    let width = content_width(frame.area().width);
    let height = typing::height(&app.session, width);
    let area = centered_area(frame.area(), width, height);

    typing::render(frame, area, &app.session);
}

/// 80% of the terminal width, clamped to `MIN_WIDTH..=MAX_WIDTH`,
/// but never wider than the terminal itself.
fn content_width(total: u16) -> u16 {
    // `total - total / 5` is 80% without the overflow risk of `total * 8`.
    let eighty_percent = total - total / 5;
    eighty_percent.clamp(MIN_WIDTH, MAX_WIDTH).min(total)
}

/// A `width` × `height` rect centred in `area`, horizontally and vertically.
/// If `area` is smaller, the layout shrinks the rect to fit.
fn centered_area(area: Rect, width: u16, height: u16) -> Rect {
    let [column] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [centered] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(column);
    centered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Renders the app after typing "fx" (the 'x' is a mistake, should be 'i').
    fn render_typing_screen(width: u16, height: u16) -> Terminal<TestBackend> {
        let mut app = App::new();
        for c in ['f', 'x'] {
            app.update(Event::Key(KeyEvent::new(
                KeyCode::Char(c),
                KeyModifiers::NONE,
            )));
        }

        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        terminal
    }

    #[test]
    fn typing_screen_snapshot() {
        insta::assert_snapshot!(render_typing_screen(80, 12).backend());
    }

    #[test]
    fn narrow_typing_screen_wraps_command() {
        insta::assert_snapshot!(render_typing_screen(40, 12).backend());
    }

    #[test]
    fn width_is_eighty_percent_within_bounds() {
        assert_eq!(content_width(100), 80);
    }

    #[test]
    fn width_is_capped_on_wide_terminals() {
        assert_eq!(content_width(200), MAX_WIDTH);
    }

    #[test]
    fn width_has_a_minimum() {
        assert_eq!(content_width(70), MIN_WIDTH);
    }

    #[test]
    fn width_never_exceeds_terminal() {
        assert_eq!(content_width(50), 50);
    }

    #[test]
    fn area_is_centered() {
        let area = centered_area(Rect::new(0, 0, 100, 20), 60, 4);
        assert_eq!(area, Rect::new(20, 8, 60, 4));
    }
}
