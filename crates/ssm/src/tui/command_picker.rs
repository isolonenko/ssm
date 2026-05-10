use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use ssm_core::config::CommandConfig;

pub fn render(f: &mut Frame, host_alias: &str, commands: &[CommandConfig], selected: usize) {
    let area = f.area();

    // Center the modal: ~50% width, ~50% height
    let modal_width = (area.width as f32 * 0.50) as u16;
    let modal_height = (area.height as f32 * 0.50) as u16;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = ratatui::layout::Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    let title = format!(" Commands: {} ", host_alias);
    let block = Block::default()
        .title(title.as_str())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)))
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout: [list area, hint line]
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Max width for command preview
    let max_cmd_width = v_chunks[0].width.saturating_sub(24) as usize;

    let items: Vec<ListItem> = commands
        .iter()
        .map(|cmd| {
            let truncated_cmd = if cmd.command.len() > max_cmd_width && max_cmd_width > 3 {
                format!("{}...", &cmd.command[..max_cmd_width - 3])
            } else {
                cmd.command.clone()
            };

            Line::from(vec![
                Span::styled(
                    format!("{:<20}", cmd.name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(truncated_cmd, Style::default().fg(Color::Gray)),
            ])
        })
        .map(ListItem::new)
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 60, 40))
                .fg(Color::Rgb(80, 200, 120))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    f.render_stateful_widget(list, v_chunks[0], &mut list_state);

    // Hint line
    let hint = Span::styled(
        "Enter: run & show  S: session  Y: copy  Esc: close",
        Style::default().fg(Color::Gray),
    );
    f.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(hint)).alignment(Alignment::Center),
        v_chunks[1],
    );
}
