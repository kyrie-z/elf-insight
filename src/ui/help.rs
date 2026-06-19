use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(f: &mut Frame, _area: Rect) {
    let help_text = vec![
        "┌─ Keybindings ─────────────────────────────────────────────┐",
        "│                                                            │",
        "│  Navigation                          Search                │",
        "│  ──────────                          ──────                │",
        "│  ↑↓    Move focus          /     Open search              │",
        "│  →/Enter  Expand node      Enter Execute search           │",
        "│  ←     Collapse node       n/N   Next/prev result         │",
        "│  Tab   Switch panel        Esc   Close search             │",
        "│                                                            │",
        "│  Scrolling                          View                  │",
        "│  ─────────                          ────                  │",
        "│  ↑↓    Line scroll         ←→    Switch function (disasm) │",
        "│  PgUp/PgDn  Page scroll    g     Goto offset (hexdump)    │",
        "│  Home/End   Jump top/bottom                               │",
        "│                                                            │",
        "│  Global                                                    │",
        "│  ──────                                                    │",
        "│  q     Quit                                                │",
        "│  ?/h   Toggle this help                                   │",
        "│  Esc   Close help                                         │",
        "│                                                            │",
        "└────────────────────────────────────────────────────────────┘",
    ]
    .join("\n");

    let popup_area = centered_rect(60, 60, f.area());

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