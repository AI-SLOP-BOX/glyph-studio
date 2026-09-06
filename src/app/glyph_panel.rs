use super::*;

impl GlyphStudioApp {
    pub(crate) fn glyph_panel(&mut self, ctx: &egui::Context) {
        if self.show_glyph_list {
            egui::SidePanel::left("glyph_list_panel")
                .default_width(250.0)
                .resizable(true)
                .width_range(220.0..=320.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("グリフ一覧");
                        ui.add_space(ui.available_width().max(0.0) - 26.0);
                        if ui
                            .small_button("×")
                            .on_hover_text("グリフ一覧を閉じる")
                            .clicked()
                        {
                            self.show_glyph_list = false;
                        }
                    });
                    ui.separator();
                    let new_selection = glyph_list::show_glyph_list(
                        ui,
                        &self.project,
                        &self.current_glyph,
                        &mut self.glyph_search,
                        &mut self.focus_glyph_search,
                        &mut self.glyph_sort_by_unicode,
                        &mut self.glyph_list_only_unassigned,
                        &mut self.glyph_list_grid_view,
                        &mut self.selected_glyphs,
                    );
                    if new_selection != self.current_glyph {
                        self.current_glyph = new_selection;
                        self.glyph_rename_input = self.current_glyph.clone().unwrap_or_default();
                        self.clear_geometry_selection();
                    }
                    if let Some(action) = glyph_list::show_glyph_actions(
                        ui,
                        &mut self.project,
                        &self.current_glyph,
                        &mut self.glyph_rename_input,
                        &mut self.selected_glyphs,
                    ) {
                        match action {
                            glyph_list::GlyphAction::Add(name) => {
                                self.current_glyph = Some(name);
                                self.save_state();
                            }
                            glyph_list::GlyphAction::Duplicate(_, name) => {
                                self.current_glyph = Some(name);
                                self.clear_geometry_selection();
                                self.save_state();
                            }
                            glyph_list::GlyphAction::DuplicateMany(names) => {
                                if let Some(name) = names.last().cloned() {
                                    self.current_glyph = Some(name);
                                }
                                self.selected_glyphs = names.into_iter().collect();
                                self.clear_geometry_selection();
                                self.status_message = format!(
                                    "{}個のグリフを複製しました",
                                    self.selected_glyphs.len()
                                );
                                self.save_state();
                            }
                            glyph_list::GlyphAction::Delete(name) => {
                                self.current_glyph = self
                                    .project
                                    .glyph_names_sorted()
                                    .first()
                                    .map(|s| s.to_string());
                                self.clear_geometry_selection();
                                self.status_message = format!("グリフを削除しました: {name}");
                                self.save_state();
                            }
                            glyph_list::GlyphAction::DeleteMany(names) => {
                                self.current_glyph = self
                                    .project
                                    .glyph_names_sorted()
                                    .first()
                                    .map(|s| s.to_string());
                                self.clear_canvas_selection();
                                self.status_message =
                                    format!("{}個のグリフを削除しました", names.len());
                                self.save_state();
                            }
                            glyph_list::GlyphAction::Move(_, _) => {
                                self.save_state();
                            }
                            glyph_list::GlyphAction::Rename(old_name, new_name) => {
                                if self.current_glyph.as_deref() == Some(old_name.as_str()) {
                                    self.current_glyph = Some(new_name.clone());
                                }
                                self.glyph_rename_input = new_name;
                                self.save_state();
                            }
                            glyph_list::GlyphAction::MetricsKeysApplied(count) => {
                                self.status_message =
                                    format!("メトリクスキーを{}グリフへ適用しました", count);
                                self.save_state();
                            }
                        }
                    }
                    ui.separator();
                    ui.collapsing("一括編集", |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.batch_glyphs_input)
                                .hint_text("A B C（空欄=全グリフ）"),
                        );
                        ui.add(
                            egui::TextEdit::multiline(&mut self.batch_unicode_input)
                                .desired_rows(2)
                                .hint_text("Unicode一括設定: A=U+0041\nB=U+0042"),
                        );
                        let batch_target_label = if !self.batch_glyphs_input.trim().is_empty() {
                            "対象: 入力欄のグリフ"
                        } else if self.selected_glyphs.is_empty() {
                            "対象: 全グリフ"
                        } else {
                            "対象: 選択中のグリフ"
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{}（字幅・余白・変形など）",
                                batch_target_label
                            ))
                            .small()
                            .color(Color32::LIGHT_BLUE),
                        );
                        if ui.button("Unicodeを一括設定").clicked() {
                            let mut assignments = Vec::new();
                            let mut parse_error = None;
                            for (line_number, line) in self.batch_unicode_input.lines().enumerate()
                            {
                                let line = line.trim();
                                if line.is_empty() || line.starts_with('#') {
                                    continue;
                                }
                                let Some((name, value)) = line.split_once('=') else {
                                    parse_error = Some(format!(
                                        "{}行目: グリフ名=U+XXXX形式で入力してください",
                                        line_number + 1
                                    ));
                                    break;
                                };
                                let value = value.trim();
                                let value = value
                                    .strip_prefix("U+")
                                    .or_else(|| value.strip_prefix("u+"))
                                    .unwrap_or(value);
                                let Ok(codepoint) = u32::from_str_radix(value, 16) else {
                                    parse_error =
                                        Some(format!("{}行目: Unicodeが不正です", line_number + 1));
                                    break;
                                };
                                assignments.push((name.trim().to_string(), codepoint));
                            }
                            let result = parse_error.map_or_else(
                                || self.project.set_unicode_assignments_strict(&assignments),
                                Err,
                            );
                            match result {
                                Ok(changed) if changed > 0 => {
                                    self.status_message =
                                        format!("{}グリフのUnicodeを設定しました", changed);
                                    self.save_state();
                                }
                                Ok(_) => {
                                    self.status_message =
                                        "Unicode設定: 変更できるグリフがありません".to_string();
                                }
                                Err(error) => {
                                    self.status_message = format!("Unicode設定エラー: {error}");
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut self.batch_width).speed(1.0));
                            if ui.button("字幅を一括設定").clicked() {
                                let names: Vec<String> =
                                    if self.batch_glyphs_input.trim().is_empty() {
                                        if self.selected_glyphs.is_empty() {
                                            self.project
                                                .glyph_names_sorted()
                                                .into_iter()
                                                .map(str::to_string)
                                                .collect()
                                        } else {
                                            self.selected_glyphs.iter().cloned().collect()
                                        }
                                    } else {
                                        self.batch_glyphs_input
                                            .split(|c: char| c == ',' || c.is_whitespace())
                                            .filter(|name| !name.is_empty())
                                            .map(str::to_string)
                                            .collect()
                                    };
                                if !self.batch_width.is_finite() || self.batch_width < 0.0 {
                                    self.status_message =
                                        "字幅設定エラー: 0以上の数値を指定してください".to_string();
                                } else {
                                    match self.project.set_widths_batch(
                                        names.iter().map(|name| (name, self.batch_width)),
                                    ) {
                                        Ok(changed) if changed > 0 => {
                                            self.status_message = format!(
                                                "{}グリフの字幅を一括設定しました",
                                                changed
                                            );
                                            self.save_state();
                                        }
                                        Ok(_) => {}
                                        Err(error) => {
                                            self.status_message =
                                                format!("字幅設定エラー: {error}");
                                        }
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("左右余白:");
                            ui.add(
                                egui::DragValue::new(&mut self.batch_left_side_bearing).speed(1.0),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.batch_right_side_bearing).speed(1.0),
                            );
                            if ui.button("一括適用").clicked() {
                                let names: Vec<String> =
                                    if self.batch_glyphs_input.trim().is_empty() {
                                        if self.selected_glyphs.is_empty() {
                                            self.project
                                                .glyph_names_sorted()
                                                .into_iter()
                                                .map(str::to_string)
                                                .collect()
                                        } else {
                                            self.selected_glyphs.iter().cloned().collect()
                                        }
                                    } else {
                                        self.batch_glyphs_input
                                            .split(|c: char| c == ',' || c.is_whitespace())
                                            .filter(|name| !name.is_empty())
                                            .map(str::to_string)
                                            .collect()
                                    };
                                if !self.batch_left_side_bearing.is_finite()
                                    || !self.batch_right_side_bearing.is_finite()
                                    || self.batch_left_side_bearing < 0.0
                                    || self.batch_right_side_bearing < 0.0
                                {
                                    self.status_message =
                                        "左右余白設定エラー: 0以上の数値を指定してください"
                                            .to_string();
                                } else {
                                    match self.project.set_side_bearings_batch(names.iter().map(
                                        |name| {
                                            (
                                                name,
                                                self.batch_left_side_bearing,
                                                self.batch_right_side_bearing,
                                            )
                                        },
                                    )) {
                                        Ok(changed) if changed > 0 => {
                                            self.status_message = format!(
                                                "{}グリフの左右余白を設定しました",
                                                changed
                                            );
                                            self.save_state();
                                        }
                                        Ok(_) => {}
                                        Err(error) => {
                                            self.status_message =
                                                format!("左右余白設定エラー: {error}");
                                        }
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("カーニングG:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_left_kerning_group)
                                    .hint_text("左グループ")
                                    .desired_width(100.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_right_kerning_group)
                                    .hint_text("右グループ")
                                    .desired_width(100.0),
                            );
                            if ui.button("一括設定").clicked() {
                                let names: Vec<String> =
                                    if self.batch_glyphs_input.trim().is_empty() {
                                        if self.selected_glyphs.is_empty() {
                                            self.project
                                                .glyph_names_sorted()
                                                .into_iter()
                                                .map(str::to_string)
                                                .collect()
                                        } else {
                                            self.selected_glyphs.iter().cloned().collect()
                                        }
                                    } else {
                                        self.batch_glyphs_input
                                            .split(|c: char| c == ',' || c.is_whitespace())
                                            .filter(|name| !name.is_empty())
                                            .map(str::to_string)
                                            .collect()
                                    };
                                match self.project.set_kerning_groups(
                                    &names,
                                    &self.batch_left_kerning_group,
                                    &self.batch_right_kerning_group,
                                ) {
                                    Ok(changed) if changed > 0 => {
                                        self.status_message = format!(
                                            "{}グリフのカーニンググループを設定しました",
                                            changed
                                        );
                                        self.save_state();
                                    }
                                    Ok(_) => {
                                        self.status_message =
                                            "カーニンググループに変更はありません".to_string();
                                    }
                                    Err(error) => {
                                        self.status_message =
                                            format!("カーニンググループ設定エラー: {error}");
                                    }
                                }
                            }
                        });
                        if ui.button("アウトライン右端へ字幅を一括フィット").clicked()
                        {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.project.fit_widths_to_outlines(&names);
                            if changed > 0 {
                                self.status_message = format!(
                                    "{}グリフの字幅をアウトラインへフィットしました",
                                    changed
                                );
                                self.save_state();
                            }
                        }
                        if ui.button("アウトラインを字幅中央へ一括配置").clicked() {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.project.center_glyphs_in_width(&names);
                            if changed > 0 {
                                self.status_message =
                                    format!("{}グリフを字幅中央へ配置しました", changed);
                                self.save_state();
                            }
                        }
                        if ui.button("コンポーネントアンカーを一括整列").clicked() {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.project.align_all_component_anchors(&names);
                            if changed > 0 {
                                self.status_message =
                                    format!("{}個のコンポーネントをアンカー整列しました", changed);
                                self.save_state();
                            }
                        }
                        if ui.button("全輪郭の向きを一括反転").clicked() {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.project.reverse_glyph_contours(&names);
                            if changed > 0 {
                                self.status_message =
                                    format!("{}グリフの輪郭方向を反転しました", changed);
                                self.save_state();
                            }
                        }
                        if ui.button("輪郭方向を一括自動調整").clicked() {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.project.normalize_glyph_winding(&names);
                            if changed > 0 {
                                self.status_message =
                                    format!("{}グリフの輪郭方向を自動調整しました", changed);
                                self.save_state();
                            }
                        }
                        if ui.button("コンポーネントを一括輪郭化").clicked() {
                            let names: Vec<String> = if self.batch_glyphs_input.trim().is_empty() {
                                if self.selected_glyphs.is_empty() {
                                    self.project
                                        .glyph_names_sorted()
                                        .into_iter()
                                        .map(str::to_string)
                                        .collect()
                                } else {
                                    self.selected_glyphs.iter().cloned().collect()
                                }
                            } else {
                                self.batch_glyphs_input
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            };
                            let changed = self.decompose_named_components(&names);
                            if changed > 0 {
                                self.status_message =
                                    format!("{}グリフのコンポーネントを輪郭化しました", changed);
                                self.save_state();
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.batch_dx)
                                    .speed(1.0)
                                    .prefix("X "),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.batch_dy)
                                    .speed(1.0)
                                    .prefix("Y "),
                            );
                            if ui.button("位置を一括移動").clicked() {
                                let names: Vec<String> =
                                    if self.batch_glyphs_input.trim().is_empty() {
                                        if self.selected_glyphs.is_empty() {
                                            self.project
                                                .glyph_names_sorted()
                                                .into_iter()
                                                .map(str::to_string)
                                                .collect()
                                        } else {
                                            self.selected_glyphs.iter().cloned().collect()
                                        }
                                    } else {
                                        self.batch_glyphs_input
                                            .split(|c: char| c == ',' || c.is_whitespace())
                                            .filter(|name| !name.is_empty())
                                            .map(str::to_string)
                                            .collect()
                                    };
                                let changed = self.project.translate_glyphs(
                                    &names,
                                    self.batch_dx,
                                    self.batch_dy,
                                );
                                if changed > 0 {
                                    self.status_message =
                                        format!("{}グリフを一括移動しました", changed);
                                    self.save_state();
                                }
                            }
                        });
                    });
                });
        }
    }
}
