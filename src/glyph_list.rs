use crate::font_data::{FontProject, GlyphData};
use egui::{Color32, Pos2, RichText, ScrollArea, Stroke, Ui, Vec2};
use kurbo::{flatten, PathEl};
use std::collections::HashSet;

fn draw_thumbnail(
    painter: &egui::Painter,
    rect: egui::Rect,
    project: &FontProject,
    glyph: &GlyphData,
) {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include_point = |x: f64, y: f64| {
        bounds = Some(match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    };
    for contour in &glyph.contours {
        for point in &contour.points {
            include_point(point.x, point.y);
        }
    }
    let mut visited = HashSet::new();
    for component in &glyph.components {
        include_component_bounds(
            project,
            &component.base,
            [
                component.x_scale,
                component.xy_scale,
                component.yx_scale,
                component.y_scale,
                component.x_offset,
                component.y_offset,
            ],
            4,
            &mut bounds,
            &mut visited,
        );
    }
    let Some((min_x, min_y, max_x, max_y)) = bounds else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "·",
            egui::FontId::proportional(22.0),
            Color32::from_gray(100),
        );
        return;
    };
    let scale = (rect.width() as f64 / (max_x - min_x).max(1.0))
        .min(rect.height() as f64 / (max_y - min_y).max(1.0))
        * 0.82;
    let map = |x: f64, y: f64| {
        Pos2::new(
            rect.center().x + ((x - (min_x + max_x) * 0.5) * scale) as f32,
            rect.center().y - ((y - (min_y + max_y) * 0.5) * scale) as f32,
        )
    };
    for contour in &glyph.contours {
        let mut previous = None;
        let mut subpath_start = None;
        flatten(contour.to_bezpath(), 0.5, |element| match element {
            PathEl::MoveTo(point) => {
                let mapped = map(point.x, point.y);
                previous = Some(mapped);
                subpath_start = Some(mapped);
            }
            PathEl::LineTo(point) => {
                let current = map(point.x, point.y);
                if let Some(start) = previous {
                    painter.line_segment(
                        [start, current],
                        Stroke::new(1.2_f32, Color32::from_rgb(210, 215, 225)),
                    );
                }
                previous = Some(current);
            }
            PathEl::ClosePath => {
                if let (Some(start), Some(end)) = (subpath_start, previous) {
                    painter.line_segment(
                        [end, start],
                        Stroke::new(1.2_f32, Color32::from_rgb(210, 215, 225)),
                    );
                }
                previous = subpath_start;
            }
            PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
        });
    }
    for component in &glyph.components {
        draw_component_thumbnail(
            painter,
            project,
            &component.base,
            [
                component.x_scale,
                component.xy_scale,
                component.yx_scale,
                component.y_scale,
                component.x_offset,
                component.y_offset,
            ],
            &map,
            4,
            &mut HashSet::new(),
        );
    }
}

