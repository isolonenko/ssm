use crate::tui::app::{CommandWizardState, CommandWizardStep};
use crate::tui::wizard::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

fn step_hint(state: &CommandWizardState) -> &'static str {
    match state.step {
        CommandWizardStep::Name => "Required. A short name, e.g. logs, disk-usage.",
        CommandWizardStep::Command => "The shell command to run on the remote host.",
    }
}

pub fn render(f: &mut Frame, state: &CommandWizardState) {
    let area = f.area();
    let modal = centered_rect(50, 30, area);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(" Add Command ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let idx = state.step_index();
    let progress = format!("Step {}/2", idx + 1);
    let label = state.step_label();
    let value = state.current_value();
    let hint = step_hint(state);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // step progress
            Constraint::Length(1), // spacer
            Constraint::Length(1), // label + value
            Constraint::Length(1), // spacer
            Constraint::Length(2), // hint
        ])
        .split(inner);

    let progress_para = Paragraph::new(Line::from(Span::styled(
        progress,
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )))
    .alignment(Alignment::Right);
    f.render_widget(progress_para, chunks[0]);

    let input_line = Line::from(vec![
        Span::styled(
            format!("{}: ", label),
            Style::default()
                .fg(Color::Rgb(80, 200, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
        Span::styled("█", Style::default().fg(Color::Rgb(80, 200, 120))),
    ]);
    let input_para = Paragraph::new(input_line);
    f.render_widget(input_para, chunks[2]);

    let hint_para = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Gray),
    )));
    f.render_widget(hint_para, chunks[4]);
}
