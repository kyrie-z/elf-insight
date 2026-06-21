use crate::elf::parser::ElfData;
use crate::elf::disasm::{DisasmResult, disassemble_section, merge_symbols_with_functions};
use crate::ui::tree::{TreeNode, TreeNodeType, TreeState};
use crate::ui::overview::OverviewState;
use crate::ui::info::InfoState;
use crate::ui::hexdump::HexdumpState;
use crate::ui::disasm::DisasmState;
use crate::ui::strings::StringsState;
use crate::ui::layout_map::LayoutMapState;
use crate::ui::search::SearchState;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

#[derive(Clone)]
pub enum DetailView {
    Overview,
    LayoutMap,
    StructuredInfo,
    Hexdump,
    Disassembly,
    Strings,
}

pub struct App {
    pub data: ElfData,
    pub tree: TreeState,
    pub overview: OverviewState,
    pub layout_map: LayoutMapState,
    pub info: InfoState,
    pub hexdump: HexdumpState,
    pub disasm: DisasmState,
    pub strings: StringsState,
    pub search: SearchState,
    pub current_view: DetailView,
    pub focus: Focus,
    pub should_quit: bool,
    pub show_help: bool,
    pub pending_g: bool,
    pub disasm_cache: Option<DisasmResult>,
    pub current_disasm_section: Option<usize>,
    pub prev_view: Option<DetailView>,
    pub prev_node: Option<TreeNodeType>,
    pub disasm_subfocus: DisasmSubFocus,
    pub section_view_mode: Option<SectionViewMode>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum SectionViewMode {
    Hexdump,
    Disassembly,
    Strings,
    Dynamic,
    Info,
}

#[derive(PartialEq, Eq)]
pub enum DisasmSubFocus {
    FuncList,
    Instructions,
}

#[derive(PartialEq, Eq)]
pub enum Focus {
    Tree,
    Detail,
    Search,
}

impl App {
    pub fn new(data: ElfData) -> Self {
        let tree = build_tree(&data);
        App {
            data,
            tree: TreeState::new(tree),
            overview: OverviewState::new(),
            layout_map: LayoutMapState::new(),
            info: InfoState::new(),
            hexdump: HexdumpState::new(),
            disasm: DisasmState::new(),
            strings: StringsState::new(),
            search: SearchState::new(),
            current_view: DetailView::Overview,
            focus: Focus::Tree,
            should_quit: false,
            show_help: false,
            pending_g: false,
            disasm_cache: None,
            current_disasm_section: None,
            prev_view: None,
            prev_node: None,
            disasm_subfocus: DisasmSubFocus::FuncList,
            section_view_mode: None,
        }
    }
}

fn build_tree(data: &ElfData) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    nodes.push(TreeNode {
        label: "Overview".into(),
        node_type: TreeNodeType::Overview,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    nodes.push(TreeNode {
        label: "Layout Map".into(),
        node_type: TreeNodeType::LayoutMap,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    nodes.push(TreeNode {
        label: "ELF Header".into(),
        node_type: TreeNodeType::ElfHeader,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    nodes.push(TreeNode {
        label: format!("Program Headers (0x{:x}-0x{:x})", data.phoff, data.phoff + data.phnum as u64 * data.phentsize as u64),
        node_type: TreeNodeType::ProgramHeaders,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    nodes.push(TreeNode {
        label: format!("Section Headers (0x{:x}-0x{:x})", data.shoff, data.shoff + data.shnum as u64 * data.shentsize as u64),
        node_type: TreeNodeType::SectionHeaders,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    let section_children: Vec<TreeNode> = data
        .sections
        .iter()
        .map(|s| TreeNode {
            label: if s.size > 0 && s.offset > 0 {
                format!("[{}] {} (0x{:x}-0x{:x})", s.index, s.name, s.offset, s.offset + s.size)
            } else {
                format!("[{}] {}", s.index, s.name)
            },
            node_type: TreeNodeType::SectionBody { index: s.index },
            depth: 1,
            children: vec![],
            expanded: true,
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Sections ({})", data.sections.len()),
        node_type: TreeNodeType::SectionsGroup,
        depth: 0,
        children: section_children,
        expanded: true,
    });

    let segment_children: Vec<TreeNode> = data
        .segments
        .iter()
        .map(|s| TreeNode {
            label: format!("[{}] {} (0x{:x}-0x{:x})", s.index, s.ty, s.vaddr, s.vaddr + s.memsz),
            node_type: TreeNodeType::Segment { index: s.index },
            depth: 1,
            children: vec![],
            expanded: true,
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Segments ({})", data.segments.len()),
        node_type: TreeNodeType::SegmentsGroup,
        depth: 0,
        children: segment_children,
        expanded: true,
    });

    let symbol_children: Vec<TreeNode> = data
        .symbols
        .iter()
        .enumerate()
        .map(|(i, sym)| {
            let prefix = match sym.ty {
                crate::elf::parser::SymbolType::Function => "[F]",
                crate::elf::parser::SymbolType::Object => "[O]",
                _ => "[?]",
            };
            TreeNode {
                label: format!("{} {}", prefix, sym.name),
                node_type: TreeNodeType::Symbol { index: i },
                depth: 1,
                children: vec![],
                expanded: true,
            }
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Symbols ({})", data.symbols.len()),
        node_type: TreeNodeType::SymbolsGroup,
        depth: 0,
        children: symbol_children,
        expanded: true,
    });

    nodes
}

pub fn run_app(data: ElfData) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(data);
    let res = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| render(f, app))?;
        handle_events(app)?;

        if app.search.no_matches_timer > 0 {
            app.search.no_matches_timer -= 1;
        }
    }
    Ok(())
}

fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.area());

    crate::ui::tree::render(f, app, chunks[0]);
    crate::ui::render_detail(f, app, chunks[1]);
    crate::ui::search::render(f, app, f.area());

    if app.show_help {
        crate::ui::help::render(f, f.area());
    }
}

fn handle_events(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if event::poll(std::time::Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_key(app, key.code);
            }
        }
    }
    Ok(())
}

fn scroll_mut(app: &mut App) -> &mut usize {
    match app.current_view {
        DetailView::Overview => &mut app.overview.scroll,
        DetailView::LayoutMap => &mut app.layout_map.scroll,
        DetailView::StructuredInfo => &mut app.info.scroll,
        DetailView::Hexdump => &mut app.hexdump.scroll,
        DetailView::Disassembly => &mut app.disasm.scroll,
        DetailView::Strings => &mut app.strings.scroll,
    }
}

fn sel_line_mut(app: &mut App) -> &mut usize {
    match app.current_view {
        DetailView::Overview => &mut app.overview.selected_line,
        DetailView::StructuredInfo => &mut app.info.selected_line,
        DetailView::Strings => &mut app.strings.selected_line,
        _ => &mut app.overview.selected_line,
    }
}

fn is_line_view(app: &App) -> bool {
    matches!(app.current_view, DetailView::Overview | DetailView::StructuredInfo | DetailView::Strings)
}

fn scroll_up(app: &mut App, n: usize) {
    if is_line_view(app) {
        *sel_line_mut(app) = sel_line_mut(app).saturating_sub(n);
    } else {
        *scroll_mut(app) = scroll_mut(app).saturating_sub(n);
    }
}

fn scroll_down(app: &mut App, n: usize) {
    if is_line_view(app) {
        *sel_line_mut(app) += n;
    } else {
        *scroll_mut(app) += n;
    }
}

fn scroll_top(app: &mut App) {
    if is_line_view(app) {
        *sel_line_mut(app) = 0;
    } else {
        *scroll_mut(app) = 0;
    }
}

fn scroll_bottom(app: &mut App) {
    if is_line_view(app) {
        *sel_line_mut(app) = usize::MAX;
    } else {
        *scroll_mut(app) = usize::MAX;
    }
}

fn handle_key(app: &mut App, key: KeyCode) {
    if app.show_help {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Char('h') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    if app.search.active {
        match key {
            KeyCode::Esc => {
                app.search.active = false;
                app.search.input.clear();
                app.search.results.clear();
                app.focus = Focus::Tree;
            }
            KeyCode::Enter => {
                crate::ui::search::do_search(app);
                app.search.active = false;
                app.focus = Focus::Detail;
            }
            KeyCode::Backspace => {
                app.search.input.pop();
            }
            KeyCode::Char(c) => {
                app.search.input.push(c);
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('q') => app.should_quit = true,

        // Back navigation
        KeyCode::Esc => {
            // Clear search highlights first
            if !app.search.query.is_empty() {
                app.search.query.clear();
                app.search.results.clear();
                app.search.current_result = 0;
            } else if app.focus == Focus::Detail
                && matches!(app.current_view, DetailView::Disassembly)
                && app.disasm_subfocus == DisasmSubFocus::Instructions
            {
                app.disasm_subfocus = DisasmSubFocus::FuncList;
            } else if let Some(ref prev) = app.prev_view {
                let v = prev.clone();
                let n = app.prev_node.clone();
                app.current_view = v;
                app.prev_view = None;
                app.prev_node = None;
                if let Some(node) = n {
                    app.tree.select_node(&node);
                }
            }
        }

        // Help
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('H') => {
            if app.focus == Focus::Detail {
                app.show_help = true;
            }
        }

        // Search
        KeyCode::Char('/') => {
            app.search.active = true;
            app.search.input.clear();
            app.focus = Focus::Search;
        }
        KeyCode::Char('n') => {
            crate::ui::search::next_result(app);
        }
        KeyCode::Char('N') => {
            crate::ui::search::prev_result(app);
        }

        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Tree => Focus::Detail,
                Focus::Detail => Focus::Tree,
                Focus::Search => Focus::Search,
            };
        }

        // less-style navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.pending_g = false;
            if app.focus == Focus::Tree {
                app.tree.move_up();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Hexdump => {
                        if app.hexdump.cursor_offset >= 16 {
                            app.hexdump.cursor_offset -= 16;
                        }
                        let row = app.hexdump.cursor_offset / 16;
                        if row < app.hexdump.scroll {
                            scroll_up(app, 1);
                        }
                    }
                    DetailView::Disassembly => {
                        if app.disasm_subfocus == DisasmSubFocus::FuncList {
                            if app.disasm.selected_function > 0 {
                                app.disasm.selected_function -= 1;
                                app.disasm.scroll = 0;
                            }
                        } else {
                            scroll_up(app, 1);
                        }
                    }
                    DetailView::LayoutMap => {
                        if app.layout_map.selected_row > 0 {
                            app.layout_map.selected_row -= 1;
                        }
                    }
                    _ => scroll_up(app, 1),
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.pending_g = false;
            if app.focus == Focus::Tree {
                app.tree.move_down();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Hexdump => {
                        app.hexdump.cursor_offset += 16;
                        let row = app.hexdump.cursor_offset / 16;
                        let visible_rows = 20;
                        if row >= app.hexdump.scroll.saturating_add(visible_rows - 1) {
                            scroll_down(app, 1);
                        }
                    }
                    DetailView::Disassembly => {
                        if app.disasm_subfocus == DisasmSubFocus::FuncList {
                            if let Some(ref disasm) = app.disasm_cache {
                                if app.disasm.selected_function + 1 < disasm.functions.len() {
                                    app.disasm.selected_function += 1;
                                    app.disasm.scroll = 0;
                                }
                            }
                        } else {
                            scroll_down(app, 1);
                        }
                    }
                    DetailView::LayoutMap => {
                        if app.layout_map.selected_row + 1 < app.layout_map.region_count {
                            app.layout_map.selected_row += 1;
                        }
                    }
                    _ => scroll_down(app, 1),
                }
            }
        }

        // Hexdump cursor movement
        KeyCode::Char('h') => {
            app.pending_g = false;
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
                if app.hexdump.cursor_offset > 0 {
                    app.hexdump.cursor_offset -= 1;
                    let row = app.hexdump.cursor_offset / 16;
                    if row < app.hexdump.scroll {
                        scroll_up(app, 1);
                    }
                }
            }
        }
        KeyCode::Char('l') => {
            app.pending_g = false;
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
                app.hexdump.cursor_offset += 1;
                let row = app.hexdump.cursor_offset / 16;
                let visible_rows = 20;
                if row >= app.hexdump.scroll.saturating_add(visible_rows - 1) {
                    scroll_down(app, 1);
                }
            }
        }
        KeyCode::Right => {
            app.pending_g = false;
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
                app.hexdump.cursor_offset += 1;
                let row = app.hexdump.cursor_offset / 16;
                let visible_rows = 20;
                if row >= app.hexdump.scroll.saturating_add(visible_rows - 1) {
                    scroll_down(app, 1);
                }
            } else if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
                if app.disasm_subfocus == DisasmSubFocus::FuncList {
                    app.disasm_subfocus = DisasmSubFocus::Instructions;
                }
            } else if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }
        KeyCode::Left => {
            app.pending_g = false;
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
                if app.hexdump.cursor_offset > 0 {
                    app.hexdump.cursor_offset -= 1;
                    let row = app.hexdump.cursor_offset / 16;
                    if row < app.hexdump.scroll {
                        scroll_up(app, 1);
                    }
                }
            } else if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
                if app.disasm_subfocus == DisasmSubFocus::Instructions {
                    app.disasm_subfocus = DisasmSubFocus::FuncList;
                }
            } else if app.focus == Focus::Tree {
                app.tree.collapse_current();
            }
        }
        KeyCode::Char('m') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                cycle_section_view(app);
            }
        }
        KeyCode::Enter => {
            app.pending_g = false;
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::LayoutMap) {
                layout_map_enter(app);
            } else if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
                app.disasm.scroll = 0;
            } else if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }

        // less-style: gg = top, G = bottom
        KeyCode::Char('g') => {
            if app.focus == Focus::Detail {
                if app.pending_g {
                    app.pending_g = false;
                    scroll_top(app);
                    if matches!(app.current_view, DetailView::Hexdump) {
                        app.hexdump.cursor_offset = 0;
                    }
                } else {
                    app.pending_g = true;
                }
            }
        }
        KeyCode::Char('G') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                if matches!(app.current_view, DetailView::Hexdump) {
                    if let Some(section) = get_hex_section(app) {
                        let len = section.data.len();
                        app.hexdump.cursor_offset = len.saturating_sub(1);
                    }
                }
                scroll_bottom(app);
            }
        }

        // less-style: Ctrl+u/d = half page, Space = page down, b = page up
        KeyCode::Char('u') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_up(app, 10);
            }
        }
        KeyCode::Char('d') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_down(app, 10);
            }
        }
        KeyCode::Char(' ') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_down(app, 20);
            }
        }
        KeyCode::Char('b') => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_up(app, 20);
            }
        }
        KeyCode::PageUp => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_up(app, 10);
            }
        }
        KeyCode::PageDown => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_down(app, 10);
            }
        }
        KeyCode::Home => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_top(app);
            }
        }
        KeyCode::End => {
            app.pending_g = false;
            if app.focus == Focus::Detail {
                scroll_bottom(app);
            }
        }
        _ => {
            app.pending_g = false;
        }
    }
}

