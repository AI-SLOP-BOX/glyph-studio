use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_canvas_header(&mut self, ui: &mut egui::Ui) -> bool {
        let mut fit_view_requested = false;
        let mut width_changed = false;
        let mut bearing_request: Option<(String, f64, f64)> = None;
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

        fit_view_requested
    }
}
