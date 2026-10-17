//! The Typing screen: the target command, coloured by what has been typed,
//! with live speed and time, a progress bar, a blinking cursor and a border
//! that flashes when a key is wrong.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{BAR_ROWS, BORDER_ROWS, HINT_ROWS, bar_line, command_rows, inner_width, wrap_spans};
use crate::anim;
use crate::app::App;
use crate::session::Session;
use crate::stats;
use crate::theme;

/// A full on-and-off cycle of the cursor. Measured from the latest key
/// press, so the first half (solid) always follows a keystroke: the cursor
/// only starts blinking once the user pauses.
const BLINK_PERIOD: Duration = Duration::from_millis(1060);
/// How long the border stays tinted after a wrong key.
const MISTAKE_FLASH: Duration = Duration::from_millis(250);

/// Total rows this screen needs at `width` columns: the wrapped command,
/// the panel borders, the progress bar and the hint line. No empty rows.
pub fn height(session: &Session, width: u16) -> u16 {
    command_rows(session.target().len(), width) + BORDER_ROWS + BAR_ROWS + HINT_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let session = &app.session;
    let panel_height = command_rows(session.target().len(), area.width) + BORDER_ROWS;
    let [panel_area, bar_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height),
        Constraint::Length(BAR_ROWS),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let mut block = theme::panel_block(true)
        .title(" type ")
        .title_top(live_stats(app).right_aligned());
    if let Some(mistake_at) = app.mistake_at {
        let t = anim::progress(mistake_at, app.now, MISTAKE_FLASH);
        if t < 1.0 {
            block = block.border_style(theme::mistake_border(t));
        }
    }

    let lines = wrap_spans(target_spans(session, cursor_style(app)), area.width);
    frame.render_widget(Paragraph::new(lines).block(block), panel_area);

    // The bar is as wide as the text inside the panel, and lines up with it.
    let done = session.typed().len() as f64 / session.target().len().max(1) as f64;
    let bar_area = bar_area.inner(Margin::new(1, 0));
    frame.render_widget(bar_line(done, inner_width(area.width)), bar_area);

    frame.render_widget(Span::styled("esc menu", theme::hint()), hint_area);
}

/// " 42 wpm · 0:07 " once typing has started, " start typing " before.
fn live_stats(app: &App) -> Line<'static> {
    let Some(started_at) = app.session.started_at() else {
        return Line::styled(" start typing ", theme::hint());
    };
    let seconds = app.now.saturating_duration_since(started_at).as_secs();
    let wpm = stats::live_wpm(&app.session, app.now);
    Line::from(vec![
        Span::styled(format!(" {wpm:.0}"), theme::stat_value()),
        Span::styled(" wpm · ", theme::stat_label()),
        Span::styled(
            format!("{}:{:02} ", seconds / 60, seconds % 60),
            theme::stat_value(),
        ),
    ])
}

/// The solid block during the first half of each blink, a thin underline
/// during the second half.
fn cursor_style(app: &App) -> Style {
    let since = app.last_key_at.unwrap_or(app.screen_since);
    if anim::phase(since, app.now, BLINK_PERIOD) < 0.5 {
        theme::cursor()
    } else {
        theme::cursor_dim()
    }
}

/// One span per target character, styled by how far the user has got.
/// A mistake still shows the *target* character, so the user sees which
/// symbol they should have typed.
fn target_spans(session: &Session, cursor: Style) -> Vec<Span<'static>> {
    let target = session.target();
    let typed = session.typed();

    let mut spans = Vec::with_capacity(target.len());
    for (i, &expected) in target.iter().enumerate() {
        let style = if i < typed.len() {
            if typed[i] == expected {
                theme::correct()
            } else {
                theme::incorrect(expected)
            }
        } else if i == typed.len() {
            cursor
        } else {
            theme::untyped()
        };
        spans.push(Span::styled(expected.to_string(), style));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_app;
    use crate::event::{Event, press_at};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::crossterm::event::KeyCode;
    use std::time::Instant;

    /// Checks every colour/modifier that `style` sets. Fields the style leaves
    /// unset (like the background of a typed char) come from the panel instead.
    fn assert_cell_has(cell: &Cell, style: Style) {
        if let Some(fg) = style.fg {
            assert_eq!(cell.fg, fg);
        }
        if let Some(bg) = style.bg {
            assert_eq!(cell.bg, bg);
        }
        assert!(cell.modifier.contains(style.add_modifier));
    }

    /// An app on the Typing screen for "ls | wc", after typing "lx" at `t0`.
    fn app_after_lx(t0: Instant) -> App {
        let mut app = test_app(&["ls | wc"]);
        app.update(press_at(KeyCode::Enter, t0));
        app.update(press_at(KeyCode::Char('l'), t0));
        app.update(press_at(KeyCode::Char('x'), t0));
        app
    }

    fn draw(app: &App) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), app))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn chars_are_styled_by_typing_state() {
        let buffer = draw(&app_after_lx(Instant::now()));

        // Text starts inside the border, at (1, 1).
        assert_cell_has(&buffer[(1, 1)], theme::correct());
        assert_cell_has(&buffer[(2, 1)], theme::incorrect('s'));
        assert_eq!(buffer[(2, 1)].symbol(), "s");
        assert_cell_has(&buffer[(3, 1)], theme::cursor());
        assert_cell_has(&buffer[(4, 1)], theme::untyped());
    }

    #[test]
    fn cursor_blinks_after_a_pause() {
        let t0 = Instant::now();
        let mut app = app_after_lx(t0);
        // 600 ms after the last key is in the second half of the blink.
        app.update(Event::Tick {
            at: t0 + Duration::from_millis(600),
        });
        assert_cell_has(&draw(&app)[(3, 1)], theme::cursor_dim());
    }

    #[test]
    fn border_flashes_after_a_mistake_then_settles() {
        let t0 = Instant::now();
        let mut app = app_after_lx(t0);
        assert_cell_has(&draw(&app)[(0, 0)], theme::mistake_border(0.0));

        app.update(Event::Tick {
            at: t0 + Duration::from_secs(1),
        });
        assert_cell_has(&draw(&app)[(0, 0)], theme::border_focused());
    }

    #[test]
    fn height_counts_text_borders_bar_and_hint() {
        assert_eq!(height(&Session::new("ls -l"), 20), 1 + 2 + 1 + 1);
    }
}
