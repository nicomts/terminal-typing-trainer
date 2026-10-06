mod app;
mod corpus;
mod event;
mod session;
mod stats;
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
    // Load before taking over the terminal, so a corpus error prints normally.
    let commands = corpus::load()?;
    let app = App::new(commands, rand::make_rng())?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, app);
    // Restore even when `run` returned an error, then hand the error on.
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;
        let event = event::next(TICK_RATE)?;
        app.update(event);
    }
    Ok(())
}
