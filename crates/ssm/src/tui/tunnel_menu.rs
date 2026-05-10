use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use ssm_core::{config::Host, tunnel::TunnelRegistry};

pub fn render(f: &mut Frame, host: &Host, registry: &TunnelRegistry, selected: usize) {
    let area = f.area();

    // Center the modal: ~45% width, ~50% height
    let modal_width = (area.width as f32 * 0.45) as u16;
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

    let title = format!(" Tunnels: {} ", host.alias);
    let block = Block::default()
        .title(title.as_str())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)))
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    if host.tunnels.is_empty() {
        let msg = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
            "No tunnels configured",
            Style::default().fg(Color::Gray),
        )))
        .alignment(Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    // Layout: [list area, hint line]
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = host
        .tunnels
        .iter()
        .map(|t| {
            let is_running = registry
                .find(&host.alias, &t.name)
                .map(|e| ssm_core::tunnel::is_pid_alive(e.pid))
                .unwrap_or(false);

            let (bullet, status_style) = if is_running {
                ("●", Style::default().fg(Color::Green))
            } else {
                ("○", Style::default().fg(Color::Gray))
            };

            let port_mapping = format!(
                "{}:{} → {}:{}",
                "localhost", t.local_port, t.remote_host, t.remote_port
            );

            Line::from(vec![
                Span::styled(format!("{} ", bullet), status_style),
                Span::raw(format!("{:<20}", t.name)),
                Span::styled(port_mapping, Style::default().fg(Color::Gray)),
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
        "Enter: toggle  A: add  Esc: close",
        Style::default().fg(Color::Gray),
    );
    f.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(hint)).alignment(Alignment::Center),
        v_chunks[1],
    );
}
