use crate::elf::parser::ElfData;
use crate::elf::disasm::{DisasmResult, disassemble_section, merge_symbols_with_functions};
use crate::ui::tree::{TreeNode, TreeNodeType, TreeState};
use crate::ui::overview::OverviewState;
use crate::ui::info::InfoState;
use crate::ui::hexdump::HexdumpState;
use crate::ui::disasm::DisasmState;
use crate::ui::strings::StringsState;
use crate::ui::search::SearchState;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

pub enum DetailView {
    Overview,
    StructuredInfo,
    Hexdump,
    Disassembly,
    Strings,
}

pub struct App {
    pub data: ElfData,
    pub tree: TreeState,
    pub overview: OverviewState,
    pub info: InfoState,
    pub hexdump: HexdumpState,
    pub disasm: DisasmState,
    pub strings: StringsState,
    pub search: SearchState,
    pub current_view: DetailView,
    pub focus: Focus,
    pub should_quit: bool,
    pub show_help: bool,
    pub disasm_cache: Option<DisasmResult>,
    pub current_disasm_section: Option<usize>,
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
            info: InfoState::new(),
            hexdump: HexdumpState::new(),
            disasm: DisasmState::new(),
            strings: StringsState::new(),
            search: SearchState::new(),
            current_view: DetailView::Overview,
            focus: Focus::Tree,
            should_quit: false,
            show_help: false,
            disasm_cache: None,
            current_disasm_section: None,
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
        label: "ELF Header".into(),
        node_type: TreeNodeType::ElfHeader,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    let section_children: Vec<TreeNode> = data
        .sections
        .iter()
        .map(|s| TreeNode {
            label: format!("[{}] {}", s.index, s.name),
            node_type: TreeNodeType::SectionBody {
                index: s.index,
            },
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(data);
    let res = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
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
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key.code);
                }
            }
            Event::Mouse(mouse) => {
                handle_mouse(app, mouse);
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            scroll_detail(app, 3);
        }
        MouseEventKind::ScrollUp => {
            scroll_detail(app, -3);
        }
        MouseEventKind::Down(_) => {
            // Click on right side (column > 30%) switches focus to detail
            app.focus = Focus::Detail;
        }
        _ => {}
    }
}

