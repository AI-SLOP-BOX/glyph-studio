use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn draw_canvas_scene(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) -> (egui::Response, egui::Painter, egui::Rect, egui::Pos2) {
        let available_size = ui.available_size();
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        let rect = response.rect;
        let origin = Pos2::new(
            rect.center().x + self.canvas.pan.x,
            rect.center().y + self.canvas.pan.y,
        );

        painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));
        let empty_glyph = self
            .current_glyph
            .as_ref()
            .and_then(|name| self.project.glyphs.get(name))
            .and_then(|glyph| {
                glyph
                    .layers
                    .get(&self.current_master_id)
                    .or_else(|| glyph.layers.values().next())
            })
            .is_some_and(|layer| layer.contours.is_empty() && layer.components.is_empty());
        if self.current_glyph.is_none() || empty_glyph {
            let title = self
                .current_glyph
                .as_deref()
                .map_or("グリフを選択", |name| {
                    if empty_glyph {
                        name
                    } else {
                        "グリフを選択"
                    }
                });
            painter.text(
                rect.center() - Vec2::new(0.0, 18.0),
                egui::Align2::CENTER_CENTER,
                title,
                egui::FontId::proportional(20.0),
                Color32::from_rgb(205, 210, 222),
            );
            painter.text(
                rect.center() + Vec2::new(0.0, 16.0),
                egui::Align2::CENTER_CENTER,
                "ペンツール (P) で描き始める  ·  SVGを読み込むこともできます",
                egui::FontId::proportional(13.0),
                Color32::from_rgb(135, 143, 160),
            );
        }
        if self.current_tool == Tool::Select {
            if let (Some(mouse_pos), Some(name)) = (response.hover_pos(), &self.current_glyph) {
                if let Some(glyph) = self.project.glyphs.get(name) {
                    let anchor_hit = self.canvas.show_anchors
                        && (glyph
                            .layers
                            .get(&self.current_master_id)
                            .map(|layer| {
                                layer.anchors.iter().any(|anchor| {
                                    self.canvas
                                        .glyph_to_screen(anchor.x, anchor.y, origin)
                                        .distance(mouse_pos)
                                        <= 9.0
                                })
                            })
                            .unwrap_or(false)
                            || glyph.anchors.iter().any(|anchor| {
                                self.canvas
                                    .glyph_to_screen(anchor.x, anchor.y, origin)
                                    .distance(mouse_pos)
                                    <= 9.0
                            }));
                    let advance_x = self.canvas.glyph_to_screen(glyph.width, 0.0, origin).x;
                    let bearing_edge = self
                        .project
                        .outline_bounds_for_glyph(name)
                        .map(|(min_x, _, max_x, _)| {
                            let left_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                            let right_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
                            (mouse_pos.x - left_x).abs() <= 8.0
                                || (mouse_pos.x - right_x).abs() <= 8.0
                        })
                        .unwrap_or(false);
                    if anchor_hit {
                        ctx.set_cursor_icon(egui::CursorIcon::Move);
                    } else if (mouse_pos.x - advance_x).abs() <= 8.0 || bearing_edge {
                        ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }
            }
        }
        if let Some(pointer) = response.hover_pos() {
            let (x, y) = self.canvas.screen_to_glyph(pointer, origin);
            painter.text(
                rect.right_bottom() - Vec2::new(12.0, 10.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("X {:>7.1}  Y {:>7.1}", x, y),
                egui::FontId::monospace(11.0),
                Color32::from_gray(170),
            );
        }

        self.canvas.draw_grid(&painter, rect, origin);

        self.canvas.draw_metrics(
            &painter,
            rect,
            origin,
            self.project.metadata.units_per_em,
            self.project.metadata.ascender,
            self.project.metadata.descender,
            self.current_glyph
                .as_ref()
                .and_then(|name| self.project.glyphs.get(name))
                .map(|glyph| glyph.width)
                .unwrap_or(self.project.metadata.units_per_em),
        );
        if self.show_side_glyphs {
            if let Some(current_name) = self.current_glyph.as_deref() {
                let names = self.project.glyph_names_sorted();
                if let Some(current_index) = names.iter().position(|name| *name == current_name) {
                    let layer_width = |name: &str| {
                        self.project
                            .glyphs
                            .get(name)
                            .and_then(|glyph| {
                                glyph
                                    .layers
                                    .get(&self.current_master_id)
                                    .map(|layer| layer.width)
                                    .or(Some(glyph.width))
                            })
                            .unwrap_or(self.project.metadata.units_per_em)
                    };
                    let side_color = Color32::from_rgba_premultiplied(130, 155, 185, 72);
                    if current_index > 0 {
                        let previous = names[current_index - 1];
                        let previous_origin = Pos2::new(
                            origin.x
                                - ((layer_width(previous)
                                    + self
                                        .project
                                        .kerning_for_glyphs(previous, current_name)
                                        .unwrap_or(0.0)) as f32
                                    * self.canvas.zoom),
                            origin.y,
                        );
                        self.canvas.draw_master_overlay(
                            &painter,
                            &self.project,
                            previous,
                            &self.current_master_id,
                            previous_origin,
                            side_color,
                        );
                        painter.text(
                            Pos2::new(previous_origin.x, rect.bottom() - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("‹ {previous}"),
                            egui::FontId::monospace(10.0),
                            Color32::from_rgba_premultiplied(170, 190, 215, 150),
                        );
                    }
                    if let Some(next) = names.get(current_index + 1) {
                        let next_origin = Pos2::new(
                            origin.x
                                + ((layer_width(current_name)
                                    + self
                                        .project
                                        .kerning_for_glyphs(current_name, next)
                                        .unwrap_or(0.0)) as f32
                                    * self.canvas.zoom),
                            origin.y,
                        );
                        self.canvas.draw_master_overlay(
                            &painter,
                            &self.project,
                            next,
                            &self.current_master_id,
                            next_origin,
                            side_color,
                        );
                        painter.text(
                            Pos2::new(next_origin.x, rect.bottom() - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{next} ›"),
                            egui::FontId::monospace(10.0),
                            Color32::from_rgba_premultiplied(170, 190, 215, 150),
                        );
                    }
                }
            }
        }
        if let Some(name) = &self.current_glyph {
            if let Some((min_x, _, max_x, _)) = self.project.outline_bounds_for_glyph(name) {
                let lsb_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                let rsb_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
                let bearing_stroke =
                    Stroke::new(0.8_f32, Color32::from_rgba_premultiplied(255, 180, 80, 130));
                for x in [lsb_x, rsb_x] {
                    painter.line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        bearing_stroke,
                    );
                }
                let width = self
                    .project
                    .glyphs
                    .get(name)
                    .map(|glyph| glyph.width)
                    .unwrap_or_default();
                painter.text(
                    Pos2::new(lsb_x + 4.0, rect.bottom() - 22.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("LSB {:.0}", min_x),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(255, 190, 100),
                );
                painter.text(
                    Pos2::new(rsb_x - 4.0, rect.bottom() - 22.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("RSB {:.0}", width - max_x),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(255, 190, 100),
                );
            }
        }
        if self.canvas.show_guidelines {
            self.canvas.draw_guidelines(
                &painter,
                self.project.guidelines_for_master(&self.current_master_id),
                rect,
                origin,
            );
            if let Some(GuidelineTarget::Global(index)) = self.selected_guideline {
                if let Some(guide) = self
                    .project
                    .guidelines_for_master(&self.current_master_id)
                    .get(index)
                {
                    self.canvas
                        .draw_guideline_highlight(&painter, guide, rect, origin);
                }
            }
        }

        if let Some(name) = &self.current_glyph {
            if self.canvas.show_background_images {
                if let Some(path) = self
                    .project
                    .background_images
                    .get(name)
                    .and_then(|masters| masters.get(&self.current_master_id))
                    .filter(|path| !path.trim().is_empty())
                    .cloned()
                {
                    let modified = std::fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .ok();
                    let cache_is_current = self
                        .background_cache
                        .get(&path)
                        .is_some_and(|(cached_modified, _)| *cached_modified == modified);
                    if !cache_is_current {
                        if let Ok(bytes) = std::fs::read(&path) {
                            let is_svg = std::path::Path::new(&path)
                                .extension()
                                .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
                            let image = if is_svg {
                                egui_extras::RetainedImage::from_svg_bytes(
                                    format!("background:{path}"),
                                    &bytes,
                                )
                            } else {
                                egui_extras::RetainedImage::from_image_bytes(
                                    format!("background:{path}"),
                                    &bytes,
                                )
                            };
                            if let Ok(image) = image {
                                self.background_cache
                                    .insert(path.clone(), (modified, image));
                            }
                        }
                    }
                    if let Some((_, image)) = self.background_cache.get(&path) {
                        let opacity = self
                            .project
                            .background_opacities
                            .get(name)
                            .and_then(|masters| masters.get(&self.current_master_id))
                            .copied()
                            .unwrap_or(0.35);
                        let transform = self
                            .project
                            .background_transforms
                            .get(name)
                            .and_then(|masters| masters.get(&self.current_master_id))
                            .copied()
                            .unwrap_or(crate::font_data::BackgroundImageTransform {
                                x: 0.0,
                                y: 0.0,
                                scale: 1.0,
                                rotation: 0.0,
                                flip_x: false,
                                flip_y: false,
                            });
                        self.canvas.draw_background_image(
                            &painter,
                            image.texture_id(ctx),
                            image.size(),
                            origin,
                            opacity,
                            transform,
                        );
                    }
                }
            }
            if self.canvas.show_guidelines {
                if let Some(glyph) = self.project.glyphs.get(name) {
                    self.canvas.draw_guidelines(
                        &painter,
                        glyph.guidelines_for_master(&self.current_master_id),
                        rect,
                        origin,
                    );
                    if let Some(GuidelineTarget::Glyph(index)) = self.selected_guideline {
                        if let Some(guide) = glyph
                            .guidelines_for_master(&self.current_master_id)
                            .get(index)
                        {
                            self.canvas
                                .draw_guideline_highlight(&painter, guide, rect, origin);
                        }
                    }
                }
            }
            if self.show_interpolation_overlay && self.project.masters.len() >= 2 {
                if let Some(glyph) = self.project.glyphs.get(name) {
                    let mut drew_bilinear = false;
                    let mut axis_tags = std::collections::BTreeSet::new();
                    for master in &self.project.masters {
                        axis_tags.extend(master.axes.keys().cloned());
                    }
                    let axis_tags: Vec<String> = axis_tags.into_iter().collect();
                    if axis_tags.len() >= 2 {
                        let axis_bounds = |tag: &str| {
                            let values: Vec<f64> = self
                                .project
                                .masters
                                .iter()
                                .filter_map(|master| master.axes.get(tag).copied())
                                .collect();
                            values
                                .iter()
                                .copied()
                                .fold(None::<(f64, f64)>, |bounds, value| {
                                    Some(match bounds {
                                        Some((min, max)) => (min.min(value), max.max(value)),
                                        None => (value, value),
                                    })
                                })
                        };
                        let x_bounds = axis_bounds(&axis_tags[0]);
                        let y_bounds = axis_bounds(&axis_tags[1]);
                        let target_x = x_bounds.map_or(0.0, |(min, max)| {
                            min + (max - min) * self.interpolation_x_factor as f64
                        });
                        let target_y = y_bounds.map_or(0.0, |(min, max)| {
                            min + (max - min) * self.interpolation_y_factor as f64
                        });
                        if let Some(layer) = self.project.interpolate_glyph_bilinear(
                            name,
                            &axis_tags[0],
                            &axis_tags[1],
                            target_x,
                            target_y,
                        ) {
                            self.canvas.draw_layer(
                                &painter,
                                &layer,
                                origin,
                                Color32::from_rgba_premultiplied(120, 180, 255, 90),
                            );
                            if self.canvas.show_anchors {
                                self.canvas.draw_anchors(&painter, &layer.anchors, origin);
                            }
                            drew_bilinear = true;
                        }
                    }
                    if !drew_bilinear {
                        let first = if self
                            .project
                            .masters
                            .iter()
                            .any(|master| master.id == self.interpolation_from_master)
                        {
                            &self.interpolation_from_master
                        } else {
                            &self.project.masters[0].id
                        };
                        let last = if self
                            .project
                            .masters
                            .iter()
                            .any(|master| master.id == self.interpolation_to_master)
                        {
                            &self.interpolation_to_master
                        } else {
                            &self.project.masters[self.project.masters.len() - 1].id
                        };
                        if let (Some(a), Some(b)) =
                            (glyph.layers.get(first), glyph.layers.get(last))
                        {
                            if let Some(layer) = a.interpolate(b, self.interpolation_factor as f64)
                            {
                                self.canvas.draw_layer(
                                    &painter,
                                    &layer,
                                    origin,
                                    Color32::from_rgba_premultiplied(120, 180, 255, 90),
                                );
                                if self.canvas.show_anchors {
                                    self.canvas.draw_anchors(&painter, &layer.anchors, origin);
                                }
                            }
                        }
                    }
                }
            }
            if self.show_all_masters_overlay {
                if let Some(glyph) = self.project.glyphs.get(name) {
                    for (index, master) in self.project.masters.iter().enumerate() {
                        if master.id == self.current_master_id {
                            continue;
                        }
                        if glyph.layers.contains_key(&master.id) {
                            let colors = [
                                Color32::from_rgba_premultiplied(255, 130, 130, 75),
                                Color32::from_rgba_premultiplied(130, 210, 255, 75),
                                Color32::from_rgba_premultiplied(180, 140, 255, 75),
                            ];
                            self.canvas.draw_master_overlay(
                                &painter,
                                &self.project,
                                name,
                                &master.id,
                                origin,
                                colors[index % colors.len()],
                            );
                        }
                    }
                }
            }
            let mut axis_values = self
                .project
                .masters
                .iter()
                .find(|master| master.id == self.current_master_id)
                .map(|master| master.axes.clone())
                .unwrap_or_default();
            if let Some(master) = self
                .project
                .masters
                .iter()
                .find(|master| master.id == self.current_master_id)
            {
                axis_values.entry("wght".into()).or_insert(master.weight);
                axis_values.entry("wdth".into()).or_insert(master.width);
            }
            if let Some(layer) = self.project.conditional_layer_for_glyph(name, &axis_values) {
                self.canvas.draw_conditional_layer(
                    &painter,
                    &self.project,
                    &layer.layer,
                    &self.current_master_id,
                    origin,
                    Color32::from_rgb(220, 180, 90),
                );
                if self.canvas.show_anchors {
                    self.canvas
                        .draw_anchors(&painter, &layer.layer.anchors, origin);
                }
            } else if !self.canvas.draw_color_glyph(
                &painter,
                &self.project,
                name,
                &self.current_master_id,
                self.preview_color_palette,
                origin,
            ) {
                self.canvas
                    .draw_glyph_recursive(&painter, &self.project, name, origin);
                if self.canvas.show_contour_direction {
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        self.canvas
                            .draw_contour_directions(&painter, &glyph.contours, origin);
                    }
                }
            }
            if self.canvas.show_anchors {
                let anchors = self.project.anchors_for_glyph(name);
                self.canvas.draw_anchors(&painter, &anchors, origin);
            }
            let selected_components = self.selected_component_indices();
            if let Some(glyph) = self.project.glyphs.get(name) {
                for component in selected_components {
                    self.canvas.draw_component_selection(
                        &painter,
                        &self.project,
                        glyph,
                        component,
                        origin,
                        !self.edit_all_masters,
                    );
                }
            }
        }

        self.canvas
            .draw_pen_preview(&painter, &self.pen_state, origin);
        self.canvas.draw_selection(&painter);
        self.canvas.draw_ruler(&painter, origin);

        (response, painter, rect, origin)
    }
}
