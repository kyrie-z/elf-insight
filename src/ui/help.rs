use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(f: &mut Frame, _area: Rect) {
    let help_text = vec![
        " Navigation (vi-style in detail panel)",
        " ─────────────────────────────────────",
        "  ↑↓ / jk   Move focus / Line scroll",
        "  →, Enter  Expand node (tree) / Next byte (hex)",
        "  ←         Collapse node (tree) / Prev byte (hex)",
        "  h / l     Prev/next byte (hexdump)",
        "  Tab       Switch panel (tree ↔ detail)",
        "",
        " Search",
        " ──────",
        "  /         Open search bar",
        "  Enter     Execute search",
        "  n / N     Next / previous result",
        "  Esc       Close search",
        "",
        " Scrolling",
        " ─────────",
        "  PgUp/Dn   Page scroll",
        "  u / d     Half-page up / down (vi-style)",
        "  gg        Jump to top (detail panel)",
        "  G         Jump to bottom (detail panel)",
        "  Home/End  Jump to top / bottom",
        "",
        " Views",
        " ─────",
        "  ←→        Switch function (disassembly)",
        "",
        " Global",
        " ──────",
        "  q         Quit",
        "  ?         Toggle this help",
        "  Esc / q   Close help",
    ]
    .join("\n");

    let popup_area = centered_rect(52, 80, f.area());

    let p = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)).fg(Color::White));

    f.render_widget(Clear, popup_area);
    f.render_widget(p, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}