use super::*;

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
}
