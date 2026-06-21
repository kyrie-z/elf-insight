use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

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

    let cursor_style = Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 100));

    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::raw(format!("{:10} │ ", "Offset")),
        Span::raw("00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F"),
        Span::raw(" │ ASCII"),
    ]));
    lines.push(Line::from("─".repeat(area.width as usize - 2)));

    let base_addr = section.addr;
    let start_row = app.hexdump.scroll;

    for row in start_row..(start_row + visible_rows).min(total_rows) {
        let offset = row * BYTES_PER_ROW;
        let end = (offset + BYTES_PER_ROW).min(data.len());
        let row_data = &data[offset..end];

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(format!("0x{:08x} │ ", base_addr + offset as u64)));

        // Hex bytes
        for i in 0..BYTES_PER_ROW {
            if i == 8 {
                spans.push(Span::raw(" "));
            }
            if i < row_data.len() {
                let byte_pos = offset + i;
                let s = format!("{:02x}", row_data[i]);
                if byte_pos == app.hexdump.cursor_offset {
                    spans.push(Span::styled(s, cursor_style));
                } else {
                    spans.push(Span::raw(s));
                }
                spans.push(Span::raw(" "));
            } else {
                spans.push(Span::raw("   "));
            }
        }
        spans.push(Span::raw("│ "));

        // ASCII
        for i in 0..BYTES_PER_ROW {
            if i < row_data.len() {
                let byte_pos = offset + i;
                let ch = if row_data[i].is_ascii_graphic() || row_data[i] == b' ' {
                    row_data[i] as char
                } else {
                    '·'
                };
                if byte_pos == app.hexdump.cursor_offset {
                    spans.push(Span::styled(ch.to_string(), cursor_style));
                } else {
                    spans.push(Span::raw(ch.to_string()));
                }
            } else {
                spans.push(Span::raw(" "));
            }
        }

        let line = Line::from(spans);
        lines.push(line);
    }

    let cursor_addr = section.addr + app.hexdump.cursor_offset as u64;
    let sector_end = section.addr + section.size;

    let modes = crate::app::available_modes(section);
    let mode_str: Vec<&str> = modes.iter().map(|m| match m {
        crate::app::SectionViewMode::Hexdump => "Hexdump",
        crate::app::SectionViewMode::Disassembly => "Disasm",
        crate::app::SectionViewMode::Strings => "Strings",
        crate::app::SectionViewMode::Dynamic => "Dynamic",
    }).collect();

    let title = format!(
        "{} - 0x{:x}-0x{:x} [Hexdump] {}  0x{:x}",
        section.name,
        section.addr,
        sector_end,
        mode_str.join("|"),
        cursor_addr
    );

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll)
        .position(app.hexdump.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}