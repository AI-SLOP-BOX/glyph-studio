use super::*;

impl GlyphStudioApp {
    pub(super) fn multi_axis_preview_layer(
        &self,
        name: &str,
    ) -> Option<crate::font_data::GlyphLayer> {
        if !self.show_interpolation_overlay || self.project.masters.len() < 4 {
            return None;
        }
        let mut axes = std::collections::BTreeSet::new();
        for master in &self.project.masters {
            axes.extend(master.axes.keys().cloned());
        }
        let axes: Vec<String> = axes.into_iter().collect();
        if axes.len() < 2 {
            return None;
        }
        let bounds = |tag: &str| {
            self.project
                .masters
                .iter()
                .filter_map(|master| master.axes.get(tag).copied())
                .fold(None::<(f64, f64)>, |result, value| {
                    Some(match result {
                        Some((min, max)) => (min.min(value), max.max(value)),
                        None => (value, value),
                    })
                })
        };
        let x = bounds(&axes[0])?;
        let y = bounds(&axes[1])?;
        self.project.interpolate_glyph_bilinear(
            name,
            &axes[0],
            &axes[1],
            x.0 + (x.1 - x.0) * self.interpolation_x_factor as f64,
            y.0 + (y.1 - y.0) * self.interpolation_y_factor as f64,
        )
    }

    pub(super) fn save_state(&mut self) {
        self.project.sync_active_layer(&self.current_master_id);
        self.history.push(&self.project, &self.current_glyph);
    }

    pub(super) fn clear_canvas_selection(&mut self) {
        self.canvas.selected_points.clear();
        self.canvas.selected_nodes.clear();
        self.canvas.selected_contour = None;
        self.canvas.selected_component = None;
        self.canvas.selected_components.clear();
        self.selected_glyphs.clear();
        self.contour_clipboard = None;
        self.component_clipboard = None;
        self.validation_issues.clear();
        self.selected_guideline = None;
    }

    pub(super) fn clear_geometry_selection(&mut self) {
        self.canvas.selected_points.clear();
        self.canvas.selected_nodes.clear();
        self.canvas.selected_contour = None;
        self.canvas.selected_component = None;
        self.canvas.selected_components.clear();
        self.component_resize = None;
        self.anchor_drag = None;
        self.spacing_drag = None;
    }

    pub(super) fn select_component(&mut self, index: usize, additive: bool) {
        if additive {
            if self.canvas.selected_components.is_empty() {
                if let Some(current) = self.canvas.selected_component {
                    self.canvas.selected_components.push(current);
                }
            }
            if let Some(position) = self
                .canvas
                .selected_components
                .iter()
                .position(|&selected| selected == index)
            {
                self.canvas.selected_components.remove(position);
            } else {
                self.canvas.selected_components.push(index);
            }
        } else {
            self.canvas.selected_components = vec![index];
        }
        self.canvas.selected_component = self.canvas.selected_components.last().copied();
    }

    pub(super) fn selected_component_indices(&self) -> Vec<usize> {
        if self.canvas.selected_components.is_empty() {
            self.canvas.selected_component.into_iter().collect()
        } else {
            self.canvas.selected_components.clone()
        }
    }

    pub(super) fn duplicate_selected_glyphs(&mut self) -> usize {
        let source_names: Vec<String> = if self.selected_glyphs.is_empty() {
            self.current_glyph.clone().into_iter().collect()
        } else {
            self.project
                .glyph_names_sorted()
                .into_iter()
                .filter(|name| {
                    self.selected_glyphs
                        .iter()
                        .any(|selected| selected == *name)
                })
                .map(str::to_string)
                .collect()
        };
        let mut duplicated_names = Vec::new();
        for source_name in source_names {
            if let Some(new_name) = self.project.duplicate_glyph(&source_name) {
                duplicated_names.push(new_name);
            }
        }
        if let Some(new_name) = duplicated_names.last().cloned() {
            self.current_glyph = Some(new_name);
            self.selected_glyphs = duplicated_names.into_iter().collect();
            self.save_state();
        }
        self.selected_glyphs.len()
    }

