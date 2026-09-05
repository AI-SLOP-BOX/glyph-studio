use crate::font_data::{Contour, ContourPoint, FontProject, GlyphData};

pub fn generate_all_japanese(project: &mut FontProject) {
    let chars = generate_japanese_chars();
    for ch in chars {
        let _ = generate_glyph_from_strokes(project, ch);
    }
}

fn generate_japanese_chars() -> Vec<u32> {
    let mut chars = Vec::new();

    for ch in 0x3041u32..=0x309F {
        chars.push(ch);
    }
    for ch in 0x30A0u32..=0x30FF {
        chars.push(ch);
    }
    for ch in 0x4E00u32..=0x9FA0 {
        chars.push(ch);
    }
    for ch in 0x3400u32..=0x4DBF {
        chars.push(ch);
    }
    for ch in 0x3000u32..=0x303F {
        chars.push(ch);
    }

    chars
}

fn glyph_name_for_char(ch: u32) -> String {
    format!("uni{:04X}", ch)
}

pub fn generate_glyph_from_strokes(project: &mut FontProject, unicode: u32) -> Option<GlyphData> {
    let ch = char::from_u32(unicode)?;
    let name = glyph_name_for_char(unicode);

    // Never destroy work that already has an outline. Generation fills
    // scaffolds; it does not overwrite authored glyphs.
    if let Some(existing) = project.get_glyph(&name) {
        if !existing.contours.is_empty() {
            return Some(existing.clone());
        }
    }

    let strokes = stroke_data(ch);
    let contours = build_glyph_from_strokes(&strokes);

    let mut glyph = GlyphData::new(name.clone(), Some(unicode));
    glyph.width = 600.0;
    glyph.contours = contours;

    project.glyphs.insert(name.clone(), glyph);
    project.get_glyph(&name).cloned()
}

#[derive(Debug, Clone, Copy)]
struct Stroke {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    width: f64,
}

fn stroke_data(ch: char) -> Vec<Stroke> {
    match ch {
        '一' => vec![Stroke {
            x1: 60.0,
            y1: 400.0,
            x2: 940.0,
            y2: 400.0,
            width: 60.0,
        }],
        '二' => vec![
            Stroke {
                x1: 60.0,
                y1: 400.0,
                x2: 940.0,
                y2: 400.0,
                width: 60.0,
            },
            Stroke {
                x1: 60.0,
                y1: 100.0,
                x2: 500.0,
                y2: 100.0,
                width: 50.0,
            },
        ],
        '三' => vec![
            Stroke {
                x1: 60.0,
                y1: 400.0,
                x2: 940.0,
                y2: 400.0,
                width: 60.0,
            },
            Stroke {
                x1: 60.0,
                y1: 100.0,
                x2: 500.0,
                y2: 100.0,
                width: 50.0,
            },
            Stroke {
                x1: 60.0,
                y1: 600.0,
                x2: 500.0,
                y2: 600.0,
                width: 50.0,
            },
        ],
        '口' => vec![
            Stroke {
                x1: 60.0,
                y1: 100.0,
                x2: 940.0,
                y2: 100.0,
                width: 60.0,
            },
            Stroke {
                x1: 940.0,
                y1: 100.0,
                x2: 940.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 940.0,
                y1: 900.0,
                x2: 60.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 60.0,
                y1: 900.0,
                x2: 60.0,
                y2: 100.0,
                width: 60.0,
            },
        ],
        '日' => vec![
            Stroke {
                x1: 60.0,
                y1: 100.0,
                x2: 940.0,
                y2: 100.0,
                width: 60.0,
            },
            Stroke {
                x1: 940.0,
                y1: 100.0,
                x2: 940.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 940.0,
                y1: 900.0,
                x2: 60.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 60.0,
                y1: 900.0,
                x2: 60.0,
                y2: 100.0,
                width: 60.0,
            },
            Stroke {
                x1: 60.0,
                y1: 500.0,
                x2: 900.0,
                y2: 500.0,
                width: 50.0,
            },
        ],
        _ => vec![
            Stroke {
                x1: 100.0,
                y1: 100.0,
                x2: 900.0,
                y2: 100.0,
                width: 60.0,
            },
            Stroke {
                x1: 900.0,
                y1: 100.0,
                x2: 900.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 900.0,
                y1: 900.0,
                x2: 100.0,
                y2: 900.0,
                width: 60.0,
            },
            Stroke {
                x1: 100.0,
                y1: 900.0,
                x2: 100.0,
                y2: 100.0,
                width: 60.0,
            },
        ],
    }
}

fn build_glyph_from_strokes(strokes: &[Stroke]) -> Vec<Contour> {
    let mut contours = Vec::new();

    for stroke in strokes {
        let contour = stroke_to_contour(*stroke);
        contours.push(contour);
    }

    contours
}

fn stroke_to_contour(stroke: Stroke) -> Contour {
    let hw = stroke.width / 2.0;
    let dx = stroke.x2 - stroke.x1;
    let dy = stroke.y2 - stroke.y1;
    let len = (dx * dx + dy * dy).sqrt();
    let nx = -dy / len * hw;
    let ny = dx / len * hw;

    let points = vec![
        ContourPoint::on_curve(stroke.x1 + nx, stroke.y1 + ny),
        ContourPoint::off_curve(stroke.x1 + nx * 2.0, stroke.y1 + ny * 2.0),
        ContourPoint::on_curve(stroke.x2 + nx, stroke.y2 + ny),
        ContourPoint::off_curve(stroke.x2 + nx * 2.0, stroke.y2 + ny * 2.0),
        ContourPoint::on_curve(stroke.x2 - nx, stroke.y2 - ny),
        ContourPoint::off_curve(stroke.x2 - nx * 2.0, stroke.y2 - ny * 2.0),
        ContourPoint::on_curve(stroke.x1 - nx, stroke.y1 - ny),
        ContourPoint::off_curve(stroke.x1 - nx * 2.0, stroke.y1 - ny * 2.0),
    ];

    Contour { points }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_preserves_authored_outline() {
        let mut project = FontProject::new();
        let name = glyph_name_for_char('一' as u32);
        let mut glyph = GlyphData::new(name.clone(), Some('一' as u32));
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(1.0, 2.0),
                ContourPoint::on_curve(3.0, 4.0),
                ContourPoint::on_curve(5.0, 6.0),
            ],
        });
        project.glyphs.insert(name.clone(), glyph);
        let generated = generate_glyph_from_strokes(&mut project, '一' as u32).unwrap();
        assert_eq!(generated.contours[0].points[0].x, 1.0);
    }
}
