//! Turns terminal input into the app's own `Event` type.

use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::crossterm::event::{self, KeyEvent, KeyEventKind};

pub enum Event {
    /// A key was pressed at time `at`. The time is read here, once, so
    /// `App::update` never needs the clock and tests can pass fixed times.
    Key { key: KeyEvent, at: Instant },
    /// Nothing relevant happened before the timeout. The loop redraws anyway.
    Tick,
}

/// Waits up to `timeout` for input.
///
/// Only key *presses* become `Event::Key`: Windows also reports key releases,
/// which would otherwise type every character twice. Everything else
/// (releases, resizes, mouse, timeout) becomes `Event::Tick`. A resize needs
/// no special handling because the loop redraws on every event.
pub fn next(timeout: Duration) -> Result<Event> {
    if !event::poll(timeout)? {
        return Ok(Event::Tick);
    }
    match event::read()? {
        event::Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Event::Key {
            key,
            at: Instant::now(),
        }),
        _ => Ok(Event::Tick),
    }
}

/// A key press without modifiers at time `at`, for tests.
#[cfg(test)]
pub fn press_at(code: ratatui::crossterm::event::KeyCode, at: Instant) -> Event {
    Event::Key {
        key: KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::NONE),
        at,
    }
}
