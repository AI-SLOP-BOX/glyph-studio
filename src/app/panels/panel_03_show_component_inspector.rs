use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_component_inspector(&mut self, ui: &mut egui::Ui) {
        let Some(component_index) = self.canvas.selected_component else {
            return;
        };
        let Some(glyph_name) = self.current_glyph.clone() else {
            return;
        };
        let Some(component) = self
            .project
            .glyphs
            .get(&glyph_name)
            .and_then(|glyph| glyph.components.get(component_index))
            .cloned()
        else {
            return;
        };
        let selected_indices = self.selected_component_indices();
        let has_multiple_components = selected_indices.len() > 1;
        let mut align_selected_components = false;
        let mut delete_selected_components = false;
        ui.separator();
        egui::CollapsingHeader::new("選択コンポーネント")
            .default_open(true)
            .show(ui, |ui| {
                if has_multiple_components {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}個の部品を選択中", selected_indices.len()));
                        if ui
                            .small_button("選択部品をアンカー整列")
                            .on_hover_text("選択した全ての部品を全マスターでアンカー整列")
                            .clicked()
                        {
                            align_selected_components = true;
                        }
                        if ui
                            .small_button("選択部品を削除")
                            .on_hover_text("選択した部品を全マスターから削除")
                            .clicked()
                        {
                            delete_selected_components = true;
                        }
                    });
                    ui.separator();
                }
                let mut base = component.base.clone();
                ui.horizontal(|ui| {
                    ui.label("参照");
                    let base_response =
                        ui.add(egui::TextEdit::singleline(&mut base).desired_width(140.0));
                    let open_requested = base_response.lost_focus()
                        && ui.input(|input| input.key_pressed(Key::Enter));
                    if self.project.glyphs.contains_key(&base)
                        && (open_requested
                            || ui
                                .small_button("開く")
                                .on_hover_text("参照先グリフをキャンバスで編集")
                                .clicked())
                    {
                        self.current_glyph = Some(base.clone());
                        self.glyph_rename_input = base.clone();
                        self.clear_canvas_selection();
                    }
                });
                let base_exists = self.project.glyphs.contains_key(&base);
                if !base_exists {
                    ui.colored_label(Color32::from_rgb(220, 90, 80), "参照先グリフがありません");
                }
                let mut apply_all_layers = false;
                let mut x_scale = component.x_scale;
                let mut y_scale = component.y_scale;
                let mut xy_scale = component.xy_scale;
                let mut yx_scale = component.yx_scale;
                let mut x_offset = component.x_offset;
                let mut y_offset = component.y_offset;
                ui.checkbox(&mut self.component_scale_linked, "縦横比を固定")
                    .on_hover_text("X倍率とY倍率を同じ値に連動");
                for (label, value) in [
                    ("X倍率", &mut x_scale),
                    ("Y倍率", &mut y_scale),
                    ("XY", &mut xy_scale),
                    ("YX", &mut yx_scale),
                    ("X位置", &mut x_offset),
                    ("Y位置", &mut y_offset),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value).speed(0.01));
                    });
                }
                if self.component_scale_linked {
                    let x_changed = (x_scale - component.x_scale).abs() > f64::EPSILON;
                    let y_changed = (y_scale - component.y_scale).abs() > f64::EPSILON;
                    if x_changed && !y_changed {
                        y_scale = x_scale;
                    } else if y_changed && !x_changed {
                        x_scale = y_scale;
                    }
                }
                if ui.small_button("変形をリセット").clicked() {
                    x_scale = 1.0;
                    y_scale = 1.0;
                    xy_scale = 0.0;
                    yx_scale = 0.0;
                    x_offset = 0.0;
                    y_offset = 0.0;
                }
                let mut aligned = false;
                if ui
                    .button("アンカーで位置合わせ")
                    .on_hover_text("親グリフと参照先の対応するアンカーを合わせる（全マスター）")
                    .clicked()
                {
                    if self
                        .project
                        .align_component_anchors_all_layers(&glyph_name, component_index)
                    {
                        aligned = true;
                        self.status_message =
                            "コンポーネントをアンカーへ位置合わせしました".to_string();
                        self.save_state();
                    } else {
                        self.status_message = "対応するアンカーが見つかりません".to_string();
                    }
                }
                if ui
                    .small_button("選択部品を複製")
                    .on_hover_text("現在の変形のまま部品を複製")
                    .clicked()
                    && self
                        .project
                        .duplicate_component_all_layers(&glyph_name, component_index)
                {
                    let new_index = self
                        .project
                        .glyphs
                        .get(&glyph_name)
                        .map(|glyph| glyph.components.len().saturating_sub(1))
                        .unwrap_or(component_index);
                    self.canvas.selected_component = Some(new_index);
                    self.canvas.selected_components = vec![new_index];
                    self.save_state();
                    self.status_message = "コンポーネントを複製しました".to_string();
                }
                if ui
                    .button("参照・変形を全マスターへ適用")
                    .on_hover_text("参照先と変形値を全マスターの同じ部品へ反映")
                    .clicked()
                {
                    apply_all_layers = true;
                }
                let changed = [
                    x_scale - component.x_scale,
                    y_scale - component.y_scale,
                    xy_scale - component.xy_scale,
                    yx_scale - component.yx_scale,
                    x_offset - component.x_offset,
                    y_offset - component.y_offset,
                ]
                .iter()
                .any(|delta| delta.abs() > f64::EPSILON)
                    || base != component.base;
                if base_exists && (changed || apply_all_layers) && !aligned {
                    if let Some(target) = self
                        .project
                        .glyphs
                        .get_mut(&glyph_name)
                        .and_then(|glyph| glyph.components.get_mut(component_index))
                    {
                        target.base = base.clone();
                        target.x_scale = x_scale;
                        target.y_scale = y_scale;
                        target.xy_scale = xy_scale;
                        target.yx_scale = yx_scale;
                        target.x_offset = x_offset;
                        target.y_offset = y_offset;
                        if apply_all_layers {
                            if let Some(glyph) = self.project.glyphs.get_mut(&glyph_name) {
                                for layer in glyph.layers.values_mut() {
                                    if let Some(component) =
                                        layer.components.get_mut(component_index)
                                    {
                                        component.base = base.clone();
                                        component.x_scale = x_scale;
                                        component.y_scale = y_scale;
                                        component.xy_scale = xy_scale;
                                        component.yx_scale = yx_scale;
                                        component.x_offset = x_offset;
                                        component.y_offset = y_offset;
                                    }
                                }
                            }
                        }
                        self.save_state();
                    }
                }
            });
        if align_selected_components {
            let aligned = selected_indices
                .iter()
                .filter(|&&index| {
                    self.project
                        .align_component_anchors_all_layers(&glyph_name, index)
                })
                .count();
            if aligned > 0 {
                self.save_state();
                self.status_message = format!("{}個の部品をアンカー整列しました", aligned);
            } else {
                self.status_message = "対応するアンカーが見つかりません".to_string();
            }
        }
        if delete_selected_components {
            let mut indices = selected_indices;
            indices.sort_unstable_by(|left, right| right.cmp(left));
            let mut removed = 0;
            for index in indices {
                if self
                    .project
                    .remove_component_all_layers(&glyph_name, index)
                    .is_ok()
                {
                    removed += 1;
                }
            }
            if removed > 0 {
                self.clear_geometry_selection();
                self.save_state();
                self.status_message = format!("{}個の部品を削除しました", removed);
            }
        }
    }
}
