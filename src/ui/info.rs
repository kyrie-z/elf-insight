pub struct InfoState {
    pub scroll: usize,
}

impl InfoState {
    pub fn new() -> Self {
        InfoState { scroll: 0 }
    }
}

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let text = "Structured Info - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(p, area);
}