fn layout_map_enter(app: &mut App) {
    use crate::ui::layout_map::{LayoutTarget, build_regions};
    let regions = build_regions(&app.data);
    if let Some(region) = regions.get(app.layout_map.selected_row) {
        if let Some(ref target) = region.target {
            app.prev_view = Some(DetailView::LayoutMap);
            app.prev_node = Some(TreeNodeType::LayoutMap);
            match target {
                LayoutTarget::ElfHeader => {
                    app.tree.select_node(&TreeNodeType::ElfHeader);
                    app.current_view = DetailView::StructuredInfo;
                }
                LayoutTarget::ProgramHeaders => {
                    app.tree.select_node(&TreeNodeType::ProgramHeaders);
                    app.current_view = DetailView::StructuredInfo;
                }
                LayoutTarget::SectionHeaders => {
                    app.tree.select_node(&TreeNodeType::SectionHeaders);
                    app.current_view = DetailView::StructuredInfo;
                }
                LayoutTarget::SectionBody(index) => {
                    app.tree.select_node(&TreeNodeType::SectionBody { index: *index });
                    update_view(app);
                }
            }
        }
    }
}

fn get_hex_section(app: &App) -> Option<&crate::elf::parser::SectionInfo> {
    match &app.tree.selected_node {
        Some(TreeNodeType::SectionBody { index }) => Some(&app.data.sections[*index]),
        _ => None,
    }
}

