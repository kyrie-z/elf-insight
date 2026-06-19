use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

const BYTES_PER_ROW: usize = 16;

pub enum HexCursor {
    Hex,
    Ascii,
}

pub struct HexdumpState {
    pub scroll: usize,
    pub cursor_offset: usize,
    pub cursor_mode: HexCursor,
    pub goto_input: String,
    pub goto_mode: bool,
}

impl HexdumpState {
    pub fn new() -> Self {
        HexdumpState {
            scroll: 0,
            cursor_offset: 0,
            cursor_mode: HexCursor::Hex,
            goto_input: String::new(),
            goto_mode: false,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let section_index = match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => *index,
        _ => {
            let p = Paragraph::new("No section selected")
                .block(Block::default().borders(Borders::ALL).title("Hexdump"));
            f.render_widget(p, area);
            return;
        }
    };

    let section = &app.data.sections[section_index];
    let data = &section.data;

    if data.is_empty() {
        let p = Paragraph::new(format!("Section {} has no data (NOBITS)", section.name))
            .block(Block::default().borders(Borders::ALL).title("Hexdump"));
        f.render_widget(p, area);
        return;
    }

    let total_rows = data.len().div_ceil(BYTES_PER_ROW);
    let visible_rows = area.height.saturating_sub(3) as usize;
    let max_scroll = total_rows.saturating_sub(visible_rows);

    if app.hexdump.scroll > max_scroll {
        app.hexdump.scroll = max_scroll;
    }

    if app.hexdump.cursor_offset >= data.len() {
        app.hexdump.cursor_offset = data.len().saturating_sub(1);
    }

    let cursor_row = app.hexdump.cursor_offset / BYTES_PER_ROW;
    if cursor_row < app.hexdump.scroll {
        app.hexdump.scroll = cursor_row;
    }
    if cursor_row >= app.hexdump.scroll + visible_rows {
        app.hexdump.scroll = cursor_row.saturating_sub(visible_rows - 1);
    }
    if app.hexdump.scroll > max_scroll {
        app.hexdump.scroll = max_scroll;
    }

    let mut lines = Vec::new();

    // Header
    lines.push(format!(
        "{:10} │ {:47} │ {}",
        "Offset", "00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F", "ASCII"
    ));
    lines.push(format!("{}", "─".repeat(area.width as usize - 2)));

    let base_addr = section.addr;
    let start_row = app.hexdump.scroll;

    for row in start_row..(start_row + visible_rows).min(total_rows) {
        let offset = row * BYTES_PER_ROW;
        let end = (offset + BYTES_PER_ROW).min(data.len());
        let row_data = &data[offset..end];

        // Hex part
        let hex_str: Vec<String> = row_data.iter().enumerate().map(|(i, b)| {
            let s = format!("{:02x}", b);
            if offset + i == app.hexdump.cursor_offset {
                format!("[{}]", s)
            } else {
                s
            }
        }).collect();
        let hex_line = hex_str.join(" ");
        let hex_padded = format!("{:47}", hex_line);

        // ASCII part
        let ascii_str: String = row_data.iter().enumerate().map(|(_i, &b)| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '·'
            }
        }).collect();

        lines.push(format!(
            "0x{:08x} │ {} │ {}",
            base_addr + offset as u64,
            hex_padded,
            ascii_str
        ));
    }

    let text = lines.join("\n");
    let title = format!(
        "{} - 0x{:x}-0x{:x}",
        section.name,
        section.addr,
        section.addr + section.size
    );

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(p, area);
}