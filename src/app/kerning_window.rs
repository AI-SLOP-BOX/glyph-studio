use super::*;

impl GlyphStudioApp {
    pub(crate) fn kerning_window(&mut self, ctx: &egui::Context) {
        if self.show_kerning_window {
            let kerning_before = self.project.clone();
            let mut close_kerning = false;
            let mut remove_pair = None;
            egui::Window::new("カーニング一覧")
                .open(&mut self.show_kerning_window)
                .resizable(true)
                .default_width(560.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("検索");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kerning_pair_filter)
                                .hint_text("左右グリフ名またはUnicode")
                                .desired_width(220.0),
                        );
                        if !self.kerning_pair_filter.is_empty()
                            && ui.small_button("×").on_hover_text("検索をクリア").clicked()
                        {
                            self.kerning_pair_filter.clear();
                        }
                        let master_name = self
                            .project
                            .masters
                            .iter()
                            .find(|master| master.id == self.current_master_id)
                            .map(|master| master.name.as_str())
                            .unwrap_or(self.current_master_id.as_str());
                        ui.label(format!("{master_name} · {}ペア", self.project.kerning.len()));
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let filter = self.kerning_pair_filter.trim().to_ascii_lowercase();
                        let mut pairs: Vec<_> = self.project.kerning.keys().cloned().collect();
                        pairs.sort();
                        let mut visible_pair_count = 0;
                        if pairs.is_empty() {
                            ui.colored_label(
                                Color32::from_gray(160),
                                "カーニングペアはまだありません。プロパティからペアを追加してください。",
                            );
                        }
                        for (left, right) in pairs {
                            let names = format!("{left} {right}").to_ascii_lowercase();
                            let chars = [left.as_str(), right.as_str()]
                                .iter()
                                .filter_map(|name| {
                                    self.project
                                        .glyphs
                                        .get(*name)
                                        .and_then(|glyph| glyph.unicode)
                                        .and_then(char::from_u32)
                                })
                                .collect::<String>();
                            if !filter.is_empty()
                                && !names.contains(&filter)
                                && !chars.to_ascii_lowercase().contains(&filter)
                            {
                                continue;
                            }
                            visible_pair_count += 1;
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        self.current_glyph.as_deref() == Some(left.as_str()),
                                        format!("{left} → {right}"),
                                    )
                                    .clicked()
                                {
                                    self.current_glyph = Some(left.clone());
                                    self.kerning_right = right.clone();
                                    self.feature_left = left.clone();
                                    self.feature_right = right.clone();
                                    if let Some(value) =
                                        self.project.kerning.get(&(left.clone(), right.clone()))
                                    {
                                        self.feature_kerning_value = format!("{value:.0}");
                                    }
                                    self.show_properties = true;
                                    self.status_message =
                                        format!("{} → {} を編集対象にしました", left, right);
                                }
                                if !chars.is_empty() {
                                    ui.label(
                                        egui::RichText::new(chars.clone())
                                            .size(18.0)
                                            .color(Color32::from_rgb(225, 225, 235)),
                                    )
                                    .on_hover_text("実際のUnicode文字によるペア表示");
                                }
                                let has_group = self
                                    .project
                                    .glyphs
                                    .get(&left)
                                    .is_some_and(|glyph| !glyph.left_kerning_group.trim().is_empty())
                                    || self
                                        .project
                                        .glyphs
                                        .get(&right)
                                        .is_some_and(|glyph| !glyph.right_kerning_group.trim().is_empty());
                                ui.small(if has_group { "例外" } else { "明示" })
                                    .on_hover_text(if has_group {
                                        "グループ指定を持つグリフの明示的な例外ペア"
                                    } else {
                                        "明示的に設定されたペア"
                                    });
                                if let Some(value) =
                                    self.project.kerning.get_mut(&(left.clone(), right.clone()))
                                {
                                    ui.add(
                                        egui::DragValue::new(value)
                                            .speed(1.0)
                                            .range(-2000.0..=2000.0)
                                            .suffix(" u"),
                                    );
                                }
                                if ui.small_button("プレビュー").clicked() {
                                    let pair_text = [left.as_str(), right.as_str()]
                                        .iter()
                                        .filter_map(|name| {
                                            self.project
                                                .glyphs
                                                .get(*name)
                                                .and_then(|glyph| glyph.unicode)
                                                .and_then(char::from_u32)
                                        })
                                        .collect::<String>();
                                    self.preview_text = if pair_text.is_empty() {
                                        format!("{left} {right}")
                                    } else {
                                        pair_text
                                    };
                                    self.show_preview = true;
                                }
                                if ui.small_button("削除").clicked() {
                                    remove_pair = Some((left.clone(), right.clone()));
                                }
                            });
                        }
                        if !filter.is_empty() {
                            ui.small(format!("表示: {visible_pair_count}件"));
                            if visible_pair_count == 0 {
                                ui.colored_label(
                                    Color32::from_gray(160),
                                    "一致するペアがありません。検索語を変更してください。",
                                );
                            }
                        }
                    });
                    if ui.button("閉じる").clicked() {
                        close_kerning = true;
                    }
                });
            if close_kerning {
                self.show_kerning_window = false;
            }
            if let Some(pair) = remove_pair {
                self.project.kerning.remove(&pair);
            }
            if self.project != kerning_before {
                self.save_state();
            }
        }
    }
}