fn available_modes(section: &crate::elf::parser::SectionInfo) -> Vec<SectionViewMode> {
    let mut modes = vec![SectionViewMode::Info];
    if section.size > 0 && section.offset > 0 {
        modes.push(SectionViewMode::Hexdump);
        if section.flags.contains('X') {
            modes.push(SectionViewMode::Disassembly);
        }
        if section.name.contains("str") {
            modes.push(SectionViewMode::Strings);
        }
        if section.name == ".dynamic" {
            modes.push(SectionViewMode::Dynamic);
        }
    }
    modes
}

fn cycle_section_view(app: &mut App) {
    if let Some(TreeNodeType::SectionBody { index }) = &app.tree.selected_node {
        let section = &app.data.sections[*index];
        let modes = available_modes(section);
        if let Some(cur) = &app.section_view_mode {
            if let Some(pos) = modes.iter().position(|m| m == cur) {
                let next = (pos + 1) % modes.len();
                app.section_view_mode = Some(modes[next].clone());
                switch_section_view(app, *index, &modes[next]);
            }
        }
    }
}

fn switch_section_view(app: &mut App, index: usize, mode: &SectionViewMode) {
    match mode {
        SectionViewMode::Hexdump => app.current_view = DetailView::Hexdump,
        SectionViewMode::Disassembly => {
            let section = &app.data.sections[index];
            if app.current_disasm_section != Some(index) && section.flags.contains('X') {
                let disasm = disassemble_section(&section.data, section.addr);
                let merged = merge_symbols_with_functions(&app.data.symbols, disasm.functions);
                app.disasm_cache = Some(DisasmResult {
                    functions: merged,
                    all_instructions: disasm.all_instructions,
                    bitness: disasm.bitness,
                });
                app.current_disasm_section = Some(index);
                app.disasm.selected_function = 0;
                app.disasm.scroll = 0;
            }
            app.current_view = DetailView::Disassembly;
        }
        SectionViewMode::Strings => app.current_view = DetailView::Strings,
        SectionViewMode::Dynamic => app.current_view = DetailView::StructuredInfo,
        SectionViewMode::Info => app.current_view = DetailView::StructuredInfo,
    }
}

