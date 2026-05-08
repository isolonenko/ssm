use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, filter_text: &str, active: bool) {
    let show = active || !filter_text.is_empty();

    let (text, style) = if show {
        let cursor = if active { "█" } else { "" };
        let content = format!("/ {}{}", filter_text, cursor);
        let color = if active { Color::Yellow } else { Color::Gray };
        (content, Style::default().fg(color))
    } else {
        (String::new(), Style::default().fg(Color::Gray))
    };

    let line = Line::from(Span::styled(text, style));
    let para = Paragraph::new(line);
    f.render_widget(para, area);
}
