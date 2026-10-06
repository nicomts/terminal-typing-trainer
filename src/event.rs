//! Turns terminal input into the app's own `Event` type.

use std::time::Duration;

use color_eyre::Result;
use ratatui::crossterm::event::{self, KeyEvent, KeyEventKind};

pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
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
        event::Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Event::Key(key)),
        _ => Ok(Event::Tick),
    }
}
