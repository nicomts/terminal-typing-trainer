//! Top-level layout. Reads `App`, never changes it.
//!
//! Also holds the helpers the screens share: wrapping the command and the
//! "weakest symbols" line. They are private here, yet the screen modules can
//! call them: a child module can see everything in its parent module.

mod logo;
mod menu;
mod results;
mod typing;

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use crate::anim;
use crate::app::{App, Screen};
use crate::stats::Profile;
use crate::theme;

const MIN_WIDTH: u16 = 60;
const MAX_WIDTH: u16 = 110;

/// Rows taken by a panel's top and bottom border.
const BORDER_ROWS: u16 = 2;
/// Rows taken by the key hint under a panel.
const HINT_ROWS: u16 = 1;
/// Rows taken by a progress or accuracy bar.
const BAR_ROWS: u16 = 1;
/// How long a new screen takes to fade in.
const FADE_IN: Duration = Duration::from_millis(180);
/// How many symbols the "weakest" line lists.
const WEAKEST_SHOWN: usize = 3;

pub fn render(frame: &mut Frame, app: &App) {
    // Paint the whole screen so the terminal's own background never shows.
    // This block has no borders; bordered panels come from `theme::panel_block`.
    frame.render_widget(Block::new().style(theme::app()), frame.area());

    // The width comes first because the height depends on it:
    // a narrower panel wraps the command onto more lines.
    let width = content_width(frame.area().width);

    match &app.screen {
        Screen::Menu { selected } => {
            let area = centered_area(frame.area(), width, menu::height());
            menu::render(frame, area, app, *selected);
        }
        Screen::Typing => {
            let height = typing::height(&app.session, width);
            let area = centered_area(frame.area(), width, height);
            typing::render(frame, area, app);
        }
        Screen::Results { stats, new_best } => {
            let height = results::height(&app.session, width);
            let area = centered_area(frame.area(), width, height);
            results::render(frame, area, app, stats, *new_best);
        }
    }

    fade_in(frame, anim::progress(app.screen_since, app.now, FADE_IN));
}

