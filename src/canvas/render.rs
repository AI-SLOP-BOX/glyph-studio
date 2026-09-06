use super::*;

impl CanvasState {
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
    pub(super) fn draw_colored_glyph_recursive_inner(
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
}
