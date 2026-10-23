//! The app's own input type, and turning terminal input into it.
//!
//! `App` only sees `Key` and `Event`, never crossterm's types, so the same
//! `App` runs in the terminal and in the browser (see `web.rs`). Each front
//! end converts its own key events into `Key`.

use web_time::Instant;

/// The keys the app reacts to. Everything else is dropped where the key
/// event is read, so `App` never sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A typed character. Ctrl and Alt shortcuts (Ctrl+C, Alt+x) are not
    /// text, so they never become a `Char`. Shift is fine: 'A' and '|'
    /// are plain characters.
    Char(char),
    Enter,
    Esc,
    Tab,
    Backspace,
    Up,
    Down,
}

pub enum Event {
    /// A key was pressed at time `at`. The time is read here, once, so
    /// `App::update` never needs the clock and tests can pass fixed times.
    Key { key: Key, at: Instant },
    /// Nothing relevant happened before the timeout. The loop redraws
    /// anyway, and the time `at` moves the animations forward.
    Tick { at: Instant },
}

/// Waits up to `timeout` for terminal input.
///
/// Only key *presses* become `Event::Key`: Windows also reports key releases,
/// which would otherwise type every character twice. Everything else
/// (releases, resizes, mouse, timeout, keys we don't use) becomes
/// `Event::Tick`. A resize needs no special handling because the loop
/// redraws on every event.
#[cfg(not(target_arch = "wasm32"))]
pub fn next(timeout: std::time::Duration) -> color_eyre::Result<Event> {
    use ratatui::crossterm::event::{self, KeyEventKind};

    if !event::poll(timeout)? {
        return Ok(Event::Tick { at: Instant::now() });
    }
    let at = Instant::now();
    match event::read()? {
        event::Event::Key(key) if key.kind == KeyEventKind::Press => match from_crossterm(key) {
            Some(key) => Ok(Event::Key { key, at }),
            None => Ok(Event::Tick { at }),
        },
        _ => Ok(Event::Tick { at }),
    }
}

/// The `Key` for a crossterm key event, or `None` if the app doesn't use it.
#[cfg(not(target_arch = "wasm32"))]
fn from_crossterm(key: ratatui::crossterm::event::KeyEvent) -> Option<Key> {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    let is_shortcut = key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    match key.code {
        KeyCode::Char(c) if !is_shortcut => Some(Key::Char(c)),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        _ => None,
    }
}

/// A key press at time `at`, for tests.
#[cfg(test)]
pub fn press_at(key: Key, at: Instant) -> Event {
    Event::Key { key, at }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn ctrl_and_alt_shortcuts_are_not_text() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let alt_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(from_crossterm(ctrl_c), None);
        assert_eq!(from_crossterm(alt_x), None);
    }

    #[test]
    fn shifted_symbols_are_text() {
        let pipe = KeyEvent::new(KeyCode::Char('|'), KeyModifiers::SHIFT);
        assert_eq!(from_crossterm(pipe), Some(Key::Char('|')));
    }

    #[test]
    fn unused_keys_are_dropped() {
        let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(from_crossterm(f1), None);
    }
}