/// Blends every cell from the background towards its real colors. At
/// `t` = 1.0 (the usual case, once the fade is over) nothing changes.
///
/// Done as one pass over the finished frame, so the screens don't need to
/// know about the fade at all.
fn fade_in(frame: &mut Frame, t: f64) {
    if t >= 1.0 {
        return;
    }
    let t = anim::ease_out(t);
    for cell in frame.buffer_mut().content.iter_mut() {
        cell.fg = theme::fade_in(cell.fg, t);
        cell.bg = theme::fade_in(cell.bg, t);
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

/// Columns a panel uses on each side for its border (1) and the padding
/// inside it (1). Must match `theme::panel_block`.
const PANEL_SIDE_COLUMNS: u16 = 2;

/// Columns available for text inside a panel `width` wide (minus border and
/// padding on both sides). At least 1, so a tiny terminal never divides by
/// zero.
fn inner_width(width: u16) -> usize {
    usize::from(width.saturating_sub(2 * PANEL_SIDE_COLUMNS)).max(1)
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

/// A bar `width` columns wide, filled from the left by `fraction`
/// (0.0 to 1.0) of its length.
fn bar_line(fraction: f64, width: usize) -> Line<'static> {
    let fraction = fraction.clamp(0.0, 1.0);
    // Rounded to whole cells. `as usize` can't overflow here, because the
    // product is between 0 and `width`.
    let filled = (fraction * width as f64).round() as usize;
    Line::from(vec![
        Span::styled("━".repeat(filled), theme::bar_filled()),
        Span::styled("━".repeat(width - filled), theme::bar_empty()),
    ])
}

/// "weakest  ' 46%  { 38%  | 25%": over all rounds, by plain miss rate.
fn weakest_line(profile: &Profile) -> Line<'static> {
    let weakest = profile.weakest(WEAKEST_SHOWN);
    if weakest.is_empty() {
        return Line::styled("no weak symbols yet", theme::hint());
    }
    let mut spans = vec![Span::styled("weakest", theme::hint())];
    for (symbol, miss_rate) in weakest {
        // The space gets its own span: in the mistake style it would be
        // underlined along with the symbol.
        spans.push(Span::styled(" ", theme::hint()));
        spans.push(Span::styled(symbol.to_string(), theme::incorrect(symbol)));
        spans.push(Span::styled(
            format!(" {:.0}% ", miss_rate * 100.0),
            theme::hint(),
        ));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_app;
    use crate::event::{Event, press_at};
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

    /// Lets 10 seconds pass, so every entrance animation (fade-in,
    /// count-up) has finished and only the final state is drawn.
    fn settle(app: &mut App) {
        app.update(Event::Tick {
            at: app.now + Duration::from_secs(10),
        });
    }

    /// Draws `app` once its animations are over.
    fn draw_settled(mut app: App, width: u16, height: u16) -> Terminal<TestBackend> {
        settle(&mut app);
        draw(&app, width, height)
    }

    /// The text of row `y`, for checking what a line says.
    fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text
    }

    /// A test app past the menu, on the Typing screen.
    fn started_app(texts: &[&str]) -> App {
        let mut app = test_app(texts);
        app.update(press_at(KeyCode::Enter, Instant::now()));
        app
    }

    /// The app after typing "fx" (the 'x' is a mistake, should be 'i').
    fn app_with_a_mistake() -> App {
        let mut app = started_app(&[FIND]);
        type_text(&mut app, "fx");
        app
    }

    #[test]
    fn menu_with_empty_profile() {
        insta::assert_snapshot!(draw_settled(test_app(&[FIND]), 80, 16).backend());
    }

    #[test]
    fn menu_with_second_item_selected() {
        let mut app = test_app(&["ls -l | wc -l"]);
        // Play one round with a mistake on '|', so the totals show up.
        app.update(press_at(KeyCode::Enter, Instant::now()));
        type_text(&mut app, "ls -l x wc -l");
        app.update(press_at(KeyCode::Esc, Instant::now()));
        app.update(press_at(KeyCode::Down, Instant::now()));
        insta::assert_snapshot!(draw_settled(app, 80, 16).backend());
    }

    #[test]
    fn menu_selection_is_styled() {
        let terminal = draw_settled(test_app(&[FIND]), 80, 16);
        let buffer = terminal.backend().buffer();
        // The 64-wide panel starts at column 8, the menu is 14 rows centred
        // in 16, and the first item is the 6th row of it. After the border
        // and one column of padding, "▸ all topics" starts at column 10, so
        // its "a" is at column 12.
        let selected = &buffer[(12, 6)];
        assert_eq!(selected.symbol(), "a");
        assert_eq!(Some(selected.fg), theme::menu_selected().fg);
        assert_eq!(Some(selected.bg), theme::menu_selected().bg);
        let other = &buffer[(12, 7)];
        assert_eq!(Some(other.fg), theme::menu_item().fg);
    }

    #[test]
    fn screens_fade_in() {
        let app = test_app(&[FIND]);
        // Drawn at the very moment the menu appeared: everything is still
        // the background color.
        let terminal = draw(&app, 80, 16);
        let cell = &terminal.backend().buffer()[(12, 6)];
        assert_eq!(cell.fg, theme::fade_in(cell.fg, 0.0));
        assert_ne!(Some(cell.fg), theme::menu_selected().fg);
    }

    #[test]
    fn results_count_up_to_the_final_numbers() {
        let mut app = started_app(&["ls -l | wc -l"]);
        type_text(&mut app, "ls -l | wc -l");
        // Results panel: the stats are on the row after the command.
        let stats_row = |terminal: &Terminal<TestBackend>| -> String {
            let mut row = String::new();
            for y in 0..terminal.backend().buffer().area.height {
                let text = row_text(terminal, y);
                if text.contains("wpm") {
                    row = text;
                }
            }
            row
        };
        let start = draw(&app, 80, 12);
        assert!(stats_row(&start).contains("│ 0 wpm   0% acc"));

        app.update(Event::Tick {
            at: app.now + Duration::from_millis(200),
        });
        let middle = stats_row(&draw(&app, 80, 12));
        assert!(
            !middle.contains("│ 65 wpm") && !middle.contains("│ 0 wpm"),
            "{middle}"
        );

        settle(&mut app);
        assert!(stats_row(&draw(&app, 80, 12)).contains("│ 65 wpm   100% acc"));
    }

    #[test]
    fn typing_screen_snapshot() {
        insta::assert_snapshot!(draw_settled(app_with_a_mistake(), 80, 12).backend());
    }

    #[test]
    fn narrow_typing_screen_wraps_command() {
        insta::assert_snapshot!(draw_settled(app_with_a_mistake(), 40, 12).backend());
    }

    #[test]
    fn results_without_mistakes() {
        let mut app = started_app(&["ls -l | wc -l"]);
        type_text(&mut app, "ls -l | wc -l");
        insta::assert_snapshot!(draw_settled(app, 80, 12).backend());
    }

    #[test]
    fn results_with_missed_symbols() {
        let mut app = started_app(&[FIND]);
        // Miss both quotes and the opening brace, without correcting them.
        let typed: String = FIND
            .chars()
            .map(|c| if c == '\'' || c == '{' { 'x' } else { c })
            .collect();
        type_text(&mut app, &typed);
        insta::assert_snapshot!(draw_settled(app, 80, 14).backend());
    }

    /// Every span drawn in the mistake style holds just the symbol, so the
    /// underline never runs under the spaces around it.
    fn assert_only_symbols_marked(line: &Line, symbols: &[char]) {
        let mut marked = Vec::new();
        for span in &line.spans {
            if span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
            {
                marked.push(span.content.to_string());
            }
        }
        let expected: Vec<String> = symbols.iter().map(|c| c.to_string()).collect();
        assert_eq!(marked, expected);
    }

    #[test]
    fn weakest_line_underlines_only_the_symbols() {
        let mut app = started_app(&["a | b {"]);
        type_text(&mut app, "a x b x");
        assert_only_symbols_marked(&weakest_line(&app.profile), &['{', '|']);
    }

    #[test]
    fn missed_line_underlines_only_the_symbols() {
        let mut app = started_app(&["a | b {"]);
        type_text(&mut app, "a x b x");
        let Screen::Results { stats, .. } = &app.screen else {
            panic!("expected the Results screen");
        };
        assert_only_symbols_marked(&results::missed_line(stats), &['{', '|']);
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
        // Panel 20 wide leaves 16 columns for text: 2 borders, 2 padding.
        assert_eq!(command_rows(5, 20), 1);
    }

    #[test]
    fn command_that_fills_a_row_exactly_takes_one_row() {
        assert_eq!(command_rows(16, 20), 1);
    }

    #[test]
    fn long_command_wraps_onto_more_rows() {
        assert_eq!(command_rows(17, 20), 2);
    }

    #[test]
    fn line_breaks_match_the_row_count() {
        let spans: Vec<Span> = "abcdefghijklmnopqrstuvwxyz"
            .chars()
            .map(|c| Span::raw(c.to_string()))
            .collect();
        let lines = wrap_spans(spans, 20);
        assert_eq!(lines.len(), usize::from(command_rows(26, 20)));
        assert_eq!(lines[0].width(), 16);
    }

    #[test]
    fn tiny_width_does_not_divide_by_zero() {
        assert_eq!(inner_width(0), 1);
        assert_eq!(command_rows(2, 2), 2);
    }
}
