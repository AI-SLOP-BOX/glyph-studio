use super::*;

impl GlyphStudioApp {
    pub(crate) fn validation_window(&mut self, ctx: &egui::Context) {
        if !self.validation_issues.is_empty() {
            let mut close_validation = false;
            let mut jump_to_glyph = None;
            let mut rerun_validation = false;
            let glyph_issue_count = self
                .validation_issues
                .iter()
                .filter(|issue| issue.glyph_name.is_some())
                .count();
            let visible_issue_count = if self.validation_glyphs_only {
                glyph_issue_count
            } else {
                self.validation_issues.len()
            };
            egui::Window::new("フォント検証結果")
                .resizable(true)
                .default_width(520.0)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "{}件の問題があります",
                        self.validation_issues.len()
                    ));
                    ui.small(format!(
                        "グリフ関連 {}件 / フォント全体 {}件",
                        glyph_issue_count,
                        self.validation_issues.len() - glyph_issue_count
                    ));
                    ui.checkbox(&mut self.validation_glyphs_only, "グリフ関連のみ");
                    ui.small(format!("表示中 {}件", visible_issue_count));
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("ペア").small().color(Color32::GRAY));
                        ui.add_space(72.0);
                        ui.label(egui::RichText::new("値").small().color(Color32::GRAY));
                        ui.add_space(42.0);
                        ui.small("負の値 = 詰める");
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for issue in self.validation_issues.iter().filter(|issue| {
                            !self.validation_glyphs_only || issue.glyph_name.is_some()
                        }) {
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    Color32::from_rgb(220, 80, 70),
                                    format!("• {}", issue.message),
                                );
                                if let Some(name) = issue.glyph_name.as_ref() {
                                    if ui.small_button("移動").clicked() {
                                        jump_to_glyph = Some(name.clone());
                                        close_validation = true;
                                    }
                                }
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("修正後に再検証").clicked() {
                            rerun_validation = true;
                        }
                        if ui.button("閉じる").clicked() {
                            close_validation = true;
                        }
                    });
                });
            if rerun_validation {
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
                    close_validation = true;
                    "再検証完了: 問題はありません".to_string()
                } else {
                    format!(
                        "再検証完了: {}件の問題があります",
                        self.validation_issues.len()
                    )
                };
            }
            if close_validation {
                self.validation_issues.clear();
            }
            if let Some(name) = jump_to_glyph {
                self.current_glyph = Some(name.clone());
                self.glyph_rename_input = name;
                self.clear_canvas_selection();
                self.status_message = format!(
                    "検証エラーのグリフへ移動しました: {}",
                    self.current_glyph.as_deref().unwrap_or_default()
                );
            }
        }
    }
}
