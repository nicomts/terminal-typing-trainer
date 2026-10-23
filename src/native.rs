//! The terminal app: takes over the terminal and runs the event loop.

use std::env;
use std::path::Path;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use rand::SeedableRng;
use rand::rngs::StdRng;
use ratatui::DefaultTerminal;
use web_time::Instant;

use crate::app::App;
use crate::{corpus, event, storage, ui};

/// How long to wait for a key before redrawing anyway. 33 ms is about 30
/// frames a second, enough for smooth animations. ratatui only sends the
/// cells that changed to the terminal, so a still screen costs very little.
const TICK_RATE: Duration = Duration::from_millis(33);
/// Environment variable that fixes the random seed.
const SEED_VAR: &str = "TYPE_TRAINER_SEED";

pub fn main() -> Result<()> {
    // Install color-eyre first: `ratatui::init` then wraps its panic hook,
    // so on a panic the terminal is restored *before* the report is printed.
    color_eyre::install()?;
    // Load before taking over the terminal, so errors print normally.
    let commands = corpus::load()?;
    let stats_file = storage::data_file()?;
    let profile = storage::load(&stats_file)?;
    let app = App::new(commands, profile, make_rng()?, Instant::now())?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, app, &stats_file);
    // Restore even when `run` returned an error, then hand the error on.
    ratatui::restore();
    result
}

/// A random generator seeded from the OS, or from `TYPE_TRAINER_SEED` if
/// set. A fixed seed always picks the same commands, which the scripted
/// screenshots in `docs/` rely on.
fn make_rng() -> Result<StdRng> {
    match env::var(SEED_VAR) {
        Ok(seed) => {
            let seed: u64 = seed
                .parse()
                .wrap_err_with(|| format!("{SEED_VAR} must be a whole number, not {seed:?}"))?;
            Ok(StdRng::seed_from_u64(seed))
        }
        Err(_) => Ok(rand::make_rng()),
    }
}

fn run(terminal: &mut DefaultTerminal, mut app: App, stats_file: &Path) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;
        let event = event::next(TICK_RATE)?;
        app.update(event);

        // `update` only flags the change; file I/O happens here, outside it.
        if app.profile_changed {
            storage::save(stats_file, &app.profile)?;
            app.profile_changed = false;
        }
    }
    Ok(())
}
