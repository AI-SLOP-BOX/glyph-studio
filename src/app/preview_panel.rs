use super::*;

impl GlyphStudioApp {
    pub(crate) fn preview_panel(&mut self, ctx: &egui::Context) {
        if self.show_preview {
            egui::TopBottomPanel::bottom("preview_panel")
                .default_height(180.0)
                .resizable(true)
                .height_range(140.0..=360.0)
                .show(ctx, |ui| {
                    let mut preview_feature_tags = vec![
                        "liga", "kern", "mark", "mkmk", "calt", "rvrn", "ccmp", "locl", "rlig",
                        "salt", "frac", "sups", "subs", "vert", "ss01",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                    for (tag, _) in
                        crate::core::extract_feature_blocks(&self.project.feature_source())
                    {
                        let tag = String::from_utf8_lossy(&tag.to_be_bytes()).to_string();
                        if !preview_feature_tags.iter().any(|item| item == &tag) {
                            preview_feature_tags.push(tag);
                        }
                    }
                    preview_feature_tags.sort();
                    // Keep the control strip usable when the side panels leave
                    // only a narrow preview width. Glyphs-style workflows
                    // need feature toggles and spacing controls to remain
                    // discoverable instead of disappearing off-screen.
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("プレビュー");
                        if self.show_interpolation_overlay {
                            let from_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.interpolation_from_master)
                                .map(|master| master.name.as_str())
                                .unwrap_or("始点");
                            let to_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.interpolation_to_master)
                                .map(|master| master.name.as_str())
                                .unwrap_or("終点");
                            ui.label(
                                egui::RichText::new(format!(
                                    "比較: {from_name} → {to_name} ({:.0}%)",
                                    self.interpolation_factor * 100.0
                                ))
                                .small()
                                .color(Color32::LIGHT_BLUE),
                            );
                        }
                        ui.add(
                            egui::TextEdit::multiline(&mut self.preview_text)
                                .desired_width(260.0)
                                .desired_rows(2)
                                .hint_text("テキストを入力（改行対応）"),
                        );
                        ui.label("機能:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.preview_features)
                                .desired_width(150.0)
                                .hint_text("liga,salt,kern"),
                        );
                        if ui
                            .small_button("標準")
                            .on_hover_text("liga・kern・markを有効化")
                            .clicked()
                        {
                            self.preview_features = "liga,kern,mark".to_string();
                        }
                        if ui
                            .small_button("全OFF")
                            .on_hover_text("OpenType機能をすべて無効化")
                            .clicked()
                        {
                            self.preview_features.clear();
                        }
                        for tag in &preview_feature_tags {
                            let enabled = preview_feature_enabled(&self.preview_features, tag);
                            if ui
                                .selectable_label(enabled, tag)
                                .on_hover_text(format!("{tag} のON/OFF"))
                                .clicked()
                            {
                                toggle_preview_feature(&mut self.preview_features, tag);
                            }
                        }
                        ui.add(
                            egui::Slider::new(&mut self.preview_scale, 0.015..=0.12).text("サイズ"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.preview_line_spacing, 0.5..=2.5)
                                .text("行間"),
                        );
                        ui.checkbox(&mut self.preview_vertical_metrics, "縦メトリクス");
                        ui.checkbox(&mut self.preview_dark_background, "暗い背景");
                        ui.label("基準");
                        egui::ComboBox::from_id_salt("spacing_reference")
                            .selected_text(self.spacing_reference.to_string())
                            .show_ui(ui, |ui| {
                                for reference in ['H', 'O', 'n', 'o'] {
                                    ui.selectable_value(
                                        &mut self.spacing_reference,
                                        reference,
                                        reference.to_string(),
                                    );
                                }
                            });
                        for sample in ["HH", "HO", "nn", "oo"] {
                            if ui.small_button(sample).clicked() {
                                self.show_preview = true;
                                self.preview_text = sample.to_string();
                            }
                        }
                        if ui
                            .small_button("左右確認")
                            .on_hover_text("現在グリフを基準字形で挟んでスペーシング確認")
                            .clicked()
                        {
                            self.show_preview = true;
                            let current = self
                                .current_glyph
                                .as_deref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| glyph.unicode)
                                .and_then(char::from_u32)
                                .unwrap_or('□');
                            self.preview_text = format!(
                                "{}{current}{}",
                                self.spacing_reference, self.spacing_reference
                            );
                        }
                    });
                    let preview_background = if self.preview_dark_background {
                        Color32::from_rgb(25, 27, 31)
                    } else {
                        Color32::from_rgb(245, 246, 248)
                    };
                    ui.painter().rect_filled(
                        ui.available_rect_before_wrap(),
                        0.0,
                        preview_background,
                    );
                    let mut preview_clicked: Option<String> = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for preview_line in self.preview_text.split('\n') {
                                    if preview_line.is_empty() {
                                        let blank_line_height = (self.project.metadata.units_per_em
                                            as f32
                                            * self.preview_scale
                                            * self.preview_line_spacing)
                                            .clamp(50.0, 320.0);
                                        ui.allocate_space(Vec2::new(
                                            ui.available_width().max(1.0),
                                            blank_line_height,
                                        ));
                                        continue;
                                    }
                                    egui::ScrollArea::horizontal().show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let mut previous_name: Option<String> = None;
                                            let mut previous_origin: Option<Pos2> = None;
                                            let kern_enabled = preview_feature_enabled(
                                                &self.preview_features,
                                                "kern",
                                            );
                                            let names = preview_glyph_names(
                                                &self.project,
                                                preview_line,
                                                &self.preview_features,
                                            );
                                            for name in names {
                                                let mut drawn_origin = None;
                                                if kern_enabled {
                                                    if let Some(previous) = &previous_name {
                                                        if let Some(kern) = self
                                                            .project
                                                            .kerning_for_glyphs(previous, &name)
                                                        {
                                                            ui.add_space(
                                                                (kern as f32 * 0.04)
                                                                    .clamp(-40.0, 80.0),
                                                            );
                                                        }
                                                    }
                                                }
                                                if let Some(glyph) = self.project.glyphs.get(&name)
                                                {
                                                    let multi_axis_layer =
                                                        self.multi_axis_preview_layer(&name);
                                                    let fallback_layer = if self
                                                        .show_interpolation_overlay
                                                        && self.project.masters.len() >= 2
                                                    {
                                                        let from = self
                                                            .project
                                                            .masters
                                                            .iter()
                                                            .find(|master| {
                                                                master.id
                                                                    == self
                                                                        .interpolation_from_master
                                                            })
                                                            .map(|master| &master.id)
                                                            .unwrap_or(&self.project.masters[0].id);
                                                        let to = self
                                                            .project
                                                            .masters
                                                            .iter()
                                                            .find(|master| {
                                                                master.id
                                                                    == self.interpolation_to_master
                                                            })
                                                            .map(|master| &master.id)
                                                            .unwrap_or(
                                                                &self.project.masters[self
                                                                    .project
                                                                    .masters
                                                                    .len()
                                                                    - 1]
                                                                .id,
                                                            );
                                                        glyph.layers.get(from).and_then(|a| {
                                                            glyph.layers.get(to).and_then(|b| {
                                                                a.interpolate(
                                                                    b,
                                                                    self.interpolation_factor
                                                                        as f64,
                                                                )
                                                            })
                                                        })
                                                    } else {
                                                        glyph
                                                            .layers
                                                            .get(&self.current_master_id)
                                                            .cloned()
                                                    };
                                                    let interpolated_layer =
                                                        multi_axis_layer.or(fallback_layer);
                                                    let preview_layer = interpolated_layer.as_ref();
                                                    let contours = preview_layer
                                                        .map(|layer| &layer.contours)
                                                        .unwrap_or(&glyph.contours);
                                                    let components = preview_layer
                                                        .map(|layer| &layer.components)
                                                        .unwrap_or(&glyph.components);
                                                    let preview_width = preview_layer
                                                        .map(|layer| layer.width)
                                                        .unwrap_or(glyph.width);
                                                    let scale = self.preview_scale;
                                                    let cell_width = (preview_width as f32 * scale
                                                        + 8.0)
                                                        .clamp(20.0, 120.0);
                                                    let line_height =
                                                        (self.project.metadata.units_per_em as f32
                                                            * scale
                                                            * self.preview_line_spacing)
                                                            .clamp(50.0, 320.0);
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        Vec2::new(cell_width, line_height),
                                                        egui::Sense::click(),
                                                    );
                                                    let unicode = glyph
                                                        .unicode
                                                        .map(|value| format!("U+{value:04X}"))
                                                        .unwrap_or_else(|| {
                                                            "Unicode未設定".to_string()
                                                        });
                                                    let response = response
                                                        .on_hover_cursor(
                                                            egui::CursorIcon::PointingHand,
                                                        )
                                                        .on_hover_text(format!(
                                                            "{name} · {unicode}\nクリックで編集"
                                                        ));
                                                    if response.clicked() {
                                                        preview_clicked = Some(name.clone());
                                                    }
                                                    let painter = ui.painter();
                                                    let cell_color = if response.hovered() {
                                                        Color32::from_rgb(58, 68, 82)
                                                    } else if self.current_glyph.as_deref()
                                                        == Some(name.as_str())
                                                    {
                                                        Color32::from_rgb(48, 55, 68)
                                                    } else {
                                                        Color32::from_rgb(40, 40, 45)
                                                    };
                                                    painter.rect_filled(rect, 0.0, cell_color);
                                                    let baseline = rect.center().y + 100.0 * scale;
                                                    painter.line_segment(
                                                        [
                                                            Pos2::new(rect.left(), baseline),
                                                            Pos2::new(rect.right(), baseline),
                                                        ],
                                                        Stroke::new(
                                                            1.0_f32,
                                                            Color32::from_rgb(75, 105, 125),
                                                        ),
                                                    );
                                                    for metric in [
                                                        self.project.metadata.ascender,
                                                        self.project.metadata.descender,
                                                    ] {
                                                        let y = baseline - metric as f32 * scale;
                                                        painter.line_segment(
                                                            [
                                                                Pos2::new(rect.left(), y),
                                                                Pos2::new(rect.right(), y),
                                                            ],
                                                            Stroke::new(
                                                                1.0_f32,
                                                                Color32::from_rgb(65, 80, 90),
                                                            ),
                                                        );
                                                    }
                                                    if self.preview_vertical_metrics {
                                                        let vertical = self
                                                            .project
                                                            .vertical_metrics_for_glyph_in_master(
                                                                &name,
                                                                &self.current_master_id,
                                                            );
                                                        let vertical_origin_y = baseline
                                                            - vertical.top_side_bearing as f32
                                                                * scale;
                                                        let vertical_end_y = vertical_origin_y
                                                            - vertical.advance_height as f32
                                                                * scale;
                                                        let x = rect.right() - 8.0;
                                                        let metric_color =
                                                            Color32::from_rgb(90, 190, 205);
                                                        painter.line_segment(
                                                            [
                                                                Pos2::new(x, vertical_origin_y),
                                                                Pos2::new(x, vertical_end_y),
                                                            ],
                                                            Stroke::new(1.0_f32, metric_color),
                                                        );
                                                        painter.circle_filled(
                                                            Pos2::new(x, vertical_origin_y),
                                                            2.5,
                                                            metric_color,
                                                        );
                                                        painter.text(
                                                            Pos2::new(
                                                                rect.right() - 4.0,
                                                                vertical_end_y,
                                                            ),
                                                            egui::Align2::RIGHT_BOTTOM,
                                                            format!(
                                                                "v {}",
                                                                vertical.advance_height.round()
                                                            ),
                                                            egui::FontId::monospace(9.0),
                                                            metric_color,
                                                        );
                                                    }

                                                    let mut origin =
                                                        Pos2::new(rect.center().x, baseline);
                                                    if let Some(previous) = &previous_name {
                                                        if let Some(previous_origin) =
                                                            previous_origin
                                                        {
                                                            if let Some((dx, dy)) =
                                                                preview_mark_attachment(
                                                                    &self.project,
                                                                    previous,
                                                                    &name,
                                                                )
                                                            {
                                                                origin = Pos2::new(
                                                                    previous_origin.x + dx * scale,
                                                                    previous_origin.y - dy * scale,
                                                                );
                                                            }
                                                        }
                                                    }
                                                    drawn_origin = Some(origin);
                                                    for contour in contours {
                                                        let points = preview_contour_points(
                                                            contour, origin, scale,
                                                        );
                                                        if points.len() >= 3 {
                                                            painter.add(
                                                                egui::Shape::convex_polygon(
                                                                    points,
                                                                    Color32::WHITE,
                                                                    Stroke::NONE,
                                                                ),
                                                            );
                                                        }
                                                    }
                                                    for component in components {
                                                        let mut polygons = Vec::new();
                                                        preview_nested_component_polygons(
                                                            &self.project,
                                                            &component.base,
                                                            origin,
                                                            scale,
                                                            component_transform(component),
                                                            &mut std::collections::HashSet::new(),
                                                            &mut polygons,
                                                        );
                                                        for points in polygons {
                                                            if points.len() >= 3 {
                                                                painter.add(
                                                                    egui::Shape::convex_polygon(
                                                                        points,
                                                                        Color32::from_gray(190),
                                                                        Stroke::NONE,
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    let line_height =
                                                        (self.project.metadata.units_per_em as f32
                                                            * self.preview_scale
                                                            * self.preview_line_spacing)
                                                            .clamp(50.0, 320.0);
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        Vec2::new(50.0, line_height),
                                                        egui::Sense::click(),
                                                    );
                                                    let response = response
                                                        .on_hover_cursor(
                                                            egui::CursorIcon::PointingHand,
                                                        )
                                                        .on_hover_text(format!(
                                                            "{name}\nクリックで編集"
                                                        ));
                                                    if response.clicked() {
                                                        preview_clicked = Some(name.clone());
                                                    }
                                                    let painter = ui.painter();
                                                    let border_color = if response.hovered() {
                                                        Color32::from_rgb(230, 130, 110)
                                                    } else {
                                                        Color32::from_rgb(200, 90, 80)
                                                    };
                                                    painter.rect_stroke(
                                                        rect,
                                                        0.0,
                                                        Stroke::new(1.0_f32, border_color),
                                                        egui::StrokeKind::Outside,
                                                    );
                                                    painter.text(
                                                        rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        "?",
                                                        egui::FontId::proportional(24.0),
                                                        Color32::from_rgb(230, 120, 100),
                                                    );
                                                }
                                                previous_origin = drawn_origin;
                                                previous_name = Some(name);
                                            }
                                        });
                                    });
                                }
                            });
                        });
                    if let Some(name) = preview_clicked {
                        if self.current_glyph.as_deref() != Some(name.as_str()) {
                            self.current_glyph = Some(name.clone());
                            self.glyph_rename_input = name;
                            self.clear_canvas_selection();
                        }
                    }
                });
        }
    }
}
