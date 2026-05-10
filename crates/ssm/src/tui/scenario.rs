use crate::tui::wizard::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ssm_core::config::{Config, Scenario};
use ssm_core::tunnel::TunnelRegistry;

pub fn render_menu(
    f: &mut Frame,
    config: &Config,
    registry: &TunnelRegistry,
    selected: usize,
) {
    let area = f.area();
    let modal = centered_rect(60, 55, area);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(" Scenarios ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    if config.scenarios.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let msg = Paragraph::new(Line::from(Span::styled(
            "No scenarios yet. Press A to create one.",
            Style::default().fg(Color::Gray),
        )))
        .alignment(Alignment::Center);
        f.render_widget(msg, chunks[0]);

        let hint = Paragraph::new(Line::from(Span::styled(
            "A: create  Esc: close",
            Style::default().fg(Color::Gray),
        )))
        .alignment(Alignment::Center);
        f.render_widget(hint, chunks[1]);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = config
        .scenarios
        .iter()
        .map(|scenario| {
            let (running, total) = scenario_status(scenario, config, registry);
            let (bullet, color) = if total == 0 {
                ("○", Color::Gray)
            } else if running == total {
                ("●", Color::Green)
            } else if running > 0 {
                ("◐", Color::Yellow)
            } else {
                ("○", Color::Gray)
            };

            Line::from(vec![
                Span::styled(format!("{} ", bullet), Style::default().fg(color)),
                Span::styled(
                    format!("{:<20}", scenario.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{}/{} tunnels", running, total),
                    Style::default().fg(Color::Gray),
                ),
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
    f.render_stateful_widget(list, chunks[0], &mut list_state);

    let hint = Paragraph::new(Line::from(Span::styled(
        "Enter: toggle all  A: create  D: delete  Esc: close",
        Style::default().fg(Color::Gray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[1]);
}

pub fn render_create(
    f: &mut Frame,
    name: &str,
    config: &Config,
    toggles: &[bool],
    scroll: usize,
) {
    let area = f.area();
    let modal = centered_rect(65, 70, area);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(" Create Scenario ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)));

    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // name input
            Constraint::Length(1), // spacer
            Constraint::Min(1),   // tunnel list
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // Name input
    let name_line = if name.is_empty() {
        Line::from(vec![
            Span::styled(
                "Name: ",
                Style::default()
                    .fg(Color::Rgb(80, 200, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("type scenario name, then ↓ to select tunnels", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "Name: ",
                Style::default()
                    .fg(Color::Rgb(80, 200, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(name),
            Span::styled("█", Style::default().fg(Color::Rgb(80, 200, 120))),
        ])
    };
    f.render_widget(Paragraph::new(name_line), chunks[0]);

    // Tunnel selection list
    let all_tunnels = collect_all_tunnels(config);
    let mut lines: Vec<Line> = Vec::new();

    for (i, (host_alias, tunnel_name, _)) in all_tunnels.iter().enumerate() {
        let checked = toggles.get(i).copied().unwrap_or(false);
        let marker = if checked { "[x]" } else { "[ ]" };
        let highlight = i == scroll;

        let style = if highlight {
            Style::default()
                .fg(Color::Rgb(80, 200, 120))
                .add_modifier(Modifier::BOLD)
        } else if checked {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", marker), style),
            Span::styled(format!("{}:{}", host_alias, tunnel_name), style),
        ]));
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, chunks[2]);

    let hint = Paragraph::new(Line::from(Span::styled(
        "Space: toggle  Enter: save  Esc: cancel",
        Style::default().fg(Color::Gray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[3]);
}

/// Returns (running_count, total_count) for a scenario.
fn scenario_status(scenario: &Scenario, _config: &Config, registry: &TunnelRegistry) -> (usize, usize) {
    let total = scenario.tunnels.len();
    let running = scenario
        .tunnels
        .iter()
        .filter(|st| {
            registry
                .find(&st.host, &st.tunnel)
                .map(|e| ssm_core::tunnel::is_pid_alive(e.pid))
                .unwrap_or(false)
        })
        .count();
    (running, total)
}

/// Collect all tunnels across all hosts as (host_alias, tunnel_name, tunnel_config) triples.
pub fn collect_all_tunnels(config: &Config) -> Vec<(String, String, ssm_core::config::TunnelConfig)> {
    let mut result = Vec::new();
    for host in &config.hosts {
        for tunnel in &host.tunnels {
            result.push((host.alias.clone(), tunnel.name.clone(), tunnel.clone()));
        }
    }
    result
}
