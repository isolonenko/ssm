use crate::tui::wizard::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use ssm_core::import::ParsedHost;

pub fn render_paste(f: &mut Frame, buffer: &str) {
    let area = f.area();
    let modal = centered_rect(70, 60, area);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(" Import Tunnels — Paste SSH Commands ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let display = if buffer.is_empty() {
        "Paste your ssh -L commands here...\n\nExample:\n  ssh -l user -nNTL 5432:localhost:5432 host.com -p 22".to_string()
    } else {
        let line_count = buffer.lines().count();
        let preview: String = buffer.lines().take(20).collect::<Vec<_>>().join("\n");
        if line_count > 20 {
            format!("{}\n... ({} more lines)", preview, line_count - 20)
        } else {
            preview
        }
    };

    let style = if buffer.is_empty() {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };

    let para = Paragraph::new(display)
        .style(style)
        .wrap(Wrap { trim: false });
    f.render_widget(para, chunks[0]);

    let hint = Paragraph::new(Line::from(Span::styled(
        "Enter: parse & preview  Esc: cancel",
        Style::default().fg(Color::Gray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[1]);
}

pub fn render_preview(f: &mut Frame, parsed: &[ParsedHost], hosts: &[ssm_core::config::Host], scroll: u16) {
    let area = f.area();
    let modal = centered_rect(70, 70, area);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(" Import Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();

    if parsed.is_empty() {
        lines.push(Line::from(Span::styled(
            "No SSH tunnel commands found in pasted text.",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for parsed_host in parsed {
            let existing = hosts.iter().find(|h| h.hostname == parsed_host.hostname);

            let header = if let Some(h) = existing {
                Line::from(vec![
                    Span::styled("→ ", Style::default().fg(Color::Rgb(80, 200, 120))),
                    Span::styled(
                        format!("{}", parsed_host.hostname),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  (add to existing: {})", h.alias),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            } else {
                let alias = ssm_core::import::alias_from_hostname(&parsed_host.hostname);
                Line::from(vec![
                    Span::styled("+ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}", parsed_host.hostname),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  (new host: {})", alias),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            };
            lines.push(header);

            if parsed_host.port != 22 {
                lines.push(Line::from(Span::styled(
                    format!("    port: {}", parsed_host.port),
                    Style::default().fg(Color::Gray),
                )));
            }

            for tunnel in &parsed_host.tunnels {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("⊙ ", Style::default().fg(Color::Rgb(80, 200, 120))),
                    Span::raw(format!(
                        "{}:{} → {}:{}",
                        "localhost", tunnel.local_port, tunnel.remote_host, tunnel.remote_port
                    )),
                ]));
            }

            lines.push(Line::from(""));
        }
    }

    let para = Paragraph::new(lines)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, chunks[0]);

    let hint_text = if parsed.is_empty() {
        "Esc: back"
    } else {
        "Y: apply  Esc: cancel  ↑↓: scroll"
    };
    let hint = Paragraph::new(Line::from(Span::styled(
        hint_text,
        Style::default().fg(Color::Gray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[1]);
}
