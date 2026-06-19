use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

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

    let bar_width = inner.width as usize;
    let bar_y = inner.y;
    let legend_y = inner.y + 1;

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

        // Gap
        if start > prev_end {
            let gap_start = ((prev_end as f64 / file_size as f64) * bar_width as f64) as u16;
            let gap_end = ((start as f64 / file_size as f64) * bar_width as f64) as u16;
            let gap_start = gap_start.min(inner.width.saturating_sub(1));
            let gap_end = gap_end.min(inner.width);
            if gap_end > gap_start {
                let gap_width = (gap_end - gap_start).min(area.right().saturating_sub(inner.x + gap_start));
                if gap_width > 0 {
                    let gap_rect = Rect::new(inner.x + gap_start, bar_y, gap_width, 1);
                    f.render_widget(Paragraph::new("").style(Style::default().bg(Color::DarkGray)), gap_rect);
                }
            }
        }

        let end = start + section.size as usize;
        let bar_start = ((start as f64 / file_size as f64) * bar_width as f64) as u16;
        let bar_end = ((end as f64 / file_size as f64) * bar_width as f64) as u16;
        let bar_start = bar_start.min(inner.width.saturating_sub(1));
        let bar_end = bar_end.min(inner.width);
        let bar_len = bar_end.saturating_sub(bar_start).max(1);
        // ensure bar doesn't overflow the area
        let bar_len = bar_len.min(area.right().saturating_sub(inner.x + bar_start));

        let color = if section.flags.contains('X') {
            Color::Rgb(200, 80, 80)
        } else if section.flags.contains('W') {
            Color::Rgb(80, 120, 200)
        } else if section.name.contains("str") {
            Color::Rgb(120, 180, 80)
        } else if section.name == ".bss" || section.ty == "NOBITS" {
            Color::Rgb(150, 150, 150)
        } else {
            Color::Rgb(180, 160, 60)
        };

        let bar_rect = Rect::new(inner.x + bar_start, bar_y, bar_len, 1);
        if bar_rect.right() < area.right() {
            f.render_widget(Paragraph::new("").style(Style::default().bg(color)), bar_rect);
        }

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

    let legend_width = (legend_items.len() as u16 * 10).min(inner.width);
    let legend_x = inner.x + (inner.width.saturating_sub(legend_width) / 2).min(inner.width.saturating_sub(1));

    for (i, (label, color)) in legend_items.iter().enumerate() {
        let x = legend_x + i as u16 * 10;
        if x + 10 > area.right() {
            break;
        }
        let swatch = Rect::new(x, legend_y, 2, 1);
        let label_rect = Rect::new(x + 3, legend_y, 7, 1);
        f.render_widget(Paragraph::new("").style(Style::default().bg(*color)), swatch);
        f.render_widget(Paragraph::new(*label).style(Style::default().fg(Color::White)), label_rect);
    }

    // Section labels
    let mut label_y = legend_y + 2;
    if label_y < inner.bottom() {
        let mut last_x = 0u16;
        for (offset, section) in &sections {
            if label_y >= inner.bottom() {
                break;
            }
            let start = *offset;
            let bar_start = ((start as f64 / file_size as f64) * bar_width as f64) as u16;
            let sec_width = ((section.size as f64 / file_size as f64) * bar_width as f64) as u16;

            let label = if section.name.len() > 8 {
                format!("{:.8}", section.name)
            } else {
                section.name.clone()
            };

            let label_x = inner.x + bar_start + sec_width.saturating_sub(label.len() as u16) / 2;
            let label_x = label_x.min(inner.right().saturating_sub(label.len() as u16));

            if label_x > last_x {
                let label_rect = Rect::new(label_x, label_y, label.len() as u16, 1);
                if label_rect.right() < area.right() {
                    f.render_widget(
                        Paragraph::new(label).style(Style::default().fg(Color::Gray)),
                        label_rect,
                    );
                    last_x = label_x;
                }
            }
        }
    }
}