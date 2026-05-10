use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ssm_core::{config::Host, tunnel::TunnelRegistry};

pub fn render(f: &mut Frame, area: Rect, host: &Host, registry: &TunnelRegistry) {
    let label_style = Style::default().fg(Color::Gray);
    let value_style = Style::default();

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Host:  ", label_style),
            Span::styled(host.hostname.clone(), value_style),
        ]),
        Line::from(vec![
            Span::styled("User:  ", label_style),
            Span::styled(
                host.user.clone().unwrap_or_else(|| "-".to_string()),
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("Port:  ", label_style),
            Span::styled(host.port.to_string(), value_style),
        ]),
        Line::from(vec![
            Span::styled("Key:   ", label_style),
            Span::styled(
                host.identity_file
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "-".to_string()),
                value_style,
            ),
        ]),
    ];

    // Tunnels section
    if !host.tunnels.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Tunnels",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::UNDERLINED),
        )));
        for tc in &host.tunnels {
            let is_running = registry
                .find(&host.alias, &tc.name)
                .map(|e| ssm_core::tunnel::is_pid_alive(e.pid))
                .unwrap_or(false);
            let dot_color = if is_running { Color::Green } else { Color::Gray };
            let dot = if is_running { "●" } else { "○" };
            let desc = format!(
                " {} → {}:{} (local:{})",
                tc.name, tc.remote_host, tc.remote_port, tc.local_port
            );
            lines.push(Line::from(vec![
                Span::styled(dot, Style::default().fg(dot_color)),
                Span::raw(desc),
            ]));
        }
    }

    // Commands section
    if !host.commands.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Commands",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::UNDERLINED),
        )));
        for cmd in &host.commands {
            let truncated = if cmd.command.len() > 40 {
                format!("{}…", &cmd.command[..40])
            } else {
                cmd.command.clone()
            };
            lines.push(Line::from(vec![
                Span::styled(cmd.name.clone(), Style::default().fg(Color::Gray)),
                Span::raw(": "),
                Span::raw(truncated),
            ]));
        }
    }

    let title = format!(" {} ", host.alias);
    let para = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
            .style(Style::default().bg(Color::Rgb(15, 15, 20))),
    );

    f.render_widget(para, area);
}
