pub mod tree;
pub mod overview;
pub mod info;
pub mod hexdump;
pub mod disasm;
pub mod strings;
pub mod search;
pub mod help;
pub mod layout_map;

use crate::app::{App, DetailView};
use ratatui::prelude::*;

pub fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    match app.current_view {
        DetailView::Overview => overview::render(f, app, area),
        DetailView::LayoutMap => layout_map::render(f, app, area),
        DetailView::StructuredInfo => info::render(f, app, area),
        DetailView::Hexdump => hexdump::render(f, app, area),
        DetailView::Disassembly => disasm::render(f, app, area),
        DetailView::Strings => strings::render(f, app, area),
    }
}