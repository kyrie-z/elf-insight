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

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.search.active {
        return;
    }
    let search_area = Rect {
        y: area.height.saturating_sub(3),
        height: 3,
        ..area
    };
    let p = Paragraph::new(format!("/{}", app.search.input))
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(p, search_area);
}

pub fn do_search(_app: &mut App) {}

pub fn next_result(_app: &mut App) {}

pub fn prev_result(_app: &mut App) {}