fn scroll_detail(app: &mut App, delta: isize) {
    if delta > 0 {
        let d = delta as usize;
        match app.current_view {
            DetailView::Overview => app.overview.scroll += d,
            DetailView::Hexdump => app.hexdump.scroll += d,
            DetailView::Disassembly => app.disasm.scroll += d,
            DetailView::Strings => app.strings.scroll += d,
            DetailView::StructuredInfo => app.info.scroll += d,
        }
    } else {
        let d = (-delta) as usize;
        match app.current_view {
            DetailView::Overview => app.overview.scroll = app.overview.scroll.saturating_sub(d),
            DetailView::Hexdump => app.hexdump.scroll = app.hexdump.scroll.saturating_sub(d),
            DetailView::Disassembly => app.disasm.scroll = app.disasm.scroll.saturating_sub(d),
            DetailView::Strings => app.strings.scroll = app.strings.scroll.saturating_sub(d),
            DetailView::StructuredInfo => app.info.scroll = app.info.scroll.saturating_sub(d),
        }
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
        KeyCode::Char('?') | KeyCode::Char('h') => {
            app.show_help = true;
        }
        KeyCode::Char('/') => {
            app.search.active = true;
            app.search.input.clear();
            app.focus = Focus::Search;
        }
        KeyCode::Char('n') => {
            if app.focus == Focus::Detail {
                crate::ui::search::next_result(app);
            }
        }
        KeyCode::Char('N') => {
            if app.focus == Focus::Detail {
                crate::ui::search::prev_result(app);
            }
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Tree => Focus::Detail,
                Focus::Detail => Focus::Tree,
                Focus::Search => Focus::Search,
            };
        }
        KeyCode::Up => {
            if app.focus == Focus::Tree {
                app.tree.move_up();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll = app.overview.scroll.saturating_sub(1),
                    DetailView::Hexdump => app.hexdump.scroll = app.hexdump.scroll.saturating_sub(1),
                    DetailView::Disassembly => app.disasm.scroll = app.disasm.scroll.saturating_sub(1),
                    DetailView::Strings => app.strings.scroll = app.strings.scroll.saturating_sub(1),
                    DetailView::StructuredInfo => app.info.scroll = app.info.scroll.saturating_sub(1),
                }
            }
        }
        KeyCode::Down => {
            if app.focus == Focus::Tree {
                app.tree.move_down();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll += 1,
                    DetailView::Hexdump => app.hexdump.scroll += 1,
                    DetailView::Disassembly => app.disasm.scroll += 1,
                    DetailView::Strings => app.strings.scroll += 1,
                    DetailView::StructuredInfo => app.info.scroll += 1,
                }
            }
        }
        KeyCode::Right | KeyCode::Enter => {
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
                if let Some(ref disasm) = app.disasm_cache {
                    if app.disasm.selected_function + 1 < disasm.functions.len() {
                        app.disasm.selected_function += 1;
                        app.disasm.scroll = 0;
                    }
                }
            } else if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }
        KeyCode::Left => {
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
                if app.disasm.selected_function > 0 {
                    app.disasm.selected_function -= 1;
                    app.disasm.scroll = 0;
                }
            } else if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }
        KeyCode::Char('g') => {
            if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
                app.hexdump.goto_mode = true;
                app.hexdump.goto_input.clear();
                app.focus = Focus::Search;
            }
        }
        KeyCode::PageUp => {
            if app.focus == Focus::Detail {
                let visible = 10;
                match app.current_view {
                    DetailView::Overview => app.overview.scroll = app.overview.scroll.saturating_sub(visible),
                    DetailView::Hexdump => app.hexdump.scroll = app.hexdump.scroll.saturating_sub(visible),
                    DetailView::Disassembly => app.disasm.scroll = app.disasm.scroll.saturating_sub(visible),
                    DetailView::Strings => app.strings.scroll = app.strings.scroll.saturating_sub(visible),
                    DetailView::StructuredInfo => app.info.scroll = app.info.scroll.saturating_sub(visible),
                }
            }
        }
        KeyCode::PageDown => {
            if app.focus == Focus::Detail {
                let visible = 10;
                match app.current_view {
                    DetailView::Overview => app.overview.scroll += visible,
                    DetailView::Hexdump => app.hexdump.scroll += visible,
                    DetailView::Disassembly => app.disasm.scroll += visible,
                    DetailView::Strings => app.strings.scroll += visible,
                    DetailView::StructuredInfo => app.info.scroll += visible,
                }
            }
        }
        KeyCode::Home => {
            if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll = 0,
                    DetailView::Hexdump => app.hexdump.scroll = 0,
                    DetailView::Disassembly => app.disasm.scroll = 0,
                    DetailView::Strings => app.strings.scroll = 0,
                    DetailView::StructuredInfo => app.info.scroll = 0,
                }
            }
        }
        KeyCode::End => {
            if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll = usize::MAX,
                    DetailView::Hexdump => app.hexdump.scroll = usize::MAX,
                    DetailView::Disassembly => app.disasm.scroll = usize::MAX,
                    DetailView::Strings => app.strings.scroll = usize::MAX,
                    DetailView::StructuredInfo => app.info.scroll = usize::MAX,
                }
            }
        }
        _ => {}
    }
}

fn update_view(app: &mut App) {
    if let Some(ref node_type) = app.tree.selected_node {
        app.current_view = match node_type {
            TreeNodeType::Overview => DetailView::Overview,
            TreeNodeType::ElfHeader => DetailView::StructuredInfo,
            TreeNodeType::SectionsGroup => DetailView::Overview,
            TreeNodeType::SectionHeader { .. } => DetailView::StructuredInfo,
            TreeNodeType::SectionBody { index } => {
                let section = &app.data.sections[*index];
                if section.name == ".text" || section.flags.contains('X') {
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
                    DetailView::Disassembly
                } else if section.name.contains("str") {
                    DetailView::Strings
                } else {
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