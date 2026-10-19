//! The Results screen: the command with missed positions marked, speed and
//! accuracy (counting up from zero), an accuracy bar, the command's
//! explanation, which symbols were missed this round, and the weakest
//! symbols over all rounds.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{
    BORDER_ROWS, HINT_ROWS, bar_line, command_rows, inner_width, weakest_line, wrap_spans,
};
use crate::anim;
use crate::app::App;
use crate::session::Session;
use crate::stats::RoundStats;
use crate::theme;

/// Rows inside the panel under the command: stats, accuracy bar,
/// explanation, missed symbols, weakest symbols.
const SUMMARY_ROWS: u16 = 5;
/// How long the numbers take to count up to their final value.
const COUNT_UP: Duration = Duration::from_millis(700);
/// One rise and fall of the "new best" glow.
const BEST_PULSE: Duration = Duration::from_millis(1200);

/// Total rows this screen needs at `width` columns. No empty rows.
pub fn height(session: &Session, width: u16) -> u16 {
    panel_height(session, width) + HINT_ROWS
}

fn panel_height(session: &Session, width: u16) -> u16 {
    command_rows(session.target().len(), width) + SUMMARY_ROWS + BORDER_ROWS
}

pub fn render(frame: &mut Frame, area: Rect, app: &App, stats: &RoundStats, new_best: bool) {
    let session = &app.session;
    let [panel_area, hint_area] = Layout::vertical([
        Constraint::Length(panel_height(session, area.width)),
        Constraint::Length(HINT_ROWS),
    ])
    .areas(area);

    // How far the count-up has got: 0.0 when the screen appears, 1.0 after.
    let counted = anim::ease_out(anim::progress(app.screen_since, app.now, COUNT_UP));

    let mut lines = wrap_spans(command_spans(session), area.width);
    let mut stats_spans = stats_spans(stats.wpm * counted, stats.accuracy * counted);
    if new_best {
        let glow = anim::pulse(app.screen_since, app.now, BEST_PULSE);
        stats_spans.push(Span::styled("   ★ new best", theme::glow(glow)));
    }
    lines.push(Line::from(stats_spans));
    lines.push(bar_line(
        stats.accuracy * counted / 100.0,
        inner_width(area.width),
    ));
    lines.push(Line::styled(
        app.command().explain.to_string(),
        theme::hint(),
    ));
    lines.push(missed_line(stats));
    lines.push(weakest_line(&app.profile));

    let panel = Paragraph::new(lines).block(theme::panel_block(true).title(" results "));
    frame.render_widget(panel, panel_area);

    frame.render_widget(
        Span::styled("enter next · tab retry · esc menu", theme::hint()),
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
fn stats_spans(wpm: f64, accuracy: f64) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("{wpm:.0}"), theme::stat_value()),
        Span::styled(" wpm   ", theme::stat_label()),
        Span::styled(format!("{accuracy:.0}%"), theme::stat_value()),
        Span::styled(" acc", theme::stat_label()),
    ]
}

/// "missed  ' ×2  { ×1", or "no missed symbols".
pub(super) fn missed_line(stats: &RoundStats) -> Line<'static> {
    if stats.missed.is_empty() {
        return Line::styled("no missed symbols", theme::hint());
    }
    let mut spans = vec![Span::styled("missed ", theme::hint())];
    for &(symbol, count) in &stats.missed {
        // The space gets its own span: in the mistake style it would be
        // underlined along with the symbol.
        spans.push(Span::styled(" ", theme::hint()));
        spans.push(Span::styled(symbol.to_string(), theme::incorrect(symbol)));
        spans.push(Span::styled(format!(" ×{count} "), theme::hint()));
    }
    Line::from(spans)
}
