use super::*;

impl CanvasState {
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
