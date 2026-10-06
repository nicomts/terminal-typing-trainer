//! App state and the `update` function that changes it.
//!
//! `update` never touches the terminal; `ui::render` only reads this state.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::event::Event;
use crate::session::Session;

/// The single command for milestone 1. The corpus replaces this later.
pub const COMMAND: &str = r#"find /var/log -name '*.log' -mtime +7 -exec gzip {} \;"#;

pub struct App {
    pub session: Session,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            session: Session::new(COMMAND),
            should_quit: false,
        }
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl+C or Alt+x are shortcuts, not text. Shift is fine:
        // 'A' and '|' arrive as plain characters with the SHIFT flag.
        let is_shortcut = key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char(c) if !is_shortcut => self.session.type_char(c),
            KeyCode::Backspace => self.session.backspace(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn esc_quits() {
        let mut app = App::new();
        app.update(press(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn chars_are_typed() {
        let mut app = App::new();
        app.update(press(KeyCode::Char('f')));
        assert_eq!(app.session.typed(), &['f']);
    }

    #[test]
    fn ctrl_c_does_not_type_c() {
        let mut app = App::new();
        app.update(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn shifted_symbols_are_typed() {
        let mut app = App::new();
        app.update(Event::Key(KeyEvent::new(
            KeyCode::Char('|'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(app.session.typed(), &['|']);
    }

    #[test]
    fn backspace_removes_a_char() {
        let mut app = App::new();
        app.update(press(KeyCode::Char('f')));
        app.update(press(KeyCode::Backspace));
        assert!(app.session.typed().is_empty());
    }
}
