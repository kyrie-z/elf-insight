use crate::app::{App, Focus};
use crate::ui::search;
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct OverviewState {
    pub scroll: usize,
    pub selected_line: usize,
}

impl OverviewState {
    pub fn new() -> Self {
        OverviewState { scroll: 0, selected_line: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;

    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("ELF Header"));
    lines.push(format!("  Magic:   {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!("  Class:   {}", if data.class == 2 { "ELF64" } else { "ELF32" }));
    lines.push(format!("  Data:    {}", if data.data == 1 { "2's complement, little endian" } else { "2's complement, big endian" }));
    lines.push(format!("  Version: {} (current)", data.version));
    lines.push(format!("  OS/ABI:  {}", data.os_abi));
    let type_display = if data.is_pie {
        format!("{} (PIE executable)", data.elf_type)
    } else {
        data.elf_type.clone()
    };
    lines.push(format!("  Type:    {}", type_display));
    lines.push(format!("  Machine: {}", data.machine));
    lines.push(format!("  Entry:   0x{:x}", data.entry));
    lines.push(format!("  PH off:  0x{:x} ({} entries, {} bytes each)", data.phoff, data.phnum, data.phentsize));
    lines.push(format!("  SH off:  0x{:x} ({} entries, {} bytes each)", data.shoff, data.shnum, data.shentsize));
    lines.push(format!("  Flags:   0x{:x}", data.flags));
    lines.push(String::new());

    lines.push(format!("[Nr] Name                  Type       Address    Offset    Size      Flags"));
    for s in &data.sections {
        let name = if s.name.len() > 20 { format!("{:.20}", s.name) } else { format!("{:20}", s.name) };
        lines.push(format!("[{:2}] {} {:10} 0x{:08x} 0x{:06x} 0x{:06x} {:3}", s.index, name, s.ty, s.addr, s.offset, s.size, s.flags));
    }
    lines.push(String::new());

    lines.push(format!("Program Headers:  Type       Offset   VirtAddr  PhysAddr  FileSiz  MemSiz   Flg Align"));
    for s in &data.segments {
        lines.push(format!("  {:14} 0x{:06x} 0x{:08x} 0x{:08x} 0x{:06x} 0x{:06x} {:3} 0x{:x}", s.ty, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align));
    }

    let total = lines.len();
    let visible = area.height.saturating_sub(2) as usize;
    let max_scroll = total.saturating_sub(visible);

    if app.overview.selected_line >= total {
        app.overview.selected_line = total.saturating_sub(1);
    }
    // Auto-scroll to keep cursor visible
    let sel = app.overview.selected_line;
    if sel < app.overview.scroll {
        app.overview.scroll = sel;
    }
    if sel >= app.overview.scroll + visible {
        app.overview.scroll = sel.saturating_sub(visible - 1);
    }
    if app.overview.scroll > max_scroll {
        app.overview.scroll = max_scroll;
    }

    let query = &app.search.query;
    let hl = Style::default().fg(Color::Yellow).bg(Color::Rgb(80, 80, 0));
    let cursor_style = Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 100));

    let start = app.overview.scroll;
    let end = (start + visible).min(total);
    let text_lines: Vec<Line> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_num = start + i;
            let hl_line = search::highlight_line(line, query, hl);
            if line_num == app.overview.selected_line {
                Line::from(hl_line.iter().map(|s| Span::styled(s.content.clone(), cursor_style)).collect::<Vec<_>>())
            } else {
                hl_line
            }
        })
        .collect();

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let p = Paragraph::new(text_lines)
        .block(Block::default().borders(Borders::ALL).title(format!("Overview - {}", app.data.file_path)).border_style(border_style));

    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.overview.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}