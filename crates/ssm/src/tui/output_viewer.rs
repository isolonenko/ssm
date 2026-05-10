use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, output: &str, scroll: u16) {
    let area = f.area();

    // Center the modal: ~70% width, ~70% height
    let modal_width = (area.width as f32 * 0.70) as u16;
    let modal_height = (area.height as f32 * 0.70) as u16;
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
        .title(" Output ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 120)))
        .style(Style::default().bg(Color::Rgb(15, 15, 20)))
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout: [content, hint]
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let para = Paragraph::new(output)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .style(Style::default().fg(Color::White));

    f.render_widget(para, v_chunks[0]);

    let hint = Span::styled(
        "↑↓: scroll  Esc/q: close",
        Style::default().fg(Color::Gray),
    );
    f.render_widget(
        Paragraph::new(Line::from(hint)).alignment(Alignment::Center),
        v_chunks[1],
    );
}
