use crate::app::{App, DetailView};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct SearchState {
    pub active: bool,
    pub input: String,
    pub results: Vec<usize>,
    pub current_result: usize,
    pub no_matches_timer: u8,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            active: false,
            input: String::new(),
            results: Vec::new(),
            current_result: 0,
            no_matches_timer: 0,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.search.active {
        return;
    }
    let search_area = Rect {
        y: area.height.saturating_sub(3),
        height: 3,
        width: area.width.min(40),
        x: area.width.saturating_sub(40),
    };
    let display = if app.search.no_matches_timer > 0 {
        format!("/{}  [No matches]", app.search.input)
    } else {
        format!("/{}_", app.search.input)
    };
    let p = Paragraph::new(display)
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(p, search_area);
}

pub fn do_search(app: &mut App) {
    let query = app.search.input.clone();
    if query.is_empty() {
        return;
    }

    app.search.results.clear();

    if query.starts_with("0x") || query.starts_with("0X") {
        if let Ok(addr) = u64::from_str_radix(&query[2..], 16) {
            match app.current_view {
                DetailView::Hexdump => {
                    if let Some(section) = get_current_section(app) {
                        let offset = addr.saturating_sub(section.addr);
                        if (offset as u64) < section.size {
                            app.search.results.push(offset as usize);
                        }
                    }
                }
                DetailView::Disassembly => {
                    if let Some(disasm) = &app.disasm_cache {
                        for (i, insn) in disasm.all_instructions.iter().enumerate() {
                            if insn.address == addr {
                                app.search.results.push(i);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    } else {
        match app.current_view {
            DetailView::Overview => {
                for (i, line) in build_overview_lines(app).iter().enumerate() {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        app.search.results.push(i);
                    }
                }
            }
            DetailView::Disassembly => {
                if let Some(disasm) = &app.disasm_cache {
                    for (i, insn) in disasm.all_instructions.iter().enumerate() {
                        if insn.mnemonic.contains(&query)
                            || insn.operands.contains(&query)
                        {
                            app.search.results.push(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if app.search.results.is_empty() {
        app.search.no_matches_timer = 120;
    } else {
        app.search.current_result = 0;
        apply_search_result(app);
    }
}

pub fn next_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = (app.search.current_result + 1) % app.search.results.len();
    apply_search_result(app);
}

pub fn prev_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = if app.search.current_result == 0 {
        app.search.results.len() - 1
    } else {
        app.search.current_result - 1
    };
    apply_search_result(app);
}

fn apply_search_result(app: &mut App) {
    if let Some(&pos) = app.search.results.get(app.search.current_result) {
        match app.current_view {
            DetailView::Overview => {
                app.overview.scroll = pos;
            }
            DetailView::Hexdump => {
                app.hexdump.scroll = pos / 16;
            }
            DetailView::Disassembly => {
                app.disasm.scroll = pos;
            }
            _ => {}
        }
    }
}

fn get_current_section(app: &App) -> Option<&crate::elf::parser::SectionInfo> {
    match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => {
            Some(&app.data.sections[*index])
        }
        _ => None,
    }
}

fn build_overview_lines(app: &App) -> Vec<String> {
    let data = &app.data;
    let mut lines = Vec::new();
    lines.push(format!("ELF Header Magic: {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!("{} {} {}", data.elf_type, data.machine, data.os_abi));
    lines.push(format!("Entry: 0x{:x}", data.entry));
    for s in &data.sections {
        lines.push(format!("{} {} {:?}", s.name, s.ty, s.addr));
    }
    for s in &data.segments {
        lines.push(format!("{} {:?}", s.ty, s.vaddr));
    }
    lines
}