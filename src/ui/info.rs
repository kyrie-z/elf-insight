use crate::app::{App, Focus};
use crate::ui::search;
use crate::ui::tree::TreeNodeType;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct InfoState {
    pub scroll: usize,
    pub selected_line: usize,
}

impl InfoState {
    pub fn new() -> Self {
        InfoState { scroll: 0, selected_line: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let node_type = app.tree.selected_node.clone();
    let lines = match node_type {
        Some(TreeNodeType::ElfHeader) => render_elf_header(app),
        Some(TreeNodeType::ProgramHeaders) => render_program_headers(app),
        Some(TreeNodeType::SectionHeaders) => render_section_headers(app),
        Some(TreeNodeType::SectionHeader { index }) => render_section_header(app, index),
        Some(TreeNodeType::SectionBody { index }) => {
            let s = &app.data.sections[index];
            if s.name == ".dynamic" && !app.data.dynamic.is_empty() {
                render_dynamic(app)
            } else {
                render_section_body_info(app, index)
            }
        },
        Some(TreeNodeType::Segment { index }) => render_segment(app, index),
        Some(TreeNodeType::Symbol { index }) => render_symbol(app, index),
        _ => vec!["Select a node to view details".into()],
    };

    let total = lines.len();
    let visible = area.height.saturating_sub(2) as usize;
    let max_scroll = total.saturating_sub(visible);

    if app.info.selected_line >= total {
        app.info.selected_line = total.saturating_sub(1);
    }
    let sel = app.info.selected_line;
    if sel < app.info.scroll {
        app.info.scroll = sel;
    }
    if sel >= app.info.scroll + visible {
        app.info.scroll = sel.saturating_sub(visible - 1);
    }
    if app.info.scroll > max_scroll {
        app.info.scroll = max_scroll;
    }

    let query = &app.search.query;
    let hl = Style::default().fg(Color::Yellow).bg(Color::Rgb(80, 80, 0));
    let cursor_style = Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 100));

    let start = app.info.scroll;
    let end = (start + visible).min(total);
    let text_lines: Vec<Line> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_num = start + i;
            let hl_line = search::highlight_line(line, query, hl);
            if line_num == app.info.selected_line {
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

    let title = match &app.tree.selected_node {
        Some(TreeNodeType::SectionBody { index }) => {
            let s = &app.data.sections[*index];
            if s.name == ".dynamic" && !app.data.dynamic.is_empty() {
                format!("{} - {} entries [Dynamic]", s.name, app.data.dynamic.len())
            } else {
                format!("Details [Info]")
            }
        }
        _ => "Details".into(),
    };

    let p = Paragraph::new(text_lines).block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));
    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.info.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn render_elf_header(app: &App) -> Vec<String> {
    let d = &app.data;
    vec![
        format!("Magic:         {:02x?}", &d.raw_bytes[..16]),
        format!("Class:         {}", if d.class == 2 { "ELF64" } else { "ELF32" }),
        format!("Data:          {}", if d.data == 1 { "2's complement, little endian" } else { "2's complement, big endian" }),
        format!("Version:       {} (current)", d.version),
        format!("OS/ABI:        {}", d.os_abi),
        format!("ABI Version:   {}", d.abi_version),
        format!("Type:          {}", d.elf_type),
        format!("Machine:       {}", d.machine),
        format!("Version:       0x{:x}", d.version),
        format!("Entry point:   0x{:x}", d.entry),
        format!("PH offset:     0x{:x} ({} entries, {} bytes each)", d.phoff, d.phnum, d.phentsize),
        format!("SH offset:     0x{:x} ({} entries, {} bytes each)", d.shoff, d.shnum, d.shentsize),
        format!("Flags:         0x{:x}", d.flags),
        format!("EH size:       {} bytes", d.ehsize),
        format!("SH strndx:     {}", d.shstrndx),
    ]
}

fn render_dynamic(app: &App) -> Vec<String> {
    let d = &app.data;
    if d.dynamic.is_empty() {
        return vec!["No dynamic entries".into()];
    }
    let mut lines = vec![
        format!("Dynamic entries: {} total", d.dynamic.len()),
        format!("{:<4} {:<20} {:<20}", "Idx", "Tag", "Value"),
    ];
    for (i, entry) in d.dynamic.iter().enumerate() {
        lines.push(format!(
            "{:<4} {:<20} {:<20}",
            i, entry.tag, entry.value
        ));
    }
    lines
}

fn render_program_headers(app: &App) -> Vec<String> {
    let d = &app.data;
    let mut lines = vec![
        format!("Program Headers: {} entries at offset 0x{:x}", d.phnum, d.phoff),
        format!("Entry size: {} bytes", d.phentsize),
        format!(""),
        format!("{:<6} {:<14} {:<10} {:<10} {:<10} {:<10} {:<10} {:<6} {:<6}",
            "Idx", "Type", "Offset", "VirtAddr", "PhysAddr", "FileSiz", "MemSiz", "Flags", "Align"),
    ];
    for s in &d.segments {
        lines.push(format!(
            "{:<6} {:<14} 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x} {:<6} 0x{:x}",
            s.index, s.ty, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align
        ));
    }
    lines
}

