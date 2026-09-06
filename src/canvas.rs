use crate::font_data::{
    ColorGradient, ColorGradientExtend, ColorGradientKind, Contour, ContourPoint, FontProject,
    GlyphAnchor, GlyphComponent, GlyphData, Guideline,
};
use crate::tools::PenState;
use egui::{Color32, Pos2, Rect, Stroke, TextureId, Vec2};
use kurbo::{flatten, CubicBez, Line, ParamCurveNearest, PathEl, Point, QuadBez};

#[derive(Debug, Clone)]
pub struct CanvasState {
    pub zoom: f32,
    pub pan: Vec2,
    pub selected_points: Vec<usize>,
    pub selected_contour: Option<usize>,
    /// Selected nodes as (contour index, point index). The legacy fields above
    /// are kept for the single-contour toolbar actions.
    pub selected_nodes: Vec<(usize, usize)>,
    pub selected_component: Option<usize>,
    pub selected_components: Vec<usize>,
    pub point_radius: f32,
    pub show_grid: bool,
    pub show_metrics: bool,
    pub show_guidelines: bool,
    pub show_background_images: bool,
    pub show_contour_direction: bool,
    pub show_node_indices: bool,
    pub show_anchors: bool,
    pub snap_to_grid: bool,
    pub snap_to_guidelines: bool,
    pub snap_to_anchors: bool,
    pub grid_size: f32,
    pub selection_start: Option<Pos2>,
    pub selection_rect: Option<Rect>,
    pub ruler_start: Option<Pos2>,
    pub ruler_end: Option<Pos2>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            selected_points: Vec::new(),
            selected_contour: None,
            selected_nodes: Vec::new(),
            selected_component: None,
            selected_components: Vec::new(),
            point_radius: 5.0,
            show_grid: true,
            show_metrics: true,
            show_guidelines: true,
            show_background_images: true,
            show_contour_direction: false,
            show_node_indices: false,
            show_anchors: true,
            snap_to_grid: false,
            snap_to_guidelines: false,
            snap_to_anchors: false,
            grid_size: 100.0,
            selection_start: None,
            selection_rect: None,
            ruler_start: None,
            ruler_end: None,
        }
    }
}

fn gradient_color(point: Point, gradient: &ColorGradient, palette: &[[u8; 4]]) -> Color32 {
    let raw_position = match gradient.kind {
        ColorGradientKind::Linear => {
            let dx = gradient.x1 - gradient.x0;
            let dy = gradient.y1 - gradient.y0;
            let rotation_dx = gradient.x2 - gradient.x0;
            let rotation_dy = gradient.y2 - gradient.y0;
            let determinant = dx * rotation_dy - dy * rotation_dx;
            if determinant.abs() > f64::EPSILON {
                ((point.x - gradient.x0) * rotation_dy - (point.y - gradient.y0) * rotation_dx)
                    / determinant
            } else {
                0.0
            }
        }
        ColorGradientKind::Radial => {
            let radius = gradient.radius1.abs().max(f64::EPSILON);
            (((point.x - gradient.x0).powi(2) + (point.y - gradient.y0).powi(2)).sqrt()
                - gradient.radius0)
                / (radius - gradient.radius0).max(f64::EPSILON)
        }
        ColorGradientKind::Sweep => {
            let angle = (point.y - gradient.y0)
                .atan2(point.x - gradient.x0)
                .to_degrees();
            let mut delta = angle - gradient.start_angle;
            while delta < 0.0 {
                delta += 360.0;
            }
            let span = (gradient.end_angle - gradient.start_angle)
                .abs()
                .max(f64::EPSILON);
            delta / span
        }
    };
    let position = match gradient.extend {
        ColorGradientExtend::Pad => raw_position.clamp(0.0, 1.0),
        ColorGradientExtend::Repeat => raw_position.rem_euclid(1.0),
        ColorGradientExtend::Reflect => {
            let repeated = raw_position.rem_euclid(2.0);
            if repeated > 1.0 {
                2.0 - repeated
            } else {
                repeated
            }
        }
    };
    let mut stops = gradient.effective_stops();
    stops.sort_by(|left, right| {
        left.offset
            .partial_cmp(&right.offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(first) = stops.first() else {
        return Color32::TRANSPARENT;
    };
    let Some(last) = stops.last() else {
        return Color32::TRANSPARENT;
    };
    let (left, right, t) = if position <= first.offset {
        (first, first, 0.0)
    } else if position >= last.offset {
        (last, last, 0.0)
    } else {
        let pair = stops
            .windows(2)
            .find(|pair| position <= pair[1].offset)
            .expect("position is inside the stop range");
        let span = (pair[1].offset - pair[0].offset).max(f64::EPSILON);
        (
            &pair[0],
            &pair[1],
            ((position - pair[0].offset) / span).clamp(0.0, 1.0),
        )
    };
    let color = |stop: &crate::font_data::ColorGradientStop| {
        palette
            .get(usize::from(stop.palette_index))
            .copied()
            .unwrap_or([0, 0, 0, 0])
    };
    let start = color(left);
    let end = color(right);
    let alpha =
        |value: u8, stop_alpha: f64| (f64::from(value) * stop_alpha.clamp(0.0, 1.0)).round() as u8;
    let channel = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        channel(start[0], end[0]),
        channel(start[1], end[1]),
        channel(start[2], end[2]),
        channel(alpha(start[3], left.alpha), alpha(end[3], right.alpha)),
    )
}

fn compose_transform(
    parent: (f64, f64, f64, f64, f64, f64),
    component: &GlyphComponent,
) -> (f64, f64, f64, f64, f64, f64) {
    (
        parent.0 * component.x_scale + parent.2 * component.yx_scale,
        parent.1 * component.x_scale + parent.3 * component.yx_scale,
        parent.0 * component.xy_scale + parent.2 * component.y_scale,
        parent.1 * component.xy_scale + parent.3 * component.y_scale,
        parent.0 * component.x_offset + parent.2 * component.y_offset + parent.4,
        parent.1 * component.x_offset + parent.3 * component.y_offset + parent.5,
    )
}

mod geometry;
mod interaction;
mod layers;
mod render;
#[cfg(test)]
mod tests;
