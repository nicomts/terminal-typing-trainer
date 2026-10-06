//! App state and the `update` function that changes it.
//!
//! `update` never touches the terminal; `ui::render` only reads this state.

use color_eyre::Result;
use color_eyre::eyre::bail;
use rand::rngs::StdRng;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::corpus::{self, Command};
use crate::event::Event;
use crate::session::Session;

pub struct App {
    pub session: Session,
    pub should_quit: bool,
    commands: Vec<Command>,
    /// Index into `commands` of the command being typed. Always valid,
    /// because `commands` is never empty and never changes.
    current: usize,
    /// Passed in rather than created here, so tests can use a fixed seed.
    rng: StdRng,
}

impl App {
    pub fn new(commands: Vec<Command>, mut rng: StdRng) -> Result<Self> {
        if commands.is_empty() {
            bail!("cannot start without any commands");
        }
        let current = corpus::pick(commands.len(), &mut rng);
        let session = Session::new(&commands[current].text);
        Ok(App {
            session,
            should_quit: false,
            commands,
            current,
            rng,
        })
    }

    /// The command being typed.
    pub fn command(&self) -> &Command {
        &self.commands[self.current]
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
            KeyCode::Enter if self.session.is_finished() => self.next_round(),
            KeyCode::Char(c) if !is_shortcut => self.session.type_char(c),
            KeyCode::Backspace => self.session.backspace(),
            _ => {}
        }
    }

    /// Starts a new round with a different random command.
    fn next_round(&mut self) {
        self.current = corpus::pick_next(self.commands.len(), self.current, &mut self.rng);
        self.session = Session::new(&self.commands[self.current].text);
    }
}

/// An `App` over a small, fixed corpus with a fixed seed, for tests.
#[cfg(test)]
pub fn test_app(texts: &[&str]) -> App {
    use rand::SeedableRng;

    let commands = texts
        .iter()
        .map(|text| Command {
            text: text.to_string(),
            explain: "Explain this".to_string(),
            tags: vec!["test".to_string()],
        })
        .collect();
    App::new(commands, StdRng::seed_from_u64(7)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.update(press(KeyCode::Char(c)));
        }
    }

    #[test]
    fn empty_corpus_is_an_error() {
        use rand::SeedableRng;
        assert!(App::new(Vec::new(), StdRng::seed_from_u64(7)).is_err());
    }

    #[test]
    fn esc_quits() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn chars_are_typed() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Char('l')));
        assert_eq!(app.session.typed(), &['l']);
    }

    #[test]
    fn ctrl_c_does_not_type_c() {
        let mut app = test_app(&["ls"]);
        app.update(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn shifted_symbols_are_typed() {
        let mut app = test_app(&["ls"]);
        app.update(Event::Key(KeyEvent::new(
            KeyCode::Char('|'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(app.session.typed(), &['|']);
    }

    #[test]
    fn backspace_removes_a_char() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Char('l')));
        app.update(press(KeyCode::Backspace));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn enter_does_nothing_before_the_round_is_finished() {
        let mut app = test_app(&["ls", "pwd"]);
        let before = app.command().text.clone();
        app.update(press(KeyCode::Char('x')));
        app.update(press(KeyCode::Enter));
        assert_eq!(app.command().text, before);
        assert_eq!(app.session.typed(), &['x']);
    }

    #[test]
    fn enter_after_finishing_starts_a_different_command() {
        let mut app = test_app(&["ls", "pwd"]);
        let first = app.command().text.clone();
        type_text(&mut app, &first);
        app.update(press(KeyCode::Enter));

        assert_ne!(app.command().text, first);
        assert!(app.session.typed().is_empty());
        let target: String = app.session.target().iter().collect();
        assert_eq!(target, app.command().text);
    }
}
