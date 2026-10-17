//! App state and the `update` function that changes it.
//!
//! `update` never touches the terminal; `ui::render` only reads this state.
//!
//! Animations need to know the time, but `render` must not read the clock,
//! so every event carries its time and `update` stores it in `now`, next to
//! the moments animations measure from (`screen_since`, `mistake_at`, ...).

use std::time::Instant;

use color_eyre::Result;
use color_eyre::eyre::bail;
use rand::rngs::StdRng;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::corpus::{self, Command, Topic};
use crate::event::Event;
use crate::session::Session;
use crate::stats::{self, Profile, RoundStats};

/// Rows in the menu: "all topics", then one per topic.
pub const MENU_ROWS: usize = 1 + Topic::ALL.len();

/// Which screen is showing. A variant can carry data: the Menu holds which
/// row is selected, and the Results screen owns the stats of the round it
/// shows, computed once when the round ended.
pub enum Screen {
    /// `selected` is a row index, `0..MENU_ROWS`.
    Menu {
        selected: usize,
    },
    Typing,
    /// `new_best` is true when this round beat the best wpm of earlier rounds.
    Results {
        stats: RoundStats,
        new_best: bool,
    },
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
    /// Time of the latest event. The "current time" for everything drawn.
    pub now: Instant,
    /// When the current screen appeared, for its fade-in and count-ups.
    pub screen_since: Instant,
    /// Time of the latest key press, so the cursor can stay solid while
    /// the user types and only blink when they pause.
    pub last_key_at: Option<Instant>,
    /// Time of the latest mistyped key, for the border flash.
    pub mistake_at: Option<Instant>,
    commands: Vec<Command>,
    /// Index into `commands` of the command being typed. Always valid,
    /// because `commands` is never empty and never changes.
    current: usize,
    /// The topic chosen in the menu; `None` is "all topics".
    topic: Option<Topic>,
    /// Passed in rather than created here, so tests can use a fixed seed.
    rng: StdRng,
}

impl App {
    /// `now` is the start time. Passed in, like `rng`, so tests control it.
    pub fn new(
        commands: Vec<Command>,
        profile: Profile,
        mut rng: StdRng,
        now: Instant,
    ) -> Result<Self> {
        if commands.is_empty() {
            bail!("cannot start without any commands");
        }
        // The menu shows first, but `session` always needs a command, so
        // pick one now. Starting a round from the menu picks a fresh one.
        let current = corpus::pick_weighted(&commands, &profile, None, None, &mut rng);
        let session = Session::new(&commands[current].text);
        Ok(App {
            screen: Screen::Menu { selected: 0 },
            session,
            should_quit: false,
            profile,
            profile_changed: false,
            now,
            screen_since: now,
            last_key_at: None,
            mistake_at: None,
            commands,
            current,
            topic: None,
            rng,
        })
    }

