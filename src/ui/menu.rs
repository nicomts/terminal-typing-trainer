//! The Menu screen: pick a topic to practise, and see totals over all rounds.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{BORDER_ROWS, HINT_ROWS, weakest_line};
use crate::app::{MENU_ROWS, topic_for_row};
use crate::stats::Profile;
use crate::theme;

/// Rows inside the panel under the menu: a blank separator, the totals and
/// the weakest symbols.
const SUMMARY_ROWS: u16 = 3;

/// Total rows this screen needs. Unlike the other screens, it doesn't
/// depend on the width: every row is short enough for the narrowest panel.
pub fn height() -> u16 {
    panel_height() + HINT_ROWS
}

fn panel_height() -> u16 {
    // MENU_ROWS is a small constant, so the conversion can't fail in
    // practice; `unwrap_or` keeps it free of `unwrap()` anyway.
    let item_rows = u16::try_from(MENU_ROWS).unwrap_or(u16::MAX);
    item_rows + SUMMARY_ROWS + BORDER_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, selected: usize, profile: &Profile) {
    let [panel_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height()),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let mut lines = Vec::new();
    for row in 0..MENU_ROWS {
        lines.push(item_line(row, row == selected));
    }
    lines.push(Line::default());
    lines.push(totals_line(profile));
    lines.push(weakest_line(profile));

    let panel =
        Paragraph::new(lines).block(theme::panel_block(true).title(" terminal typing trainer "));
    frame.render_widget(panel, panel_area);

    frame.render_widget(
        Span::styled("↑↓ choose · enter start · esc quit", theme::hint()),
        hint_area,
    );
}

/// "> shell" when selected, "  shell" otherwise. The marker means the
/// selection doesn't rely on color alone.
fn item_line(row: usize, is_selected: bool) -> Line<'static> {
    let label = match topic_for_row(row) {
        None => "all topics",
        Some(topic) => topic.name(),
    };
    if is_selected {
        Line::styled(format!("> {label}"), theme::menu_selected())
    } else {
        Line::styled(format!("  {label}"), theme::menu_item())
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
