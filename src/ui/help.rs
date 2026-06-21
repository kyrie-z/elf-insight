use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(f: &mut Frame, _area: Rect) {
    let help_text = vec![
        " Navigation (less-style)",
        " ──────────────────────",
        "  ↑↓ / jk   Line up / down",
        "  gg        Jump to top",
        "  G         Jump to bottom",
        "  Space     Page down",
        "  b         Page up",
        "  u / d     Half-page up / down",
        "  Home/End  Jump to top / bottom",
        "  PgUp/Dn   Page up / down",
        "",
        " Panels & Tree",
        " ─────────────",
        "  Tab       Switch tree ↔ detail",
        "  Enter     Expand node / Enter layout region",
        "  ←         Collapse node (tree)",
        "  ←→        Hex byte cursor / Disasm panel switch",
        "  h / l     Hexdump byte cursor (left/right)",
        "  m         Cycle section view mode",
        "  Esc       Clear search → Back to func list → Back",
        "",
        " Search",
        " ──────",
        "  /         Open search bar",
        "  Enter     Execute search (in search bar)",
        "  n / N     Next / previous result",
        "  Esc       Close search bar / Clear highlights",
        "",
        " Disassembly",
        " ───────────",
        "  ↑↓/jk    Switch function (func list focus)",
        "  ←→        Switch panel (func list ↔ instructions)",
        "  Esc       Back to func list",
        "",
        " Help & Quit",
        " ───────────",
        "  ? / H     Toggle this help",
        "  q / Esc   Close help / Quit",
    ]
    .join("\n");

    let popup_area = centered_rect(60, 90, f.area());

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