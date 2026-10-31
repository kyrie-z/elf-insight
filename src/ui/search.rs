use crate::app::{App, DetailView};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct SearchState {
    pub active: bool,
    pub input: String,
    pub query: String,
    pub results: Vec<usize>,
    pub current_result: usize,
    pub no_matches_timer: u8,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            active: false,
            input: String::new(),
            query: String::new(),
            results: Vec::new(),
            current_result: 0,
            no_matches_timer: 0,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.search.active {
        return;
    }
    let search_area = Rect {
        y: area.height.saturating_sub(3),
        height: 3,
        width: area.width.min(40),
        x: area.width.saturating_sub(40),
    };
    let display = if app.search.no_matches_timer > 0 {
        format!("/{}  [No matches]", app.search.input)
    } else if !app.search.results.is_empty() {
        format!(
            "/{}  [{}/{}]",
            app.search.input,
            app.search.current_result + 1,
            app.search.results.len()
        )
    } else {
        format!("/{}_", app.search.input)
    };
    let p = Paragraph::new(display).block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(p, search_area);
}

pub fn do_search(app: &mut App) {
    let query = app.search.input.clone();
    app.search.query = query.clone();
    if query.is_empty() {
        return;
    }

    app.search.results.clear();
    app.search.current_result = 0;

    if query.starts_with("0x") || query.starts_with("0X") {
        if let Ok(addr) = u64::from_str_radix(&query[2..], 16) {
            match app.current_view {
                DetailView::Hexdump => {
                    if let Some(section) = get_current_section(app) {
                        let offset = addr.saturating_sub(section.addr);
                        if offset < section.size {
                            app.search.results = vec![offset as usize];
                        }
                    }
                }
                DetailView::Disassembly => {
                    if let Some(disasm) = &app.disasm_cache {
                        for (i, insn) in disasm.all_instructions.iter().enumerate() {
                            if insn.address == addr {
                                app.search.results = vec![i];
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    } else {
        if matches!(app.current_view, DetailView::LayoutMap) {
            let regions = crate::ui::layout_map::build_regions(&app.data);
            for (i, region) in regions.iter().enumerate() {
                if region.label.to_lowercase().contains(&query.to_lowercase()) {
                    app.search.results.push(i);
                }
            }
            if !app.search.results.is_empty() {
                app.layout_map.selected_row = app.search.results[0];
            } else {
                app.search.no_matches_timer = 120;
            }
            return;
        }

        match app.current_view {
            DetailView::Overview => {
                let lines = build_overview_lines(app);
                let mut results = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        results.push(i);
                    }
                }
                app.search.results = results;
            }
            DetailView::Disassembly => {
                if let Some(disasm) = &app.disasm_cache {
                    let mut results = Vec::new();
                    for (i, insn) in disasm.all_instructions.iter().enumerate() {
                        if insn.mnemonic.to_lowercase().contains(&query.to_lowercase())
                            || insn.operands.to_lowercase().contains(&query.to_lowercase())
                        {
                            results.push(i);
                        }
                    }
                    app.search.results = results;
                }
            }
            DetailView::Hexdump => {
                if let Some(section_index) = get_current_section_index(app) {
                    let data = &app.data.sections[section_index].data;
                    let query_bytes = query.as_bytes();
                    let mut results = Vec::new();
                    // Search raw bytes for exact match
                    for start in 0..data
                        .len()
                        .saturating_sub(query_bytes.len().saturating_sub(1))
                    {
                        if data[start..].starts_with(query_bytes) && !results.contains(&start) {
                            results.push(start);
                        }
                    }
                    // Also search case-insensitive ASCII
                    let query_lower = query.to_lowercase();
                    for (row, chunk) in data.chunks(16).enumerate() {
                        let ascii: String = chunk
                            .iter()
                            .map(|&b| {
                                if b.is_ascii_graphic() || b == b' ' {
                                    b as char
                                } else {
                                    '.'
                                }
                            })
                            .collect();
                        let ascii_lower = ascii.to_lowercase();
                        if let Some(col) = ascii_lower.find(&query_lower) {
                            let offset = row * 16 + col;
                            if !results.contains(&offset) {
                                results.push(offset);
                            }
                        }
                    }
                    results.sort();
                    results.dedup();
                    app.search.results = results;
                }
            }
            DetailView::Strings => {
                if let Some(section_index) = get_current_section_index(app) {
                    let data = &app.data.sections[section_index].data;
                    let strings = crate::ui::strings::extract_strings(data);
                    let mut results = Vec::new();
                    for (i, (_offset, s)) in strings.iter().enumerate() {
                        if s.to_lowercase().contains(&query.to_lowercase()) {
                            results.push(i);
                        }
                    }
                    app.search.results = results;
                }
            }
            _ => {}
        }
    }

    if app.search.results.is_empty() {
        app.search.no_matches_timer = 120;
    } else {
        apply_search_result(app);
    }
}

pub fn next_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = (app.search.current_result + 1) % app.search.results.len();
    apply_search_result(app);
}

pub fn prev_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = if app.search.current_result == 0 {
        app.search.results.len() - 1
    } else {
        app.search.current_result - 1
    };
    apply_search_result(app);
}

fn apply_search_result(app: &mut App) {
    if let Some(&pos) = app.search.results.get(app.search.current_result) {
        match app.current_view {
            DetailView::Overview => {
                app.overview.selected_line = pos;
            }
            DetailView::Hexdump => {
                app.hexdump.cursor_offset = pos;
                app.hexdump.scroll = pos / 16;
            }
            DetailView::Disassembly => {
                if let Some(disasm) = &app.disasm_cache {
                    if pos < disasm.all_instructions.len() {
                        for (fi, func) in disasm.functions.iter().enumerate() {
                            if pos >= func.start_idx && pos < func.end_idx {
                                app.disasm.selected_function = fi;
                                app.disasm.scroll = pos - func.start_idx;
                                return;
                            }
                        }
                    }
                }
            }
            DetailView::LayoutMap => {
                app.layout_map.selected_row = pos;
            }
            DetailView::Strings => {
                app.strings.selected_line = pos;
            }
            _ => {}
        }
    }
}

fn get_current_section_index(app: &App) -> Option<usize> {
    match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => Some(*index),
        _ => None,
    }
}

fn get_current_section(app: &App) -> Option<&crate::elf::parser::SectionInfo> {
    match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => {
            Some(&app.data.sections[*index])
        }
        _ => None,
    }
}

