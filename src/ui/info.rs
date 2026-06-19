use crate::app::{App, Focus};
use crate::ui::tree::TreeNodeType;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct InfoState {
    pub scroll: usize,
}

impl InfoState {
    pub fn new() -> Self {
        InfoState { scroll: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let node_type = app.tree.selected_node.clone();
    let lines = match node_type {
        Some(TreeNodeType::ElfHeader) => render_elf_header(app),
        Some(TreeNodeType::SectionHeader { index }) => render_section_header(app, index),
        Some(TreeNodeType::SectionBody { index }) => render_section_body_info(app, index),
        Some(TreeNodeType::Segment { index }) => render_segment(app, index),
        Some(TreeNodeType::Symbol { index }) => render_symbol(app, index),
        _ => vec!["Select a node to view details".into()],
    };

    let text = lines.join("\n");
    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Details").border_style(border_style));
    f.render_widget(p, area);
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
    let s = &app.data.segments[index];
    vec![
        format!("Type:       {}", s.ty),
        format!("Flags:      {}", s.flags),
        format!("Offset:     0x{:x}", s.offset),
        format!("VirtAddr:   0x{:016x}", s.vaddr),
        format!("PhysAddr:   0x{:016x}", s.paddr),
        format!("FileSiz:    0x{:x} ({} bytes)", s.filesz, s.filesz),
        format!("MemSiz:     0x{:x} ({} bytes)", s.memsz, s.memsz),
        format!("Align:      0x{:x}", s.align),
    ]
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