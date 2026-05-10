use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame) {
    let area = f.area();

    // Center the modal: ~50% width, ~60% height
    let modal_width = (area.width as f32 * 0.50) as u16;
    let modal_height = (area.height as f32 * 0.60) as u16;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = ratatui::layout::Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)))
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Padding on the sides
    let padded = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner)[1];

    let header_style = Style::default()
        .fg(Color::Rgb(80, 200, 120))
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::Gray);

    fn key_line<'a>(
        key: &'a str,
        desc: &'a str,
        key_style: Style,
        desc_style: Style,
    ) -> Line<'a> {
        Line::from(vec![
            Span::styled(format!("  {:<14}", key), key_style),
            Span::styled(desc, desc_style),
        ])
    }

    let lines: Vec<Line> = vec![
        Line::from(Span::styled("Navigation", header_style)),
        Line::from(Span::styled("──────────────────────────────", dim_style)),
        key_line("j / Down", "Move down", key_style, desc_style),
        key_line("k / Up", "Move up", key_style, desc_style),
        key_line("/", "Open filter", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Actions", header_style)),
        Line::from(Span::styled("──────────────────────────────", dim_style)),
        key_line("Enter", "SSH connect", key_style, desc_style),
        key_line("T", "Open tunnel menu", key_style, desc_style),
        key_line("Shift+T", "Toggle all tunnels", key_style, desc_style),
        key_line("S", "Scenarios", key_style, desc_style),
        key_line("C", "Open command picker", key_style, desc_style),
        key_line("I", "Import tunnels", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Management", header_style)),
        Line::from(Span::styled("──────────────────────────────", dim_style)),
        key_line("A", "Add host", key_style, desc_style),
        key_line("E", "Edit host", key_style, desc_style),
        key_line("D", "Delete host", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Other", header_style)),
        Line::from(Span::styled("──────────────────────────────", dim_style)),
        key_line("?", "Toggle this help", key_style, desc_style),
        key_line("q / Ctrl-C", "Quit", key_style, desc_style),
        key_line("Esc", "Cancel / close", key_style, desc_style),
    ];

    let para = Paragraph::new(lines);
    f.render_widget(para, padded);
}
