pub enum HexCursor {
    Hex,
    Ascii,
}

pub struct HexdumpState {
    pub scroll: usize,
    pub cursor_offset: usize,
    pub cursor_mode: HexCursor,
    pub goto_input: String,
    pub goto_mode: bool,
}

impl HexdumpState {
    pub fn new() -> Self {
        HexdumpState {
            scroll: 0,
            cursor_offset: 0,
            cursor_mode: HexCursor::Hex,
            goto_input: String::new(),
            goto_mode: false,
        }
    }
}

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let text = "Hexdump - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Hexdump"));
    f.render_widget(p, area);
}