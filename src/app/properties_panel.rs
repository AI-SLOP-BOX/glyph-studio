use super::*;

impl GlyphStudioApp {
    pub(crate) fn properties_panel(&mut self, ctx: &egui::Context) {
        if self.show_properties {
            egui::SidePanel::right("properties_panel")
                .default_width(300.0)
                .resizable(true)
                .width_range(280.0..=380.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("プロパティ");
                                ui.add_space(ui.available_width().max(0.0) - 26.0);
                                if ui
                                    .small_button("×")
                                    .on_hover_text("プロパティを閉じる")
                                    .clicked()
                                {
                                    self.show_properties = false;
                                }
                            });
                            ui.separator();
                            self.show_node_inspector(ui);
                            self.show_component_inspector(ui);
                            let masters_before = self.project.masters.clone();
                            ui.heading("編集マスター");
                            if self.project.masters.len() >= 2 {
                                if !self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == self.interpolation_from_master)
                                {
                                    self.interpolation_from_master =
                                        self.project.masters[0].id.clone();
                                }
                                if !self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == self.interpolation_to_master)
                                {
                                    self.interpolation_to_master = self.project.masters
                                        [self.project.masters.len() - 1]
                                        .id
                                        .clone();
                                }
                                ui.horizontal(|ui| {
                                    ui.label("補間:");
                                    egui::ComboBox::from_id_salt("interpolation_from_master")
                                        .selected_text(
                                            self.project
                                                .masters
                                                .iter()
                                                .find(|master| {
                                                    master.id == self.interpolation_from_master
                                                })
                                                .map(|master| master.name.as_str())
                                                .unwrap_or("-"),
                                        )
                                        .show_ui(ui, |ui| {
                                            for master in &self.project.masters {
                                                ui.selectable_value(
                                                    &mut self.interpolation_from_master,
                                                    master.id.clone(),
                                                    &master.name,
                                                );
                                            }
                                        });
                                    ui.label("→");
                                    egui::ComboBox::from_id_salt("interpolation_to_master")
                                        .selected_text(
                                            self.project
                                                .masters
                                                .iter()
                                                .find(|master| {
                                                    master.id == self.interpolation_to_master
                                                })
                                                .map(|master| master.name.as_str())
                                                .unwrap_or("-"),
                                        )
                                        .show_ui(ui, |ui| {
                                            for master in &self.project.masters {
                                                ui.selectable_value(
                                                    &mut self.interpolation_to_master,
                                                    master.id.clone(),
                                                    &master.name,
                                                );
                                            }
                                        });
                                });
                                ui.add(
                                    egui::Slider::new(&mut self.interpolation_factor, 0.0..=1.0)
                                        .text("補間プレビュー"),
                                );
                                let compatibility_issues = master_compatibility_issues(
                                    &self.project,
                                    &self.interpolation_from_master,
                                    &self.interpolation_to_master,
                                );
                                if compatibility_issues.is_empty() {
                                    ui.colored_label(
                                        Color32::from_rgb(70, 150, 80),
                                        "✓ 全グリフ互換",
                                    );
                                } else {
                                    ui.colored_label(
                                        Color32::from_rgb(210, 130, 40),
                                        format!("⚠ 非互換 {}件", compatibility_issues.len()),
                                    );
                                    for issue in compatibility_issues.iter().take(3) {
                                        ui.small(issue);
                                    }
                                    if compatibility_issues.len() > 3 {
                                        ui.small(format!(
                                            "ほか{}件",
                                            compatibility_issues.len() - 3
                                        ));
                                    }
                                }
                            }
                            let previous_master = self.current_master_id.clone();
                            egui::ComboBox::from_id_salt("current_master")
                                .selected_text(
                                    self.project
                                        .masters
                                        .iter()
                                        .find(|master| master.id == self.current_master_id)
                                        .map(|master| master.name.as_str())
                                        .unwrap_or("Regular"),
                                )
                                .show_ui(ui, |ui| {
                                    for master in &self.project.masters {
                                        ui.selectable_value(
                                            &mut self.current_master_id,
                                            master.id.clone(),
                                            &master.name,
                                        );
                                    }
                                });
                            if ui.button("＋ マスターを追加").clicked() {
                                let mut index = self.project.masters.len() + 1;
                                while self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == format!("master{index}"))
                                {
                                    index += 1;
                                }
                                let id = format!("master{index}");
                                let name = format!("Master {index}");
                                self.project.masters.push(crate::font_data::FontMaster {
                                    id: id.clone(),
                                    name,
                                    weight: 400.0,
                                    width: 100.0,
                                    is_bracket: false,
                                    axes: std::collections::HashMap::new(),
                                });
                                self.project.switch_master(&self.current_master_id, &id);
                                self.current_master_id = id;
                                self.save_state();
                            }
                            if self.project.masters.len() > 1
                                && ui.button("現在のマスターを全マスターへコピー").clicked()
                            {
                                self.project.sync_active_layer(&self.current_master_id);
                                let copied = if self.selected_glyphs.is_empty() {
                                    self.project.copy_master_to_all(&self.current_master_id)
                                } else {
                                    self.project.copy_master_to_all_for_glyphs(
                                        &self.current_master_id,
                                        self.selected_glyphs.iter().map(String::as_str),
                                    )
                                };
                                self.status_message =
                                    format!("{}件のグリフレイヤーをコピーしました", copied);
                                self.save_state();
                            }
                            let mut duplicate_master_requested = false;
                            if ui
                                .button("現在のマスターを複製")
                                .on_hover_text("名前・軸値・全グリフのレイヤーを複製")
                                .clicked()
                            {
                                duplicate_master_requested = true;
                            }
                            if self.current_master_id != previous_master {
                                self.project
                                    .switch_master(&previous_master, &self.current_master_id);
                                self.save_state();
                            }
                            if duplicate_master_requested {
                                let source_id = self.current_master_id.clone();
                                if let Some(new_id) = self.project.duplicate_master(&source_id) {
                                    self.project.switch_master(&source_id, &new_id);
                                    self.current_master_id = new_id.clone();
                                    self.save_state();
                                    self.status_message =
                                        format!("マスターを複製しました: {new_id}");
                                }
                            }
                            let mut add_axis_tag = None;
                            let mut remove_axis_tag = None;
                            let mut delete_master_id = None;
                            let mut move_master_delta = None;
                            let mut default_master_changed = false;
                            let can_delete_master = self.project.masters.len() > 1;
                            let master_index = self
                                .project
                                .masters
                                .iter()
                                .position(|master| master.id == self.current_master_id)
                                .unwrap_or(0);
                            let master_count = self.project.masters.len();
                            if let Some(master) = self
                                .project
                                .masters
                                .iter_mut()
                                .find(|master| master.id == self.current_master_id)
                            {
                                ui.horizontal(|ui| {
                                    ui.label("名称:");
                                    ui.text_edit_singleline(&mut master.name);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("順序:");
                                    if ui
                                        .add_enabled(master_index > 0, egui::Button::new("↑"))
                                        .on_hover_text("前のマスターへ移動")
                                        .clicked()
                                    {
                                        move_master_delta = Some(-1);
                                    }
                                    if ui
                                        .add_enabled(
                                            master_index + 1 < master_count,
                                            egui::Button::new("↓"),
                                        )
                                        .on_hover_text("次のマスターへ移動")
                                        .clicked()
                                    {
                                        move_master_delta = Some(1);
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Weight:");
                                    ui.add(egui::DragValue::new(&mut master.weight).speed(1.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Width:");
                                    ui.add(egui::DragValue::new(&mut master.width).speed(1.0));
                                });
                                if !master.axes.is_empty() {
                                    ui.separator();
                                    ui.label("可変軸:");
                                    let mut axis_tags: Vec<String> =
                                        master.axes.keys().cloned().collect();
                                    axis_tags.sort();
                                    for tag in axis_tags {
                                        if let Some(value) = master.axes.get_mut(&tag) {
                                            ui.horizontal(|ui| {
                                                ui.label(&tag);
                                                ui.add(egui::DragValue::new(value).speed(0.1));
                                                if ui.small_button("削除").clicked() {
                                                    remove_axis_tag = Some(tag.clone());
                                                }
                                            });
                                        }
                                    }
                                }
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.master_axis_tag_input)
                                            .hint_text("タグ (例: opsz)")
                                            .desired_width(90.0),
                                    );
                                    let tag =
                                        self.master_axis_tag_input.trim().to_ascii_lowercase();
                                    let valid_axis_tag = tag.len() == 4
                                        && tag.is_ascii()
                                        && tag.chars().all(|ch| ch.is_ascii_alphanumeric())
                                        && !master.axes.contains_key(&tag);
                                    if ui
                                        .add_enabled(valid_axis_tag, egui::Button::new("軸を追加"))
                                        .on_hover_text("4文字の英数字タグを全マスターへ追加")
                                        .clicked()
                                    {
                                        add_axis_tag = Some(tag.clone());
                                        self.master_axis_tag_input.clear();
                                    }
                                    if !self.master_axis_tag_input.trim().is_empty()
                                        && !valid_axis_tag
                                    {
                                        let message = if tag.len() != 4 {
                                            "タグは4文字で入力"
                                        } else if !tag.is_ascii()
                                            || !tag.chars().all(|ch| ch.is_ascii_alphanumeric())
                                        {
                                            "英数字のみ使用可能"
                                        } else {
                                            "この軸は既に存在"
                                        };
                                        ui.colored_label(Color32::from_rgb(220, 140, 70), message);
                                    }
                                });
                                ui.checkbox(&mut master.is_bracket, "Bracket master");
                                if ui
                                    .button(if self.project.default_master_id == master.id {
                                        "基準マスター"
                                    } else {
                                        "基準に設定"
                                    })
                                    .clicked()
                                    && self.project.default_master_id != master.id
                                {
                                    self.project.default_master_id = master.id.clone();
                                    default_master_changed = true;
                                }
                                if can_delete_master && ui.button("このマスターを削除").clicked()
                                {
                                    delete_master_id = Some(master.id.clone());
                                }
                            }
                            if let Some(tag) = add_axis_tag {
                                for master in &mut self.project.masters {
                                    master.axes.entry(tag.clone()).or_insert(0.0);
                                }
                            }
                            if let Some(tag) = remove_axis_tag {
                                for master in &mut self.project.masters {
                                    master.axes.remove(&tag);
                                }
                                self.project.axis_names.remove(&tag);
                            }
                            if default_master_changed {
                                self.save_state();
                            }
                            if let Some(delete_id) = delete_master_id {
                                if self.project.remove_master(&delete_id) {
                                    let fallback = self
                                        .project
                                        .masters
                                        .first()
                                        .map(|master| master.id.clone())
                                        .unwrap_or_default();
                                    self.current_master_id = fallback;
                                    self.save_state();
                                }
                            }
                            if let Some(delta) = move_master_delta {
                                if self.project.move_master(&self.current_master_id, delta) {
                                    self.save_state();
                                }
                            }
                            let instances_before = self.project.instances.clone();
                            let mut delete_instance = None;
                            let mut add_instance = false;
                            egui::CollapsingHeader::new("名前付きインスタンス")
                                .default_open(false)
                                .show(ui, |ui| {
                                    let mut axis_tags = std::collections::BTreeSet::new();
                                    for master in &self.project.masters {
                                        axis_tags.extend(master.axes.keys().cloned());
                                    }
                                    for (index, instance) in
                                        self.project.instances.iter_mut().enumerate()
                                    {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("{}.", index + 1));
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut instance.name)
                                                        .desired_width(130.0),
                                                );
                                                if ui.small_button("削除").clicked() {
                                                    delete_instance = Some(index);
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::DragValue::new(&mut instance.weight)
                                                        .prefix("Weight ")
                                                        .speed(1.0),
                                                );
                                                ui.add(
                                                    egui::DragValue::new(&mut instance.width)
                                                        .prefix("Width ")
                                                        .speed(1.0),
                                                );
                                            });
                                            for tag in &axis_tags {
                                                let value =
                                                    instance.axes.entry(tag.clone()).or_insert(0.0);
                                                ui.add(
                                                    egui::DragValue::new(value)
                                                        .prefix(format!("{tag} "))
                                                        .speed(0.1),
                                                );
                                            }
                                        });
                                    }
                                    if ui.small_button("＋ インスタンスを追加").clicked()
                                    {
                                        add_instance = true;
                                    }
                                });
                            if let Some(index) = delete_instance {
                                self.project.instances.remove(index);
                            }
                            if add_instance {
                                let (name, axes, weight, width) = self
                                    .project
                                    .masters
                                    .iter()
                                    .find(|master| master.id == self.current_master_id)
                                    .map(|master| {
                                        (
                                            format!("{} Instance", master.name),
                                            master.axes.clone(),
                                            master.weight,
                                            master.width,
                                        )
                                    })
                                    .unwrap_or((
                                        "New Instance".to_string(),
                                        HashMap::new(),
                                        400.0,
                                        100.0,
                                    ));
                                self.project.instances.push(crate::font_data::FontInstance {
                                    name,
                                    axes,
                                    weight,
                                    width,
                                });
                            }
                            if self.project.instances != instances_before {
                                self.save_state();
                            }
                            if self.project.masters != masters_before {
                                self.save_state();
                            }
                            let before = self.project.clone();
                            let master_before_properties = self.current_master_id.clone();
                            properties::show_properties(
                                ui,
                                &mut self.properties_filter,
                                &mut self.project,
                                &self.current_glyph,
                                &mut self.component_base,
                                &mut self.kerning_right,
                                &mut self.kerning_pair_filter,
                                &mut self.preview_text,
                                &mut self.show_preview,
                                &mut self.feature_left,
                                &mut self.feature_right,
                                &mut self.feature_replacement,
                                &mut self.feature_kerning_value,
                                &mut self.feature_target_tag,
                                &mut self.feature_anchor_x,
                                &mut self.feature_anchor_y,
                                &mut self.unicode_alias_input,
                                &mut self.unicode_variation_selector,
                                &mut self.current_master_id,
                                &mut self.master_map_drag,
                                &mut self.color_layer_glyph,
                                &mut self.preview_color_palette,
                                &mut self.conditional_layer_axis,
                                &mut self.conditional_layer_min,
                                &mut self.conditional_layer_max,
                                &mut self.conditional_layer_axis_2,
                                &mut self.conditional_layer_min_2,
                                &mut self.conditional_layer_max_2,
                                &mut self.conditional_layer_axis_3,
                                &mut self.conditional_layer_min_3,
                                &mut self.conditional_layer_max_3,
                                &mut self.conditional_layer_axis_4,
                                &mut self.conditional_layer_min_4,
                                &mut self.conditional_layer_max_4,
                                &mut self.conditional_layer_extra,
                            );
                            if self.project != before && self.master_map_drag.is_none() {
                                self.save_state();
                            }
                            if self.current_master_id != master_before_properties {
                                self.project.switch_master(
                                    &master_before_properties,
                                    &self.current_master_id,
                                );
                                self.save_state();
                            }
                        });
                });
        }
    }
}
