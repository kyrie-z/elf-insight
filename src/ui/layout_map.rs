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

    // Build all regions in the file
    #[derive(Clone)]
    struct Region {
        start: u64,
        end: u64,
        label: String,
        color: Color,
    }

    let mut regions: Vec<Region> = Vec::new();

    // ELF Header
    let eh_size = data.ehsize as u64;
    regions.push(Region {
        start: 0,
        end: eh_size,
        label: "ELF Header".into(),
        color: Color::Rgb(255, 200, 60),
    });

    // Program Headers
    if data.phnum > 0 {
        let ph_start = data.phoff;
        let ph_end = data.phoff + data.phnum as u64 * data.phentsize as u64;
        regions.push(Region {
            start: ph_start,
            end: ph_end,
            label: "PHDR".into(),
            color: Color::Rgb(100, 200, 255),
        });
    }

    // Section Headers
    if data.shnum > 0 {
        let sh_start = data.shoff;
        let sh_end = data.shoff + data.shnum as u64 * data.shentsize as u64;
        regions.push(Region {
            start: sh_start,
            end: sh_end,
            label: "SHDR".into(),
            color: Color::Rgb(180, 140, 255),
        });
    }

    // Section data regions
    for s in &data.sections {
        if s.size == 0 || s.offset == 0 {
            continue;
        }
        let color = if s.flags.contains('X') {
            Color::Rgb(220, 80, 80)
        } else if s.name.contains("str") {
            Color::Rgb(100, 200, 100)
        } else if s.flags.contains('W') {
            Color::Rgb(80, 140, 220)
        } else {
            Color::Rgb(200, 180, 80)
        };
        regions.push(Region {
            start: s.offset,
            end: s.offset + s.size,
            label: if s.name.len() > 10 { format!("{:.10}", s.name) } else { s.name.clone() },
            color,
        });
    }

    // Sort by start position
    regions.sort_by_key(|r| r.start);

    // Merge overlapping regions (keep the last in sort order for overlaps)
    let mut merged: Vec<Region> = Vec::new();
    for r in regions {
        if let Some(last) = merged.last_mut() {
            if r.start <= last.end {
                if r.end > last.end {
                    last.end = r.end;
                }
                // Don't replace label/color for overlaps
                continue;
            }
        }
        merged.push(r);
    }

    // Draw gaps and regions
    let mut cursor = 0u64;
    for region in &merged {
        // Gap before this region
        if region.start > cursor {
            let gap_start = ((cursor as f64 / file_size as f64) * bar_width as f64) as u16;
            let gap_end = ((region.start as f64 / file_size as f64) * bar_width as f64) as u16;
            let gap_width = (gap_end.saturating_sub(gap_start)).min(area.right().saturating_sub(inner.x + gap_start));
            if gap_width > 0 {
                let gap_rect = Rect::new(inner.x + gap_start, bar_y, gap_width, 1);
                f.render_widget(Paragraph::new("").style(Style::default().bg(Color::Rgb(60, 60, 60))), gap_rect);
            }
        }

        let bar_start = ((region.start as f64 / file_size as f64) * bar_width as f64) as u16;
        let bar_end = ((region.end as f64 / file_size as f64) * bar_width as f64) as u16;
        let bar_start = bar_start.min(inner.width.saturating_sub(1));
        let bar_end = bar_end.min(inner.width);
        let bar_len = (bar_end.saturating_sub(bar_start)).max(1);
        let bar_len = bar_len.min(area.right().saturating_sub(inner.x + bar_start));

        let bar_rect = Rect::new(inner.x + bar_start, bar_y, bar_len, 1);
        f.render_widget(Paragraph::new("").style(Style::default().bg(region.color)), bar_rect);

        cursor = region.end;
    }

    // Legend
    let legend_items = vec![
        ("ELF Hdr", Color::Rgb(255, 200, 60)),
        ("PHDR", Color::Rgb(100, 200, 255)),
        ("SHDR", Color::Rgb(180, 140, 255)),
        ("Code", Color::Rgb(220, 80, 80)),
        ("Data", Color::Rgb(200, 180, 80)),
        ("Writable", Color::Rgb(80, 140, 220)),
        ("Strings", Color::Rgb(100, 200, 100)),
        ("Gap", Color::Rgb(60, 60, 60)),
    ];

    let legend_width = (legend_items.len() as u16 * 12).min(inner.width);
    let legend_x = inner.x + (inner.width.saturating_sub(legend_width) / 2).min(inner.width.saturating_sub(1));
    let legend_y = inner.y + 1;

    for (i, (label, color)) in legend_items.iter().enumerate() {
        let x = legend_x + i as u16 * 12;
        if x + 11 > area.right() {
            break;
        }
        let swatch = Rect::new(x, legend_y, 2, 1);
        f.render_widget(Paragraph::new("").style(Style::default().bg(*color)), swatch);
        let label_rect = Rect::new(x + 3, legend_y, label.len() as u16, 1);
        f.render_widget(Paragraph::new(*label).style(Style::default().fg(Color::White)), label_rect);
    }

    // Section labels below the bar
    let mut label_y = legend_y + 2;
    if label_y < inner.bottom() {
        let mut last_x = 0u16;
        for region in &merged {
            if label_y >= inner.bottom() {
                break;
            }
            let bar_start = ((region.start as f64 / file_size as f64) * bar_width as f64) as u16;
            let sec_width = ((region.end.saturating_sub(region.start) as f64 / file_size as f64) * bar_width as f64) as u16;

            let label = region.label.clone();
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