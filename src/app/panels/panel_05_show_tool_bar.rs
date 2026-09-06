use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_tool_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tool_bar").show(ctx, |ui| {
            ui.set_min_height(38.0);
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("ツール").small().color(Color32::GRAY));
                let tools = [
                    Tool::Select,
                    Tool::Pen,
                    Tool::Knife,
                    Tool::Hand,
                    Tool::Ruler,
                ];
                for tool in &tools {
                    let selected = self.current_tool == *tool;
                    let response = ui
                        .selectable_label(selected, format!("{} {}", tool.icon(), tool.name()))
                        .on_hover_text(format!("{}ツール（{}）", tool.name(), tool.shortcut()));
                    if response.clicked() {
                        self.current_tool = *tool;
                        if *tool != Tool::Pen {
                            self.pen_state.cancel();
                            self.pen_drag_start = None;
                        }
                        if *tool != Tool::Knife {
                            self.knife_first_cut = None;
                        }
                    }
                }

                ui.separator();
                let can_undo = self.history.current_index > 0;
                let can_redo = self.history.current_index + 1 < self.history.entries.len();
                if ui
                    .add_enabled(can_undo, egui::Button::new("↶"))
                    .on_hover_text("取り消す（⌘Z）")
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("↷"))
                    .on_hover_text("やり直す（⌘⇧Z）")
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .small_button("保存")
                    .on_hover_text("プロジェクトを保存（⌘S）")
                    .clicked()
                {
                    self.save_project_file();
                }
                ui.menu_button("書き出し", |ui| {
                    if ui
                        .button("TTF")
                        .on_hover_text("検証してTTFを書き出す")
                        .clicked()
                    {
                        self.export_ttf_file();
                        ui.close_menu();
                    }
                    if ui
                        .button("静的OTF")
                        .on_hover_text("基準マスターから静的CFF/OTFを書き出す")
                        .clicked()
                    {
                        self.export_otf_file();
                        ui.close_menu();
                    }
                    if ui
                        .button("WOFF2")
                        .on_hover_text("検証してWOFF2を書き出す")
                        .clicked()
                    {
                        self.export_woff2_file();
                        ui.close_menu();
                    }
                    if ui
                        .button("WOFF")
                        .on_hover_text("検証してWOFFを書き出す")
                        .clicked()
                    {
                        self.export_woff_file();
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.label(egui::RichText::new("パネル").small().color(Color32::GRAY));
                ui.toggle_value(&mut self.show_glyph_list, "一覧");
                ui.toggle_value(&mut self.show_properties, "プロパティ");
                ui.toggle_value(&mut self.show_preview, "プレビュー");
                ui.menu_button("レイアウト", |ui| {
                    if ui
                        .button("標準")
                        .on_hover_text("一覧・キャンバス・プロパティを表示")
                        .clicked()
                    {
                        self.show_glyph_list = true;
                        self.show_properties = true;
                        self.show_preview = true;
                        ui.close_menu();
                    }
                    if ui
                        .button("編集集中")
                        .on_hover_text("キャンバスを広く使う")
                        .clicked()
                    {
                        self.show_glyph_list = true;
                        self.show_properties = false;
                        self.show_preview = false;
                        ui.close_menu();
                    }
                    if ui
                        .button("組み")
                        .on_hover_text("プレビュー重視のレイアウト")
                        .clicked()
                    {
                        self.show_glyph_list = false;
                        self.show_properties = false;
                        self.show_preview = true;
                        ui.close_menu();
                    }
                });
                if ui
                    .small_button("?")
                    .on_hover_text("ショートカット一覧")
                    .clicked()
                {
                    self.show_shortcuts = true;
                }
                if ui
                    .small_button("検証")
                    .on_hover_text("書き出し前にフォント全体を検証")
                    .clicked()
                {
                    self.validation_issues = crate::core::validate_project_detailed(&self.project);
                    if self.show_interpolation_overlay {
                        self.validation_issues
                            .extend(crate::core::validate_interpolation(
                                &self.project,
                                &self.interpolation_from_master,
                                &self.interpolation_to_master,
                            ));
                    }
                    self.status_message = if self.validation_issues.is_empty() {
                        "検証完了: 問題はありません".to_string()
                    } else {
                        format!(
                            "検証完了: {}件の問題があります",
                            self.validation_issues.len()
                        )
                    };
                }
                if !self.validation_issues.is_empty() {
                    let glyph_issue_count = self
                        .validation_issues
                        .iter()
                        .filter(|issue| {
                            issue.glyph_name.as_deref() == self.current_glyph.as_deref()
                        })
                        .count();
                    let label = if glyph_issue_count > 0 {
                        format!("⚠ {}件", glyph_issue_count)
                    } else {
                        format!("⚠ 全体{}件", self.validation_issues.len())
                    };
                    if ui
                        .small_button(label)
                        .on_hover_text("最初のグリフ問題へ移動")
                        .clicked()
                    {
                        if let Some(name) = self
                            .validation_issues
                            .iter()
                            .find_map(|issue| issue.glyph_name.clone())
                        {
                            self.current_glyph = Some(name.clone());
                            self.glyph_rename_input = name.clone();
                            self.clear_canvas_selection();
                            self.status_message =
                                format!("検証エラーのグリフへ移動しました: {name}");
                        } else {
                            self.status_message = "検証結果を表示しています".to_string();
                        }
                    }
                }
                if ui
                    .small_button("カーニング")
                    .on_hover_text("全カーニングペアを一覧表示")
                    .clicked()
                {
                    self.show_kerning_window = true;
                }
                ui.separator();

                // Keep the active editing context visible even when the side
                // panels are collapsed. This is especially useful in a
                // multi-master workflow where it is easy to lose track of
                // which glyph/layer is currently being edited.
                let active_glyph = self.current_glyph.as_deref().unwrap_or("グリフ未選択");
                let active_master = self
                    .project
                    .masters
                    .iter()
                    .find(|master| master.id == self.current_master_id)
                    .map(|master| master.name.as_str())
                    .unwrap_or("マスター未選択");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{}  ·  {}", active_glyph, active_master))
                        .strong()
                        .color(Color32::from_rgb(220, 225, 235)),
                )
                .on_hover_text("現在編集中のグリフとマスター");
                ui.label(
                    egui::RichText::new(format!("{}字", self.project.glyphs.len()))
                        .small()
                        .color(Color32::GRAY),
                );
                if self.saved_history_index != self.history.current_index {
                    ui.label(
                        egui::RichText::new("● 未保存")
                            .small()
                            .color(Color32::from_rgb(245, 183, 77)),
                    )
                    .on_hover_text("変更があります。ファイルメニューから保存できます");
                }
                ui.separator();

                ui.menu_button("グリフ", |ui| {
                    if ui.button("日本語グリフを生成").clicked() {
                        crate::core::generate_all_japanese(&mut self.project);
                        self.current_glyph = self
                            .project
                            .glyph_names_sorted()
                            .first()
                            .map(|s| s.to_string());
                        self.status_message = format!(
                            "全日本語グリフを生成しました: {} グリフ",
                            self.project.glyphs.len()
                        );
                        self.save_state();
                        ui.close_menu();
                    }
                    if ui.button("＋ 新しいグリフ").clicked() {
                        let name = format!("glyph_{}", self.project.glyphs.len());
                        self.project.add_glyph(name.clone(), None);
                        self.current_glyph = Some(name);
                        self.save_state();
                        ui.close_menu();
                    }
                    if ui.button("選択中を複製").clicked() {
                        let count = self.duplicate_selected_glyphs();
                        if count > 0 {
                            self.status_message = format!("{}個のグリフを複製しました", count);
                        }
                        ui.close_menu();
                    }
                    let has_components = self.current_glyph.as_ref().is_some_and(|name| {
                        self.project
                            .glyphs
                            .get(name)
                            .is_some_and(|glyph| !glyph.components.is_empty())
                    });
                    if ui
                        .add_enabled(has_components, egui::Button::new("コンポーネントを輪郭化"))
                        .clicked()
                    {
                        self.decompose_current_components();
                        ui.close_menu();
                    }
                    if ui.button("現在のグリフを削除").clicked() {
                        if let Some(name) = self.current_glyph.clone() {
                            self.project.remove_glyph(&name);
                            self.current_glyph = self
                                .project
                                .glyph_names_sorted()
                                .first()
                                .map(|s| s.to_string());
                            self.save_state();
                        }
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.menu_button("輪郭操作", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(480.0)
                        .show(ui, |ui| {
                            let has_selection = !self.canvas.selected_points.is_empty()
                                || self.canvas.selected_component.is_some();
                            ui.horizontal(|ui| {
                                let can_copy = self.current_glyph.as_ref().is_some_and(|name| {
                                    self.canvas.selected_component.is_some_and(|index| {
                                        self.project
                                            .glyphs
                                            .get(name)
                                            .and_then(|glyph| glyph.components.get(index))
                                            .is_some()
                                    })
                                });
                                if ui
                                    .add_enabled(
                                        can_copy,
                                        egui::Button::new("コンポーネントをコピー"),
                                    )
                                    .clicked()
                                {
                                    if let (Some(name), Some(index)) =
                                        (self.current_glyph.clone(), self.canvas.selected_component)
                                    {
                                        self.component_clipboard =
                                            self.project.glyphs.get(&name).and_then(|glyph| {
                                                glyph.components.get(index).cloned()
                                            });
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        self.component_clipboard.is_some()
                                            && self.current_glyph.is_some(),
                                        egui::Button::new("コンポーネントを貼り付け"),
                                    )
                                    .clicked()
                                {
                                    if let (Some(name), Some(component)) = (
                                        self.current_glyph.clone(),
                                        self.component_clipboard.clone(),
                                    ) {
                                        if let Some(new_index) =
                                            self.project.add_component_all_layers(&name, component)
                                        {
                                            self.canvas.selected_component = Some(new_index);
                                            self.canvas.selected_components = vec![new_index];
                                            self.canvas.selected_points.clear();
                                            self.canvas.selected_nodes.clear();
                                            self.canvas.selected_contour = None;
                                            self.save_state();
                                        }
                                    }
                                }
                            });
                            if self.canvas.selected_nodes.len() == 1 {
                                let (ci, pi) = self.canvas.selected_nodes[0];
                                let mut changed = false;
                                if let Some(name) = self.current_glyph.clone() {
                                    if let Some(point) = self
                                        .project
                                        .glyphs
                                        .get_mut(&name)
                                        .and_then(|glyph| glyph.contours.get_mut(ci))
                                        .and_then(|contour| contour.points.get_mut(pi))
                                    {
                                        ui.horizontal(|ui| {
                                            ui.label("ノード座標");
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut point.x)
                                                        .prefix("X ")
                                                        .speed(1.0),
                                                )
                                                .changed();
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut point.y)
                                                        .prefix("Y ")
                                                        .speed(1.0),
                                                )
                                                .changed();
                                        });
                                    }
                                }
                                if changed {
                                    self.save_state();
                                }
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("スムーズ"))
                                .clicked()
                            {
                                if let (Some(name), Some(ci)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let nodes: Vec<(usize, usize)> =
                                            if self.canvas.selected_nodes.is_empty() {
                                                self.canvas
                                                    .selected_points
                                                    .iter()
                                                    .map(|&pi| (ci, pi))
                                                    .collect()
                                            } else {
                                                self.canvas.selected_nodes.clone()
                                            };
                                        match glyph.set_smooth_nodes_all_layers(&nodes, true) {
                                            Ok(()) => self.save_state(),
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("コーナー"))
                                .clicked()
                            {
                                if let (Some(name), Some(ci)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let nodes: Vec<(usize, usize)> =
                                            if self.canvas.selected_nodes.is_empty() {
                                                self.canvas
                                                    .selected_points
                                                    .iter()
                                                    .map(|&pi| (ci, pi))
                                                    .collect()
                                            } else {
                                                self.canvas.selected_nodes.clone()
                                            };
                                        match glyph.set_smooth_nodes_all_layers(&nodes, false) {
                                            Ok(()) => self.save_state(),
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("オン/オフ曲線"))
                                .clicked()
                            {
                                if let Some(name) = self.current_glyph.clone() {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
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
                                        match glyph.toggle_curve_nodes_all_layers(&nodes) {
                                            Ok(()) => self.save_state(),
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("輪郭を削除"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(contour_index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.remove_contour_all_layers(contour_index) {
                                            Ok(()) => {
                                                self.canvas.selected_points.clear();
                                                self.canvas.selected_nodes.clear();
                                                self.canvas.selected_contour = None;
                                                self.save_state();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("輪郭を複製"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(ci)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    let mut contour = self
                                        .project
                                        .glyphs
                                        .get(&name)
                                        .and_then(|glyph| glyph.contours.get(ci))
                                        .cloned();
                                    if let Some(contour) = contour.as_mut() {
                                        for point in &mut contour.points {
                                            point.x += 50.0;
                                            point.y += 50.0;
                                        }
                                    }
                                    if let Some(contour) = contour {
                                        if let Some(new_ci) =
                                            self.project.add_contour_all_layers(&name, contour)
                                        {
                                            let point_count = self
                                                .project
                                                .glyphs
                                                .get(&name)
                                                .and_then(|glyph| glyph.contours.get(new_ci))
                                                .map_or(0, |contour| contour.points.len());
                                            self.canvas.selected_contour = Some(new_ci);
                                            self.canvas.selected_points =
                                                (0..point_count).collect();
                                            self.canvas.selected_nodes = self
                                                .canvas
                                                .selected_points
                                                .iter()
                                                .map(|&pi| (new_ci, pi))
                                                .collect();
                                            self.save_state();
                                            self.status_message = "輪郭を複製しました".to_string();
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("輪郭をコピー"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(ci)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(contour) = self
                                        .project
                                        .glyphs
                                        .get(&name)
                                        .and_then(|glyph| glyph.contours.get(ci))
                                    {
                                        self.contour_clipboard = Some(contour.clone());
                                        self.status_message = "輪郭をコピーしました".to_string();
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.contour_clipboard.is_some(),
                                    egui::Button::new("輪郭を貼り付け"),
                                )
                                .clicked()
                            {
                                if let Some(name) = self.current_glyph.clone() {
                                    if let Some(mut contour) = self.contour_clipboard.clone() {
                                        for point in &mut contour.points {
                                            point.x += 50.0;
                                            point.y += 50.0;
                                        }
                                        if let Some(new_ci) =
                                            self.project.add_contour_all_layers(&name, contour)
                                        {
                                            let point_count = self
                                                .project
                                                .glyphs
                                                .get(&name)
                                                .and_then(|glyph| glyph.contours.get(new_ci))
                                                .map_or(0, |contour| contour.points.len());
                                            self.canvas.selected_contour = Some(new_ci);
                                            self.canvas.selected_points =
                                                (0..point_count).collect();
                                            self.canvas.selected_nodes = self
                                                .canvas
                                                .selected_points
                                                .iter()
                                                .map(|&pi| (new_ci, pi))
                                                .collect();
                                            self.save_state();
                                            self.status_message =
                                                "輪郭を貼り付けました".to_string();
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("方向反転"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(contour_index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.reverse_contour_all_layers(contour_index) {
                                            Ok(()) => self.save_state(),
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("方向を自動調整"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(ci)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        if let Some(contour) = glyph.contours.get(ci) {
                                            let should_reverse = contour.signed_area() > 0.0;
                                            if should_reverse {
                                                glyph.reverse_contour_all_layers(ci).ok();
                                            }
                                            self.save_state();
                                        }
                                    }
                                }
                            }
                            if ui.button("全輪郭の方向を調整").clicked() {
                                if let Some(name) = self.current_glyph.clone() {
                                    if self.project.normalize_glyph_winding(&[name]) > 0 {
                                        self.save_state();
                                        self.status_message =
                                            "全輪郭の方向を調整しました".to_string();
                                    }
                                }
                            }
                            if ui.button("重複ノードを整理").clicked() {
                                let names: Vec<String> = if self.selected_glyphs.is_empty() {
                                    self.current_glyph.iter().cloned().collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                };
                                let removed = self.project.remove_duplicate_nodes(&names);
                                if removed > 0 {
                                    self.save_state();
                                    self.status_message =
                                        format!("重複ノードを{}個整理しました", removed);
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("選択輪郭と次を統合"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.union_contours_all_layers(index) {
                                            Ok(()) => {
                                                self.canvas.selected_contour = Some(index);
                                                self.save_state();
                                                self.status_message =
                                                    "輪郭を全マスターで統合しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui.button("全輪郭を統合").clicked() {
                                if let Some(name) = self.current_glyph.clone() {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.union_all_contours_all_layers() {
                                            Ok(()) => {
                                                self.canvas.selected_contour = Some(0);
                                                self.save_state();
                                                self.status_message =
                                                    "全輪郭を全マスターで統合しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("選択輪郭から次を削除"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.difference_contours_all_layers(index) {
                                            Ok(()) => {
                                                self.canvas.selected_contour = Some(index);
                                                self.save_state();
                                                self.status_message =
                                                    "輪郭を全マスターで差分処理しました"
                                                        .to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("選択輪郭と次の交差部分"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.intersection_contours_all_layers(index) {
                                            Ok(()) => {
                                                self.canvas.selected_contour = Some(index);
                                                self.save_state();
                                                self.status_message =
                                                    "交差部分を全マスターで残しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.canvas.selected_contour.is_some(),
                                    egui::Button::new("選択輪郭と次のXOR"),
                                )
                                .clicked()
                            {
                                if let (Some(name), Some(index)) =
                                    (self.current_glyph.clone(), self.canvas.selected_contour)
                                {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.xor_contours_all_layers(index) {
                                            Ok(()) => {
                                                self.canvas.selected_contour = Some(index);
                                                self.save_state();
                                                self.status_message =
                                                    "輪郭を全マスターでXOR処理しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                }
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("拡大"))
                                .clicked()
                            {
                                self.transform_selection(1.1, 0.0);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("縮小"))
                                .clicked()
                            {
                                self.transform_selection(0.9, 0.0);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("左右反転"))
                                .clicked()
                            {
                                self.flip_selection(true);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("上下反転"))
                                .clicked()
                            {
                                self.flip_selection(false);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("↺ 回転"))
                                .clicked()
                            {
                                self.transform_selection(1.0, -std::f64::consts::PI / 18.0);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("↻ 回転"))
                                .clicked()
                            {
                                self.transform_selection(1.0, std::f64::consts::PI / 18.0);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("水平整列"))
                                .clicked()
                            {
                                self.align_selection(true);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("垂直整列"))
                                .clicked()
                            {
                                self.align_selection(false);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("水平分布"))
                                .clicked()
                            {
                                self.distribute_selection(true);
                            }
                            if ui
                                .add_enabled(has_selection, egui::Button::new("垂直分布"))
                                .clicked()
                            {
                                self.distribute_selection(false);
                            }
                            if ui.button("字幅を右端に合わせる").clicked() {
                                self.fit_width_to_outline();
                            }
                            if ui.button("左余白を0に揃える").clicked() {
                                self.align_left_side_bearing();
                            }
                            if ui.button("アウトラインを中央配置").clicked() {
                                self.center_outline_in_width();
                            }
                        });
                });
            });
        });
    }
}
