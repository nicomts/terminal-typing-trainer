//! Top-level layout. Reads `App`, never changes it.

mod typing;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::widgets::Block;

use crate::app::App;
use crate::theme;

/// Width of the centred content column.
const CONTENT_WIDTH: u16 = 70;

pub fn render(frame: &mut Frame, app: &App) {
    // Paint the whole screen so the terminal's own background never shows.
    // This block has no borders; bordered panels come from `theme::panel_block`.
    frame.render_widget(Block::new().style(theme::app()), frame.area());

    let [column] = Layout::horizontal([Constraint::Length(CONTENT_WIDTH)])
        .flex(Flex::Center)
        .areas(frame.area());
    let [content] = Layout::vertical([Constraint::Length(typing::HEIGHT)])
        .flex(Flex::Center)
        .areas(column);

    typing::render(frame, content, &app.session);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn typing_screen_snapshot() {
        let mut app = App::new();
        // "fx" : the second character is a mistake (should be 'i').
        for c in ['f', 'x'] {
            app.update(Event::Key(KeyEvent::new(
                KeyCode::Char(c),
                KeyModifiers::NONE,
            )));
        }

        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }
}
