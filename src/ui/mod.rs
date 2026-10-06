//! Top-level layout. Reads `App`, never changes it.
//!
//! Also holds the helpers both screens use to wrap the command. They are
//! private here, yet `typing` and `results` can call them: a child module
//! can see everything in its parent module.

mod results;
mod typing;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use crate::app::{App, Screen};
use crate::theme;

const MIN_WIDTH: u16 = 60;
const MAX_WIDTH: u16 = 110;

/// Rows taken by a panel's top and bottom border.
const BORDER_ROWS: u16 = 2;
/// Rows taken by the key hint under a panel.
const HINT_ROWS: u16 = 1;

pub fn render(frame: &mut Frame, app: &App) {
    // Paint the whole screen so the terminal's own background never shows.
    // This block has no borders; bordered panels come from `theme::panel_block`.
    frame.render_widget(Block::new().style(theme::app()), frame.area());

    // The width comes first because the height depends on it:
    // a narrower panel wraps the command onto more lines.
    let width = content_width(frame.area().width);

    match &app.screen {
        Screen::Typing => {
            let height = typing::height(&app.session, width);
            let area = centered_area(frame.area(), width, height);
            typing::render(frame, area, &app.session);
        }
        Screen::Results(stats) => {
            let height = results::height(&app.session, width);
            let area = centered_area(frame.area(), width, height);
            results::render(
                frame,
                area,
                &app.session,
                stats,
                &app.profile,
                &app.command().explain,
            );
        }
    }
}

/// 80% of the terminal width, clamped to `MIN_WIDTH..=MAX_WIDTH`,
/// but never wider than the terminal itself.
fn content_width(total: u16) -> u16 {
    // `total - total / 5` is 80% without the overflow risk of `total * 8`.
    let eighty_percent = total - total / 5;
    eighty_percent.clamp(MIN_WIDTH, MAX_WIDTH).min(total)
}

/// A `width` × `height` rect centred in `area`, horizontally and vertically.
/// If `area` is smaller, the layout shrinks the rect to fit.
fn centered_area(area: Rect, width: u16, height: u16) -> Rect {
    let [column] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [centered] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(column);
    centered
}

/// Columns available for text inside a panel `width` wide (minus the two
/// side borders). At least 1, so a tiny terminal never divides by zero.
fn inner_width(width: u16) -> usize {
    usize::from(width.saturating_sub(2)).max(1)
}

/// How many rows a command of `len` characters wraps to inside a panel
/// `width` wide. Must match the line breaks made by `wrap_spans`.
fn command_rows(len: usize, width: u16) -> u16 {
    let rows = len.div_ceil(inner_width(width)).max(1);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// Breaks one-span-per-character text into lines that fill a panel `width`
/// wide (the last line may be shorter). We wrap by character ourselves
/// instead of using `Paragraph::wrap`, so the row count is simple arithmetic
/// and a panel's height always matches what is drawn.
fn wrap_spans(spans: Vec<Span<'static>>, width: u16) -> Vec<Line<'static>> {
    let per_line = inner_width(width);
    let mut lines = Vec::new();
    let mut line = Vec::new();
    for span in spans {
        line.push(span);
        if line.len() == per_line {
            // `line` moves into the Line; start a fresh Vec for the next one.
            lines.push(Line::from(line));
            line = Vec::new();
        }
    }
    if !line.is_empty() {
        lines.push(Line::from(line));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_app;
    use crate::event::press_at;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::KeyCode;
    use std::time::{Duration, Instant};

    const FIND: &str = r#"find /var/log -name '*.log' -mtime +7 -exec gzip {} \;"#;

    fn draw(app: &App, width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal
    }

    /// Types `text` with one keystroke every 200 ms, so the WPM shown in
    /// snapshots never depends on how fast the test runs.
    fn type_text(app: &mut App, text: &str) {
        let mut at = Instant::now();
        for c in text.chars() {
            app.update(press_at(KeyCode::Char(c), at));
            at += Duration::from_millis(200);
        }
    }

    /// The app after typing "fx" (the 'x' is a mistake, should be 'i').
    fn app_with_a_mistake() -> App {
        let mut app = test_app(&[FIND]);
        type_text(&mut app, "fx");
        app
    }

    #[test]
    fn typing_screen_snapshot() {
        insta::assert_snapshot!(draw(&app_with_a_mistake(), 80, 12).backend());
    }

    #[test]
    fn narrow_typing_screen_wraps_command() {
        insta::assert_snapshot!(draw(&app_with_a_mistake(), 40, 12).backend());
    }

    #[test]
    fn results_without_mistakes() {
        let mut app = test_app(&["ls -l | wc -l"]);
        type_text(&mut app, "ls -l | wc -l");
        insta::assert_snapshot!(draw(&app, 80, 10).backend());
    }

    #[test]
    fn results_with_missed_symbols() {
        let mut app = test_app(&[FIND]);
        // Miss both quotes and the opening brace, without correcting them.
        let typed: String = FIND
            .chars()
            .map(|c| if c == '\'' || c == '{' { 'x' } else { c })
            .collect();
        type_text(&mut app, &typed);
        insta::assert_snapshot!(draw(&app, 80, 12).backend());
    }

    #[test]
    fn width_is_eighty_percent_within_bounds() {
        assert_eq!(content_width(100), 80);
    }

    #[test]
    fn width_is_capped_on_wide_terminals() {
        assert_eq!(content_width(200), MAX_WIDTH);
    }

    #[test]
    fn width_has_a_minimum() {
        assert_eq!(content_width(70), MIN_WIDTH);
    }

    #[test]
    fn width_never_exceeds_terminal() {
        assert_eq!(content_width(50), 50);
    }

    #[test]
    fn area_is_centered() {
        let area = centered_area(Rect::new(0, 0, 100, 20), 60, 4);
        assert_eq!(area, Rect::new(20, 8, 60, 4));
    }

    #[test]
    fn short_command_takes_one_row() {
        // Panel 20 wide leaves 18 columns for text.
        assert_eq!(command_rows(5, 20), 1);
    }

    #[test]
    fn command_that_fills_a_row_exactly_takes_one_row() {
        assert_eq!(command_rows(18, 20), 1);
    }

    #[test]
    fn long_command_wraps_onto_more_rows() {
        assert_eq!(command_rows(19, 20), 2);
    }

    #[test]
    fn line_breaks_match_the_row_count() {
        let spans: Vec<Span> = "abcdefghijklmnopqrstuvwxyz"
            .chars()
            .map(|c| Span::raw(c.to_string()))
            .collect();
        let lines = wrap_spans(spans, 20);
        assert_eq!(lines.len(), usize::from(command_rows(26, 20)));
        assert_eq!(lines[0].width(), 18);
    }

    #[test]
    fn tiny_width_does_not_divide_by_zero() {
        assert_eq!(inner_width(0), 1);
        assert_eq!(command_rows(2, 2), 2);
    }
}
