mod app;
mod event;
mod session;
mod theme;
mod ui;

use std::time::Duration;

use color_eyre::Result;
use ratatui::DefaultTerminal;

use crate::app::App;

/// How long to wait for a key before redrawing anyway.
const TICK_RATE: Duration = Duration::from_millis(100);

fn main() -> Result<()> {
    // Install color-eyre first: `ratatui::init` then wraps its panic hook,
    // so on a panic the terminal is restored *before* the report is printed.
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    // Restore even when `run` returned an error, then hand the error on.
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;
        let event = event::next(TICK_RATE)?;
        app.update(event);
    }
    Ok(())
}
