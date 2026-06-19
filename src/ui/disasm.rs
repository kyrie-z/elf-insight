use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct DisasmState {
    pub selected_function: usize,
    pub scroll: usize,
}

impl DisasmState {
    pub fn new() -> Self {
        DisasmState {
            selected_function: 0,
            scroll: 0,
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

    let mut lines = Vec::new();

    // Function list
    let func_names: Vec<String> = disasm
        .functions
        .iter()
        .map(|f| {
            if disasm.functions.iter().position(|x| x.start_addr == f.start_addr) == Some(app.disasm.selected_function) {
                format!("[{}]", f.name)
            } else {
                f.name.clone()
            }
        })
        .collect();
    lines.push(format!("Functions: {}", func_names.join(" | ")));
    lines.push(format!("{}", "─".repeat(area.width as usize - 2)));

    // Instructions for selected function
    if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        let visible_rows = area.height.saturating_sub(4) as usize;
        let total_insns = func.instructions.len();
        let max_scroll = total_insns.saturating_sub(visible_rows);

        if app.disasm.scroll > max_scroll {
            app.disasm.scroll = max_scroll;
        }

        let start = app.disasm.scroll;
        let end = (start + visible_rows).min(total_insns);

        for insn in &func.instructions[start..end] {
            let bytes_str: String = insn
                .bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!(
                "  0x{:08x}: {:20}  {} {}",
                insn.address, bytes_str, insn.mnemonic, insn.operands
            ));
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.disasm.scroll);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    let title = if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        format!("Disassembly - {} (0x{:x}-0x{:x})", func.name, func.start_addr, func.end_addr)
    } else {
        "Disassembly".into()
    };

    let text = lines.join("\n");
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(p, area);
}