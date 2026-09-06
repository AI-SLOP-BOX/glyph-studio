use super::*;

impl GlyphStudioApp {
    pub(super) fn show_status_bar(&mut self, ctx: &egui::Context) {
        let mut save_requested = false;
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.set_min_height(26.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_message).small());
                if !self.canvas.selected_nodes.is_empty() {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{}ノード選択",
                            self.canvas.selected_nodes.len()
                        ))
                        .small()
                        .color(Color32::LIGHT_BLUE),
                    );
                }
                if self.edit_all_masters {
                    ui.separator();
                    ui.label(
                        egui::RichText::new("⚠ 全マスター編集 ON")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(245, 183, 77)),
                    )
                    .on_hover_text("ノードとコンポーネントのドラッグが全マスターへ反映されます");
                }
                let dirty = self
                    .saved_project
                    .as_ref()
                    .is_none_or(|saved| saved != &self.project);
                if dirty
                    && ui
                        .small_button("保存")
                        .on_hover_text("現在のプロジェクトを保存（⌘S）")
                        .clicked()
                {
                    save_requested = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("⌘Z Undo  ·  ⌘⇧Z Redo")
                            .small()
                            .color(Color32::GRAY),
                    );
                    ui.label(format!("ズーム: {:.0}%", self.canvas.zoom * 100.0));
                    let master_name = self
                        .project
                        .masters
                        .iter()
                        .find(|master| master.id == self.current_master_id)
                        .map(|master| master.name.as_str())
                        .unwrap_or(self.current_master_id.as_str());
                    ui.label(format!("マスター: {master_name}"));
                    if let Some(name) = &self.current_glyph {
                        ui.label(format!("グリフ: {}", name));
                    }
                });
            });
        });
        if save_requested {
            self.save_project_file();
        }
    }

    pub(super) fn show_glyph_canvas(&mut self, ctx: &egui::Context) {
        if self.canvas.selected_component.is_none() {
            self.canvas.selected_components.clear();
        }
        let referenced_backgrounds: HashSet<String> = self
            .project
            .background_images
            .values()
            .flat_map(|masters| masters.values())
            .filter(|path| !path.trim().is_empty())
            .cloned()
            .collect();
        self.background_cache
            .retain(|path, _| referenced_backgrounds.contains(path));
        let panel_frame = egui::Frame::default()
            .fill(Color32::from_rgb(30, 30, 35))
            .inner_margin(0.0);
        let mut fit_view_requested = false;
        let mut width_changed = false;
        let mut bearing_request: Option<(String, f64, f64)> = None;

        egui::CentralPanel::default()
        .frame(panel_frame)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgb(36, 37, 44))
                .inner_margin(egui::Margin::symmetric(12, 7))
                .show(ui, |ui| {
                    let previous_master = self.current_master_id.clone();
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .small_button("‹")
                            .on_hover_text("前のグリフ (Shift+Tab)")
                            .clicked()
                        {
                            self.select_relative_glyph(-1);
                        }
                        ui.label(
                            egui::RichText::new(
                                self.current_glyph.as_deref().unwrap_or("グリフ未選択"),
                            )
                            .strong(),
                        );
                        if let Some(name) = self.current_glyph.as_deref() {
                            if let Some(glyph) = self.project.glyphs.get(name) {
                                let unicode = glyph
                                    .unicode
                                    .map(|value| format!("U+{value:04X}"))
                                    .unwrap_or_else(|| "Unicode未設定".to_string());
                                ui.label(
                                    egui::RichText::new(unicode)
                                        .small()
                                        .color(Color32::LIGHT_GRAY),
                                );
                                let layer = glyph
                                    .layers
                                    .get(&self.current_master_id)
                                    .or_else(|| glyph.layers.values().next());
                                if let Some(layer) = layer {
                                    let min_x = layer
                                        .contours
                                        .iter()
                                        .flat_map(|contour| contour.points.iter())
                                        .map(|point| point.x)
                                        .fold(f64::INFINITY, f64::min);
                                    let max_x = layer
                                        .contours
                                        .iter()
                                        .flat_map(|contour| contour.points.iter())
                                        .map(|point| point.x)
                                        .fold(f64::NEG_INFINITY, f64::max);
                                    let lsb = if min_x.is_finite() { min_x } else { 0.0 };
                                    let rsb = if max_x.is_finite() {
                                        layer.width - max_x
                                    } else {
                                        layer.width
                                    };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "幅 {:.0}  L {:.0}  R {:.0}",
                                            layer.width, lsb, rsb
                                        ))
                                        .small()
                                        .color(Color32::from_rgb(170, 190, 205)),
                                    )
                                    .on_hover_text("現在のマスターの字幅・左右サイドベアリング");
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}輪郭 · {}部品",
                                            layer.contours.len(),
                                            layer.components.len()
                                        ))
                                        .small()
                                        .color(Color32::from_gray(150)),
                                    );
                                }
                            }
                        }
                        if ui
                            .small_button("›")
                            .on_hover_text("次のグリフ (Tab)")
                            .clicked()
                        {
                            self.select_relative_glyph(1);
                        }
                        ui.separator();
                        let master_name = self
                            .project
                            .masters
                            .iter()
                            .find(|master| master.id == self.current_master_id)
                            .map(|master| master.name.clone())
                            .unwrap_or_else(|| self.current_master_id.clone());
                        let master_ids: Vec<String> =
                            self.project.masters.iter().map(|master| master.id.clone()).collect();
                        let current_master_index = master_ids
                            .iter()
                            .position(|id| id == &self.current_master_id)
                            .unwrap_or(0);
                        if ui
                            .add_enabled(current_master_index > 0, egui::Button::new("‹"))
                            .on_hover_text("前のマスター（⌘↑）")
                            .clicked()
                        {
                            let index = current_master_index.saturating_sub(1);
                            self.current_master_id = master_ids[index].clone();
                        }
                        egui::ComboBox::from_id_salt("canvas_current_master")
                            .selected_text(format!(
                                "マスター {} / {}: {master_name}",
                                current_master_index + 1,
                                master_ids.len()
                            ))
                            .width(118.0)
                            .show_ui(ui, |ui| {
                                for master in &self.project.masters {
                                    ui.selectable_value(
                                        &mut self.current_master_id,
                                        master.id.clone(),
                                        &master.name,
                                    );
                                }
                            });
                        if ui
                            .add_enabled(
                                current_master_index + 1 < master_ids.len(),
                                egui::Button::new("›"),
                            )
                            .on_hover_text("次のマスター（⌘↓）")
                            .clicked()
                        {
                            let index = (current_master_index + 1)
                                .min(master_ids.len().saturating_sub(1));
                            self.current_master_id = master_ids[index].clone();
                        }
                        if self.project.masters.len() >= 2 {
                            if !self.project.masters.iter().any(|master| {
                                master.id == self.interpolation_from_master
                            }) {
                                self.interpolation_from_master =
                                    self.project.masters[0].id.clone();
                            }
                            if !self
                                .project
                                .masters
                                .iter()
                                .any(|master| master.id == self.interpolation_to_master)
                            {
                                self.interpolation_to_master = self
                                    .project
                                    .masters
                                    .last()
                                    .map(|master| master.id.clone())
                                    .unwrap_or_default();
                            }
                            ui.toggle_value(&mut self.show_interpolation_overlay, "比較")
                                .on_hover_text("補間結果のオーバーレイ表示");
                            ui.toggle_value(&mut self.show_all_masters_overlay, "全マスター")
                                .on_hover_text("現在のマスター以外の輪郭を薄く重ねて表示");
                            let all_masters_response = ui
                                .checkbox(&mut self.edit_all_masters, "全マスターへ反映")
                                .on_hover_text(
                                    "ノードとコンポーネントのドラッグを全マスターへ同期",
                                );
                            if all_masters_response.changed() {
                                self.status_message = if self.edit_all_masters {
                                    "全マスター編集をONにしました。ノード移動は全マスターへ反映されます"
                                        .to_string()
                                } else {
                                    "全マスター編集をOFFにしました".to_string()
                                };
                            }
                            if self.show_all_masters_overlay {
                                let overlay_colors = [
                                    Color32::from_rgb(255, 150, 150),
                                    Color32::from_rgb(130, 210, 255),
                                    Color32::from_rgb(180, 140, 255),
                                ];
                                for (index, master) in self.project.masters.iter().enumerate() {
                                    if master.id != self.current_master_id {
                                        ui.colored_label(
                                            overlay_colors[index % overlay_colors.len()],
                                            &master.name,
                                        );
                                    }
                                }
                            }
                            if self.show_interpolation_overlay {
                                egui::ComboBox::from_id_salt("canvas_interpolation_from")
                                    .selected_text(
                                        self.project
                                            .masters
                                            .iter()
                                            .find(|master| {
                                                master.id == self.interpolation_from_master
                                            })
                                            .map(|master| format!("始点 {}", master.name))
                                            .unwrap_or_else(|| "始点".to_string()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for master in &self.project.masters {
                                            ui.add_enabled_ui(
                                                master.id != self.interpolation_to_master,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut self.interpolation_from_master,
                                                        master.id.clone(),
                                                        &master.name,
                                                    );
                                                },
                                            );
                                        }
                                    });
                                egui::ComboBox::from_id_salt("canvas_interpolation_to")
                                    .selected_text(
                                        self.project
                                            .masters
                                            .iter()
                                            .find(|master| {
                                                master.id == self.interpolation_to_master
                                            })
                                            .map(|master| format!("終点 {}", master.name))
                                            .unwrap_or_else(|| "終点".to_string()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for master in &self.project.masters {
                                            ui.add_enabled_ui(
                                                master.id != self.interpolation_from_master,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut self.interpolation_to_master,
                                                        master.id.clone(),
                                                        &master.name,
                                                    );
                                                },
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.interpolation_factor,
                                        0.0..=1.0,
                                    )
                                    .text("補間"),
                                );
                                let mut overlay_axes = std::collections::BTreeSet::new();
                                for master in &self.project.masters {
                                    overlay_axes.extend(master.axes.keys().cloned());
                                }
                                let overlay_axes: Vec<String> = overlay_axes.into_iter().collect();
                                if overlay_axes.len() >= 2 {
                                    ui.small(format!(
                                        "2軸補間: {} × {}",
                                        overlay_axes[0], overlay_axes[1]
                                    ));
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.interpolation_x_factor,
                                            0.0..=1.0,
                                        )
                                        .text(&overlay_axes[0]),
                                    );
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.interpolation_y_factor,
                                            0.0..=1.0,
                                        )
                                        .text(&overlay_axes[1]),
                                    );
                                }
                            }
                        }
                        if let Some(name) = self.current_glyph.clone() {
                            let mut width = self
                                .project
                                .glyphs
                                .get(&name)
                                .map(|glyph| glyph.width)
                                .unwrap_or_default();
                            let width_response = ui
                                .add(
                                    egui::DragValue::new(&mut width)
                                        .prefix("幅 ")
                                        .suffix(" u")
                                        .speed(1.0),
                                )
                                .on_hover_text("現在グリフの字幅（全マスターへ反映）");
                            if width_response.drag_started() {
                                self.width_drag_active = true;
                            }
                            let width_value_changed = width_response.changed();
                            if width_value_changed {
                                self.project.set_width_for_glyphs(
                                    std::slice::from_ref(&name),
                                    width,
                                );
                            }
                            if width_response.drag_stopped() {
                                self.width_drag_active = false;
                                width_changed = true;
                            } else if width_value_changed && !self.width_drag_active {
                                width_changed = true;
                            }
                            if let Some((min_x, _, max_x, _)) =
                                self.project.outline_bounds_for_glyph(&name)
                            {
                                let width = self
                                    .project
                                    .glyphs
                                    .get(&name)
                                    .map(|glyph| glyph.width)
                                    .unwrap_or_default();
                                let mut left = min_x;
                                let mut right = width - max_x;
                                let left_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut left)
                                            .prefix("LSB ")
                                            .speed(1.0),
                                    )
                                    .on_hover_text("左サイドベアリング（全マスター）\nキャンバス上のオレンジ線をドラッグして調整")
                                    .changed();
                                let right_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut right)
                                            .prefix("RSB ")
                                            .speed(1.0),
                                    )
                                    .on_hover_text("右サイドベアリング（全マスター）\nキャンバス上のオレンジ線をドラッグして調整")
                                    .changed();
                                if left_changed || right_changed {
                                    bearing_request = Some((name, left, right));
                                }
                            }
                        }
                        ui.separator();
                        if ui.small_button("−").on_hover_text("ズームアウト").clicked()
                        {
                            self.canvas.zoom = (self.canvas.zoom / 1.15).clamp(0.05, 20.0);
                        }
                        if ui
                            .small_button("100%")
                            .on_hover_text("ズームを100%に戻す")
                            .clicked()
                        {
                            self.canvas.zoom = 1.0;
                        }
                        if ui.small_button("＋").on_hover_text("ズームイン").clicked() {
                            self.canvas.zoom = (self.canvas.zoom * 1.15).clamp(0.05, 20.0);
                        }
                        let mut zoom_percent = self.canvas.zoom * 100.0;
                        if ui
                            .add(
                                egui::DragValue::new(&mut zoom_percent)
                                    .suffix("%")
                                    .range(5.0..=2000.0)
                                    .speed(1.0),
                            )
                            .on_hover_text("ズーム倍率を直接入力")
                            .changed()
                        {
                            self.canvas.zoom = (zoom_percent / 100.0).clamp(0.05, 20.0);
                        }
                        if ui
                            .small_button("中央")
                            .on_hover_text("表示位置を中央に戻す")
                            .clicked()
                        {
                            self.canvas.pan = Vec2::ZERO;
                        }
                        if ui
                            .small_button("全体")
                            .on_hover_text("現在のグリフ全体をキャンバスに収める (F)")
                            .clicked()
                        {
                            fit_view_requested = true;
                        }
                        ui.separator();
                        ui.toggle_value(&mut self.canvas.show_grid, "グリッド")
                            .on_hover_text("グリッド表示の切り替え");
                        ui.toggle_value(&mut self.canvas.snap_to_grid, "吸着")
                            .on_hover_text("グリッドへのスナップ切り替え");
                        ui.toggle_value(&mut self.canvas.snap_to_guidelines, "ガイド吸着")
                            .on_hover_text("水平・垂直ガイドへのスナップ切り替え");
                        ui.toggle_value(&mut self.canvas.snap_to_anchors, "アンカー吸着")
                            .on_hover_text("現在のグリフのアンカーへのスナップ切り替え");
                        ui.toggle_value(&mut self.canvas.show_guidelines, "ガイド")
                            .on_hover_text("ガイド表示の切り替え (G)");
                        ui.toggle_value(
                            &mut self.canvas.show_contour_direction,
                            "輪郭方向",
                        )
                        .on_hover_text("輪郭の進行方向を表示");
                        ui.toggle_value(&mut self.canvas.show_node_indices, "ノード番号")
                            .on_hover_text("ノード番号を表示 (N)");
                        ui.toggle_value(&mut self.canvas.show_anchors, "アンカー")
                            .on_hover_text("アンカー表示の切り替え");
                        ui.toggle_value(&mut self.canvas.show_background_images, "背景")
                            .on_hover_text("背景画像表示の切り替え");
                        ui.toggle_value(&mut self.show_side_glyphs, "前後字形")
                            .on_hover_text("現在のグリフの左右に隣接する字形を薄く表示（スペーシング確認）");
                        let has_canvas_selection = !self.canvas.selected_nodes.is_empty()
                            || !self.canvas.selected_points.is_empty()
                            || self.canvas.selected_component.is_some();
                        if has_canvas_selection
                            && ui
                                .small_button("選択解除")
                                .on_hover_text("キャンバス上の選択をすべて解除")
                                .clicked()
                        {
                            self.clear_canvas_selection();
                        }
                        let selection_label = if self.canvas.selected_components.len() > 1 {
                            format!(
                                "選択: {}部品",
                                self.canvas.selected_components.len()
                            )
                        } else if let Some(component_index) =
                            self.canvas.selected_component
                        {
                            self.current_glyph
                                .as_ref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| glyph.components.get(component_index))
                                .map(|component| format!("選択: 部品 {}", component.base))
                                .unwrap_or_else(|| "選択: 部品".to_string())
                        } else if !self.canvas.selected_nodes.is_empty() {
                            format!("選択: {}ノード", self.canvas.selected_nodes.len())
                        } else if !self.canvas.selected_points.is_empty() {
                            format!("選択: {}ノード", self.canvas.selected_points.len())
                        } else {
                            "選択: なし".to_string()
                        };
                        ui.colored_label(
                            if has_canvas_selection {
                                Color32::from_rgb(255, 210, 80)
                            } else {
                                Color32::from_rgb(150, 155, 170)
                            },
                            selection_label,
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(format!("ツール: {}", self.current_tool.name()));
                            },
                        );
                    });
                    if self.current_master_id != previous_master {
                        self.selected_guideline = None;
                        self.guideline_drag = None;
                        self.project
                            .switch_master(&previous_master, &self.current_master_id);
                        self.save_state();
                        let display_name = self
                            .project
                            .masters
                            .iter()
                            .find(|master| master.id == self.current_master_id)
                            .map(|master| master.name.as_str())
                            .unwrap_or(self.current_master_id.as_str());
                        self.status_message =
                            format!("マスターを{}に切り替えました", display_name);
                    }
                    if width_changed {
                        self.save_state();
                        self.status_message = "字幅を変更しました".to_string();
                    }
                    if let Some((name, left, right)) = bearing_request.take() {
                        if self.project.set_side_bearings(&[name], left, right) > 0 {
                            self.save_state();
                            self.status_message = "サイドベアリングを変更しました".to_string();
                        }
                    }
                });
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
                .is_some_and(|layer| {
                    layer.contours.is_empty() && layer.components.is_empty()
                });
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
                if let (Some(mouse_pos), Some(name)) =
                    (response.hover_pos(), &self.current_glyph)
                {
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
                    if let Some(current_index) =
                        names.iter().position(|name| *name == current_name)
                    {
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
                                            .unwrap_or(0.0))
                                        as f32
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
                                            .unwrap_or(0.0))
                                        as f32
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
                if let Some((min_x, _, max_x, _)) = self.project.outline_bounds_for_glyph(name)
                {
                    let lsb_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                    let rsb_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
                    let bearing_stroke = Stroke::new(
                        0.8_f32,
                        Color32::from_rgba_premultiplied(255, 180, 80, 130),
                    );
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
                self.canvas
                    .draw_guidelines(
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
                                let is_svg =
                                    std::path::Path::new(&path).extension().is_some_and(
                                        |extension| extension.eq_ignore_ascii_case("svg"),
                                    );
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
                                values.iter().copied().fold(
                                    None::<(f64, f64)>,
                                    |bounds, value| {
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
                                if let Some(layer) =
                                    a.interpolate(b, self.interpolation_factor as f64)
                                {
                                    self.canvas.draw_layer(
                                        &painter,
                                        &layer,
                                        origin,
                                        Color32::from_rgba_premultiplied(120, 180, 255, 90),
                                    );
                                    if self.canvas.show_anchors {
                                        self.canvas.draw_anchors(
                                            &painter,
                                            &layer.anchors,
                                            origin,
                                        );
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
                if let Some(layer) =
                    self.project.conditional_layer_for_glyph(name, &axis_values)
                {
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

            // Handle keyboard shortcuts
            let mut toggle_background_requested = false;
            let mut toggle_side_glyphs_requested = false;
            let mut undo_requested = false;
            let mut redo_requested = false;
            let mut delete_requested = false;
            let mut escape_pressed = false;
            let mut select_all_requested = false;
            let mut select_all_glyphs_requested = false;
            let mut toggle_guides_requested = false;
            let mut toggle_contour_direction_requested = false;
            let mut toggle_metrics_requested = false;
            let mut toggle_node_indices_requested = false;
            let mut toggle_all_masters_requested = false;
            let mut node_action_requested: Option<NodeAction> = None;
            let mut nudge: Option<(f64, f64)> = None;
            let mut reset_view_requested = false;
            let mut zoom_delta: Option<(f32, Pos2)> = None;
            let mut new_tool: Option<Tool> = None;
            let wants_keyboard_input = ctx.wants_keyboard_input();

            ctx.input(|i| {
                if !wants_keyboard_input && i.key_pressed(Key::V) {
                    new_tool = Some(Tool::Select);
                }
                if !wants_keyboard_input && i.key_pressed(Key::P) {
                    new_tool = Some(Tool::Pen);
                }
                if !wants_keyboard_input && i.key_pressed(Key::H) {
                    new_tool = Some(Tool::Hand);
                }
                if !wants_keyboard_input && i.key_pressed(Key::K) {
                    new_tool = Some(Tool::Knife);
                }

                if !wants_keyboard_input
                    && i.modifiers.command
                    && i.key_pressed(Key::Z)
                    && !i.modifiers.shift
                {
                    undo_requested = true;
                }
                if !wants_keyboard_input
                    && i.modifiers.command
                    && i.modifiers.shift
                    && i.key_pressed(Key::Z)
                {
                    redo_requested = true;
                }
                if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::Y) {
                    redo_requested = true;
                }

                if !wants_keyboard_input
                    && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
                {
                    delete_requested = true;
                }

                if !wants_keyboard_input && i.key_pressed(Key::Escape) {
                    escape_pressed = true;
                }

                if !wants_keyboard_input
                    && (i.modifiers.command || i.modifiers.ctrl)
                    && i.key_pressed(Key::A)
                {
                    if i.modifiers.shift {
                        select_all_glyphs_requested = true;
                    } else {
                        select_all_requested = true;
                    }
                }
                if !wants_keyboard_input && i.key_pressed(Key::Tab) {
                    let delta = if i.modifiers.shift { -1 } else { 1 };
                    self.select_relative_glyph(delta);
                }
                if !wants_keyboard_input && i.key_pressed(Key::PageUp) {
                    self.select_relative_glyph(-1);
                }
                if !wants_keyboard_input && i.key_pressed(Key::PageDown) {
                    self.select_relative_glyph(1);
                }
                if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::ArrowUp) {
                    self.select_relative_master(-1);
                }
                if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::ArrowDown) {
                    self.select_relative_master(1);
                }
                if !wants_keyboard_input && i.key_pressed(Key::Home) {
                    self.select_edge_glyph(false);
                }
                if !wants_keyboard_input && i.key_pressed(Key::End) {
                    self.select_edge_glyph(true);
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::G)
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    toggle_guides_requested = true;
                }
                if !wants_keyboard_input && i.key_pressed(Key::I) {
                    toggle_background_requested = true;
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::B)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt
                {
                    toggle_side_glyphs_requested = true;
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::D)
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    toggle_contour_direction_requested = true;
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::M)
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    toggle_metrics_requested = true;
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::N)
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    toggle_node_indices_requested = true;
                }
                if !wants_keyboard_input
                    && i.modifiers.command
                    && i.modifiers.shift
                    && i.key_pressed(Key::M)
                {
                    toggle_all_masters_requested = true;
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::S)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt
                {
                    node_action_requested = Some(NodeAction::Smooth);
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::C)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt
                {
                    node_action_requested = Some(NodeAction::Corner);
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::T)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt
                {
                    node_action_requested = Some(NodeAction::ToggleCurve);
                }
                if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::Num0) {
                    reset_view_requested = true;
                }
                if !wants_keyboard_input
                    && i.modifiers.command
                    && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals))
                {
                    zoom_delta = Some((3.0, rect.center()));
                } else if !wants_keyboard_input
                    && i.modifiers.command
                    && i.key_pressed(Key::Minus)
                {
                    zoom_delta = Some((-3.0, rect.center()));
                }
                if !wants_keyboard_input
                    && i.key_pressed(Key::F)
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    fit_view_requested = true;
                }
                let step = if i.modifiers.shift { 10.0 } else { 1.0 };
                if !wants_keyboard_input && i.key_pressed(Key::ArrowLeft) {
                    nudge = Some((-step, 0.0));
                } else if !wants_keyboard_input && i.key_pressed(Key::ArrowRight) {
                    nudge = Some((step, 0.0));
                } else if !wants_keyboard_input && i.key_pressed(Key::ArrowUp) {
                    nudge = Some((0.0, step));
                } else if !wants_keyboard_input && i.key_pressed(Key::ArrowDown) {
                    nudge = Some((0.0, -step));
                }

                for event in &i.events {
                    if let egui::Event::MouseWheel { delta, .. } = event {
                        if let Some(mouse_pos) = response.hover_pos() {
                            zoom_delta = Some((delta.y, mouse_pos));
                        }
                    }
                }
            });

            if let Some(tool) = new_tool {
                self.current_tool = tool;
                if tool != Tool::Pen {
                    self.pen_state.cancel();
                    self.pen_drag_start = None;
                }
                if tool != Tool::Knife {
                    self.knife_first_cut = None;
                }
                self.master_map_drag = None;
            }

            if undo_requested {
                self.undo();
            }
            if redo_requested {
                self.redo();
            }
            if toggle_guides_requested {
                self.canvas.show_guidelines = !self.canvas.show_guidelines;
            }
            if toggle_background_requested {
                self.canvas.show_background_images = !self.canvas.show_background_images;
            }
            if toggle_side_glyphs_requested {
                self.show_side_glyphs = !self.show_side_glyphs;
                self.status_message = if self.show_side_glyphs {
                    "前後字形を表示しました (B)".to_string()
                } else {
                    "前後字形を非表示にしました (B)".to_string()
                };
            }
            if toggle_contour_direction_requested {
                self.canvas.show_contour_direction = !self.canvas.show_contour_direction;
            }
            if toggle_metrics_requested {
                self.canvas.show_metrics = !self.canvas.show_metrics;
            }
            if toggle_node_indices_requested {
                self.canvas.show_node_indices = !self.canvas.show_node_indices;
            }
            if toggle_all_masters_requested {
                if self.project.masters.len() >= 2 {
                    self.edit_all_masters = !self.edit_all_masters;
                    self.status_message = if self.edit_all_masters {
                        "全マスター編集を有効にしました (⌘⇧M)".to_string()
                    } else {
                        "全マスター編集を無効にしました (⌘⇧M)".to_string()
                    };
                } else {
                    self.status_message =
                        "全マスター編集には2つ以上のマスターが必要です".to_string();
                }
            }
            if self.current_tool == Tool::Select {
                if let Some(action) = node_action_requested {
                    self.apply_selected_node_action(action);
                }
            }

            if delete_requested {
                if let Some(target) = self.selected_guideline.take() {
                    let removed = match target {
                        GuidelineTarget::Global(index) => {
                            let removed = (index
                                < self
                                    .project
                                    .guidelines_for_master(&self.current_master_id)
                                    .len())
                                .then(|| {
                                    self.project
                                        .guidelines_for_master_mut(&self.current_master_id)
                                        .remove(index)
                                })
                                .is_some();
                            if removed && self.edit_all_masters {
                                for (master_id, guides) in
                                    &mut self.project.guidelines_by_master
                                {
                                    if master_id != &self.current_master_id
                                        && index < guides.len()
                                    {
                                        guides.remove(index);
                                    }
                                }
                            }
                            removed
                        }
                        GuidelineTarget::Glyph(index) => self
                            .current_glyph
                            .as_ref()
                            .and_then(|name| self.project.glyphs.get_mut(name))
                            .map(|glyph| {
                                let guides = glyph
                                    .guidelines_for_master_mut(&self.current_master_id);
                                let removed =
                                    (index < guides.len()).then(|| guides.remove(index)).is_some();
                                if removed && self.edit_all_masters {
                                    for (master_id, other_guides) in
                                        &mut glyph.master_guidelines
                                    {
                                        if master_id != &self.current_master_id
                                            && index < other_guides.len()
                                        {
                                            other_guides.remove(index);
                                        }
                                    }
                                }
                                removed
                            })
                            .unwrap_or(false),
                    };
                    if removed {
                        self.guideline_drag = None;
                        self.save_state();
                        self.status_message = "ガイドを削除しました".to_string();
                        return;
                    }
                }
            }
            if delete_requested
                && (!self.canvas.selected_points.is_empty()
                || self.canvas.selected_component.is_some())
            {
                if let Some(name) = self.current_glyph.clone() {
                    let mut component_indices = self.selected_component_indices();
                    component_indices.sort_unstable_by(|left, right| right.cmp(left));
                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                        if !component_indices.is_empty() {
                            for component_index in component_indices {
                                if component_index < glyph.components.len() {
                                    glyph.components.remove(component_index);
                                    for layer in glyph.layers.values_mut() {
                                        if component_index < layer.components.len() {
                                            layer.components.remove(component_index);
                                        }
                                    }
                                }
                            }
                            self.clear_canvas_selection();
                            self.save_state();
                            return;
                        }
                        let mut selected = self.canvas.selected_nodes.clone();
                        if selected.is_empty() {
                            if let Some(ci) = self.canvas.selected_contour {
                                selected = self
                                    .canvas
                                    .selected_points
                                    .iter()
                                    .map(|&pi| (ci, pi))
                                    .collect();
                            }
                        }
                        match glyph.remove_nodes_all_layers(&selected) {
                            Ok(()) => {
                                self.clear_geometry_selection();
                                self.save_state();
                            }
                            Err(error) => self.status_message = error,
                        }
                    }
                }
            }

            if escape_pressed {
                self.pen_state.cancel();
                self.pen_drag_start = None;
                self.knife_first_cut = None;
                self.guideline_drag = None;
                self.master_map_drag = None;
                self.clear_geometry_selection();
                self.selected_guideline = None;
            }

            if select_all_glyphs_requested {
                self.selected_glyphs = self
                    .project
                    .glyph_names_sorted()
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                self.clear_geometry_selection();
                self.status_message =
                    format!("{}グリフを選択しました", self.selected_glyphs.len());
            }

            if select_all_requested && self.current_tool == Tool::Select {
                if let Some(name) = &self.current_glyph {
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        self.canvas.selected_component = None;
                        self.canvas.selected_components.clear();
                        self.canvas.selected_nodes = glyph
                            .contours
                            .iter()
                            .enumerate()
                            .flat_map(|(ci, contour)| {
                                (0..contour.points.len()).map(move |pi| (ci, pi))
                            })
                            .collect();
                        self.canvas.selected_contour = glyph.contours.first().map(|_| 0);
                        self.canvas.selected_points = glyph
                            .contours
                            .first()
                            .map(|c| (0..c.points.len()).collect())
                            .unwrap_or_default();
                    }
                }
            }

            if let Some((dx, dy)) = nudge {
                if self.current_tool == Tool::Select
                    && (self.canvas.selected_component.is_some()
                        || !self.canvas.selected_points.is_empty())
                {
                    if let Some(name) = self.current_glyph.clone() {
                        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                            if let Some(component_index) = self.canvas.selected_component {
                                if self.edit_all_masters {
                                    if let Err(error) = glyph.translate_component_all_layers(
                                        component_index,
                                        dx,
                                        dy,
                                    ) {
                                        self.status_message = error;
                                    } else {
                                        self.save_state();
                                    }
                                } else if let Some(component) =
                                    glyph.components.get_mut(component_index)
                                {
                                    component.x_offset += dx;
                                    component.y_offset += dy;
                                    self.save_state();
                                }
                            } else {
                                let nodes: Vec<(usize, usize)> =
                                    if !self.canvas.selected_nodes.is_empty() {
                                        self.canvas.selected_nodes.clone()
                                    } else if let Some(ci) = self.canvas.selected_contour {
                                        self.canvas
                                            .selected_points
                                            .iter()
                                            .map(|&pi| (ci, pi))
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                if self.edit_all_masters && !nodes.is_empty() {
                                    if let Err(error) =
                                        glyph.translate_nodes_all_layers(&nodes, dx, dy)
                                    {
                                        self.status_message = error;
                                    } else {
                                        self.save_state();
                                    }
                                } else if !nodes.is_empty() {
                                    for (ci, contour) in glyph.contours.iter_mut().enumerate() {
                                        let indices: Vec<usize> = nodes
                                            .iter()
                                            .filter_map(|&(selected_ci, pi)| {
                                                (selected_ci == ci).then_some(pi)
                                            })
                                            .collect();
                                        if !indices.is_empty() {
                                            contour.translate_points(&indices, dx, dy);
                                        }
                                    }
                                    self.save_state();
                                }
                            }
                        }
                    }
                }
            }

            if let Some((delta, mouse_pos)) = zoom_delta {
                self.canvas.zoom_at(delta, mouse_pos, rect.center());
            }
            if reset_view_requested {
                self.canvas.zoom = 1.0;
                self.canvas.pan = Vec2::ZERO;
            }
            if fit_view_requested {
                self.fit_current_glyph_to_canvas(rect);
            }

            // Handle mouse interactions
            let space_down = ctx.input(|input| input.key_down(Key::Space));
            if space_down {
                if self.space_previous_tool.is_none() && self.current_tool != Tool::Hand {
                    self.space_previous_tool = Some(self.current_tool);
                    self.current_tool = Tool::Hand;
                }
            } else if let Some(previous_tool) = self.space_previous_tool.take() {
                self.current_tool = previous_tool;
            }
            response.context_menu(|ui| {
                if ui.button("選択を解除").clicked() {
                    self.clear_canvas_selection();
                    ui.close_menu();
                }
                if ui.button("グリフの全ノードを選択").clicked() {
                    if let Some(name) = &self.current_glyph {
                        if let Some(glyph) = self.project.glyphs.get(name) {
                            self.canvas.selected_nodes = glyph
                                .contours
                                .iter()
                                .enumerate()
                                .flat_map(|(ci, contour)| {
                                    (0..contour.points.len()).map(move |pi| (ci, pi))
                                })
                                .collect();
                            self.canvas.selected_contour = glyph.contours.first().map(|_| 0);
                            self.canvas.selected_points = glyph
                                .contours
                                .first()
                                .map(|contour| (0..contour.points.len()).collect())
                                .unwrap_or_default();
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        !self.canvas.selected_nodes.is_empty(),
                        egui::Button::new("スムーズノードにする"),
                    )
                    .clicked()
                {
                    self.apply_selected_node_action(NodeAction::Smooth);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        !self.canvas.selected_nodes.is_empty(),
                        egui::Button::new("コーナーノードにする"),
                    )
                    .clicked()
                {
                    self.apply_selected_node_action(NodeAction::Corner);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("全体表示").clicked() {
                    self.fit_current_glyph_to_canvas(rect);
                    ui.close_menu();
                }
            });

            if let Some(mouse_pos) = response.hover_pos() {
                let (gx, gy) = self.canvas.screen_to_glyph(mouse_pos, origin);
                let (gx, gy) = self.canvas.snap_point(gx, gy);

                let middle_pan = ctx.input(|input| input.pointer.middle_down());
                if middle_pan {
                    if response.dragged() {
                        self.canvas.pan += response.drag_delta();
                    }
                } else {
                    match self.current_tool {
                    Tool::Select => {
                        if response.drag_started() {
                            self.spacing_drag = self
                                .current_glyph
                                .as_ref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| {
                                    let x =
                                        self.canvas.glyph_to_screen(glyph.width, 0.0, origin).x;
                                    ((mouse_pos.x - x).abs() <= 8.0)
                                        .then_some(SpacingTarget::Advance)
                                })
                                .or_else(|| {
                                    self.current_glyph
                                        .as_ref()
                                        .and_then(|name| {
                                            self.project.outline_bounds_for_glyph(name)
                                        })
                                        .and_then(|(min_x, _, max_x, _)| {
                                            let left_x = self
                                                .canvas
                                                .glyph_to_screen(min_x, 0.0, origin)
                                                .x;
                                            let right_x = self
                                                .canvas
                                                .glyph_to_screen(max_x, 0.0, origin)
                                                .x;
                                            if (mouse_pos.x - left_x).abs() <= 8.0 {
                                                Some(SpacingTarget::LeftBearing)
                                            } else if (mouse_pos.x - right_x).abs() <= 8.0 {
                                                Some(SpacingTarget::RightBearing)
                                            } else {
                                                None
                                            }
                                        })
                                });
                            self.anchor_drag = if self.canvas.show_anchors {
                                self.current_glyph.as_ref().and_then(|name| {
                                self.project.glyphs.get(name).and_then(|glyph| {
                                    glyph
                                        .layers
                                        .get(&self.current_master_id)
                                        .and_then(|layer| {
                                            layer
                                                .anchors
                                                .iter()
                                                .enumerate()
                                                .find(|(_, anchor)| {
                                                    self.canvas
                                                        .glyph_to_screen(anchor.x, anchor.y, origin)
                                                        .distance(mouse_pos)
                                                        <= 9.0
                                                })
                                                .map(|(index, _)| AnchorTarget::Layer(index))
                                        })
                                        .or_else(|| {
                                            glyph.anchors.iter().enumerate().find(|(_, anchor)| {
                                                self.canvas
                                                    .glyph_to_screen(anchor.x, anchor.y, origin)
                                                    .distance(mouse_pos)
                                                    <= 9.0
                                            }).map(|(index, _)| AnchorTarget::Glyph(index))
                                        })
                                })
                                })
                            } else {
                                None
                            };
                            let near_guide = |guide: &crate::font_data::Guideline| {
                                let center =
                                    self.canvas.glyph_to_screen(guide.x, guide.y, origin);
                                let direction = Vec2::new(
                                    (-(guide.angle as f32).to_radians()).cos(),
                                    (-(guide.angle as f32).to_radians()).sin(),
                                );
                                let offset = mouse_pos - center;
                                let distance =
                                    (offset.x * direction.y - offset.y * direction.x).abs();
                                distance <= 8.0
                            };
                            self.guideline_drag = self
                                .current_glyph
                                .as_ref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| {
                                    glyph
                                        .guidelines_for_master(&self.current_master_id)
                                        .iter()
                                        .enumerate()
                                        .find(|(_, guide)| near_guide(guide))
                                })
                                .map(|(index, _)| GuidelineTarget::Glyph(index))
                                .or_else(|| {
                                    self.project
                                        .guidelines
                                        .iter()
                                        .enumerate()
                                        .find(|(_, guide)| near_guide(guide))
                                        .map(|(index, _)| GuidelineTarget::Global(index))
                                });
                            if self.guideline_drag.is_some() {
                                self.selected_guideline = self.guideline_drag;
                            }
                            if self.guideline_drag.is_some() || self.anchor_drag.is_some() {
                                self.spacing_drag = None;
                                self.canvas.selection_start = None;
                                self.canvas.selection_rect = None;
                            }
                            if self.spacing_drag.is_some() {
                                self.canvas.selection_start = None;
                                self.canvas.selection_rect = None;
                                self.canvas.selected_component = None;
                            }
                            let hit = if let (Some(name), Some(start)) =
                                (&self.current_glyph, response.interact_pointer_pos())
                            {
                                self.project.glyphs.get(name).and_then(|glyph| {
                                    self.canvas.hit_test(start, glyph, origin)
                                })
                            } else {
                                None
                            };
                            let component_hit = if hit.is_none() {
                                self.current_glyph
                                    .as_ref()
                                    .and_then(|name| self.project.glyphs.get(name))
                                    .and_then(|glyph| {
                                        self.canvas.hit_test_component(
                                            response.interact_pointer_pos()?,
                                            &self.project,
                                            glyph,
                                            origin,
                                        )
                                    })
                            } else {
                                None
                            };
                            let mut component_hit = component_hit;
                            if let Some(component_index) = component_hit {
                                if ctx.input(|input| input.modifiers.alt)
                                    && !self.component_drag_duplicated
                                {
                                    if let Some(name) = self.current_glyph.clone() {
                                        if self.project.duplicate_component_all_layers(
                                            &name,
                                            component_index,
                                        ) {
                                            let new_index = self
                                                .project
                                                .glyphs
                                                .get(&name)
                                                .map(|glyph| glyph.components.len() - 1)
                                                .unwrap_or(component_index);
                                            component_hit = Some(new_index);
                                            self.component_drag_duplicated = true;
                                            self.save_state();
                                            self.status_message =
                                                "部品を複製しました（Optionドラッグ）".to_string();
                                        }
                                    }
                                }
                                {
                                    let handle = self.current_glyph.as_ref().and_then(|name| {
                                        response.interact_pointer_pos().and_then(|position| {
                                            self.project.glyphs.get(name).and_then(|glyph| {
                                                self.canvas.hit_test_component_handle(
                                                    position,
                                                    &self.project,
                                                    glyph,
                                                    component_index,
                                                    origin,
                                                )
                                            })
                                        })
                                    });
                                    if let (Some(name), Some(handle)) = (
                                        self.current_glyph.as_ref(),
                                        handle,
                                    ) {
                                        if let Some(component) = self
                                            .project
                                            .glyphs
                                            .get(name)
                                            .and_then(|glyph| glyph.components.get(component_index))
                                            .cloned()
                                        {
                                            self.component_resize =
                                                Some((component_index, handle, component));
                                        }
                                }
                            }
                            }
                            if self.guideline_drag.is_none() && self.spacing_drag.is_none() {
                                self.canvas.selection_start = (hit.is_none()
                                    && component_hit.is_none()
                                    && self.anchor_drag.is_none())
                                .then(|| response.interact_pointer_pos())
                                .flatten();
                                if let Some(component_index) = component_hit {
                                    self.select_component(
                                        component_index,
                                        ctx.input(|input| input.modifiers.shift),
                                    );
                                } else {
                                    self.canvas.selected_component = None;
                                    self.canvas.selected_components.clear();
                                }
                            }
                            self.canvas.selection_rect = None;
                        }
                        if response.clicked() {
                            if let Some(name) = &self.current_glyph {
                                if let Some(glyph) = self.project.glyphs.get(name) {
                                    if let Some((ci, pi)) =
                                        self.canvas.hit_test(mouse_pos, glyph, origin)
                                    {
                                        self.canvas.selected_component = None;
                                        self.canvas.selected_components.clear();
                                        let additive = ctx.input(|i| i.modifiers.shift);
                                        if additive {
                                            if let Some(pos) = self
                                                .canvas
                                                .selected_nodes
                                                .iter()
                                                .position(|&selected| selected == (ci, pi))
                                            {
                                                self.canvas.selected_nodes.remove(pos);
                                            } else {
                                                self.canvas.selected_nodes.push((ci, pi));
                                            }
                                        } else {
                                            self.canvas.selected_nodes = vec![(ci, pi)];
                                        }
                                        self.canvas.selected_points = self
                                            .canvas
                                            .selected_nodes
                                            .iter()
                                            .filter_map(|&(selected_ci, selected_pi)| {
                                                (selected_ci == ci).then_some(selected_pi)
                                            })
                                            .collect();
                                        self.canvas.selected_contour = Some(ci);
                                    } else {
                                        let component_hit = self
                                            .current_glyph
                                            .as_ref()
                                            .and_then(|name| self.project.glyphs.get(name))
                                            .and_then(|glyph| {
                                                self.canvas.hit_test_component(
                                                    mouse_pos,
                                                    &self.project,
                                                    glyph,
                                                    origin,
                                                )
                                            });
                                        if let Some(component_index) = component_hit {
                                            self.select_component(
                                                component_index,
                                                ctx.input(|input| input.modifiers.shift),
                                            );
                                        } else {
                                            self.canvas.selected_component = None;
                                            self.canvas.selected_components.clear();
                                        }
                                        self.canvas.selected_points.clear();
                                        self.canvas.selected_nodes.clear();
                                        self.canvas.selected_contour = None;
                                    }
                                }
                            }
                        }
                        if response.double_clicked() {
                            if let Some(name) = self.current_glyph.clone() {
                                let hit = self.project.glyphs.get(&name).and_then(|glyph| {
                                    self.canvas.hit_test(mouse_pos, glyph, origin)
                                });
                                let segment_hit = self.project.glyphs.get(&name).and_then(|glyph| {
                                    self.canvas.hit_test_segment(mouse_pos, glyph, origin)
                                });
                                if let Some((ci, pi)) = hit {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let smooth = glyph
                                            .contours
                                            .get(ci)
                                            .and_then(|contour| contour.points.get(pi))
                                            .map(|point| !point.smooth);
                                        if let Some(smooth) = smooth {
                                            match glyph.set_smooth_nodes_all_layers(&[(ci, pi)], smooth) {
                                                Ok(()) => {
                                                    self.canvas.selected_contour = Some(ci);
                                                    self.canvas.selected_points = vec![pi];
                                                    self.canvas.selected_nodes = vec![(ci, pi)];
                                                    self.save_state();
                                                }
                                                Err(error) => {
                                                    self.status_message = error;
                                                }
                                            }
                                        }
                                    }
                                } else if let Some((ci, start_index, factor)) = segment_hit {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.split_segment_all_layers(
                                            ci,
                                            start_index,
                                            factor,
                                        ) {
                                            Ok(inserted_index) => {
                                                self.canvas.selected_component = None;
                                                self.canvas.selected_components.clear();
                                                self.canvas.selected_contour = Some(ci);
                                                self.canvas.selected_points =
                                                    vec![inserted_index];
                                                self.canvas.selected_nodes =
                                                    vec![(ci, inserted_index)];
                                                self.save_state();
                                                self.status_message =
                                                    "曲線上にノードを追加しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                } else if let Some(component_index) = self
                                    .project
                                    .glyphs
                                    .get(&name)
                                    .and_then(|glyph| {
                                        self.canvas.hit_test_component(
                                            mouse_pos,
                                            &self.project,
                                            glyph,
                                            origin,
                                        )
                                    })
                                {
                                    if let Some(base) = self
                                        .project
                                        .glyphs
                                        .get(&name)
                                        .and_then(|glyph| glyph.components.get(component_index))
                                        .map(|component| component.base.clone())
                                    {
                                        self.current_glyph = Some(base.clone());
                                        self.glyph_rename_input = base.clone();
                                        self.clear_canvas_selection();
                                        self.status_message =
                                            format!("参照先グリフを開きました: {base}");
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(target) = self.anchor_drag {
                                if let Some(name) = self.current_glyph.clone() {
                                    let mut anchor_guidelines = self
                                        .project
                                        .guidelines_for_master(&self.current_master_id)
                                        .to_vec();
                                    if let Some(glyph) = self.project.glyphs.get(&name) {
                                        anchor_guidelines.extend(
                                            glyph
                                                .guidelines_for_master(&self.current_master_id)
                                                .iter()
                                                .cloned(),
                                        );
                                    }
                                    let (anchor_x, anchor_y) = self
                                        .canvas
                                        .snap_point_to_guidelines(gx, gy, &anchor_guidelines);
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let anchor = match target {
                                            AnchorTarget::Glyph(index) => glyph.anchors.get_mut(index),
                                            AnchorTarget::Layer(index) => glyph
                                                .layers
                                                .get_mut(&self.current_master_id)
                                                .and_then(|layer| layer.anchors.get_mut(index)),
                                        };
                                        if let Some(anchor) = anchor {
                                            anchor.x = anchor_x;
                                            anchor.y = anchor_y;
                                        }
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(target) = self.guideline_drag {
                                let guide = match target {
                                    GuidelineTarget::Global(index) => {
                                        self.project
                                            .guidelines_for_master_mut(&self.current_master_id)
                                            .get_mut(index)
                                    }
                                    GuidelineTarget::Glyph(index) => self
                                        .current_glyph
                                        .as_ref()
                                        .and_then(|name| self.project.glyphs.get_mut(name))
                                        .and_then(|glyph| {
                                            glyph
                                                .guidelines_for_master_mut(&self.current_master_id)
                                                .get_mut(index)
                                        }),
                                };
                                let mut delta = None;
                                if let Some(guide) = guide {
                                    let before = (guide.x, guide.y, guide.angle);
                                    if ctx.input(|input| input.modifiers.shift) {
                                        let center = self
                                            .canvas
                                            .glyph_to_screen(guide.x, guide.y, origin);
                                        let delta = mouse_pos - center;
                                        if delta.length_sq() > 1.0 {
                                            let mut angle =
                                                -(delta.y.atan2(delta.x).to_degrees() as f64);
                                            if ctx.input(|input| input.modifiers.command) {
                                                angle = (angle / 15.0).round() * 15.0;
                                            }
                                            guide.angle = angle;
                                        }
                                    } else {
                                        guide.x = gx;
                                        guide.y = gy;
                                    }
                                    delta = Some((
                                        guide.x - before.0,
                                        guide.y - before.1,
                                        guide.angle - before.2,
                                    ));
                                }
                                if self.edit_all_masters {
                                    if let Some((dx, dy, dangle)) = delta {
                                        match target {
                                            GuidelineTarget::Global(index) => {
                                                for (master_id, guides) in
                                                    &mut self.project.guidelines_by_master
                                                {
                                                    if master_id != &self.current_master_id {
                                                        if let Some(guide) = guides.get_mut(index) {
                                                            guide.x += dx;
                                                            guide.y += dy;
                                                            guide.angle += dangle;
                                                        }
                                                    }
                                                }
                                            }
                                            GuidelineTarget::Glyph(index) => {
                                                if let Some(name) = self.current_glyph.as_ref() {
                                                    if let Some(glyph) =
                                                        self.project.glyphs.get_mut(name)
                                                    {
                                                        for (master_id, guides) in
                                                            &mut glyph.master_guidelines
                                                        {
                                                            if master_id != &self.current_master_id {
                                                                if let Some(guide) = guides.get_mut(index) {
                                                                    guide.x += dx;
                                                                    guide.y += dy;
                                                                    guide.angle += dangle;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if response.dragged()
                            && self.guideline_drag.is_none()
                            && self.anchor_drag.is_none()
                        {
                            if let Some(target) = self.spacing_drag {
                                if let Some(name) = self.current_glyph.clone() {
                                    match target {
                                        SpacingTarget::Advance => {
                                            self.project
                                                .set_width_for_glyphs(&[name], gx.max(0.0));
                                        }
                                        SpacingTarget::LeftBearing
                                        | SpacingTarget::RightBearing => {
                                            if let Some((min_x, _, max_x, _)) =
                                                self.project.outline_bounds_for_glyph(&name)
                                            {
                                                let width = self
                                                    .project
                                                    .glyphs
                                                    .get(&name)
                                                    .map(|glyph| glyph.width)
                                                    .unwrap_or_default();
                                                let (left, right) = match target {
                                                    SpacingTarget::LeftBearing => {
                                                        (gx.max(0.0), (width - max_x).max(0.0))
                                                    }
                                                    SpacingTarget::RightBearing => (
                                                        (min_x).max(0.0),
                                                        (width - gx).max(0.0),
                                                    ),
                                                    SpacingTarget::Advance => unreachable!(),
                                                };
                                                self.project.set_side_bearings(
                                                    &[name],
                                                    left,
                                                    right,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            if self.spacing_drag.is_some() {
                                // The advance-width handle consumes this drag.
                            } else if let Some(start) = self.canvas.selection_start {
                                let selection = egui::Rect::from_two_pos(start, mouse_pos);
                                self.canvas.selection_rect = Some(selection);
                                if let Some(name) = &self.current_glyph {
                                    if let Some(glyph) = self.project.glyphs.get(name) {
                                        let mut selected_contour = None;
                                        let mut selected_points = Vec::new();
                                        let mut selected_nodes = Vec::new();
                                        for (ci, contour) in glyph.contours.iter().enumerate() {
                                            let inside: Vec<usize> = contour
                                                .points
                                                .iter()
                                                .enumerate()
                                                .filter_map(|(pi, point)| {
                                                    let screen = self.canvas.glyph_to_screen(
                                                        point.x, point.y, origin,
                                                    );
                                                    selection.contains(screen).then_some(pi)
                                                })
                                                .collect();
                                            if !inside.is_empty() {
                                                if selected_contour.is_none() {
                                                    selected_contour = Some(ci);
                                                    selected_points = inside.clone();
                                                }
                                                selected_nodes.extend(
                                                    inside.into_iter().map(|pi| (ci, pi)),
                                                );
                                            }
                                        }
                                        let selected_components: Vec<usize> = glyph
                                            .components
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(component_index, component)| {
                                                let (min_x, min_y, max_x, max_y) = self
                                                    .project
                                                    .outline_bounds_for_glyph(&component.base)?;
                                                let corners = [
                                                    (min_x, min_y),
                                                    (min_x, max_y),
                                                    (max_x, max_y),
                                                    (max_x, min_y),
                                                ];
                                                let intersects = corners.into_iter().any(|(x, y)| {
                                                    selection.contains(self.canvas.glyph_to_screen(
                                                        component.x_scale * x
                                                            + component.yx_scale * y
                                                            + component.x_offset,
                                                        component.xy_scale * x
                                                            + component.y_scale * y
                                                            + component.y_offset,
                                                        origin,
                                                    ))
                                                });
                                                intersects.then_some(component_index)
                                            })
                                            .collect();
                                        if !selected_components.is_empty() {
                                            self.canvas.selected_nodes.clear();
                                            self.canvas.selected_points.clear();
                                            self.canvas.selected_contour = None;
                                            self.canvas.selected_components =
                                                selected_components;
                                            self.canvas.selected_component =
                                                self.canvas.selected_components.last().copied();
                                        }
                                        if self.canvas.selected_components.is_empty() {
                                            self.canvas.selected_contour = selected_contour;
                                            self.canvas.selected_points = selected_points;
                                            self.canvas.selected_nodes = selected_nodes;
                                        } else {
                                            self.canvas.selected_contour = None;
                                        }
                                    }
                                }
                            }
                            if self.canvas.selection_start.is_none() {
                                let delta = self
                                    .canvas
                                    .screen_to_glyph(mouse_pos - response.drag_delta(), origin);
                                let (dx, dy) = self.canvas.screen_to_glyph(mouse_pos, origin);
                                let (ddx, ddy) = {
                                    let raw_dx = dx - delta.0;
                                    let raw_dy = dy - delta.1;
                                    if ctx.input(|input| input.modifiers.shift) {
                                        if raw_dx.abs() >= raw_dy.abs() {
                                            (raw_dx, 0.0)
                                        } else {
                                            (0.0, raw_dy)
                                        }
                                    } else {
                                        (raw_dx, raw_dy)
                                    }
                                };

                                let transformed_component = self
                                    .component_resize
                                    .clone()
                                    .and_then(|(resize_index, handle, original)| {
                                        (Some(resize_index) == self.canvas.selected_component)
                                            .then(|| {
                                                if handle == 4 {
                                                    let start = self.canvas.screen_to_glyph(
                                                        mouse_pos - response.drag_delta(),
                                                        origin,
                                                    );
                                                    Self::rotate_component_from_handle(
                                                        &self.project,
                                                        &original,
                                                        start,
                                                        (gx, gy),
                                                        ctx.input(|input| input.modifiers.command),
                                                    )
                                                } else {
                                                    Self::resize_component_from_handle(
                                                        &self.project,
                                                        &original,
                                                        handle,
                                                        (gx, gy),
                                                    )
                                                }
                                            })
                                            .flatten()
                                    });
                                let master_rotation_updates =
                                    if self.edit_all_masters {
                                        self.component_resize
                                            .as_ref()
                                            .filter(|(_, handle, _)| *handle == 4)
                                            .and_then(|(_, _, original)| {
                                                let resized = transformed_component.as_ref()?;
                                                let angle = resized.xy_scale.atan2(resized.x_scale)
                                                    - original.xy_scale.atan2(original.x_scale);
                                                let name = self.current_glyph.as_ref()?;
                                                let glyph = self.project.glyphs.get(name)?;
                                                Some(
                                                    glyph
                                                        .layers
                                                        .iter()
                                                        .filter_map(|(layer_id, layer)| {
                                                            let component =
                                                                layer.components.get(self.canvas.selected_component?)?;
                                                            Some((
                                                                layer_id.clone(),
                                                                Self::rotate_component_by_angle(
                                                                    &self.project,
                                                                    component,
                                                                    angle,
                                                                )?,
                                                            ))
                                                        })
                                                        .collect::<Vec<_>>(),
                                                )
                                            })
                                    } else {
                                        None
                                    };

                                let component_indices = self.selected_component_indices();
                                let global_guidelines = self
                                    .project
                                    .guidelines_for_master(&self.current_master_id)
                                    .to_vec();
                                if let Some(name) = &self.current_glyph {
                                    if let Some(glyph) = self.project.glyphs.get_mut(name) {
                                        if component_indices.len() > 1 {
                                            if self.edit_all_masters {
                                                for component_index in &component_indices {
                                                    if let Err(error) = glyph
                                                        .translate_component_all_layers(
                                                            *component_index,
                                                            ddx,
                                                            ddy,
                                                        )
                                                    {
                                                        self.status_message = error;
                                                    }
                                                }
                                            } else {
                                                for component_index in component_indices {
                                                    if let Some(component) =
                                                        glyph.components.get_mut(component_index)
                                                    {
                                                        component.x_offset += ddx;
                                                        component.y_offset += ddy;
                                                    }
                                                }
                                            }
                                        } else if let Some(component_index) =
                                            self.canvas.selected_component
                                        {
                                            if let Some(resized) = transformed_component {
                                                let resize_original = self
                                                    .component_resize
                                                    .as_ref()
                                                    .map(|(_, _, original)| original.clone());
                                                let scale_x = resize_original
                                                    .as_ref()
                                                    .and_then(|original| {
                                                        (original.x_scale.abs() > f64::EPSILON)
                                                            .then_some(
                                                                resized.x_scale / original.x_scale,
                                                            )
                                                    })
                                                    .unwrap_or(1.0);
                                                let scale_y = resize_original
                                                    .as_ref()
                                                    .and_then(|original| {
                                                        (original.y_scale.abs() > f64::EPSILON)
                                                            .then_some(
                                                                resized.y_scale / original.y_scale,
                                                            )
                                                    })
                                                    .unwrap_or(1.0);
                                                let offset_dx = resize_original
                                                    .as_ref()
                                                    .map_or(0.0, |original| {
                                                        resized.x_offset - original.x_offset
                                                    });
                                                let offset_dy = resize_original
                                                    .as_ref()
                                                    .map_or(0.0, |original| {
                                                        resized.y_offset - original.y_offset
                                                    });
                                                if let Some(component) =
                                                    glyph.components.get_mut(component_index)
                                                {
                                                    *component = resized;
                                                }
                                                if self.edit_all_masters {
                                                    if let Some(updates) =
                                                        master_rotation_updates
                                                    {
                                                        for (layer_id, component) in updates {
                                                            if let Some(layer_component) = glyph
                                                                .layers
                                                                .get_mut(&layer_id)
                                                                .and_then(|layer| {
                                                                    layer.components.get_mut(component_index)
                                                                })
                                                            {
                                                                *layer_component = component;
                                                            }
                                                        }
                                                    } else {
                                                        for layer in glyph.layers.values_mut() {
                                                            if let Some(component) =
                                                                layer.components.get_mut(component_index)
                                                            {
                                                                component.x_scale *= scale_x;
                                                                component.xy_scale *= scale_x;
                                                                component.yx_scale *= scale_y;
                                                                component.y_scale *= scale_y;
                                                                component.x_offset += offset_dx;
                                                                component.y_offset += offset_dy;
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if self.edit_all_masters {
                                                if let Err(error) = glyph
                                                    .translate_component_all_layers(
                                                        component_index,
                                                        ddx,
                                                        ddy,
                                                    )
                                                {
                                                    self.status_message = error;
                                                }
                                            } else if let Some(component) =
                                                glyph.components.get_mut(component_index)
                                            {
                                                component.x_offset += ddx;
                                                component.y_offset += ddy;
                                            }
                                        } else {
                                            let nodes: Vec<(usize, usize)> =
                                                if self.canvas.selected_nodes.is_empty() {
                                                    self.canvas
                                                        .selected_contour
                                                        .map(|ci| {
                                                            self.canvas
                                                                .selected_points
                                                                .iter()
                                                                .map(move |&pi| (ci, pi))
                                                                .collect()
                                                        })
                                                        .unwrap_or_default()
                                                } else {
                                                    self.canvas.selected_nodes.clone()
                                                };
                                            if self.edit_all_masters {
                                                if let Err(error) =
                                                    glyph.translate_nodes_all_layers(&nodes, ddx, ddy)
                                                {
                                                    self.status_message = error;
                                                }
                                            } else {
                                                let glyph_guidelines = glyph
                                                    .guidelines_for_master(
                                                        &self.current_master_id,
                                                    )
                                                    .to_vec();
                                                for (ci, contour) in
                                                    glyph.contours.iter_mut().enumerate()
                                                {
                                                    let indices: Vec<usize> = nodes
                                                        .iter()
                                                        .filter_map(|&(selected_ci, pi)| {
                                                            (selected_ci == ci).then_some(pi)
                                                        })
                                                        .collect();
                                                    if indices.is_empty() {
                                                        continue;
                                                    }
                                                    contour.translate_points(&indices, ddx, ddy);
                                                    for pi in indices {
                                                        if let Some(point) =
                                                            contour.points.get_mut(pi)
                                                        {
                                                            let (x, y) = self.canvas.snap_point(
                                                                point.x,
                                                                point.y,
                                                            );
                                                            let mut guidelines =
                                                                global_guidelines.clone();
                                                            guidelines.extend(
                                                                glyph_guidelines.iter().cloned(),
                                                            );
                                                            let (x, y) = self.canvas
                                                                .snap_point_to_guidelines(
                                                                    x, y, &guidelines,
                                                                );
                                                            let (x, y) = self.canvas
                                                                .snap_point_to_anchors(
                                                                    x,
                                                                    y,
                                                                    &glyph.anchors,
                                                                );
                                                            point.x = x;
                                                            point.y = y;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if response.drag_stopped() && self.guideline_drag.take().is_some() {
                            self.save_state();
                        }
                        if response.drag_stopped() && self.anchor_drag.take().is_some() {
                            self.save_state();
                            self.status_message = "アンカーを移動しました".to_string();
                        }
                        if response.drag_stopped()
                            && (!self.canvas.selected_points.is_empty()
                                || self.canvas.selected_component.is_some())
                        {
                            self.save_state();
                        }
                        if response.drag_stopped() {
                            self.component_drag_duplicated = false;
                            self.component_resize = None;
                            if self.spacing_drag.is_some() {
                                self.save_state();
                                self.spacing_drag = None;
                                self.status_message =
                                    "キャンバス上で字幅を変更しました".to_string();
                            }
                            self.canvas.selection_start = None;
                            self.canvas.selection_rect = None;
                        }
                    }
                    Tool::Pen => {
                        if response.drag_started() {
                            if let Some(start) = response.interact_pointer_pos() {
                                let point = self.canvas.screen_to_glyph(start, origin);
                                self.pen_drag_start = Some(point);
                                self.pen_state.begin_drag(point.0, point.1);
                            }
                        }
                        if response.dragged() {
                            self.pen_state.update_drag(gx, gy);
                        }
                        if response.drag_stopped() {
                            if let (Some((start_x, start_y)), Some(end)) = (
                                self.pen_drag_start.take(),
                                response.interact_pointer_pos(),
                            ) {
                                let (handle_x, handle_y) =
                                    self.canvas.screen_to_glyph(end, origin);
                                self.pen_state.add_dragged_anchor(
                                    start_x, start_y, handle_x, handle_y,
                                );
                            }
                        } else if response.clicked() && !response.dragged() {
                            let is_off_curve = ctx.input(|i| i.modifiers.shift);

                            if !self.pen_state.is_drawing {
                                self.pen_state.start_path(gx, gy);
                            } else {
                                self.pen_state.add_point(gx, gy, is_off_curve);
                            }
                        }

                        if response.double_clicked() {
                            if let Some(points) = self.pen_state.finish_path() {
                                let on_curve_count =
                                    points.iter().filter(|point| point.is_on_curve()).count();
                                if on_curve_count < 3 {
                                    self.status_message =
                                        "輪郭には3つ以上のオンカーブ点が必要です".to_string();
                                } else if let Some(name) = &self.current_glyph {
                                    if let Some(contour_index) = self
                                        .project
                                        .add_contour_all_layers(name, Contour { points })
                                    {
                                        self.canvas.selected_contour = Some(contour_index);
                                        let point_count = self
                                            .project
                                            .glyphs
                                            .get(name)
                                            .and_then(|glyph| glyph.contours.get(contour_index))
                                            .map_or(0, |contour| contour.points.len());
                                        self.canvas.selected_points = (0..point_count).collect();
                                        self.canvas.selected_nodes = self
                                            .canvas
                                            .selected_points
                                            .iter()
                                            .map(|&point_index| (contour_index, point_index))
                                            .collect();
                                        self.save_state();
                                        self.status_message = "輪郭を作成しました".to_string();
                                    }
                                }
                            }
                        }
                    }
                    Tool::Knife => {
                        if response.clicked() {
                            if let Some(name) = self.current_glyph.clone() {
                                if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                    let mut best: Option<(usize, usize, f64, f32)> = None;
                                    for (ci, contour) in glyph.contours.iter().enumerate() {
                                        for start in 0..contour.points.len().saturating_sub(1) {
                                            if !contour.points[start].is_on_curve() {
                                                continue;
                                            }
                                            let mut end = start + 1;
                                            while end < contour.points.len()
                                                && !contour.points[end].is_on_curve()
                                            {
                                                end += 1;
                                            }
                                            if end >= contour.points.len() || end - start > 3 {
                                                continue;
                                            }
                                            let p = |i: usize| {
                                                Point::new(
                                                    contour.points[i].x,
                                                    contour.points[i].y,
                                                )
                                            };
                                            let mouse = Point::new(gx, gy);
                                            let nearest = match end - start {
                                                1 => Line::new(p(start), p(end))
                                                    .nearest(mouse, 0.01),
                                                2 => {
                                                    QuadBez::new(p(start), p(start + 1), p(end))
                                                        .nearest(mouse, 0.01)
                                                }
                                                3 => CubicBez::new(
                                                    p(start),
                                                    p(start + 1),
                                                    p(start + 2),
                                                    p(end),
                                                )
                                                .nearest(mouse, 0.01),
                                                _ => continue,
                                            };
                                            let distance = (nearest.distance_sq.sqrt()
                                                * self.canvas.zoom as f64)
                                                as f32;
                                            if best.is_none_or(|(_, _, _, d)| distance < d) {
                                                best = Some((ci, start, nearest.t, distance));
                                            }
                                        }
                                        // The final segment closes the cyclic contour and
                                        // may contain one or two off-curve controls.
                                        if let Some(start) = contour
                                            .points
                                            .iter()
                                            .rposition(|point| point.is_on_curve())
                                        {
                                            let mut indices = vec![start];
                                            let mut index = (start + 1) % contour.points.len();
                                            while index != 0
                                                && !contour.points[index].is_on_curve()
                                                && indices.len() <= 3
                                            {
                                                indices.push(index);
                                                index = (index + 1) % contour.points.len();
                                            }
                                            if index == 0
                                                && indices.len() <= 3
                                                && contour.points[0].is_on_curve()
                                            {
                                                indices.push(0);
                                                let p = |i: usize| {
                                                    Point::new(
                                                        contour.points[i].x,
                                                        contour.points[i].y,
                                                    )
                                                };
                                                let mouse = Point::new(gx, gy);
                                                let nearest = match indices.len() - 1 {
                                                    1 => Line::new(p(indices[0]), p(0))
                                                        .nearest(mouse, 0.01),
                                                    2 => QuadBez::new(
                                                        p(indices[0]),
                                                        p(indices[1]),
                                                        p(0),
                                                    )
                                                    .nearest(mouse, 0.01),
                                                    3 => CubicBez::new(
                                                        p(indices[0]),
                                                        p(indices[1]),
                                                        p(indices[2]),
                                                        p(0),
                                                    )
                                                    .nearest(mouse, 0.01),
                                                    _ => unreachable!(),
                                                };
                                                let distance = (nearest.distance_sq.sqrt()
                                                    * self.canvas.zoom as f64)
                                                    as f32;
                                                if best.is_none_or(|(_, _, _, d)| distance < d)
                                                {
                                                    best =
                                                        Some((ci, start, nearest.t, distance));
                                                }
                                            }
                                        }
                                    }
                                    if let Some((ci, segment_start, t, distance)) = best {
                                        if distance <= 18.0_f32.powi(2) {
                                            {
                                                let Ok(insert_at) = glyph.split_segment_all_layers(
                                                    ci,
                                                    segment_start,
                                                    t,
                                                ) else {
                                                    return;
                                                };
                                                if let Some((first_ci, first_index)) =
                                                    self.knife_first_cut.take()
                                                {
                                                    if first_ci == ci {
                                                        let first_index =
                                                            (first_index + usize::from(insert_at <= first_index))
                                                                .min(glyph.contours[ci].points.len().saturating_sub(1));
                                                        if glyph
                                                            .cut_contour_all_layers(
                                                                ci,
                                                                first_index,
                                                                insert_at,
                                                            )
                                                            .is_ok()
                                                        {
                                                            self.canvas.selected_contour = Some(ci);
                                                            self.canvas.selected_points.clear();
                                                            self.canvas.selected_nodes.clear();
                                                            self.save_state();
                                                            self.status_message =
                                                                "輪郭を2つに分割しました".to_string();
                                                        } else {
                                                            self.knife_first_cut = Some((ci, insert_at));
                                                            self.status_message =
                                                                "この位置では分割できません。別の輪郭上をクリックしてください"
                                                                    .to_string();
                                                        }
                                                    } else {
                                                        self.knife_first_cut = Some((ci, insert_at));
                                                        self.status_message =
                                                            "1点目と同じ輪郭上をクリックしてください"
                                                                .to_string();
                                                    }
                                                } else {
                                                    self.knife_first_cut = Some((ci, insert_at));
                                                    self.canvas.selected_contour = Some(ci);
                                                    self.canvas.selected_points = vec![insert_at];
                                                    self.canvas.selected_nodes = vec![(ci, insert_at)];
                                                    self.save_state();
                                                    self.status_message =
                                                        "1点目を追加しました。もう1点クリックして輪郭を分割"
                                                            .to_string();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Tool::Hand => {
                        if response.dragged() {
                            self.canvas.pan += response.drag_delta();
                        }
                    }
                    Tool::Ruler => {
                        if response.drag_started() {
                            self.canvas.ruler_start = response.interact_pointer_pos();
                            self.canvas.ruler_end = self.canvas.ruler_start;
                        }
                        if response.dragged() {
                            self.canvas.ruler_end = response.interact_pointer_pos();
                        }
                        if response.drag_stopped() {
                            if let (Some(start), Some(end)) =
                                (self.canvas.ruler_start, self.canvas.ruler_end)
                            {
                                let (sx, sy) = self.canvas.screen_to_glyph(start, origin);
                                let (ex, ey) = self.canvas.screen_to_glyph(end, origin);
                                self.status_message = format!(
                                    "測定: Δx {:.1}, Δy {:.1}, 距離 {:.1}",
                                    ex - sx,
                                    ey - sy,
                                    ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt()
                                );
                            }
                        }
                    }
                }
                    }
                }

            if let Some(mouse_pos) = response.hover_pos() {
                painter.circle_filled(
                    mouse_pos,
                    3.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 100),
                );
                if self.current_tool != Tool::Hand {
                    let (cursor_x, cursor_y) =
                        self.canvas.screen_to_glyph(mouse_pos, origin);
                    painter.text(
                        mouse_pos + Vec2::new(10.0, 12.0),
                        egui::Align2::LEFT_TOP,
                        format!("{cursor_x:.0}, {cursor_y:.0}"),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgba_premultiplied(220, 225, 235, 190),
                    );
                }
            }
        });
    }

    pub(super) fn fit_current_glyph_to_canvas(&mut self, rect: egui::Rect) {
        let Some(name) = self.current_glyph.as_deref() else {
            return;
        };
        let bounds = self.project.outline_bounds_for_glyph(name);
        let Some((min_x, min_y, max_x, max_y)) = bounds else {
            self.canvas.zoom = 1.0;
            self.canvas.pan = Vec2::ZERO;
            return;
        };
        let width = (max_x - min_x).max(1.0) + 200.0;
        let height = (max_y - min_y).max(1.0) + 200.0;
        self.canvas.zoom = ((rect.width() as f64 / width).min(rect.height() as f64 / height))
            .clamp(0.05, 50.0) as f32;
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        self.canvas.pan = Vec2::new(
            (-center_x * self.canvas.zoom as f64) as f32,
            (center_y * self.canvas.zoom as f64) as f32,
        );
    }

    pub(super) fn decompose_current_components(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(components) = self
            .project
            .glyphs
            .get(&name)
            .map(|glyph| glyph.components.clone())
        else {
            return;
        };
        let mut contours = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        for component in &components {
            collect_decomposed_contours(
                &self.project,
                &component.base,
                component_transform(component),
                &mut visiting,
                &mut contours,
            );
        }
        let master_ids: Vec<String> = self
            .project
            .glyphs
            .get(&name)
            .into_iter()
            .flat_map(|glyph| glyph.layers.keys().cloned())
            .collect();
        let mut layer_contours = Vec::new();
        for master_id in master_ids {
            let mut decomposed = Vec::new();
            let mut visiting = std::collections::HashSet::new();
            for component in &components {
                collect_decomposed_contours_for_master(
                    &self.project,
                    &component.base,
                    &master_id,
                    component_transform(component),
                    &mut visiting,
                    &mut decomposed,
                );
            }
            layer_contours.push((master_id, decomposed));
        }
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            if contours.is_empty() {
                return;
            }
            glyph.contours.extend(contours);
            glyph.components.clear();
            for (master_id, decomposed) in layer_contours {
                if let Some(layer) = glyph.layers.get_mut(&master_id) {
                    layer.contours.extend(decomposed);
                    layer.components.clear();
                }
            }
            self.canvas.selected_contour = None;
            self.canvas.selected_points.clear();
            self.canvas.selected_nodes.clear();
            self.save_state();
            self.status_message = "コンポーネントを輪郭化しました".to_string();
        }
    }

    pub(super) fn decompose_named_components(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            let Some(components) = self.project.glyphs.get(name).map(|g| g.components.clone())
            else {
                continue;
            };
            if components.is_empty() {
                continue;
            }
            let mut contours = Vec::new();
            let mut visiting = std::collections::HashSet::new();
            for component in &components {
                collect_decomposed_contours(
                    &self.project,
                    &component.base,
                    component_transform(component),
                    &mut visiting,
                    &mut contours,
                );
            }
            let master_ids: Vec<String> = self
                .project
                .glyphs
                .get(name)
                .into_iter()
                .flat_map(|glyph| glyph.layers.keys().cloned())
                .collect();
            let mut layer_contours = Vec::new();
            for master_id in master_ids {
                let mut decomposed = Vec::new();
                let mut visiting = std::collections::HashSet::new();
                for component in &components {
                    collect_decomposed_contours_for_master(
                        &self.project,
                        &component.base,
                        &master_id,
                        component_transform(component),
                        &mut visiting,
                        &mut decomposed,
                    );
                }
                layer_contours.push((master_id, decomposed));
            }
            if let Some(glyph) = self.project.glyphs.get_mut(name) {
                if !contours.is_empty() {
                    glyph.contours.extend(contours);
                    glyph.components.clear();
                    for (master_id, decomposed) in layer_contours {
                        if let Some(layer) = glyph.layers.get_mut(&master_id) {
                            layer.contours.extend(decomposed);
                            layer.components.clear();
                        }
                    }
                    changed += 1;
                }
            }
        }
        changed
    }
}