    /// The command being typed, or just typed on the Results screen.
    pub fn command(&self) -> &Command {
        &self.commands[self.current]
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key { key, at } => {
                self.now = at;
                self.last_key_at = Some(at);
                self.handle_key(key, at);
            }
            Event::Tick { at } => self.now = at,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, at: Instant) {
        // `selected` is a `usize`, which is `Copy`, so matching copies it out
        // instead of borrowing `self.screen`. `{ .. }` matches the Results
        // fields without borrowing them. Either way `self` is free to change inside the arms.
        match self.screen {
            Screen::Menu { selected } => self.handle_menu_key(key, selected),
            Screen::Typing => {
                if key.code == KeyCode::Esc {
                    // The unfinished round is dropped, not recorded.
                    self.open_menu();
                } else {
                    self.handle_typing_key(key, at);
                }
            }
            Screen::Results { .. } => match key.code {
                KeyCode::Enter => self.start_round(),
                KeyCode::Esc => self.open_menu(),
                _ => {}
            },
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent, selected: usize) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Menu {
                    selected: selected.saturating_sub(1),
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::Menu {
                    selected: (selected + 1).min(MENU_ROWS - 1),
                };
            }
            KeyCode::Enter => {
                self.topic = topic_for_row(selected);
                self.start_round();
            }
            _ => {}
        }
    }

    /// Shows the menu with the current topic selected.
    fn open_menu(&mut self) {
        self.set_screen(Screen::Menu {
            selected: row_for_topic(self.topic),
        });
    }

    /// Switches to another screen and restarts its entrance animation.
    /// Moving the menu selection doesn't go through here: it's the same
    /// screen, so it shouldn't fade in again.
    fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.screen_since = self.now;
    }

    fn handle_typing_key(&mut self, key: KeyEvent, at: Instant) {
        // Ctrl+C or Alt+x are shortcuts, not text. Shift is fine:
        // 'A' and '|' arrive as plain characters with the SHIFT flag.
        let is_shortcut = key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

        match key.code {
            KeyCode::Char(c) if !is_shortcut => {
                let mistakes_before = self.session.mistakes().len();
                self.session.type_char(c, at);
                if self.session.mistakes().len() > mistakes_before {
                    self.mistake_at = Some(at);
                }
                if self.session.is_finished() {
                    self.finish_round();
                }
            }
            KeyCode::Backspace => self.session.backspace(),
            _ => {}
        }
    }

    fn finish_round(&mut self) {
        let stats = stats::round_stats(&self.session);
        // Compared before `record_round`, which raises `best_wpm`. The very
        // first round has nothing to beat, so it isn't called a new best.
        let new_best = self.profile.rounds > 0 && stats.wpm > self.profile.best_wpm;
        self.profile.record_round(&self.session, &stats);
        self.profile_changed = true;
        self.set_screen(Screen::Results { stats, new_best });
    }

    /// Starts a new round in the chosen topic with a different command,
    /// favouring weak symbols.
    fn start_round(&mut self) {
        // Borrows three different fields at once (two shared, one mutable);
        // the borrow checker allows that because they don't overlap.
        self.current = corpus::pick_weighted(
            &self.commands,
            &self.profile,
            self.topic,
            Some(self.current),
            &mut self.rng,
        );
        self.session = Session::new(&self.commands[self.current].text);
        self.mistake_at = None;
        self.set_screen(Screen::Typing);
    }
}

/// The topic a menu row stands for. Row 0 is "all topics" (`None`).
pub fn topic_for_row(row: usize) -> Option<Topic> {
    if row == 0 {
        None
    } else {
        Topic::ALL.get(row - 1).copied()
    }
}

