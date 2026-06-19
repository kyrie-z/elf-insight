use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;
    let file_size = data.raw_bytes.len();

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

    let bar_width = inner.width as usize;
    let bar_y = inner.y + 1;
    let label_y = inner.y + 2;

    // Collect sections with data, sorted by file offset
    let mut sections: Vec<(usize, &crate::elf::parser::SectionInfo)> = data
        .sections
        .iter()
        .filter(|s| s.size > 0 && s.offset > 0)
        .map(|s| (s.offset as usize, s))
        .collect();
    sections.sort_by_key(|(off, _)| *off);

    let mut prev_end = 0usize;
    for (offset, section) in &sections {
        let start = *offset;

        // Gap before this section
        if start > prev_end {
            let gap_start = (prev_end as f64 / file_size as f64 * bar_width as f64) as u16;
            let gap_end = (start as f64 / file_size as f64 * bar_width as f64) as u16;
            if gap_end > gap_start {
                let gap_rect = Rect::new(inner.x + gap_start, bar_y, gap_end - gap_start, 1);
                f.render_widget(Paragraph::new("").style(Style::default().bg(Color::DarkGray)), gap_rect);
            }
        }

        let end = start + section.size as usize;
        let bar_start = (start as f64 / file_size as f64 * bar_width as f64) as u16;
        let bar_end = (end as f64 / file_size as f64 * bar_width as f64) as u16;
        let bar_len = bar_end.saturating_sub(bar_start).max(1);

        let color = if section.flags.contains('X') {
            Color::Rgb(200, 80, 80)     // executable: red
        } else if section.flags.contains('W') {
            Color::Rgb(80, 120, 200)    // writable: blue
        } else if section.name.contains("str") {
            Color::Rgb(120, 180, 80)    // string tables: green
        } else if section.name == ".bss" || section.ty == "NOBITS" {
            Color::Rgb(150, 150, 150)   // NOBITS: gray
        } else {
            Color::Rgb(180, 160, 60)    // read-only data: gold
        };

        let bar_rect = Rect::new(inner.x + bar_start, bar_y, bar_len, 1);
        f.render_widget(Paragraph::new("").style(Style::default().bg(color)), bar_rect);

        prev_end = end;
    }

    // Legend
    let legend_items = vec![
        ("Exec", Color::Rgb(200, 80, 80)),
        ("Data", Color::Rgb(180, 160, 60)),
        ("Writ", Color::Rgb(80, 120, 200)),
        ("Str", Color::Rgb(120, 180, 80)),
        ("Gap", Color::DarkGray),
    ];

    let legend_width = legend_items.len() as u16 * 10;
    let legend_x = inner.x + inner.width.saturating_sub(legend_width) / 2;
    let legend_y = label_y;

    for (i, (label, color)) in legend_items.iter().enumerate() {
        let x = legend_x + i as u16 * 10;
        let swatch = Rect::new(x, legend_y, 2, 1);
        f.render_widget(Paragraph::new("").style(Style::default().bg(*color)), swatch);
        let label_rect = Rect::new(x + 3, legend_y, 7, 1);
        f.render_widget(Paragraph::new(*label).style(Style::default().fg(Color::White)), label_rect);
    }

    // Section labels below the bar
    let mut label_y_offset = legend_y + 2;
    let mut label_x_last = 0u16;

    for (offset, section) in &sections {
        if section.size == 0 {
            continue;
        }
        let start = *offset;
        let bar_start = (start as f64 / file_size as f64 * bar_width as f64) as u16;
        let sec_width = (section.size as f64 / file_size as f64 * bar_width as f64) as u16;

        let label = if section.name.len() > 8 {
            format!("{:.8}", section.name)
        } else {
            section.name.clone()
        };

        let label_x = inner.x + bar_start + sec_width.saturating_sub(label.len() as u16) / 2;

        if label_x > label_x_last && label_y_offset < inner.y + inner.height - 1 {
            let label_rect = Rect::new(label_x, label_y_offset, label.len() as u16, 1);
            f.render_widget(
                Paragraph::new(label).style(Style::default().fg(Color::Gray)),
                label_rect,
            );
            label_x_last = label_x;
        }
    }
}