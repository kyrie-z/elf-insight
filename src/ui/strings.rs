use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct StringsState {
    pub scroll: usize,
}

impl StringsState {
    pub fn new() -> Self {
        StringsState { scroll: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let section_index = match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => *index,
        _ => {
            let p = Paragraph::new("No string section selected")
                .block(Block::default().borders(Borders::ALL).title("Strings"));
            f.render_widget(p, area);
            return;
        }
    };

    let section = &app.data.sections[section_index];
    let data = &section.data;

    let strings: Vec<(usize, String)> = extract_strings(data);

    let mut lines = Vec::new();
    for (offset, s) in &strings {
        lines.push(format!("  0x{:08x}  {}", section.addr + *offset as u64, s));
    }

    let total = lines.len();
    let visible = area.height.saturating_sub(2) as usize;
    let max_scroll = total.saturating_sub(visible);

    if app.strings.scroll > max_scroll {
        app.strings.scroll = max_scroll;
    }

    let visible_lines: Vec<&str> = lines
        .iter()
        .skip(app.strings.scroll)
        .take(visible)
        .map(|s| s.as_str())
        .collect();

    let text = visible_lines.join("\n");
    let title = format!("{} - {} strings", section.name, strings.len());

    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.strings.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn extract_strings(data: &[u8]) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut start = None;

    for (i, &byte) in data.iter().enumerate() {
        if byte.is_ascii_graphic() || byte == b' ' {
            if start.is_none() {
                start = Some(i);
            }
        } else if byte == 0 {
            if let Some(s) = start {
                let len = i - s;
                if len >= 2 {
                    let string = String::from_utf8_lossy(&data[s..i]).to_string();
                    results.push((s, string));
                }
                start = None;
            }
        } else {
            start = None;
        }
    }

    results
}