pub fn build_overview_lines(app: &App) -> Vec<String> {
    let data = &app.data;
    let mut lines = Vec::new();

    lines.push("ELF Header".to_string());
    lines.push(format!("  Magic:   {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!(
        "  Class:   {}",
        if data.class == 2 { "ELF64" } else { "ELF32" }
    ));
    lines.push(format!(
        "  Data:    {}",
        if data.data == 1 {
            "2's complement, little endian"
        } else {
            "2's complement, big endian"
        }
    ));
    lines.push(format!("  Version: {} (current)", data.version));
    lines.push(format!("  OS/ABI:  {}", data.os_abi));
    let type_display = if data.is_pie {
        format!("{} (PIE executable)", data.elf_type)
    } else {
        data.elf_type.clone()
    };
    lines.push(format!("  Type:    {}", type_display));
    lines.push(format!("  Machine: {}", data.machine));
    lines.push(format!("  Entry:   0x{:x}", data.entry));
    lines.push(format!(
        "  PH off:  0x{:x} ({} entries, {} bytes each)",
        data.phoff, data.phnum, data.phentsize
    ));
    lines.push(format!(
        "  SH off:  0x{:x} ({} entries, {} bytes each)",
        data.shoff, data.shnum, data.shentsize
    ));
    lines.push(format!("  Flags:   0x{:x}", data.flags));
    lines.push(String::new());

    lines.push(
        "[Nr] Name                  Type       Address    Offset    Size      Flags".to_string(),
    );
    for s in &data.sections {
        let name = if s.name.len() > 20 {
            format!("{:.20}", s.name)
        } else {
            format!("{:20}", s.name)
        };
        lines.push(format!(
            "[{:2}] {} {:10} 0x{:08x} 0x{:06x} 0x{:06x} {:3}",
            s.index, name, s.ty, s.addr, s.offset, s.size, s.flags
        ));
    }
    lines.push(String::new());

    lines.push(
        "Program Headers:  Type       Offset   VirtAddr  PhysAddr  FileSiz  MemSiz   Flg Align"
            .to_string(),
    );
    for s in &data.segments {
        lines.push(format!(
            "  {:14} 0x{:06x} 0x{:08x} 0x{:08x} 0x{:06x} 0x{:06x} {:3} 0x{:x}",
            s.ty, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align
        ));
    }
    lines
}

