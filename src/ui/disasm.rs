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

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let text = "Disassembly - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Disassembly"));
    f.render_widget(p, area);
}