    pub(super) fn undo(&mut self) {
        if let Some(entry) = self.history.undo() {
            self.project = entry.project.clone();
            self.current_glyph = entry.current_glyph.clone();
            self.clear_geometry_selection();
            self.selected_glyphs
                .retain(|name| self.project.glyphs.contains_key(name));
            self.status_message = "取り消しました".to_string();
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(entry) = self.history.redo() {
            self.project = entry.project.clone();
            self.current_glyph = entry.current_glyph.clone();
            self.clear_geometry_selection();
            self.selected_glyphs
                .retain(|name| self.project.glyphs.contains_key(name));
            self.status_message = "やり直しました".to_string();
        }
    }

    pub(super) fn save_project_file(&mut self) {
        let path = self.project_path.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Glyph Studio Project", &["json"])
                .save_file()
        });
        if let Some(path) = path {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            let result = if extension.eq_ignore_ascii_case("glyphs") {
                io::save_glyphs(&self.project, &path)
            } else if extension.eq_ignore_ascii_case("ufo") {
                io::save_ufo(&self.project, &path)
            } else {
                io::save_project(&self.project, &path)
            };
            match result {
                Ok(()) => {
                    self.status_message = format!("プロジェクトを保存しました: {}", path.display());
                    self.project_path = Some(path);
                    self.saved_history_index = self.history.current_index;
                    self.saved_project = Some(self.project.clone());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub fn open_document_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let loaded = if extension.eq_ignore_ascii_case("glyphs") {
            io::load_glyphs(path)
        } else if extension.eq_ignore_ascii_case("ufo") {
            io::load_ufo(path)
        } else if extension.eq_ignore_ascii_case("woff2") {
            io::load_woff2(path)
        } else if extension.eq_ignore_ascii_case("woff") {
            io::load_woff(path)
        } else if matches!(extension.to_ascii_lowercase().as_str(), "ttf" | "otf") {
            io::load_ttf(path)
        } else {
            io::load_project(path)
        }?;
        self.project = loaded;
        self.clear_canvas_selection();
        self.current_master_id = self.project.default_master_id.clone();
        self.current_glyph = self
            .project
            .glyph_names_sorted()
            .first()
            .map(|name| name.to_string());
        self.glyph_rename_input = self.current_glyph.clone().unwrap_or_default();
        self.history = History::new(100);
        self.history.push(&self.project, &self.current_glyph);
        self.saved_history_index = self.history.current_index;
        self.saved_project = Some(self.project.clone());
        self.project_path = matches!(
            extension.to_ascii_lowercase().as_str(),
            "json" | "glyphs" | "ufo"
        )
        .then(|| path.to_path_buf());
        self.status_message = format!("ファイルを開きました: {}", path.display());
        Ok(())
    }

    pub(super) fn request_open_document_path(&mut self, path: &std::path::Path) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        if dirty {
            self.pending_open_path = Some(path.to_path_buf());
            self.show_unsaved_open_dialog = true;
        } else if let Err(error) = self.open_document_path(path) {
            self.status_message = format!("ファイルを開けませんでした: {error}");
        }
    }

    pub(super) fn create_new_project(&mut self) {
        self.project = FontProject::new();
        self.current_glyph = None;
        self.project_path = None;
        self.current_master_id = "regular".to_string();
        self.clear_canvas_selection();
        self.history = History::new(100);
        self.history.push(&self.project, &self.current_glyph);
        self.saved_history_index = self.history.current_index;
        self.saved_project = Some(self.project.clone());
        self.status_message = "新規プロジェクトを作成しました".to_string();
    }

    pub(super) fn request_new_project(&mut self) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        if dirty {
            self.pending_open_path = None;
            self.pending_new_project = true;
            self.show_unsaved_open_dialog = true;
        } else {
            self.create_new_project();
        }
    }

    pub(super) fn show_unsaved_open_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_open_dialog {
            return;
        }
        let mut action = None;
        egui::Window::new("未保存の変更")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("現在のプロジェクトに未保存の変更があります。");
                ui.label(if self.pending_new_project {
                    "新規プロジェクトを作成する前に、変更を保存しますか？"
                } else {
                    "別のファイルを開く前に、変更を保存しますか？"
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("保存して開く").clicked() {
                        self.save_project_file();
                        let saved = self
                            .saved_project
                            .as_ref()
                            .is_some_and(|project| project == &self.project);
                        if saved {
                            action = Some(true);
                        }
                    }
                    if ui.button("破棄して開く").clicked() {
                        action = Some(true);
                    }
                    if ui.button("キャンセル").clicked() {
                        action = Some(false);
                    }
                });
            });
        if let Some(open) = action {
            self.show_unsaved_open_dialog = false;
            if open {
                if let Some(path) = self.pending_open_path.take() {
                    if let Err(error) = self.open_document_path(&path) {
                        self.status_message = format!("ファイルを開けませんでした: {error}");
                    }
                } else if self.pending_new_project {
                    self.pending_new_project = false;
                    self.create_new_project();
                }
            } else {
                self.pending_open_path = None;
                self.pending_new_project = false;
            }
        }
    }

    pub(super) fn export_ttf_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("TrueType Font", &["ttf"])
            .save_file()
        {
            match crate::export::export_ttf(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("TTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub(super) fn export_otf_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenType Font", &["otf"])
            .save_file()
        {
            match crate::export::export_otf(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("OTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub(super) fn export_otf_cff2_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CFF2 OpenType Font", &["otf"])
            .save_file()
        {
            match crate::export::export_otf_cff2(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("CFF2 OTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub(super) fn export_woff2_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Web Open Font Format 2", &["woff2"])
            .save_file()
        {
            match crate::export::export_woff2(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("WOFF2を書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub(super) fn export_woff_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Web Open Font Format", &["woff"])
            .save_file()
        {
            match crate::export::export_woff(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("WOFFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }
}
