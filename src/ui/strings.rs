pub struct StringsState {
    pub scroll: usize,
}

impl StringsState {
    pub fn new() -> Self {
        StringsState { scroll: 0 }
    }
}

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let text = "Strings - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Strings"));
    f.render_widget(p, area);
}