fn render_section_headers(app: &App) -> Vec<String> {
    let d = &app.data;
    let mut lines = vec![
        format!("Section Headers: {} entries at offset 0x{:x}", d.shnum, d.shoff),
        format!("Entry size: {} bytes", d.shentsize),
        format!(""),
        format!("{:<4} {:<20} {:<12} {:<12} {:<10} {:<10} {:<6}",
            "Idx", "Name", "Type", "Addr", "Offset", "Size", "Flags"),
    ];
    for s in &d.sections {
        let name = if s.name.len() > 20 { format!("{:.17}...", s.name) } else { s.name.clone() };
        lines.push(format!(
            "{:<4} {:<20} {:<12} 0x{:010x} 0x{:08x} 0x{:08x} {:<6}",
            s.index, name, s.ty, s.addr, s.offset, s.size, s.flags
        ));
    }
    lines
}

fn render_section_header(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.sections.len() {
        return vec!["Section not found".into()];
    }
    let s = &app.data.sections[index];
    vec![
        format!("Name:      {}", s.name),
        format!("Type:      {}", s.ty),
        format!("Flags:     {}", s.flags),
        format!("Address:   0x{:016x}", s.addr),
        format!("Offset:    0x{:x}", s.offset),
        format!("Size:      0x{:x} ({} bytes)", s.size, s.size),
        format!("Link:      {}", s.index),
        format!("Info:      0x{:x}", s.index),
        format!("Addr align: 0x{:x}", s.index),
        format!("Ent size:  0x{:x}", s.index),
    ]
}

fn render_segment(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.segments.len() {
        return vec!["Segment not found".into()];
    }
    let seg = &app.data.segments[index];
    let mut lines = vec![
        format!("Type:       {}", seg.ty),
        format!("Flags:      {}", seg.flags),
        format!("VirtAddr:   0x{:016x}", seg.vaddr),
        format!("VirtSize:   0x{:x} ({} bytes)", seg.memsz, seg.memsz),
        format!("File offset: 0x{:x}", seg.offset),
        format!("File size:   0x{:x} ({} bytes)", seg.filesz, seg.filesz),
        format!("Align:      0x{:x}", seg.align),
        String::new(),
    ];

    // Find sections mapped into this segment
    let mut mapped: Vec<&crate::elf::parser::SectionInfo> = app
        .data
        .sections
        .iter()
        .filter(|s| {
            seg.memsz > 0
                && s.addr >= seg.vaddr
                && s.addr + s.size <= seg.vaddr + seg.memsz
                && s.size > 0
                && s.addr > 0
        })
        .collect();
    mapped.sort_by_key(|s| s.addr);

    if mapped.is_empty() {
        lines.push(format!("No sections mapped to this segment"));
    } else {
        lines.push(format!("Mapped sections:"));
        for s in mapped {
            lines.push(format!(
                "  [{:2}] {:<20} 0x{:08x}-0x{:08x}",
                s.index, s.name, s.addr, s.addr + s.size
            ));
        }
    }
    lines
}

fn render_section_body_info(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.sections.len() {
        return vec!["Section not found".into()];
    }
    let s = &app.data.sections[index];
    let data_size = s.data.len();
    vec![
        format!("Name:      {}", s.name),
        format!("Type:      {}", s.ty),
        format!("Flags:     {}", s.flags),
        format!("Address:   0x{:016x}", s.addr),
        format!("Offset:    0x{:x}", s.offset),
        format!("Size:      0x{:x} ({} bytes)", s.size, s.size),
        format!("Data size: {} bytes", data_size),
    ]
}

fn render_symbol(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.symbols.len() {
        return vec!["Symbol not found".into()];
    }
    let sym = &app.data.symbols[index];
    let type_str = match sym.ty {
        crate::elf::parser::SymbolType::Function => "FUNC",
        crate::elf::parser::SymbolType::Object => "OBJECT",
        crate::elf::parser::SymbolType::Section => "SECTION",
        crate::elf::parser::SymbolType::File => "FILE",
        crate::elf::parser::SymbolType::Other(_) => "OTHER",
    };
    vec![
        format!("Name:      {}", sym.name),
        format!("Type:      {}", type_str),
        format!("Bind:      {}", sym.bind),
        format!("Vis:       {}", sym.vis),
        format!("Value:     0x{:016x}", sym.addr),
        format!("Size:      {} bytes", sym.size),
        format!("Shndx:     {}", sym.shndx),
    ]
}