use crate::app::{App, DisasmSubFocus, Focus};
use crate::ui::search;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct DisasmState {
    pub selected_function: usize,
    pub scroll: usize,
    pub func_list_state: ListState,
}

impl DisasmState {
    pub fn new() -> Self {
        DisasmState {
            selected_function: 0,
            scroll: 0,
            func_list_state: ListState::default(),
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let disasm = match &app.disasm_cache {
        Some(d) => d,
        None => {
            let p = Paragraph::new("No disassembly available")
                .block(Block::default().borders(Borders::ALL).title("Disassembly"));
            f.render_widget(p, area);
            return;
        }
    };

    // Split into function list (left 25%) and instructions (right 75%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    // Function list
    let func_items: Vec<ListItem> = disasm
        .functions
        .iter()
        .enumerate()
        .map(|(i, func)| {
            let label = format!(" {} (0x{:x})", func.name, func.start_addr);
            if i == app.disasm.selected_function {
                ListItem::new(label).style(Style::default().bg(Color::DarkGray).fg(Color::White))
            } else {
                ListItem::new(label)
            }
        })
        .collect();

    app.disasm.func_list_state.select(Some(app.disasm.selected_function));

    let func_border_style = if app.focus == Focus::Detail && app.disasm_subfocus == DisasmSubFocus::FuncList {
        border_style
    } else {
        border_style.fg(Color::Gray)
    };

    let func_list = List::new(func_items)
        .block(Block::default().borders(Borders::ALL).title("Functions").border_style(func_border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_stateful_widget(func_list, chunks[0], &mut app.disasm.func_list_state);

    // Instructions
    let query = &app.search.query;
    let hl = Style::default().fg(Color::Yellow).bg(Color::Rgb(80, 80, 0));
    let cursor_style = Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 100));

    let mut lines: Vec<Line> = Vec::new();
    if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        let visible_rows = chunks[1].height.saturating_sub(2) as usize;
        let total_insns = func.end_idx - func.start_idx;
        let max_scroll = total_insns.saturating_sub(visible_rows);

        if app.disasm.scroll > max_scroll {
            app.disasm.scroll = max_scroll;
        }

        let start = func.start_idx + app.disasm.scroll;
        let end = (start + visible_rows).min(func.end_idx);
        let cursor_idx = func.start_idx + app.disasm.scroll;

        for (i, insn) in disasm.all_instructions[start..end].iter().enumerate() {
            let idx = start + i;
            let bytes_str: String = insn
                .bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let line_str = format!(
                "0x{:08x}: {:20}  {} {}",
                insn.address, bytes_str, insn.mnemonic, insn.operands
            );
            let line = search::highlight_line(&line_str, query, hl);
            if idx == cursor_idx {
                lines.push(Line::from(vec![Span::styled(line.to_string(), cursor_style)]));
            } else {
                lines.push(line);
            }
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.disasm.scroll);
        f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }

    let title = if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        let cursor_idx = func.start_idx + app.disasm.scroll;
        let cursor_addr = if cursor_idx < disasm.all_instructions.len() {
            disasm.all_instructions[cursor_idx].address
        } else {
            func.start_addr
        };
        let pct = if func.end_idx > func.start_idx {
            (app.disasm.scroll as f64 / (func.end_idx - func.start_idx) as f64 * 100.0) as u32
        } else {
            0
        };
        format!("{} (0x{:x}-0x{:x}) [{:>3}%] 0x{:x}", func.name, func.start_addr, func.end_addr, pct, cursor_addr)
    } else {
        "Disassembly".into()
    };

    let insn_border_style = if app.focus == Focus::Detail && app.disasm_subfocus == DisasmSubFocus::Instructions {
        border_style
    } else {
        border_style.fg(Color::Gray)
    };

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title).border_style(insn_border_style));
    f.render_widget(p, chunks[1]);
}