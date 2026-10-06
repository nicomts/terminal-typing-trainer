//! The Results screen: the command with missed positions marked, speed and
//! accuracy, the command's explanation, which symbols were missed this
//! round, and the weakest symbols over all rounds.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{BORDER_ROWS, HINT_ROWS, command_rows, wrap_spans};
use crate::session::Session;
use crate::stats::{Profile, RoundStats};
use crate::theme;

/// Rows inside the panel under the command: stats, explanation, missed
/// symbols, weakest symbols.
const SUMMARY_ROWS: u16 = 4;
/// How many symbols the "weakest" row lists.
const WEAKEST_SHOWN: usize = 3;

/// Total rows this screen needs at `width` columns. No empty rows.
pub fn height(session: &Session, width: u16) -> u16 {
    panel_height(session, width) + HINT_ROWS
}

fn panel_height(session: &Session, width: u16) -> u16 {
    command_rows(session.target().len(), width) + SUMMARY_ROWS + BORDER_ROWS
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    stats: &RoundStats,
    profile: &Profile,
    explain: &str,
) {
    let [panel_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height(session, area.width)),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    let mut lines = wrap_spans(command_spans(session), area.width);
    lines.push(stats_line(stats));
    lines.push(Line::styled(explain.to_string(), theme::hint()));
    lines.push(missed_line(stats));
    lines.push(weakest_line(profile));

    let panel = Paragraph::new(lines).block(theme::panel_block(true).title(" results "));
    frame.render_widget(panel, panel_area);

    frame.render_widget(
        Span::styled("enter next · esc quit", theme::hint()),
        hint_area,
    );
}

/// The command, with every position the user ever got wrong marked as a
/// mistake, even if they corrected it afterwards.
fn command_spans(session: &Session) -> Vec<Span<'static>> {
    let target = session.target();
    let was_missed = session.missed_positions();

    let mut spans = Vec::with_capacity(target.len());
    for (i, &c) in target.iter().enumerate() {
        let style = if was_missed[i] {
            theme::incorrect(c)
        } else {
            theme::correct()
        };
        spans.push(Span::styled(c.to_string(), style));
    }
    spans
}

/// "72 wpm   96% acc", rounded to whole numbers.
fn stats_line(stats: &RoundStats) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:.0}", stats.wpm), theme::stat_value()),
        Span::styled(" wpm   ", theme::stat_label()),
        Span::styled(format!("{:.0}%", stats.accuracy), theme::stat_value()),
        Span::styled(" acc", theme::stat_label()),
    ])
}

/// "missed  ' ×2  { ×1", or "no missed symbols".
fn missed_line(stats: &RoundStats) -> Line<'static> {
    if stats.missed.is_empty() {
        return Line::styled("no missed symbols", theme::hint());
    }
    let mut spans = vec![Span::styled("missed ", theme::hint())];
    for &(symbol, count) in &stats.missed {
        spans.push(Span::styled(format!(" {symbol}"), theme::incorrect(symbol)));
        spans.push(Span::styled(format!(" ×{count} "), theme::hint()));
    }
    Line::from(spans)
}

/// "weakest  ' 46%  { 38%  | 25%": over all rounds, by plain miss rate.
fn weakest_line(profile: &Profile) -> Line<'static> {
    let weakest = profile.weakest(WEAKEST_SHOWN);
    if weakest.is_empty() {
        return Line::styled("no weak symbols yet", theme::hint());
    }
    let mut spans = vec![Span::styled("weakest", theme::hint())];
    for (symbol, miss_rate) in weakest {
        spans.push(Span::styled(format!(" {symbol}"), theme::incorrect(symbol)));
        spans.push(Span::styled(
            format!(" {:.0}% ", miss_rate * 100.0),
            theme::hint(),
        ));
    }
    Line::from(spans)
}
