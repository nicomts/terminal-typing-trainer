//! App state and the `update` function that changes it.
//!
//! `update` never touches the terminal; `ui::render` only reads this state.

use std::time::Instant;

use color_eyre::Result;
use color_eyre::eyre::bail;
use rand::rngs::StdRng;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::corpus::{self, Command};
use crate::event::Event;
use crate::session::Session;
use crate::stats::{self, Profile, RoundStats};

/// Which screen is showing. A variant can carry data: the Results screen
/// owns the stats of the round it shows, computed once when the round ended.
pub enum Screen {
    Typing,
    Results(RoundStats),
}

pub struct App {
    pub screen: Screen,
    pub session: Session,
    pub should_quit: bool,
    /// Totals over every round, loaded at start and saved after each round.
    pub profile: Profile,
    /// Set when `profile` has changed. `update` never writes files: the main
    /// loop sees this flag, saves, and clears it.
    pub profile_changed: bool,
    commands: Vec<Command>,
    /// Index into `commands` of the command being typed. Always valid,
    /// because `commands` is never empty and never changes.
    current: usize,
    /// Passed in rather than created here, so tests can use a fixed seed.
    rng: StdRng,
}

impl App {
    pub fn new(commands: Vec<Command>, profile: Profile, mut rng: StdRng) -> Result<Self> {
        if commands.is_empty() {
            bail!("cannot start without any commands");
        }
        let current = corpus::pick_weighted(&commands, &profile, None, &mut rng);
        let session = Session::new(&commands[current].text);
        Ok(App {
            screen: Screen::Typing,
            session,
            should_quit: false,
            profile,
            profile_changed: false,
            commands,
            current,
            rng,
        })
    }

    /// The command being typed, or just typed on the Results screen.
    pub fn command(&self) -> &Command {
        &self.commands[self.current]
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key { key, at } => self.handle_key(key, at),
            Event::Tick => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent, at: Instant) {
        if key.code == KeyCode::Esc {
            self.should_quit = true;
            return;
        }
        // `_` matches the stats without borrowing them, so `self` is free
        // to be changed inside the arms.
        match self.screen {
            Screen::Typing => self.handle_typing_key(key, at),
            Screen::Results(_) => {
                if key.code == KeyCode::Enter {
                    self.next_round();
                }
            }
        }
    }

    fn handle_typing_key(&mut self, key: KeyEvent, at: Instant) {
        // Ctrl+C or Alt+x are shortcuts, not text. Shift is fine:
        // 'A' and '|' arrive as plain characters with the SHIFT flag.
        let is_shortcut = key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

        match key.code {
            KeyCode::Char(c) if !is_shortcut => {
                self.session.type_char(c, at);
                if self.session.is_finished() {
                    let stats = stats::round_stats(&self.session);
                    self.profile.record_round(&self.session, &stats);
                    self.profile_changed = true;
                    self.screen = Screen::Results(stats);
                }
            }
            KeyCode::Backspace => self.session.backspace(),
            _ => {}
        }
    }

    /// Starts a new round with a different command, favouring weak symbols.
    fn next_round(&mut self) {
        // Borrows three different fields at once (two shared, one mutable);
        // the borrow checker allows that because they don't overlap.
        self.current = corpus::pick_weighted(
            &self.commands,
            &self.profile,
            Some(self.current),
            &mut self.rng,
        );
        self.session = Session::new(&self.commands[self.current].text);
        self.screen = Screen::Typing;
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
    App::new(commands, Profile::default(), StdRng::seed_from_u64(7)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::press_at;

    fn press(code: KeyCode) -> Event {
        press_at(code, Instant::now())
    }

    fn press_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key {
            key: KeyEvent::new(code, modifiers),
            at: Instant::now(),
        }
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.update(press(KeyCode::Char(c)));
        }
    }

    fn is_results(app: &App) -> bool {
        matches!(app.screen, Screen::Results(_))
    }

    #[test]
    fn empty_corpus_is_an_error() {
        use rand::SeedableRng;
        assert!(App::new(Vec::new(), Profile::default(), StdRng::seed_from_u64(7)).is_err());
    }

    #[test]
    fn esc_quits() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn esc_quits_from_results() {
        let mut app = test_app(&["ls"]);
        type_text(&mut app, "ls");
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
        app.update(press_with(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn shifted_symbols_are_typed() {
        let mut app = test_app(&["ls"]);
        app.update(press_with(KeyCode::Char('|'), KeyModifiers::SHIFT));
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
    fn finishing_the_command_shows_results() {
        let mut app = test_app(&["ls"]);
        type_text(&mut app, "l");
        assert!(!is_results(&app));
        type_text(&mut app, "s");
        assert!(is_results(&app));
    }

    #[test]
    fn finishing_a_round_updates_the_profile() {
        let mut app = test_app(&["a | b"]);
        assert!(!app.profile_changed);
        type_text(&mut app, "a x b");

        assert!(app.profile_changed);
        assert_eq!(app.profile.rounds, 1);
        assert_eq!(app.profile.symbols[&'|'].missed, 1);
    }

    #[test]
    fn enter_does_nothing_while_typing() {
        let mut app = test_app(&["ls", "pwd"]);
        let before = app.command().text.clone();
        app.update(press(KeyCode::Char('x')));
        app.update(press(KeyCode::Enter));
        assert_eq!(app.command().text, before);
        assert_eq!(app.session.typed(), &['x']);
    }

    #[test]
    fn typing_on_results_does_nothing() {
        let mut app = test_app(&["ls", "pwd"]);
        let first = app.command().text.clone();
        type_text(&mut app, &first);
        type_text(&mut app, "abc");
        app.update(press(KeyCode::Backspace));

        assert!(is_results(&app));
        assert_eq!(app.command().text, first);
        assert!(app.session.is_finished());
    }

    #[test]
    fn enter_on_results_starts_a_different_command() {
        let mut app = test_app(&["ls", "pwd"]);
        // Cloned so the borrow of `app` ends here; typing needs `&mut app`.
        let first = app.command().text.clone();
        type_text(&mut app, &first);
        app.update(press(KeyCode::Enter));

        assert!(!is_results(&app));
        assert_ne!(app.command().text, first);
        assert!(app.session.typed().is_empty());
        let target: String = app.session.target().iter().collect();
        assert_eq!(target, app.command().text);
    }
}