fn update_view(app: &mut App) {
    if let Some(ref node_type) = app.tree.selected_node {
        app.current_view = match node_type {
            TreeNodeType::Overview => DetailView::Overview,
            TreeNodeType::LayoutMap => DetailView::LayoutMap,
            TreeNodeType::ElfHeader => DetailView::StructuredInfo,
            TreeNodeType::ProgramHeaders => DetailView::StructuredInfo,
            TreeNodeType::SectionHeaders => DetailView::StructuredInfo,
            TreeNodeType::SectionsGroup => DetailView::Overview,
            TreeNodeType::SectionHeader { .. } => DetailView::StructuredInfo,
            TreeNodeType::SectionBody { index } => {
                let section = &app.data.sections[*index];
                // Default to Hexdump for all sections with data
                if section.size == 0 || section.offset == 0 {
                    app.section_view_mode = Some(SectionViewMode::Info);
                    DetailView::StructuredInfo
                } else {
                    app.section_view_mode = Some(SectionViewMode::Hexdump);
                    // Pre-load disassembly for executable sections
                    if section.flags.contains('X') {
                        if app.current_disasm_section != Some(*index) {
                            let disasm = disassemble_section(&section.data, section.addr);
                            let merged = merge_symbols_with_functions(&app.data.symbols, disasm.functions);
                            app.disasm_cache = Some(DisasmResult {
                                functions: merged,
                                all_instructions: disasm.all_instructions,
                                bitness: disasm.bitness,
                            });
                            app.current_disasm_section = Some(*index);
                            app.disasm.selected_function = 0;
                            app.disasm.scroll = 0;
                        }
                    }
                    DetailView::Hexdump
                }
            }
            TreeNodeType::SegmentsGroup => DetailView::Overview,
            TreeNodeType::Segment { .. } => DetailView::StructuredInfo,
            TreeNodeType::SymbolsGroup => DetailView::Overview,
            TreeNodeType::Symbol { .. } => DetailView::Disassembly,
        };
    }
}