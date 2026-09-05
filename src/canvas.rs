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

impl CanvasState {
    pub fn draw_background_image(
        &self,
        painter: &egui::Painter,
        texture_id: TextureId,
        size: [usize; 2],
        origin: Pos2,
        opacity: f32,
        transform: crate::font_data::BackgroundImageTransform,
    ) {
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        let scale = transform.scale.max(0.001);
        let angle = f64::from(transform.rotation).to_radians();
        let rotate = |x: f64, y: f64| {
            (
                f64::from(transform.x) + x * angle.cos() - y * angle.sin(),
                f64::from(transform.y) + x * angle.sin() + y * angle.cos(),
            )
        };
        let points = [
            rotate(0.0, 0.0),
            rotate(size[0] as f64 * f64::from(scale), 0.0),
            rotate(
                size[0] as f64 * f64::from(scale),
                size[1] as f64 * f64::from(scale),
            ),
            rotate(0.0, size[1] as f64 * f64::from(scale)),
        ];
        let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut mesh = egui::epaint::Mesh::with_texture(texture_id);
        let u0 = if transform.flip_x { 1.0 } else { 0.0 };
        let u1 = if transform.flip_x { 0.0 } else { 1.0 };
        let v0 = if transform.flip_y { 0.0 } else { 1.0 };
        let v1 = if transform.flip_y { 1.0 } else { 0.0 };
        for (position, uv) in points.into_iter().zip([
            Pos2::new(u0, v0),
            Pos2::new(u1, v0),
            Pos2::new(u1, v1),
            Pos2::new(u0, v1),
        ]) {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: self.glyph_to_screen(position.0, position.1, origin),
                uv,
                color: Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
            });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        painter.add(mesh);
    }

    pub fn draw_guidelines(
        &self,
        painter: &egui::Painter,
        guidelines: &[Guideline],
        rect: Rect,
        origin: Pos2,
    ) {
        for guide in guidelines {
            let center = self.glyph_to_screen(guide.x, guide.y, origin);
            let angle = -(guide.angle as f32).to_radians();
            let direction = Vec2::new(angle.cos(), angle.sin());
            let half = rect.width().max(rect.height()) * 2.0;
            painter.line_segment(
                [center - direction * half, center + direction * half],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(100, 210, 255, 170)),
            );
            if !guide.name.is_empty() {
                painter.text(
                    center + Vec2::new(6.0, -6.0),
                    egui::Align2::LEFT_BOTTOM,
                    &guide.name,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(140, 220, 255),
                );
            }
        }
    }

    pub fn draw_guideline_highlight(
        &self,
        painter: &egui::Painter,
        guide: &Guideline,
        rect: Rect,
        origin: Pos2,
    ) {
        let center = self.glyph_to_screen(guide.x, guide.y, origin);
        let angle = -(guide.angle as f32).to_radians();
        let direction = Vec2::new(angle.cos(), angle.sin());
        let half = rect.width().max(rect.height()) * 2.0;
        painter.line_segment(
            [center - direction * half, center + direction * half],
            Stroke::new(2.5_f32, Color32::from_rgb(255, 210, 75)),
        );
        painter.circle_filled(center, 4.0, Color32::from_rgb(255, 210, 75));
    }

    pub fn draw_anchors(&self, painter: &egui::Painter, anchors: &[GlyphAnchor], origin: Pos2) {
        for anchor in anchors {
            let point = self.glyph_to_screen(anchor.x, anchor.y, origin);
            painter.circle_filled(point, 4.0, Color32::from_rgb(255, 170, 60));
            painter.text(
                point + Vec2::new(7.0, -7.0),
                egui::Align2::LEFT_BOTTOM,
                &anchor.name,
                egui::FontId::proportional(12.0),
                Color32::from_rgb(255, 210, 120),
            );
        }
    }

    pub fn glyph_to_screen(&self, x: f64, y: f64, origin: Pos2) -> Pos2 {
        Pos2::new(
            origin.x + x as f32 * self.zoom,
            origin.y - y as f32 * self.zoom,
        )
    }

    pub fn screen_to_glyph(&self, screen: Pos2, origin: Pos2) -> (f64, f64) {
        (
            ((screen.x - origin.x) / self.zoom) as f64,
            ((origin.y - screen.y) / self.zoom) as f64,
        )
    }

    pub fn snap_point(&self, x: f64, y: f64) -> (f64, f64) {
        if !self.snap_to_grid || self.grid_size <= 0.0 {
            return (x, y);
        }
        (
            (x / self.grid_size as f64).round() * self.grid_size as f64,
            (y / self.grid_size as f64).round() * self.grid_size as f64,
        )
    }

    pub fn snap_point_to_guidelines(&self, x: f64, y: f64, guidelines: &[Guideline]) -> (f64, f64) {
        if !self.snap_to_guidelines {
            return (x, y);
        }
        let threshold = 8.0 / f64::from(self.zoom.max(0.01));
        let mut snapped_x = x;
        let mut snapped_y = y;
        let mut best_x = threshold;
        let mut best_y = threshold;
        for guide in guidelines {
            let angle = guide.angle.to_radians();
            if angle.sin().abs() < 0.001 {
                let distance = (y - guide.y).abs();
                if distance < best_y {
                    best_y = distance;
                    snapped_y = guide.y;
                }
            } else if angle.cos().abs() < 0.001 {
                let distance = (x - guide.x).abs();
                if distance < best_x {
                    best_x = distance;
                    snapped_x = guide.x;
                }
            }
        }
        (snapped_x, snapped_y)
    }

    pub fn snap_point_to_anchors(&self, x: f64, y: f64, anchors: &[GlyphAnchor]) -> (f64, f64) {
        if !self.snap_to_anchors {
            return (x, y);
        }
        let threshold = 8.0 / f64::from(self.zoom.max(0.01));
        anchors
            .iter()
            .map(|anchor| {
                let distance = ((x - anchor.x).powi(2) + (y - anchor.y).powi(2)).sqrt();
                (distance, (anchor.x, anchor.y))
            })
            .filter(|(distance, _)| *distance < threshold)
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, point)| point)
            .unwrap_or((x, y))
    }

    pub fn draw_grid(&self, painter: &egui::Painter, rect: Rect, origin: Pos2) {
        if !self.show_grid {
            return;
        }

        let stroke = Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(100, 100, 100, 60));
        let grid = self.grid_size * self.zoom;

        if grid < 5.0 {
            return;
        }

        let start_x = ((rect.min.x - origin.x) / grid).floor() * grid + origin.x;
        let mut x = start_x;
        while x < rect.max.x {
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], stroke);
            x += grid;
        }

        let start_y = ((rect.min.y - origin.y) / grid).floor() * grid + origin.y;
        let mut y = start_y;
        while y < rect.max.y {
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], stroke);
            y += grid;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_metrics(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        origin: Pos2,
        units_per_em: f64,
        ascender: f64,
        descender: f64,
        advance_width: f64,
    ) {
        if !self.show_metrics {
            return;
        }

        let em = units_per_em as f32 * self.zoom;
        let asc = ascender as f32 * self.zoom;
        let desc = descender as f32 * self.zoom;

        let baseline_y = origin.y;
        painter.line_segment(
            [
                Pos2::new(rect.min.x, baseline_y),
                Pos2::new(rect.max.x, baseline_y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 0, 0, 180)),
        );

        let asc_y = origin.y - asc;
        painter.line_segment(
            [Pos2::new(rect.min.x, asc_y), Pos2::new(rect.max.x, asc_y)],
            Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(0, 200, 0, 120)),
        );

        let desc_y = origin.y - desc;
        painter.line_segment(
            [Pos2::new(rect.min.x, desc_y), Pos2::new(rect.max.x, desc_y)],
            Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(0, 0, 255, 120)),
        );

        painter.rect_stroke(
            Rect::from_min_size(Pos2::new(origin.x - em / 2.0, asc_y), Vec2::new(em, em)),
            0.0,
            Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(200, 200, 0, 80)),
            egui::StrokeKind::Inside,
        );

        let advance_x = origin.x + advance_width as f32 * self.zoom;
        let advance_stroke =
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(80, 190, 255, 180));
        painter.line_segment(
            [
                Pos2::new(origin.x, rect.min.y),
                Pos2::new(origin.x, rect.max.y),
            ],
            advance_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(advance_x, rect.min.y),
                Pos2::new(advance_x, rect.max.y),
            ],
            advance_stroke,
        );
        painter.text(
            Pos2::new(advance_x - 4.0, rect.min.y + 8.0),
            egui::Align2::RIGHT_TOP,
            format!("width {:.0}", advance_width),
            egui::FontId::monospace(10.0),
            Color32::from_rgb(100, 200, 255),
        );
    }

    pub fn draw_contour(
        &self,
        painter: &egui::Painter,
        contour: &Contour,
        origin: Pos2,
        color: Color32,
        contour_index: usize,
    ) {
        if contour.points.is_empty() {
            return;
        }

        let mut outline = Vec::new();
        flatten(contour.to_bezpath(), 0.75, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                outline.push(self.glyph_to_screen(point.x, point.y, origin));
            }
        });

        if outline.len() >= 3 {
            painter.add(egui::Shape::convex_polygon(
                outline.clone(),
                Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 40),
                Stroke::NONE,
            ));
        }

        for window in outline.windows(2) {
            painter.line_segment([window[0], window[1]], Stroke::new(1.0_f32, color));
        }
        if let Some(last) = outline.last() {
            if outline.len() > 2 {
                painter.line_segment([*last, outline[0]], Stroke::new(1.0_f32, color));
            }
        }

        // Show only the handles belonging to this contour's adjacent segments.
        // Connecting every nearby on-curve point becomes misleading in dense glyphs.
        for (index, point) in contour.points.iter().enumerate() {
            if point.is_on_curve() {
                continue;
            }
            let screen = self.glyph_to_screen(point.x, point.y, origin);
            let previous = (0..contour.points.len())
                .map(|offset| (index + contour.points.len() - 1 - offset) % contour.points.len())
                .find(|&candidate| contour.points[candidate].is_on_curve());
            let next = (0..contour.points.len())
                .map(|offset| (index + 1 + offset) % contour.points.len())
                .find(|&candidate| contour.points[candidate].is_on_curve());
            for candidate in [previous, next].into_iter().flatten() {
                let other = &contour.points[candidate];
                let other_screen = self.glyph_to_screen(other.x, other.y, origin);
                let handle_selected = self.selected_nodes.contains(&(contour_index, index))
                    || (self.selected_contour == Some(contour_index)
                        && self.selected_points.contains(&index));
                painter.line_segment(
                    [screen, other_screen],
                    Stroke::new(
                        if handle_selected { 1.0_f32 } else { 0.5_f32 },
                        if handle_selected {
                            Color32::from_rgba_premultiplied(255, 210, 80, 210)
                        } else {
                            Color32::from_rgba_premultiplied(100, 100, 255, 150)
                        },
                    ),
                );
            }
        }

        for (i, point) in contour.points.iter().enumerate() {
            let screen = self.glyph_to_screen(point.x, point.y, origin);
            let is_selected = self.selected_nodes.contains(&(contour_index, i))
                || (self.selected_contour == Some(contour_index)
                    && self.selected_points.contains(&i));

            if point.is_on_curve() {
                let c = if is_selected {
                    Color32::YELLOW
                } else {
                    Color32::WHITE
                };
                painter.circle_filled(screen, self.point_radius, c);
                painter.circle_stroke(
                    screen,
                    self.point_radius,
                    Stroke::new(1.0_f32, Color32::BLACK),
                );
                if point.smooth {
                    painter.circle_stroke(
                        screen,
                        self.point_radius + 3.0,
                        Stroke::new(
                            1.0_f32,
                            if is_selected {
                                Color32::from_rgb(255, 220, 90)
                            } else {
                                Color32::from_rgb(100, 210, 190)
                            },
                        ),
                    );
                }
            } else {
                let c = if is_selected {
                    Color32::YELLOW
                } else {
                    Color32::from_rgb(100, 100, 255)
                };
                let r = self.point_radius * 0.8;
                let pts = vec![
                    Pos2::new(screen.x, screen.y - r),
                    Pos2::new(screen.x + r, screen.y),
                    Pos2::new(screen.x, screen.y + r),
                    Pos2::new(screen.x - r, screen.y),
                ];
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    c,
                    Stroke::new(1.0_f32, Color32::BLACK),
                ));
            }
            if self.show_node_indices {
                painter.text(
                    screen + Vec2::new(7.0, -7.0),
                    egui::Align2::LEFT_BOTTOM,
                    i.to_string(),
                    egui::FontId::monospace(10.0),
                    if is_selected {
                        Color32::YELLOW
                    } else {
                        Color32::from_rgb(190, 200, 220)
                    },
                );
            }
        }
    }

    pub fn draw_glyph_recursive(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        origin: Pos2,
    ) {
        self.draw_glyph_recursive_inner(
            painter,
            project,
            glyph_name,
            origin,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
        );
    }

    pub fn draw_contour_directions(
        &self,
        painter: &egui::Painter,
        contours: &[Contour],
        origin: Pos2,
    ) {
        for contour in contours {
            let Some(first) = contour.points.first() else {
                continue;
            };
            let Some(second) = contour.points.get(1) else {
                continue;
            };
            let start = self.glyph_to_screen(first.x, first.y, origin);
            let end = self.glyph_to_screen(second.x, second.y, origin);
            let direction = (end - start).normalized();
            let midpoint = start + (end - start) * 0.5;
            let side = Vec2::new(-direction.y, direction.x);
            let tip = midpoint + direction * 7.0;
            painter.line_segment(
                [midpoint - direction * 7.0, tip],
                Stroke::new(1.2_f32, Color32::from_rgb(255, 175, 70)),
            );
            painter.line_segment(
                [tip, tip - direction * 5.0 + side * 3.5],
                Stroke::new(1.2_f32, Color32::from_rgb(255, 175, 70)),
            );
            painter.line_segment(
                [tip, tip - direction * 5.0 - side * 3.5],
                Stroke::new(1.2_f32, Color32::from_rgb(255, 175, 70)),
            );
        }
    }

    pub fn draw_color_glyph(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        master_id: &str,
        palette_index: usize,
        origin: Pos2,
    ) -> bool {
        let Some(layers) = project.color_layers.get(glyph_name) else {
            return false;
        };
        let Some(palette) = project
            .color_palettes
            .get(palette_index)
            .or_else(|| project.color_palettes.first())
        else {
            return false;
        };
        let mut drawn = false;
        for (index, layer) in layers.iter().enumerate() {
            let transform = project
                .color_layer_transforms
                .get(glyph_name)
                .and_then(|transforms| transforms.get(index))
                .copied()
                .flatten()
                .unwrap_or_default();
            let parent = (
                transform.xx,
                transform.yx,
                transform.xy,
                transform.yy,
                transform.dx,
                transform.dy,
            );
            if let Some(gradient) = &layer.gradient {
                drawn |= self.draw_gradient_glyph_recursive_inner(
                    painter,
                    project,
                    &layer.glyph,
                    master_id,
                    origin,
                    parent,
                    gradient,
                    palette,
                    &mut Vec::new(),
                );
            } else {
                let Some(&[r, g, b, a]) = palette.get(usize::from(layer.palette_index)) else {
                    continue;
                };
                drawn |= self.draw_colored_glyph_recursive_inner(
                    painter,
                    project,
                    &layer.glyph,
                    master_id,
                    origin,
                    parent,
                    Color32::from_rgba_unmultiplied(r, g, b, a),
                    &mut Vec::new(),
                );
            }
        }
        drawn
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_gradient_glyph_recursive_inner(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        master_id: &str,
        origin: Pos2,
        parent: (f64, f64, f64, f64, f64, f64),
        gradient: &ColorGradient,
        palette: &[[u8; 4]],
        stack: &mut Vec<String>,
    ) -> bool {
        if stack.iter().any(|name| name == glyph_name) {
            return false;
        }
        let Some(glyph) = project.glyphs.get(glyph_name) else {
            return false;
        };
        stack.push(glyph_name.to_string());
        let active_layer = glyph.layers.get(master_id);
        let contours =
            active_layer.map_or(glyph.contours.as_slice(), |layer| layer.contours.as_slice());
        for contour in contours {
            let transformed = Contour {
                points: contour
                    .points
                    .iter()
                    .map(|point| ContourPoint {
                        x: parent.0 * point.x + parent.2 * point.y + parent.4,
                        y: parent.1 * point.x + parent.3 * point.y + parent.5,
                        ..*point
                    })
                    .collect(),
            };
            self.draw_gradient_contour(painter, &transformed, origin, gradient, palette);
        }
        let mut drawn = !contours.is_empty();
        let components = active_layer.map_or(glyph.components.as_slice(), |layer| {
            layer.components.as_slice()
        });
        for component in components {
            drawn |= self.draw_gradient_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                master_id,
                origin,
                compose_transform(parent, component),
                gradient,
                palette,
                stack,
            );
        }
        stack.pop();
        drawn
    }

    fn draw_gradient_contour(
        &self,
        painter: &egui::Painter,
        contour: &Contour,
        origin: Pos2,
        gradient: &ColorGradient,
        palette: &[[u8; 4]],
    ) {
        if contour.points.is_empty() {
            return;
        }
        let mut outline = Vec::new();
        flatten(contour.to_bezpath(), 0.75, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                outline.push((point, self.glyph_to_screen(point.x, point.y, origin)));
            }
        });
        if outline.len() >= 3 {
            let mut mesh = egui::epaint::Mesh::default();
            for (point, screen) in &outline {
                mesh.colored_vertex(*screen, gradient_color(*point, gradient, palette));
            }
            for index in 1..outline.len() - 1 {
                mesh.add_triangle(0, index as u32, (index + 1) as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        for window in outline.windows(2) {
            painter.line_segment(
                [window[0].1, window[1].1],
                Stroke::new(1.0_f32, gradient_color(window[0].0, gradient, palette)),
            );
        }
        if let Some(last) = outline.last() {
            painter.line_segment(
                [last.1, outline[0].1],
                Stroke::new(1.0_f32, gradient_color(last.0, gradient, palette)),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_colored_glyph_recursive_inner(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        master_id: &str,
        origin: Pos2,
        parent: (f64, f64, f64, f64, f64, f64),
        color: Color32,
        stack: &mut Vec<String>,
    ) -> bool {
        if stack.iter().any(|name| name == glyph_name) {
            return false;
        }
        let Some(glyph) = project.glyphs.get(glyph_name) else {
            return false;
        };
        stack.push(glyph_name.to_string());
        let active_layer = glyph.layers.get(master_id);
        let contours =
            active_layer.map_or(glyph.contours.as_slice(), |layer| layer.contours.as_slice());
        for (index, contour) in contours.iter().enumerate() {
            let transformed = Contour {
                points: contour
                    .points
                    .iter()
                    .map(|point| ContourPoint {
                        x: parent.0 * point.x + parent.2 * point.y + parent.4,
                        y: parent.1 * point.x + parent.3 * point.y + parent.5,
                        ..*point
                    })
                    .collect(),
            };
            self.draw_contour(painter, &transformed, origin, color, index);
        }
        let mut drawn = !contours.is_empty();
        let components = active_layer.map_or(glyph.components.as_slice(), |layer| {
            layer.components.as_slice()
        });
        for component in components {
            drawn |= self.draw_colored_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                master_id,
                origin,
                compose_transform(parent, component),
                color,
                stack,
            );
        }
        stack.pop();
        drawn
    }

    pub fn draw_layer(
        &self,
        painter: &egui::Painter,
        layer: &crate::font_data::GlyphLayer,
        origin: Pos2,
        color: Color32,
    ) {
        for (index, contour) in layer.contours.iter().enumerate() {
            self.draw_contour(painter, contour, origin, color, index);
        }
    }

    pub fn draw_master_overlay(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        master_id: &str,
        origin: Pos2,
        color: Color32,
    ) {
        self.draw_colored_glyph_recursive_inner(
            painter,
            project,
            glyph_name,
            master_id,
            origin,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            color,
            &mut Vec::new(),
        );
    }

    pub fn draw_conditional_layer(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        layer: &crate::font_data::GlyphLayer,
        master_id: &str,
        origin: Pos2,
        color: Color32,
    ) {
        self.draw_layer(painter, layer, origin, color);
        for component in &layer.components {
            self.draw_colored_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                master_id,
                origin,
                compose_transform((1.0, 0.0, 0.0, 1.0, 0.0, 0.0), component),
                color,
                &mut Vec::new(),
            );
        }
    }

    fn draw_glyph_recursive_inner(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        origin: Pos2,
        parent: (f64, f64, f64, f64, f64, f64),
        stack: &mut Vec<String>,
    ) {
        if stack.iter().any(|name| name == glyph_name) {
            return;
        }
        let Some(glyph) = project.glyphs.get(glyph_name) else {
            return;
        };
        stack.push(glyph_name.to_string());
        for (index, contour) in glyph.contours.iter().enumerate() {
            let transformed = Contour {
                points: contour
                    .points
                    .iter()
                    .map(|point| ContourPoint {
                        x: parent.0 * point.x + parent.2 * point.y + parent.4,
                        y: parent.1 * point.x + parent.3 * point.y + parent.5,
                        ..*point
                    })
                    .collect(),
            };
            self.draw_contour(painter, &transformed, origin, Color32::WHITE, index);
        }
        for component in &glyph.components {
            let transform = compose_transform(parent, component);
            self.draw_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                origin,
                transform,
                stack,
            );
        }
        stack.pop();
    }

    pub fn draw_selection(&self, painter: &egui::Painter) {
        if let Some(rect) = self.selection_rect {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_premultiplied(80, 140, 255, 35),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0_f32, Color32::from_rgb(100, 170, 255)),
                egui::StrokeKind::Inside,
            );
        }
    }

    pub fn draw_ruler(&self, painter: &egui::Painter, origin: Pos2) {
        let (Some(start), Some(end)) = (self.ruler_start, self.ruler_end) else {
            return;
        };
        painter.line_segment(
            [start, end],
            Stroke::new(1.5_f32, Color32::from_rgb(255, 190, 40)),
        );
        let (x1, y1) = self.screen_to_glyph(start, origin);
        let (x2, y2) = self.screen_to_glyph(end, origin);
        let label = format!(
            "Δx {:.1}  Δy {:.1}  {:.1}",
            x2 - x1,
            y2 - y1,
            ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
        );
        painter.text(
            end + Vec2::new(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::monospace(11.0),
            Color32::from_rgb(255, 220, 100),
        );
    }

    pub fn transform_selected(
        &self,
        glyph: &mut GlyphData,
        contour_index: usize,
        scale: f64,
        angle_radians: f64,
    ) -> bool {
        let Some(contour) = glyph.contours.get_mut(contour_index) else {
            return false;
        };
        let selected: Vec<usize> = self
            .selected_points
            .iter()
            .copied()
            .filter(|&index| index < contour.points.len())
            .collect();
        if selected.is_empty() {
            return false;
        }
        let (cx, cy) = selected.iter().fold((0.0, 0.0), |(x, y), &index| {
            (x + contour.points[index].x, y + contour.points[index].y)
        });
        let center = (cx / selected.len() as f64, cy / selected.len() as f64);
        let (sin, cos) = angle_radians.sin_cos();
        for index in selected {
            let point = &mut contour.points[index];
            let x = (point.x - center.0) * scale;
            let y = (point.y - center.1) * scale;
            point.x = center.0 + x * cos - y * sin;
            point.y = center.1 + x * sin + y * cos;
        }
        contour.repair_smooth_handles();
        true
    }

    pub fn transform_selected_nodes(
        &self,
        glyph: &mut GlyphData,
        scale: f64,
        angle_radians: f64,
    ) -> bool {
        let nodes: Vec<(usize, usize)> = self
            .selected_nodes
            .iter()
            .copied()
            .filter(|&(ci, pi)| {
                glyph
                    .contours
                    .get(ci)
                    .and_then(|c| c.points.get(pi))
                    .is_some()
            })
            .collect();
        if nodes.is_empty() {
            return false;
        }
        let (cx, cy) = nodes.iter().fold((0.0, 0.0), |(x, y), &(ci, pi)| {
            let p = &glyph.contours[ci].points[pi];
            (x + p.x, y + p.y)
        });
        let center = (cx / nodes.len() as f64, cy / nodes.len() as f64);
        let (sin, cos) = angle_radians.sin_cos();
        for (ci, pi) in nodes {
            let p = &mut glyph.contours[ci].points[pi];
            let x = (p.x - center.0) * scale;
            let y = (p.y - center.1) * scale;
            p.x = center.0 + x * cos - y * sin;
            p.y = center.1 + x * sin + y * cos;
        }
        for contour in &mut glyph.contours {
            contour.repair_smooth_handles();
        }
        true
    }

    pub fn draw_pen_preview(&self, painter: &egui::Painter, pen_state: &PenState, origin: Pos2) {
        if pen_state.preview_points.is_empty() && pen_state.drag_preview.is_none() {
            return;
        }

        let screens: Vec<Pos2> = pen_state
            .preview_points
            .iter()
            .map(|p| self.glyph_to_screen(p.x, p.y, origin))
            .collect();

        for window in screens.windows(2) {
            painter.line_segment(
                [window[0], window[1]],
                Stroke::new(1.5_f32, Color32::from_rgb(0, 255, 100)),
            );
        }

        for point in &pen_state.preview_points {
            let screen = self.glyph_to_screen(point.x, point.y, origin);
            if point.is_on_curve() {
                painter.circle_filled(screen, self.point_radius, Color32::GREEN);
            } else {
                painter.circle_filled(
                    screen,
                    self.point_radius * 0.7,
                    Color32::from_rgb(0, 200, 0),
                );
            }
        }
        if let Some(((anchor_x, anchor_y), (handle_x, handle_y))) = pen_state.drag_preview {
            let anchor = self.glyph_to_screen(anchor_x, anchor_y, origin);
            let handle = self.glyph_to_screen(handle_x, handle_y, origin);
            painter.line_segment(
                [anchor, handle],
                Stroke::new(1.5_f32, Color32::from_rgb(255, 210, 80)),
            );
            painter.circle_filled(anchor, self.point_radius, Color32::from_rgb(255, 235, 120));
            painter.circle_filled(
                handle,
                self.point_radius * 0.7,
                Color32::from_rgb(255, 170, 60),
            );
        }
    }

    pub fn hit_test(&self, mouse: Pos2, glyph: &GlyphData, origin: Pos2) -> Option<(usize, usize)> {
        let mut closest_dist = f32::MAX;
        let mut closest = None;

        for (ci, contour) in glyph.contours.iter().enumerate() {
            for (pi, point) in contour.points.iter().enumerate() {
                let screen = self.glyph_to_screen(point.x, point.y, origin);
                let dist = mouse.distance(screen);
                if dist < self.point_radius * 2.0 && dist < closest_dist {
                    closest_dist = dist;
                    closest = Some((ci, pi));
                }
            }
        }

        closest
    }

    /// Returns the nearest authored Bezier segment under the cursor as
    /// `(contour index, segment start node index, curve factor)`.
    pub fn hit_test_segment(
        &self,
        mouse: Pos2,
        glyph: &GlyphData,
        origin: Pos2,
    ) -> Option<(usize, usize, f64)> {
        let (x, y) = self.screen_to_glyph(mouse, origin);
        let cursor = Point::new(x, y);
        let mut best: Option<(f64, usize, usize, f64)> = None;
        for (contour_index, contour) in glyph.contours.iter().enumerate() {
            let len = contour.points.len();
            if len < 2 {
                continue;
            }
            for start in 0..len {
                if !contour.points[start].is_on_curve() {
                    continue;
                }
                let mut indices = vec![start];
                let mut index = (start + 1) % len;
                while index != start && !contour.points[index].is_on_curve() && indices.len() <= 3 {
                    indices.push(index);
                    index = (index + 1) % len;
                }
                if index == start || indices.len() > 3 || !contour.points[index].is_on_curve() {
                    continue;
                }
                indices.push(index);
                let point = |point_index: usize| {
                    let point = contour.points[point_index];
                    Point::new(point.x, point.y)
                };
                let (distance_sq, factor) = match indices.len() - 1 {
                    1 => {
                        let nearest =
                            Line::new(point(indices[0]), point(indices[1])).nearest(cursor, 0.01);
                        (nearest.distance_sq, nearest.t)
                    }
                    2 => {
                        let nearest =
                            QuadBez::new(point(indices[0]), point(indices[1]), point(indices[2]))
                                .nearest(cursor, 0.01);
                        (nearest.distance_sq, nearest.t)
                    }
                    3 => {
                        let nearest = CubicBez::new(
                            point(indices[0]),
                            point(indices[1]),
                            point(indices[2]),
                            point(indices[3]),
                        )
                        .nearest(cursor, 0.01);
                        (nearest.distance_sq, nearest.t)
                    }
                    _ => continue,
                };
                if best.is_none_or(|(best_distance, _, _, _)| distance_sq < best_distance) {
                    best = Some((distance_sq, contour_index, start, factor));
                }
            }
        }
        let (distance_sq, contour_index, start, factor) = best?;
        let threshold = (12.0 / f64::from(self.zoom.max(0.01))).powi(2);
        (distance_sq <= threshold).then_some((contour_index, start, factor))
    }

    pub fn hit_test_component(
        &self,
        mouse: Pos2,
        project: &FontProject,
        glyph: &GlyphData,
        origin: Pos2,
    ) -> Option<usize> {
        glyph
            .components
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, component)| {
                let (min_x, min_y, max_x, max_y) =
                    project.outline_bounds_for_glyph(&component.base)?;
                let corners = [
                    (min_x, min_y),
                    (min_x, max_y),
                    (max_x, min_y),
                    (max_x, max_y),
                ]
                .into_iter()
                .map(|(x, y)| {
                    (
                        component.x_scale * x + component.yx_scale * y + component.x_offset,
                        component.xy_scale * x + component.y_scale * y + component.y_offset,
                    )
                });
                let (screen_min_x, screen_min_y, screen_max_x, screen_max_y) = corners.fold(
                    (
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ),
                    |(min_x, min_y, max_x, max_y), (x, y)| {
                        let screen = self.glyph_to_screen(x, y, origin);
                        (
                            min_x.min(screen.x as f64),
                            min_y.min(screen.y as f64),
                            max_x.max(screen.x as f64),
                            max_y.max(screen.y as f64),
                        )
                    },
                );
                let margin = 10.0_f64.max(self.point_radius as f64 * 2.0);
                (mouse.x as f64 >= screen_min_x - margin
                    && mouse.x as f64 <= screen_max_x + margin
                    && mouse.y as f64 >= screen_min_y - margin
                    && mouse.y as f64 <= screen_max_y + margin)
                    .then_some(index)
            })
    }

    pub fn hit_test_component_handle(
        &self,
        mouse: Pos2,
        project: &FontProject,
        glyph: &GlyphData,
        component_index: usize,
        origin: Pos2,
    ) -> Option<usize> {
        let component = glyph.components.get(component_index)?;
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&component.base)?;
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
            (max_x, min_y),
        ];
        let corner_hit = corners.into_iter().enumerate().find_map(|(index, (x, y))| {
            let screen = self.glyph_to_screen(
                component.x_scale * x + component.yx_scale * y + component.x_offset,
                component.xy_scale * x + component.y_scale * y + component.y_offset,
                origin,
            );
            (screen.distance(mouse) <= 10.0).then_some(index)
        });
        if corner_hit.is_some() {
            return corner_hit;
        }
        let top_center = self.glyph_to_screen(
            component.x_scale * (min_x + max_x) * 0.5
                + component.yx_scale * max_y
                + component.x_offset,
            component.xy_scale * (min_x + max_x) * 0.5
                + component.y_scale * max_y
                + component.y_offset,
            origin,
        );
        let rotation_handle = top_center + Vec2::new(0.0, -22.0);
        (rotation_handle.distance(mouse) <= 10.0).then_some(4)
    }

    pub fn draw_component_selection(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph: &GlyphData,
        index: usize,
        origin: Pos2,
        show_rotation_handle: bool,
    ) {
        let Some(component) = glyph.components.get(index) else {
            return;
        };
        let Some((min_x, min_y, max_x, max_y)) = project.outline_bounds_for_glyph(&component.base)
        else {
            return;
        };
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
            (max_x, min_y),
        ]
        .into_iter()
        .map(|(x, y)| {
            self.glyph_to_screen(
                component.x_scale * x + component.yx_scale * y + component.x_offset,
                component.xy_scale * x + component.y_scale * y + component.y_offset,
                origin,
            )
        })
        .collect::<Vec<_>>();
        let selection_color = Color32::from_rgb(255, 210, 80);
        for (from, to) in corners
            .iter()
            .copied()
            .zip(corners.iter().copied().cycle().skip(1))
            .take(corners.len())
        {
            painter.line_segment([from, to], Stroke::new(2.0_f32, selection_color));
        }
        for corner in corners.iter().copied() {
            painter.rect_filled(
                egui::Rect::from_center_size(corner, Vec2::splat(7.0)),
                1.0,
                selection_color,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(corner, Vec2::splat(7.0)),
                1.0,
                Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 45)),
                egui::StrokeKind::Inside,
            );
        }
        if show_rotation_handle {
            let top_center = corners[1].lerp(corners[2], 0.5);
            let rotation_handle = top_center + Vec2::new(0.0, -22.0);
            painter.line_segment(
                [top_center, rotation_handle],
                Stroke::new(1.0_f32, selection_color),
            );
            painter.circle_filled(rotation_handle, 4.0, selection_color);
            painter.circle_stroke(
                rotation_handle,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 45)),
            );
        }
        painter.text(
            corners[0] + Vec2::new(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            format!("部品: {}", component.base),
            egui::FontId::monospace(10.0),
            selection_color,
        );
    }

    pub fn zoom_at(&mut self, delta: f32, mouse: Pos2, canvas_center: Pos2) {
        let old_zoom = self.zoom;
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.05, 50.0);

        let zoom_ratio = self.zoom / old_zoom;
        let mouse_from_center = mouse - canvas_center;
        self.pan = Vec2::new(
            mouse_from_center.x - (mouse_from_center.x - self.pan.x) * zoom_ratio,
            mouse_from_center.y - (mouse_from_center.y - self.pan.y) * zoom_ratio,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_at_keeps_pointer_position_stable() {
        let mut canvas = CanvasState::default();
        let center = Pos2::new(100.0, 100.0);
        let pointer = Pos2::new(150.0, 100.0);
        let glyph_before = canvas.screen_to_glyph(pointer, center);
        canvas.zoom_at(10.0, pointer, center);
        let new_origin = center + canvas.pan;
        let glyph_after = canvas.screen_to_glyph(
            canvas.glyph_to_screen(glyph_before.0, glyph_before.1, new_origin),
            new_origin,
        );
        assert!((glyph_before.0 - glyph_after.0).abs() < 0.001);
        assert!((glyph_before.1 - glyph_after.1).abs() < 0.001);
    }

    #[test]
    fn snap_point_rounds_to_configured_grid() {
        let canvas = CanvasState {
            snap_to_grid: true,
            grid_size: 100.0,
            ..Default::default()
        };
        assert_eq!(canvas.snap_point(149.0, -151.0), (100.0, -200.0));
    }

    #[test]
    fn snap_point_to_guidelines_snaps_near_horizontal_and_vertical_guides() {
        let canvas = CanvasState {
            snap_to_guidelines: true,
            zoom: 2.0,
            ..Default::default()
        };
        let guides = vec![
            Guideline {
                x: 300.0,
                y: 0.0,
                angle: 90.0,
                name: String::new(),
            },
            Guideline {
                x: 0.0,
                y: 500.0,
                angle: 0.0,
                name: String::new(),
            },
        ];
        assert_eq!(
            canvas.snap_point_to_guidelines(303.0, 496.5, &guides),
            (300.0, 500.0)
        );
    }

    #[test]
    fn gradient_color_interpolates_linear_endpoints() {
        let gradient = ColorGradient {
            start_palette_index: 0,
            end_palette_index: 1,
            kind: ColorGradientKind::Linear,
            extend: ColorGradientExtend::default(),
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 0.0,
            x2: 0.0,
            y2: 100.0,
            stops: vec![
                crate::font_data::ColorGradientStop {
                    offset: 0.0,
                    palette_index: 0,
                    alpha: 1.0,
                },
                crate::font_data::ColorGradientStop {
                    offset: 0.5,
                    palette_index: 1,
                    alpha: 0.5,
                },
                crate::font_data::ColorGradientStop {
                    offset: 1.0,
                    palette_index: 0,
                    alpha: 1.0,
                },
            ],
            radius0: 0.0,
            radius1: 100.0,
            start_angle: 0.0,
            end_angle: 360.0,
        };
        let palette = [[255, 0, 0, 255], [0, 0, 255, 255]];
        assert_eq!(
            gradient_color(Point::new(0.0, 0.0), &gradient, &palette),
            Color32::RED
        );
        assert_eq!(
            gradient_color(Point::new(50.0, 0.0), &gradient, &palette),
            Color32::from_rgba_unmultiplied(0, 0, 255, 128)
        );
        assert_eq!(
            gradient_color(Point::new(100.0, 0.0), &gradient, &palette),
            Color32::RED
        );
    }

    #[test]
    fn gradient_color_applies_repeat_and_reflect_extensions() {
        let mut gradient = ColorGradient {
            start_palette_index: 0,
            end_palette_index: 1,
            kind: ColorGradientKind::Linear,
            extend: ColorGradientExtend::Repeat,
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 0.0,
            x2: 0.0,
            y2: 100.0,
            stops: Vec::new(),
            radius0: 0.0,
            radius1: 100.0,
            start_angle: 0.0,
            end_angle: 360.0,
        };
        let palette = [[255, 0, 0, 255], [0, 0, 255, 255]];
        assert_eq!(
            gradient_color(Point::new(-50.0, 0.0), &gradient, &palette),
            gradient_color(Point::new(50.0, 0.0), &gradient, &palette)
        );
        gradient.extend = ColorGradientExtend::Reflect;
        assert_eq!(
            gradient_color(Point::new(150.0, 0.0), &gradient, &palette),
            gradient_color(Point::new(50.0, 0.0), &gradient, &palette)
        );
    }

    #[test]
    fn snap_point_to_guidelines_leaves_distant_points_and_disabled_snap_unchanged() {
        let guides = vec![Guideline {
            x: 300.0,
            y: 500.0,
            angle: 0.0,
            name: String::new(),
        }];
        let canvas = CanvasState::default();
        assert_eq!(
            canvas.snap_point_to_guidelines(303.0, 496.0, &guides),
            (303.0, 496.0)
        );
        let canvas = CanvasState {
            snap_to_guidelines: true,
            ..Default::default()
        };
        assert_eq!(
            canvas.snap_point_to_guidelines(330.0, 488.0, &guides),
            (330.0, 488.0)
        );
    }

    #[test]
    fn snap_point_to_anchors_uses_nearest_anchor_with_zoom_scaled_threshold() {
        let canvas = CanvasState {
            snap_to_anchors: true,
            zoom: 2.0,
            ..Default::default()
        };
        let anchors = vec![
            GlyphAnchor {
                name: "top".into(),
                x: 300.0,
                y: 500.0,
            },
            GlyphAnchor {
                name: "bottom".into(),
                x: 100.0,
                y: 0.0,
            },
        ];
        assert_eq!(
            canvas.snap_point_to_anchors(303.0, 502.0, &anchors),
            (300.0, 500.0)
        );
        assert_eq!(
            canvas.snap_point_to_anchors(310.0, 500.0, &anchors),
            (310.0, 500.0)
        );
    }

    #[test]
    fn hit_test_segment_finds_closed_line_segment_and_factor() {
        let canvas = CanvasState::default();
        let glyph = GlyphData {
            name: "square".into(),
            unicode: None,
            unicodes: Vec::new(),
            width: 600.0,
            left_kerning_group: String::new(),
            right_kerning_group: String::new(),
            left_metrics_key: String::new(),
            right_metrics_key: String::new(),
            anchors: Vec::new(),
            contours: vec![Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            }],
            components: Vec::new(),
            layers: std::collections::HashMap::new(),
            guidelines: Vec::new(),
            master_guidelines: std::collections::HashMap::new(),
        };
        let hit = canvas
            .hit_test_segment(Pos2::new(50.0, 0.0), &glyph, Pos2::ZERO)
            .expect("segment should be hit");
        assert_eq!(hit.0, 0);
        assert_eq!(hit.1, 0);
        assert!((hit.2 - 0.5).abs() < 0.01);
    }

    #[test]
    fn component_hit_test_uses_transformed_component_bounds() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 2.0,
            x_offset: 300.0,
            y_offset: 200.0,
        });
        let canvas = CanvasState::default();
        assert_eq!(
            canvas.hit_test_component(Pos2::new(350.0, -300.0), &project, &composite, Pos2::ZERO,),
            Some(0)
        );
        assert_eq!(
            canvas.hit_test_component(Pos2::new(100.0, 100.0), &project, &composite, Pos2::ZERO,),
            None
        );
    }
}
