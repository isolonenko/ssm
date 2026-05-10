use crate::tui::app::WizardState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Step index (0-based) for the current wizard step.
fn step_index(state: &WizardState) -> usize {
    use crate::tui::app::WizardStep;
    match state.step {
        WizardStep::Alias => 0,
        WizardStep::Hostname => 1,
        WizardStep::User => 2,
        WizardStep::Port => 3,
        WizardStep::IdentityFile => 4,
        WizardStep::Tags => 5,
    }
}

fn step_hint(state: &WizardState) -> &'static str {
    use crate::tui::app::WizardStep;
    match state.step {
        WizardStep::Alias => "Required. A short name used to identify this host.",
        WizardStep::Hostname => "Required. IP address or hostname of the server.",
        WizardStep::User => "Optional. SSH login username (defaults to current user).",
        WizardStep::Port => "Optional. SSH port number (default: 22).",
        WizardStep::IdentityFile => "Optional. Path to your private key, e.g. ~/.ssh/id_ed25519.",
        WizardStep::Tags => "Optional. Comma-separated tags, e.g. prod, api.",
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render(f: &mut Frame, state: &WizardState, title: &str) {
    let area = f.area();
    let modal = centered_rect(50, 40, area);

    // Clear the background
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let idx = step_index(state);
    let progress = format!("Step {}/6", idx + 1);
    let label = state.step_label();
    let value = state.current_value();
    let hint = step_hint(state);

    // Layout: progress line, blank, label + input, blank, hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // step progress
            Constraint::Length(1), // spacer
            Constraint::Length(1), // label + value
            Constraint::Length(1), // spacer
            Constraint::Length(2), // hint (may wrap)
        ])
        .split(inner);

    // Progress
    let progress_para = Paragraph::new(Line::from(Span::styled(
        progress,
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )))
    .alignment(Alignment::Right);
    f.render_widget(progress_para, chunks[0]);

    // Field label + input with cursor
    let input_line = Line::from(vec![
        Span::styled(
            format!("{}: ", label),
            Style::default().fg(Color::Rgb(80, 200, 120)).add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
        Span::styled("█", Style::default().fg(Color::Rgb(80, 200, 120))),
    ]);
    let input_para = Paragraph::new(input_line);
    f.render_widget(input_para, chunks[2]);

    // Hint
    let hint_para = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Gray),
    )));
    f.render_widget(hint_para, chunks[4]);
}