fn include_component_bounds(
    project: &FontProject,
    glyph_name: &str,
    transform: [f64; 6],
    depth: usize,
    bounds: &mut Option<(f64, f64, f64, f64)>,
    visited: &mut HashSet<String>,
) {
    if !visited.insert(glyph_name.to_string()) {
        return;
    }
    let Some(glyph) = project.glyphs.get(glyph_name) else {
        visited.remove(glyph_name);
        return;
    };
    let mut include = |x: f64, y: f64| {
        *bounds = Some(match *bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    };
    for contour in &glyph.contours {
        for point in &contour.points {
            include(
                transform[0] * point.x + transform[2] * point.y + transform[4],
                transform[1] * point.x + transform[3] * point.y + transform[5],
            );
        }
    }
    if depth == 0 {
        visited.remove(glyph_name);
        return;
    }
    for component in &glyph.components {
        let child = [
            component.x_scale,
            component.xy_scale,
            component.yx_scale,
            component.y_scale,
            component.x_offset,
            component.y_offset,
        ];
        let composed = [
            transform[0] * child[0] + transform[2] * child[1],
            transform[1] * child[0] + transform[3] * child[1],
            transform[0] * child[2] + transform[2] * child[3],
            transform[1] * child[2] + transform[3] * child[3],
            transform[0] * child[4] + transform[2] * child[5] + transform[4],
            transform[1] * child[4] + transform[3] * child[5] + transform[5],
        ];
        include_component_bounds(
            project,
            &component.base,
            composed,
            depth - 1,
            bounds,
            visited,
        );
    }
    visited.remove(glyph_name);
}

fn draw_component_thumbnail(
    painter: &egui::Painter,
    project: &FontProject,
    glyph_name: &str,
    transform: [f64; 6],
    map: &dyn Fn(f64, f64) -> Pos2,
    depth: usize,
    visited: &mut HashSet<String>,
) {
    if !visited.insert(glyph_name.to_string()) {
        return;
    }
    let Some(glyph) = project.glyphs.get(glyph_name) else {
        visited.remove(glyph_name);
        return;
    };
    let apply = |x: f64, y: f64| {
        (
            transform[0] * x + transform[2] * y + transform[4],
            transform[1] * x + transform[3] * y + transform[5],
        )
    };
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(170, 200, 225));
    for contour in &glyph.contours {
        let mut previous = None;
        let mut subpath_start = None;
        flatten(contour.to_bezpath(), 0.5, |element| match element {
            PathEl::MoveTo(point) => {
                let mapped = apply(point.x, point.y);
                let mapped = map(mapped.0, mapped.1);
                previous = Some(mapped);
                subpath_start = Some(mapped);
            }
            PathEl::LineTo(point) => {
                let mapped = apply(point.x, point.y);
                let mapped = map(mapped.0, mapped.1);
                if let Some(start) = previous {
                    painter.line_segment([start, mapped], stroke);
                }
                previous = Some(mapped);
            }
            PathEl::ClosePath => {
                if let (Some(start), Some(end)) = (subpath_start, previous) {
                    painter.line_segment([end, start], stroke);
                }
                previous = subpath_start;
            }
            PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
        });
    }
    if depth == 0 {
        visited.remove(glyph_name);
        return;
    }
    for component in &glyph.components {
        let child = [
            component.x_scale,
            component.xy_scale,
            component.yx_scale,
            component.y_scale,
            component.x_offset,
            component.y_offset,
        ];
        let composed = [
            transform[0] * child[0] + transform[2] * child[1],
            transform[1] * child[0] + transform[3] * child[1],
            transform[0] * child[2] + transform[2] * child[3],
            transform[1] * child[2] + transform[3] * child[3],
            transform[0] * child[4] + transform[2] * child[5] + transform[4],
            transform[1] * child[4] + transform[3] * child[5] + transform[5],
        ];
        draw_component_thumbnail(
            painter,
            project,
            &component.base,
            composed,
            map,
            depth - 1,
            visited,
        );
    }
    visited.remove(glyph_name);
}

fn select_glyph(
    selected: &mut Option<String>,
    selected_glyphs: &mut HashSet<String>,
    current_glyph: &Option<String>,
    visible_names: &[&str],
    name: &str,
    shift: bool,
    command: bool,
) {
    *selected = Some(name.to_string());
    if shift {
        if let (Some(anchor), Some(clicked)) = (
            current_glyph.as_deref(),
            visible_names.iter().position(|item| *item == name),
        ) {
            if let Some(start) = visible_names.iter().position(|item| *item == anchor) {
                let (low, high) = if start <= clicked {
                    (start, clicked)
                } else {
                    (clicked, start)
                };
                selected_glyphs.clear();
                selected_glyphs.extend(
                    visible_names[low..=high]
                        .iter()
                        .map(|item| (*item).to_string()),
                );
            }
        } else {
            selected_glyphs.insert(name.to_string());
        }
    } else if command {
        if !selected_glyphs.remove(name) {
            selected_glyphs.insert(name.to_string());
        }
    } else {
        selected_glyphs.clear();
        selected_glyphs.insert(name.to_string());
    }
}

mod actions;
#[allow(clippy::too_many_arguments)]
mod list;

pub use actions::*;
pub use list::show_glyph_list;
