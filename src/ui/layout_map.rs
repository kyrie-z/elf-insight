use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct LayoutMapState {
    pub scroll: usize,
    pub selected_row: usize,
    pub region_count: usize,
}

impl LayoutMapState {
    pub fn new() -> Self {
        LayoutMapState { scroll: 0, selected_row: 0, region_count: 0 }
    }
}

#[derive(Clone, PartialEq)]
pub struct LayoutRegion {
    pub label: String,
    pub offset: u64,
    pub size: u64,
    pub color: Color,
    pub target: Option<LayoutTarget>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum LayoutTarget {
    ElfHeader,
    ProgramHeaders,
    SectionHeaders,
    SectionBody(usize),
}

pub fn build_regions(data: &crate::elf::parser::ElfData) -> Vec<LayoutRegion> {
    let mut regions: Vec<LayoutRegion> = Vec::new();

    regions.push(LayoutRegion {
        label: "ELF Header".into(),
        offset: 0,
        size: data.ehsize as u64,
        color: Color::Rgb(255, 200, 60),
        target: Some(LayoutTarget::ElfHeader),
    });

    if data.phnum > 0 && data.phoff > 0 {
        regions.push(LayoutRegion {
            label: format!("Program Headers ({} entries)", data.phnum),
            offset: data.phoff,
            size: data.phnum as u64 * data.phentsize as u64,
            color: Color::Rgb(100, 200, 255),
            target: Some(LayoutTarget::ProgramHeaders),
        });
    }

    if data.shnum > 0 && data.shoff > 0 {
        regions.push(LayoutRegion {
            label: format!("Section Headers ({} entries)", data.shnum),
            offset: data.shoff,
            size: data.shnum as u64 * data.shentsize as u64,
            color: Color::Rgb(180, 140, 255),
            target: Some(LayoutTarget::SectionHeaders),
        });
    }

    for s in &data.sections {
        if s.size == 0 || s.offset == 0 {
            continue;
        }
        let color = if s.flags.contains('X') {
            Color::Rgb(220, 80, 80)
        } else if s.flags.contains('W') {
            Color::Rgb(80, 140, 220)
        } else if s.name.contains("str") {
            Color::Rgb(100, 200, 100)
        } else {
            Color::Rgb(200, 180, 80)
        };
        regions.push(LayoutRegion {
            label: s.name.clone(),
            offset: s.offset,
            size: s.size,
            color,
            target: Some(LayoutTarget::SectionBody(s.index)),
        });
    }

    regions.sort_by_key(|r| r.offset);

    // Insert gaps for unmapped regions
    let mut with_gaps = Vec::new();
    let mut cursor = 0u64;
    let file_size = data.raw_bytes.len() as u64;
    for region in regions {
        if region.offset > cursor {
            with_gaps.push(LayoutRegion {
                label: format!("[unmapped] (0x{:x}-0x{:x})", cursor, region.offset),
                offset: cursor,
                size: region.offset - cursor,
                color: Color::DarkGray,
                target: None,
            });
        }
        cursor = region.offset + region.size;
        with_gaps.push(region);
    }
    // Trailing gap
    if cursor < file_size {
        with_gaps.push(LayoutRegion {
            label: format!("[unmapped] (0x{:x}-0x{:x})", cursor, file_size),
            offset: cursor,
            size: file_size - cursor,
            color: Color::DarkGray,
            target: None,
        });
    }
    with_gaps
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;
    let file_size = data.raw_bytes.len();
    if file_size == 0 {
        return;
    }

    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("ELF Layout Map")
        .border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let max_bar = (inner.width as usize).saturating_sub(70).max(10);

    let regions = build_regions(data);
    app.layout_map.region_count = regions.len();

    // Clamp selection
    if app.layout_map.selected_row >= regions.len() {
        app.layout_map.selected_row = regions.len().saturating_sub(1);
    }

    // Auto-scroll to keep selection visible
    let data_rows = regions.len() + 4; // header + separator + regions + blank + legend
    let visible = inner.height.saturating_sub(1) as usize;
    let max_scroll = data_rows.saturating_sub(visible);

    let sel_line = app.layout_map.selected_row + 2; // +2 for header + separator
    if sel_line < app.layout_map.scroll {
        app.layout_map.scroll = sel_line;
    }
    if sel_line >= app.layout_map.scroll + visible {
        app.layout_map.scroll = sel_line.saturating_sub(visible - 1);
    }
    if app.layout_map.scroll > max_scroll {
        app.layout_map.scroll = max_scroll;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Header line
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {:<40} {:>10}  {:>12}  {:>6}  ", "Name", "Offset", "Size", "Pct"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("Bar"),
    ]));
    lines.push(Line::from("─".repeat(inner.width as usize - 2)));

    let sel_style = Style::default().bg(Color::Rgb(60, 60, 80));

    for (i, region) in regions.iter().enumerate() {
        let pct = (region.size as f64 / file_size as f64) * 100.0;
        let bar_len = ((region.size as f64 / file_size as f64) * max_bar as f64) as usize;
        let bar_len = bar_len.max(1).min(max_bar);

        let label = if region.label.len() > 40 {
            format!("{:.37}...", region.label)
        } else {
            region.label.clone()
        };

        let line_style = if i == app.layout_map.selected_row {
            sel_style
        } else {
            Style::default()
        };

        let prefix = if i == app.layout_map.selected_row { "▶" } else { " " };

        let mut spans = vec![
            Span::styled(
                format!(
                    "{}{:<40}  0x{:06x}  0x{:08x}  {:5.1}%  ",
                    prefix, label, region.offset, region.size, pct
                ),
                line_style,
            ),
            Span::styled(
                "█".repeat(bar_len),
                Style::default().fg(region.color),
            ),
        ];
        lines.push(Line::from(spans));
    }

    // Legend
    lines.push(Line::from(""));
    let legend_items = vec![
        ("ELF Hdr", Color::Rgb(255, 200, 60)),
        ("PHDR", Color::Rgb(100, 200, 255)),
        ("SHDR", Color::Rgb(180, 140, 255)),
        ("Code", Color::Rgb(220, 80, 80)),
        ("Data", Color::Rgb(200, 180, 80)),
        ("Data+W", Color::Rgb(80, 140, 220)),
        ("Strings", Color::Rgb(100, 200, 100)),
        ("Gap", Color::DarkGray),
    ];
    let legend_spans: Vec<Span> = legend_items
        .iter()
        .flat_map(|(label, color)| {
            vec![
                Span::styled("█", Style::default().fg(*color)),
                Span::raw(format!(" {}  ", label)),
            ]
        })
        .collect();
    lines.push(Line::from(legend_spans));

    let p = Paragraph::new(lines).scroll((app.layout_map.scroll as u16, 0));

    f.render_widget(p, inner);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.layout_map.scroll);
    f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
}