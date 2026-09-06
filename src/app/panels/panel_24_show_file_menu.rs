use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_file_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("ファイル", |ui| {
            if ui.button("プロジェクトを開く...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Glyph Studio / Glyphs / UFO / Font",
                        &["json", "glyphs", "ufo", "ttf", "otf", "woff", "woff2"],
                    )
                    .pick_file()
                {
                    self.request_open_document_path(&path);
                }
                ui.close_menu();
            }
            if ui.button("SVGをグリフとして読み込む...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SVG", &["svg"])
                    .pick_file()
                {
                    match crate::core::load_svg(&path) {
                        Ok(mut imported) => {
                            if let Some((name, mut glyph)) = imported.glyphs.drain().next() {
                                let mut imported_name = name.clone();
                                let mut suffix = 2;
                                while self.project.glyphs.contains_key(&imported_name) {
                                    imported_name = format!("{name}.import{suffix}");
                                    suffix += 1;
                                }
                                glyph.name = imported_name.clone();
                                self.project.glyphs.insert(imported_name.clone(), glyph);
                                self.project.glyph_order.push(imported_name.clone());
                                self.current_glyph = Some(imported_name);
                                self.save_state();
                                self.status_message =
                                    format!("SVGをグリフとして追加しました: {}", path.display());
                            } else {
                                self.status_message = "SVGに有効なグリフがありません".into();
                            }
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                ui.close_menu();
            }
            if ui.button("TTF/OTF/WOFF/WOFF2を読み込む...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Font files", &["ttf", "otf", "woff", "woff2"])
                    .pick_file()
                {
                    let extension = path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .unwrap_or_default();
                    let result = if extension.eq_ignore_ascii_case("woff2") {
                        crate::core::load_woff2(&path)
                    } else if extension.eq_ignore_ascii_case("woff") {
                        crate::core::load_woff(&path)
                    } else {
                        crate::core::load_ttf(&path)
                    };
                    match result {
                        Ok(project) => {
                            self.project = project;
                            self.clear_canvas_selection();
                            self.current_master_id = self.project.default_master_id.clone();
                            self.current_glyph = self
                                .project
                                .glyph_names_sorted()
                                .first()
                                .map(|s| s.to_string());
                            self.history = History::new(100);
                            self.history.push(&self.project, &self.current_glyph);
                            self.saved_history_index = self.history.current_index;
                            self.saved_project = Some(self.project.clone());
                            self.project_path = None;
                            self.status_message =
                                format!("フォントを読み込みました: {}", path.display());
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                ui.close_menu();
            }
            if ui.button("プロジェクトを保存...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Glyph Studio Project", &["json"])
                    .save_file()
                {
                    match crate::core::save_project(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("プロジェクトを保存しました: {}", path.display());
                            self.project_path = Some(path.clone());
                            self.saved_history_index = self.history.current_index;
                            self.saved_project = Some(self.project.clone());
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                ui.close_menu();
            }
            if ui.button("Glyphs形式で書き出す...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Glyphs Project", &["glyphs"])
                    .save_file()
                {
                    match crate::core::save_glyphs(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("Glyphs形式で書き出しました: {}", path.display());
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            if ui.button("フォントを検証").clicked() {
                let mut issues = crate::core::validate_project_detailed(&self.project);
                if self.show_interpolation_overlay {
                    issues.extend(crate::core::validate_interpolation(
                        &self.project,
                        &self.interpolation_from_master,
                        &self.interpolation_to_master,
                    ));
                }
                self.validation_issues = issues.clone();
                self.status_message = if issues.is_empty() {
                    "検証完了: 問題はありません".to_string()
                } else {
                    format!("検証で{}件の問題: {}", issues.len(), issues[0].message)
                };
                ui.close_menu();
            }
            if ui.button("孤立レイヤーを整理").clicked() {
                let removed = self.project.remove_orphaned_layers();
                if removed > 0 {
                    self.save_state();
                }
                self.status_message = format!("孤立レイヤーを{}件整理しました", removed);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add(egui::Button::new("新規プロジェクト").shortcut_text("⌘N"))
                .clicked()
            {
                self.request_new_project();
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("UFOを開く...").shortcut_text("⌘O"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("UFO", &["ufo"])
                    .pick_folder()
                {
                    self.request_open_document_path(&path);
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("UFOを保存...").shortcut_text("⌘⇧S"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("UFO", &["ufo"])
                    .save_file()
                {
                    match crate::core::save_ufo(&self.project, &path) {
                        Ok(()) => {
                            self.status_message = format!("UFOを保存しました: {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = e;
                        }
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("主要フォント出力").strong());
            if ui.add(egui::Button::new("TTFをエクスポート...")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("TrueType Font", &["ttf"])
                    .save_file()
                {
                    match crate::core::export_ttf(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("TTFをエクスポートしました: {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = e;
                        }
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("静的OTFをエクスポート..."))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("OpenType Font", &["otf"])
                    .save_file()
                {
                    match crate::core::export_otf(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("OTFをエクスポートしました: {}", path.display())
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("CFF2 OTFをエクスポート..."))
                .on_hover_text("基準マスターから静的CFF2/OTFを書き出す")
                .clicked()
            {
                self.export_otf_cff2_file();
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("マスター別出力").strong());
            if ui
                .add(egui::Button::new("現在のマスターをOTF出力"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("OpenType Font", &["otf"])
                    .save_file()
                {
                    match crate::core::export_otf_for_master(
                        &self.project,
                        &self.current_master_id,
                        &path,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                format!("マスターOTFを出力しました: {}", path.display())
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("全マスターを個別OTF出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_otf_for_masters(&self.project, &directory) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個のマスターOTFを出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui.add(egui::Button::new("WOFFをエクスポート...")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Web Open Font Format", &["woff"])
                    .save_file()
                {
                    match crate::core::export_woff(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("WOFFをエクスポートしました: {}", path.display())
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("WOFF2をエクスポート..."))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Web Open Font Format 2", &["woff2"])
                    .save_file()
                {
                    match crate::core::export_woff2(&self.project, &path) {
                        Ok(()) => {
                            self.status_message =
                                format!("WOFF2をエクスポートしました: {}", path.display())
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("現在のマスターをWOFF2出力"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Web Open Font Format 2", &["woff2"])
                    .save_file()
                {
                    match crate::core::export_woff2_for_master(
                        &self.project,
                        &self.current_master_id,
                        &path,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                format!("マスターWOFF2を出力しました: {}", path.display())
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("全マスターを個別WOFF2出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_woff2_for_masters(&self.project, &directory) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個のマスターWOFF2を出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("現在のマスターをWOFF出力"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Web Open Font Format", &["woff"])
                    .save_file()
                {
                    match crate::core::export_woff_for_master(
                        &self.project,
                        &self.current_master_id,
                        &path,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                format!("マスターWOFFを出力しました: {}", path.display())
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("全マスターを個別WOFF出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_woff_for_masters(&self.project, &directory) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個のマスターWOFFを出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("現在のマスターをTTF出力"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("TrueType Font", &["ttf"])
                    .save_file()
                {
                    match crate::core::export_ttf_for_master(
                        &self.project,
                        &self.current_master_id,
                        &path,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                format!("マスターTTFを出力しました: {}", path.display());
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("補間インスタンス出力").strong());
            if ui
                .add(egui::Button::new("補間インスタンスをTTF出力..."))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("TrueType Font", &(["ttf"]))
                    .save_file()
                {
                    match crate::core::export_ttf_at_interpolation(
                        &self.project,
                        &self.interpolation_from_master,
                        &self.interpolation_to_master,
                        self.interpolation_factor as f64,
                        &path,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                format!("補間インスタンスを出力しました: {}", path.display());
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("補間インスタンス3種を一括出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_interpolation_set(
                        &self.project,
                        &self.interpolation_from_master,
                        &self.interpolation_to_master,
                        &[0.25, 0.5, 0.75],
                        &directory,
                    ) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個の補間インスタンスを出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            ui.horizontal(|ui| {
                ui.label("補間率(%):");
                ui.add(egui::TextEdit::singleline(
                    &mut self.interpolation_batch_factors,
                ));
            });
            if ui
                .add(egui::Button::new("指定した補間率を一括出力..."))
                .clicked()
            {
                let factors: Vec<f64> = self
                    .interpolation_batch_factors
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter_map(|value| value.parse::<f64>().ok())
                    .map(|value| value / 100.0)
                    .collect();
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_interpolation_set(
                        &self.project,
                        &self.interpolation_from_master,
                        &self.interpolation_to_master,
                        &factors,
                        &directory,
                    ) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個の補間インスタンスを出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("全マスターを個別TTF出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_ttf_for_masters(&self.project, &directory) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}個のマスターTTFを出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("SVG出力").strong());
            if ui.add(egui::Button::new("SVGをエクスポート...")).clicked() {
                if let Some(name) = self.current_glyph.clone() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SVG", &["svg"])
                        .save_file()
                    {
                        match crate::core::export_svg_with_palette(
                            &self.project,
                            &name,
                            self.preview_color_palette,
                            &path,
                        ) {
                            Ok(()) => {
                                self.status_message =
                                    format!("SVGをエクスポートしました: {}", path.display())
                            }
                            Err(e) => self.status_message = e,
                        }
                    }
                }
                ui.close_menu();
            }
            if ui.add(egui::Button::new("全グリフをSVG出力...")).clicked() {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_svg_with_palette(
                        &self.project,
                        self.preview_color_palette,
                        &directory,
                    ) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}グリフをSVG出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("現在のマスターを全SVG出力..."))
                .clicked()
            {
                if let Some(directory) = rfd::FileDialog::new().pick_folder() {
                    match crate::core::export_all_svg_for_master_with_palette(
                        &self.project,
                        &self.current_master_id,
                        self.preview_color_palette,
                        &directory,
                    ) {
                        Ok(count) => {
                            self.status_message = format!(
                                "{}グリフをマスター別SVG出力しました: {}",
                                count,
                                directory.display()
                            );
                        }
                        Err(e) => self.status_message = e,
                    }
                }
                ui.close_menu();
            }
        });
    }
}
