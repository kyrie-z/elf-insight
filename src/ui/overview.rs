pub struct OverviewState {
    pub scroll: usize,
}

impl OverviewState {
    pub fn new() -> Self {
        OverviewState { scroll: 0 }
    }
}

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let text = "Overview - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Overview"));
    f.render_widget(p, area);
}