/// The menu row of `topic`, the inverse of `topic_for_row`.
fn row_for_topic(topic: Option<Topic>) -> usize {
    match topic {
        None => 0,
        Some(topic) => {
            let mut row = 0;
            for (i, &t) in Topic::ALL.iter().enumerate() {
                if t == topic {
                    row = i + 1;
                }
            }
            row
        }
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
            topic: Topic::Files,
        })
        .collect();
    App::new(
        commands,
        Profile::default(),
        StdRng::seed_from_u64(7),
        Instant::now(),
    )
    .unwrap()
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

    /// A test app past the menu, on the Typing screen, in "all topics".
    fn started_app(texts: &[&str]) -> App {
        let mut app = test_app(texts);
        app.update(press(KeyCode::Enter));
        app
    }

    fn is_results(app: &App) -> bool {
        matches!(app.screen, Screen::Results { .. })
    }

    fn is_typing(app: &App) -> bool {
        matches!(app.screen, Screen::Typing)
    }

    /// The selected menu row, or `None` if the menu isn't showing.
    fn menu_row(app: &App) -> Option<usize> {
        match app.screen {
            Screen::Menu { selected } => Some(selected),
            _ => None,
        }
    }

    #[test]
    fn empty_corpus_is_an_error() {
        use rand::SeedableRng;
        let rng = StdRng::seed_from_u64(7);
        assert!(App::new(Vec::new(), Profile::default(), rng, Instant::now()).is_err());
    }

    #[test]
    fn starts_on_the_menu() {
        let app = test_app(&["ls"]);
        assert_eq!(menu_row(&app), Some(0));
    }

    #[test]
    fn esc_on_the_menu_quits() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn q_on_the_menu_quits() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn arrows_move_the_selection_within_bounds() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Up));
        assert_eq!(menu_row(&app), Some(0));
        app.update(press(KeyCode::Down));
        app.update(press(KeyCode::Char('j')));
        assert_eq!(menu_row(&app), Some(2));
        for _ in 0..10 {
            app.update(press(KeyCode::Down));
        }
        assert_eq!(menu_row(&app), Some(MENU_ROWS - 1));
        app.update(press(KeyCode::Char('k')));
        assert_eq!(menu_row(&app), Some(MENU_ROWS - 2));
    }

    #[test]
    fn enter_on_the_menu_starts_typing() {
        let app = started_app(&["ls"]);
        assert!(is_typing(&app));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn chosen_topic_limits_the_commands() {
        let mut app = test_app(&["ls", "pwd", "a | b", "a & b"]);
        app.commands[2].topic = Topic::Shell;
        app.commands[3].topic = Topic::Shell;
        // Row 3 is the third topic, shell.
        for _ in 0..3 {
            app.update(press(KeyCode::Down));
        }
        app.update(press(KeyCode::Enter));

        for _ in 0..20 {
            assert_eq!(app.command().topic, Topic::Shell);
            let text = app.command().text.clone();
            type_text(&mut app, &text);
            app.update(press(KeyCode::Enter));
        }
    }

    #[test]
    fn esc_while_typing_returns_to_the_menu_without_recording() {
        let mut app = started_app(&["ls"]);
        type_text(&mut app, "l");
        app.update(press(KeyCode::Esc));

        assert_eq!(menu_row(&app), Some(0));
        assert!(!app.should_quit);
        assert_eq!(app.profile.rounds, 0);
        assert!(!app.profile_changed);
    }

    #[test]
    fn esc_on_results_returns_to_the_menu_at_the_chosen_topic() {
        let mut app = test_app(&["ls"]);
        app.update(press(KeyCode::Down));
        app.update(press(KeyCode::Enter));
        type_text(&mut app, "ls");
        app.update(press(KeyCode::Esc));
        assert_eq!(menu_row(&app), Some(1));
        assert!(!app.should_quit);
    }

    #[test]
    fn rows_and_topics_match() {
        for row in 0..MENU_ROWS {
            assert_eq!(row_for_topic(topic_for_row(row)), row);
        }
    }

    #[test]
    fn chars_are_typed() {
        let mut app = started_app(&["ls"]);
        app.update(press(KeyCode::Char('l')));
        assert_eq!(app.session.typed(), &['l']);
    }

    #[test]
    fn q_while_typing_is_typed() {
        let mut app = started_app(&["ls"]);
        app.update(press(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert_eq!(app.session.typed(), &['q']);
    }

    #[test]
    fn ctrl_c_does_not_type_c() {
        let mut app = started_app(&["ls"]);
        app.update(press_with(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn shifted_symbols_are_typed() {
        let mut app = started_app(&["ls"]);
        app.update(press_with(KeyCode::Char('|'), KeyModifiers::SHIFT));
        assert_eq!(app.session.typed(), &['|']);
    }

    #[test]
    fn backspace_removes_a_char() {
        let mut app = started_app(&["ls"]);
        app.update(press(KeyCode::Char('l')));
        app.update(press(KeyCode::Backspace));
        assert!(app.session.typed().is_empty());
    }

    #[test]
    fn finishing_the_command_shows_results() {
        let mut app = started_app(&["ls"]);
        type_text(&mut app, "l");
        assert!(!is_results(&app));
        type_text(&mut app, "s");
        assert!(is_results(&app));
    }

    #[test]
    fn finishing_a_round_updates_the_profile() {
        let mut app = started_app(&["a | b"]);
        assert!(!app.profile_changed);
        type_text(&mut app, "a x b");

        assert!(app.profile_changed);
        assert_eq!(app.profile.rounds, 1);
        assert_eq!(app.profile.symbols[&'|'].missed, 1);
    }

    #[test]
    fn enter_does_nothing_while_typing() {
        let mut app = started_app(&["ls", "pwd"]);
        let before = app.command().text.clone();
        app.update(press(KeyCode::Char('x')));
        app.update(press(KeyCode::Enter));
        assert_eq!(app.command().text, before);
        assert_eq!(app.session.typed(), &['x']);
    }

    #[test]
    fn typing_on_results_does_nothing() {
        let mut app = started_app(&["ls", "pwd"]);
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
        let mut app = started_app(&["ls", "pwd"]);
        // Cloned so the borrow of `app` ends here; typing needs `&mut app`.
        let first = app.command().text.clone();
        type_text(&mut app, &first);
        app.update(press(KeyCode::Enter));

        assert!(is_typing(&app));
        assert_ne!(app.command().text, first);
        assert!(app.session.typed().is_empty());
        let target: String = app.session.target().iter().collect();
        assert_eq!(target, app.command().text);
    }

    #[test]
    fn events_move_the_clock() {
        let mut app = test_app(&["ls"]);
        let later = app.now + std::time::Duration::from_secs(3);
        app.update(Event::Tick { at: later });
        assert_eq!(app.now, later);
    }

    #[test]
    fn a_new_screen_restarts_its_animation() {
        let mut app = test_app(&["ls"]);
        let at = app.now + std::time::Duration::from_secs(2);
        app.update(press_at(KeyCode::Enter, at));
        assert!(is_typing(&app));
        assert_eq!(app.screen_since, at);
    }

    #[test]
    fn moving_the_menu_selection_does_not_restart_it() {
        let mut app = test_app(&["ls"]);
        let since = app.screen_since;
        let at = app.now + std::time::Duration::from_secs(2);
        app.update(press_at(KeyCode::Down, at));
        assert_eq!(app.screen_since, since);
    }

    #[test]
    fn a_wrong_key_sets_mistake_at_and_a_right_one_does_not() {
        let mut app = started_app(&["ls"]);
        app.update(press(KeyCode::Char('l')));
        assert_eq!(app.mistake_at, None);
        let at = Instant::now();
        app.update(press_at(KeyCode::Char('x'), at));
        assert_eq!(app.mistake_at, Some(at));
    }

    /// Types `text` one key per `step` from `start`, as a round of known speed.
    fn type_timed(app: &mut App, text: &str, start: Instant, step: std::time::Duration) {
        let mut at = start;
        for c in text.chars() {
            app.update(press_at(KeyCode::Char(c), at));
            at += step;
        }
    }

    fn new_best(app: &App) -> bool {
        matches!(app.screen, Screen::Results { new_best: true, .. })
    }

    #[test]
    fn first_round_is_not_a_new_best_but_a_faster_one_is() {
        use std::time::Duration;
        let mut app = started_app(&["abcdef", "ghijkl"]);
        let start = Instant::now();

        let text = app.command().text.clone();
        type_timed(&mut app, &text, start, Duration::from_millis(500));
        assert!(!new_best(&app));

        app.update(press(KeyCode::Enter));
        let text = app.command().text.clone();
        type_timed(&mut app, &text, start, Duration::from_millis(100));
        assert!(new_best(&app));
    }

    #[test]
    fn a_slower_round_is_not_a_new_best() {
        use std::time::Duration;
        let mut app = started_app(&["abcdef", "ghijkl"]);
        let start = Instant::now();

        let text = app.command().text.clone();
        type_timed(&mut app, &text, start, Duration::from_millis(100));
        app.update(press(KeyCode::Enter));
        let text = app.command().text.clone();
        type_timed(&mut app, &text, start, Duration::from_millis(500));
        assert!(!new_best(&app));
    }
}
