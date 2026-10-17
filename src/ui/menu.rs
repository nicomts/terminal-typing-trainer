//! The Menu screen: the logo, a topic to practise, and totals over all rounds.

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{BORDER_ROWS, HINT_ROWS, inner_width, logo, weakest_line};
use crate::anim;
use crate::app::{App, MENU_ROWS, topic_for_row};
use crate::stats::Profile;
use crate::theme;

/// Rows inside the panel under the menu: a blank separator, the totals and
/// the weakest symbols.
const SUMMARY_ROWS: u16 = 3;
/// Empty rows between the logo and the panel.
const LOGO_GAP: u16 = 1;
/// The shimmer crosses the logo once per period...
const SHIMMER_PERIOD: Duration = Duration::from_millis(3500);
/// ...using this share of it to cross; the rest is a pause.
const SHIMMER_SWEEP: f64 = 0.5;
/// Half the width of the bright band, in columns.
const SHIMMER_HALF_WIDTH: f64 = 4.0;

/// Total rows this screen needs. Unlike the other screens, it doesn't
/// depend on the width: every row is short enough for the narrowest panel.
pub fn height() -> u16 {
    logo::ROWS + LOGO_GAP + panel_height() + HINT_ROWS
}

fn panel_height() -> u16 {
    // MENU_ROWS is a small constant, so the conversion can't fail in
    // practice; `unwrap_or` keeps it free of `unwrap()` anyway.
    let item_rows = u16::try_from(MENU_ROWS).unwrap_or(u16::MAX);
    item_rows + SUMMARY_ROWS + BORDER_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let [logo_area, _gap, panel_area, hint_area] = Layout::vertical([
        Constraint::Length(logo::ROWS),
        Constraint::Length(LOGO_GAP),
        Constraint::Length(panel_height()),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let [logo_area] = Layout::horizontal([Constraint::Length(logo::width())])
        .flex(Flex::Center)
        .areas(logo_area);
    frame.render_widget(
        Paragraph::new(logo_lines(app.screen_since, app.now)),
        logo_area,
    );

    let row_width = inner_width(area.width);
    let mut lines = Vec::new();
    for row in 0..MENU_ROWS {
        lines.push(item_line(row, row == selected, row_width));
    }
    lines.push(Line::default());
    lines.push(totals_line(&app.profile));
    lines.push(weakest_line(&app.profile));

    let panel = Paragraph::new(lines).block(theme::panel_block(true).title(" choose a topic "));
    frame.render_widget(panel, panel_area);

    frame.render_widget(
        Span::styled("↑↓ choose · enter start · esc quit", theme::hint()),
        hint_area,
    );
}

/// The logo, one span per character, brighter where the shimmer is.
fn logo_lines(since: Instant, now: Instant) -> Vec<Line<'static>> {
    let center = shimmer_center(since, now);
    let mut lines = Vec::new();
    for row in logo::rows() {
        let mut spans = Vec::new();
        for (column, c) in row.chars().enumerate() {
            let glow = shimmer_glow(column as f64, center);
            spans.push(Span::styled(c.to_string(), theme::glow(glow)));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Column the bright band is centred on, or `None` during the pause.
/// The band starts fully left of the logo and ends fully right of it.
fn shimmer_center(since: Instant, now: Instant) -> Option<f64> {
    let p = anim::phase(since, now, SHIMMER_PERIOD);
    if p >= SHIMMER_SWEEP {
        return None;
    }
    let start = -SHIMMER_HALF_WIDTH;
    let distance = f64::from(logo::width()) + 2.0 * SHIMMER_HALF_WIDTH;
    Some(start + distance * (p / SHIMMER_SWEEP))
}

/// 1.0 at the band's centre, falling to 0.0 at its edges and beyond.
fn shimmer_glow(column: f64, center: Option<f64>) -> f64 {
    match center {
        Some(center) => (1.0 - (column - center).abs() / SHIMMER_HALF_WIDTH).max(0.0),
        None => 0.0,
    }
}

/// "▸ shell" when selected, "  shell" otherwise. The marker means the
/// selection doesn't rely on color alone. Padded to `width` so the
/// selected row's highlight spans the whole panel.
fn item_line(row: usize, is_selected: bool, width: usize) -> Line<'static> {
    let label = match topic_for_row(row) {
        None => "all topics",
        Some(topic) => topic.name(),
    };
    if is_selected {
        // `{label:<pad$}` left-aligns `label` and fills with spaces to `pad`.
        let pad = width.saturating_sub(3);
        Line::styled(format!(" ▸ {label:<pad$}"), theme::menu_selected())
    } else {
        Line::styled(format!("   {label}"), theme::menu_item())
    }
}

/// "12 rounds   best 64 wpm", or "no rounds yet".
fn totals_line(profile: &Profile) -> Line<'static> {
    if profile.rounds == 0 {
        return Line::styled("no rounds yet", theme::hint());
    }
    let rounds_label = if profile.rounds == 1 {
        " round   best "
    } else {
        " rounds   best "
    };
    Line::from(vec![
        Span::styled(profile.rounds.to_string(), theme::stat_value()),
        Span::styled(rounds_label, theme::stat_label()),
        Span::styled(format!("{:.0}", profile.best_wpm), theme::stat_value()),
        Span::styled(" wpm", theme::stat_label()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_crosses_from_left_to_right_then_pauses() {
        let start = Instant::now();
        let early = shimmer_center(start, start + SHIMMER_PERIOD / 10);
        let later = shimmer_center(start, start + SHIMMER_PERIOD * 4 / 10);
        assert!(early < later);
        assert_eq!(shimmer_center(start, start + SHIMMER_PERIOD * 9 / 10), None);
    }

    #[test]
    fn glow_is_brightest_at_the_band_centre() {
        assert_eq!(shimmer_glow(10.0, Some(10.0)), 1.0);
        assert!(shimmer_glow(12.0, Some(10.0)) < 1.0);
        assert_eq!(shimmer_glow(20.0, Some(10.0)), 0.0);
        assert_eq!(shimmer_glow(10.0, None), 0.0);
    }
}