pub fn highlight_line(line: &str, query: &str, highlight_style: Style) -> Line<'static> {
    let lower_line = line.to_lowercase();
    let lower_query = query.to_lowercase();
    if lower_query.is_empty() {
        return Line::from(line.to_string());
    }

    let mut map = LowerMap::new(line);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut last = 0;
    let mut pos = 0;
    while let Some(found) = lower_line[pos..].find(&lower_query) {
        let start_lower = pos + found;
        let end_lower = start_lower + lower_query.len();
        pos = end_lower;
        // Align the lowercased match back onto the original string. A match
        // endpoint inside the lowercase expansion of a char widens to that
        // whole char, so every span stays on a char boundary.
        let (Some((start, _)), Some((_, end))) =
            (map.char_at(start_lower), map.char_at(end_lower - 1))
        else {
            break;
        };
        let Some(text) = line.get(start..end) else {
            continue;
        };
        if start > last {
            if let Some(gap) = line.get(last..start) {
                spans.push(Span::raw(gap.to_string()));
            }
        }
        spans.push(Span::styled(text.to_string(), highlight_style));
        last = end;
    }
    if let Some(rest) = line.get(last..) {
        if !rest.is_empty() {
            spans.push(Span::raw(rest.to_string()));
        }
    }
    Line::from(spans)
}

/// Walks `line` in step with `line.to_lowercase()` and maps byte offsets of the
/// lowercased string back to the original char that produced them. Offsets must
/// be requested in non-decreasing order.
struct LowerMap<'a> {
    chars: std::str::CharIndices<'a>,
    orig_start: usize,
    orig_end: usize,
    lower_start: usize,
    lower_end: usize,
    exhausted: bool,
}

impl<'a> LowerMap<'a> {
    fn new(line: &'a str) -> Self {
        let mut map = LowerMap {
            chars: line.char_indices(),
            orig_start: 0,
            orig_end: 0,
            lower_start: 0,
            lower_end: 0,
            exhausted: false,
        };
        map.advance();
        map
    }

    fn advance(&mut self) {
        match self.chars.next() {
            Some((start, c)) => {
                self.orig_start = start;
                self.orig_end = start + c.len_utf8();
                self.lower_start = self.lower_end;
                // `str::to_lowercase` differs from per-char folding only in the
                // word-final sigma rule, which preserves the byte length.
                self.lower_end += c.to_lowercase().map(char::len_utf8).sum::<usize>();
            }
            None => self.exhausted = true,
        }
    }

    /// Original byte range of the char whose lowercase form contains
    /// `lower_offset`.
    fn char_at(&mut self, lower_offset: usize) -> Option<(usize, usize)> {
        while !self.exhausted && self.lower_end <= lower_offset {
            self.advance();
        }
        if self.exhausted || lower_offset < self.lower_start {
            return None;
        }
        Some((self.orig_start, self.orig_end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_texts(line: &Line<'_>) -> Vec<String> {
        line.spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn test_highlight_line_case_insensitive() {
        let line = highlight_line("Hello World", "world", Style::default());
        assert_eq!(span_texts(&line), vec!["Hello ", "World"]);
    }

    #[test]
    fn test_highlight_line_no_match_returns_original() {
        let line = highlight_line("Hello World", "xyz", Style::default());
        assert_eq!(span_texts(&line), vec!["Hello World"]);
    }

    #[test]
    fn test_highlight_line_expanding_lowercase_before_match_does_not_panic() {
        let line = highlight_line("ẞabc", "abc", Style::default());
        assert_eq!(span_texts(&line), vec!["ẞ", "abc"]);
    }

    #[test]
    fn test_highlight_line_expanding_lowercase_after_match_does_not_panic() {
        let line = highlight_line("xİy", "y", Style::default());
        assert_eq!(span_texts(&line), vec!["xİ", "y"]);
    }

    #[test]
    fn test_highlight_line_word_final_sigma_matches_search_semantics() {
        let style = Style::default().fg(Color::Yellow);
        for query in ["ΟΔΟΣ", "οδος"] {
            let line = highlight_line("ΟΔΟΣ", query, style);
            assert_eq!(span_texts(&line), vec!["ΟΔΟΣ"]);
            assert_eq!(line.spans[0].style, style);
        }
    }

    #[test]
    fn test_highlight_line_sigma_requires_the_final_form() {
        // `str::to_lowercase` folds the final Σ to ς, so a plain σ query does
        // not match - the same verdict the search list (`contains`) reaches.
        let line = highlight_line("ΟΣ", "σ", Style::default());
        assert_eq!(span_texts(&line), vec!["ΟΣ"]);
    }
}
