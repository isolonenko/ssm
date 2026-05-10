use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use ssm_core::{config::Host, tunnel::TunnelRegistry};

/// Determine the tunnel indicator character and color for a host.
/// - No tunnels configured  → " " (space, no color)
/// - All tunnels running    → "●" green
/// - Some tunnels running   → "◐" yellow
/// - Has dead tunnels       → "●" red
/// - None running           → "○" grey
fn tunnel_indicator(host: &Host, registry: &TunnelRegistry) -> (String, Color) {
    if host.tunnels.is_empty() {
        return (" ".to_string(), Color::Reset);
    }

    let mut running = 0usize;
    let mut dead = 0usize;

    for tc in &host.tunnels {
        match registry.find(&host.alias, &tc.name) {
            Some(entry) if ssm_core::tunnel::is_pid_alive(entry.pid) => running += 1,
            Some(_) => dead += 1,
            None => {}
        }
    }

    let total = host.tunnels.len();
    if dead > 0 {
        ("●".to_string(), Color::Red)
    } else if running == total {
        ("●".to_string(), Color::Green)
    } else if running > 0 {
        ("◐".to_string(), Color::Yellow)
    } else {
        ("○".to_string(), Color::Gray)
    }
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    hosts: &[Host],
    selected: usize,
    registry: &TunnelRegistry,
) {
    let items: Vec<ListItem> = hosts
        .iter()
        .map(|host| {
            let (dot, dot_color) = tunnel_indicator(host, registry);
            let tag_str = if host.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", host.tags.join(", "))
            };
            let line = Line::from(vec![
                Span::styled(dot, Style::default().fg(dot_color)),
                Span::raw(" "),
                Span::raw(host.alias.clone()),
                Span::styled(tag_str, Style::default().fg(Color::Gray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Hosts ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
                .style(Style::default().bg(Color::Rgb(15, 15, 20))),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 60, 40))
                .fg(Color::Rgb(80, 200, 120))
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !hosts.is_empty() {
        state.select(Some(selected));
    }

    f.render_stateful_widget(list, area, &mut state);
}
