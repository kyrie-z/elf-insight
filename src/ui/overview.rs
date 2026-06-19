use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct OverviewState {
    pub scroll: usize,
}

impl OverviewState {
    pub fn new() -> Self {
        OverviewState { scroll: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;

    let mut lines = Vec::new();

    // ELF Header
    lines.push(format!("ELF Header"));
    lines.push(format!("  Magic:   {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!(
        "  Class:   {}",
        match data.class {
            1 => "ELF32",
            2 => "ELF64",
            _ => "Unknown",
        }
    ));
    lines.push(format!(
        "  Data:    {}",
        match data.data {
            1 => "2's complement, little endian",
            2 => "2's complement, big endian",
            _ => "Unknown",
        }
    ));
    lines.push(format!("  Version: {} (current)", data.version));
    lines.push(format!("  OS/ABI:  {}", data.os_abi));
    lines.push(format!("  Type:    {}", data.elf_type));
    lines.push(format!("  Machine: {}", data.machine));
    lines.push(format!("  Entry:   0x{:x}", data.entry));
    lines.push(format!(
        "  PH off:  0x{:x} ({} entries, {} bytes each)",
        data.phoff, data.phnum, data.phentsize
    ));
    lines.push(format!(
        "  SH off:  0x{:x} ({} entries, {} bytes each)",
        data.shoff, data.shnum, data.shentsize
    ));
    lines.push(format!("  Flags:   0x{:x}", data.flags));
    lines.push(String::new());

    // Section Headers
    lines.push(format!(
        "Section Headers: [Nr] Name                 Type        Address  Offset   Size     Flags"
    ));
    for s in &data.sections {
        let name = if s.name.len() > 20 {
            format!("{}...", &s.name[..17])
        } else {
            format!("{:20}", s.name)
        };
        lines.push(format!(
            "  [{:2}] {} {:10} 0x{:08x} 0x{:06x} 0x{:06x} {:3}",
            s.index, name, s.ty, s.addr, s.offset, s.size, s.flags
        ));
    }
    lines.push(String::new());

    // Program Headers
    lines.push(format!(
        "Program Headers:  Type       Offset   VirtAddr  PhysAddr  FileSiz  MemSiz   Flg Align"
    ));
    for s in &data.segments {
        lines.push(format!(
            "  {:14} 0x{:06x} 0x{:08x} 0x{:08x} 0x{:06x} 0x{:06x} {:3} 0x{:x}",
            s.ty, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align
        ));
    }

    let text = lines.join("\n");
    let total_lines = lines.len() as u16;
    let area_height = area.height.saturating_sub(2);

    let max_scroll = total_lines.saturating_sub(area_height) as usize;
    if app.overview.scroll > max_scroll {
        app.overview.scroll = max_scroll;
    }

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(format!("Overview - {}", app.data.file_path)))
        .scroll((app.overview.scroll as u16, 0));

    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll)
        .position(app.overview.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}