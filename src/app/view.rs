use super::*;

impl eframe::App for GlyphStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        let title = self
            .project_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| {
                let marker = if dirty { " *" } else { "" };
                format!("{}{} — Glyph Studio", name.to_string_lossy(), marker)
            })
            .unwrap_or_else(|| {
                format!(
                    "Glyph Studio{} — 未保存のプロジェクト",
                    if dirty { " *" } else { "" }
                )
            });
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        let wants_keyboard_input = ctx.wants_keyboard_input();
        let dropped_paths = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if let Some(path) = dropped_paths.first() {
            self.request_open_document_path(path);
        }
        if !wants_keyboard_input
            && ctx.input(|input| input.modifiers.command && input.key_pressed(Key::F))
        {
            self.show_glyph_list = true;
            self.focus_glyph_search = true;
        }
        if !wants_keyboard_input && ctx.input(|input| input.key_pressed(Key::Slash)) {
            self.show_glyph_list = true;
            self.focus_glyph_search = true;
        }
        let export_ufo_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.modifiers.shift
                && input.key_pressed(Key::S)
        });
        if export_ufo_shortcut {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("UFO", &["ufo"])
                .save_file()
            {
                match io::save_ufo(&self.project, &path) {
                    Ok(()) => {
                        self.status_message = format!("UFOを保存しました: {}", path.display());
                    }
                    Err(error) => self.status_message = error,
                }
            }
        }
        let save_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::S)
                && !input.modifiers.shift
        });
        if save_shortcut {
            self.save_project_file();
        }
        let export_ttf_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::E)
                && !input.modifiers.shift
        });
        if export_ttf_shortcut {
            self.export_ttf_file();
        }
        let new_project_shortcut = ctx.input(|input| {
            !wants_keyboard_input && input.modifiers.command && input.key_pressed(Key::N)
        });
        if new_project_shortcut {
            self.request_new_project();
        }
        let open_project_shortcut = ctx.input(|input| {
            !wants_keyboard_input && input.modifiers.command && input.key_pressed(Key::O)
        });
        if open_project_shortcut {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Glyph Studio / Glyphs / UFO / Font",
                    &["json", "glyphs", "ufo", "ttf", "otf", "woff", "woff2"],
                )
                .pick_file()
            {
                self.request_open_document_path(&path);
            }
        }
        let duplicate_glyph_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && !input.modifiers.shift
                && input.key_pressed(Key::D)
        });
        let duplicate_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.modifiers.shift
                && input.key_pressed(Key::D)
                && self.current_glyph.is_some()
                && self.canvas.selected_component.is_some()
        });
        if duplicate_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                if self.project.duplicate_component_all_layers(&name, index) {
                    self.canvas.selected_component = self
                        .project
                        .glyphs
                        .get(&name)
                        .map(|glyph| glyph.components.len().saturating_sub(1));
                    self.save_state();
                    self.status_message = "コンポーネントを複製しました (⌘⇧D)".to_string();
                }
            }
        }
        if duplicate_glyph_shortcut {
            let count = self.duplicate_selected_glyphs();
            if count > 0 {
                self.status_message = format!("{}個のグリフを複製しました", count);
            }
        }
        let copy_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::C)
                && self.canvas.selected_component.is_some()
        });
        if copy_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                self.component_clipboard = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.components.get(index).cloned());
            }
        }
        let cut_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::X)
                && self.canvas.selected_component.is_some()
        });
        if cut_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                let component = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.components.get(index).cloned());
                if let Some(component) = component {
                    if self
                        .project
                        .remove_component_all_layers(&name, index)
                        .is_ok()
                    {
                        self.component_clipboard = Some(component);
                        self.clear_geometry_selection();
                        self.save_state();
                    }
                }
            }
        }
        let cut_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::X)
                && self.canvas.selected_component.is_none()
                && self.canvas.selected_contour.is_some()
        });
        if cut_contour_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_contour)
            {
                let contour = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.contours.get(index).cloned());
                if let Some(contour) = contour {
                    if self.project.remove_contour_all_layers(&name, index).is_ok() {
                        self.contour_clipboard = Some(contour);
                        self.clear_geometry_selection();
                        self.save_state();
                    }
                }
            }
        }
        let copy_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::C)
                && self.canvas.selected_component.is_none()
                && self.canvas.selected_contour.is_some()
        });
        if copy_contour_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_contour)
            {
                self.contour_clipboard = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.contours.get(index).cloned());
            }
        }
        let paste_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::V)
                && self.component_clipboard.is_some()
                && self.current_glyph.is_some()
        });
        if paste_component_shortcut {
            if let (Some(name), Some(component)) =
                (self.current_glyph.clone(), self.component_clipboard.clone())
            {
                if let Some(index) = self.project.add_component_all_layers(&name, component) {
                    self.canvas.selected_component = Some(index);
                    self.canvas.selected_components = vec![index];
                    self.canvas.selected_points.clear();
                    self.canvas.selected_nodes.clear();
                    self.canvas.selected_contour = None;
                    self.save_state();
                }
            }
        }
        let paste_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::V)
                && self.canvas.selected_component.is_none()
                && self.contour_clipboard.is_some()
                && self.current_glyph.is_some()
        });
        if paste_contour_shortcut {
            if let (Some(name), Some(mut contour)) =
                (self.current_glyph.clone(), self.contour_clipboard.clone())
            {
                for point in &mut contour.points {
                    point.x += 50.0;
                    point.y += 50.0;
                }
                if let Some(index) = self.project.add_contour_all_layers(&name, contour) {
                    let point_count = self
                        .project
                        .glyphs
                        .get(&name)
                        .and_then(|glyph| glyph.contours.get(index))
                        .map_or(0, |contour| contour.points.len());
                    self.canvas.selected_contour = Some(index);
                    self.canvas.selected_points = (0..point_count).collect();
                    self.canvas.selected_nodes = self
                        .canvas
                        .selected_points
                        .iter()
                        .map(|&point| (index, point))
                        .collect();
                    self.save_state();
                }
            }
        }
        let delete_selected_glyphs_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && (input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace))
                && self.selected_glyphs.len() > 1
        });
        if delete_selected_glyphs_shortcut {
            let names: Vec<String> = self.selected_glyphs.iter().cloned().collect();
            for name in &names {
                self.project.remove_glyph(name);
            }
            self.current_glyph = self
                .project
                .glyph_names_sorted()
                .first()
                .map(|name| name.to_string());
            self.clear_canvas_selection();
            self.save_state();
            self.status_message = format!("{}個のグリフを削除しました", names.len());
        }
        self.show_menu_bar(ctx);
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
                self.validation_issues = crate::export::validate_project_detailed(&self.project);
                if self.show_interpolation_overlay {
                    self.validation_issues
                        .extend(crate::export::validate_interpolation(
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

        if self.show_shortcuts {
            egui::Window::new("ショートカット")
                .open(&mut self.show_shortcuts)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("shortcut_grid")
                        .num_columns(2)
                        .spacing(Vec2::new(18.0, 6.0))
                        .show(ui, |ui| {
                            for (key, action) in [
                                ("V", "選択ツール"),
                                ("P", "ペンツール"),
                                ("K", "ナイフツール"),
                                ("H", "ハンドツール"),
                                ("R", "定規ツール"),
                                ("I", "背景画像表示"),
                                ("B", "前後字形表示"),
                                ("D", "輪郭方向表示"),
                                ("M", "メトリクス表示"),
                                ("N", "ノード番号表示"),
                                ("S / C / T", "スムーズ / コーナー / オン・オフ曲線"),
                                ("⌘Z", "取り消す"),
                                ("⌘⇧Z", "やり直す"),
                                ("⌘S", "プロジェクト保存"),
                                ("⌘E", "検証してTTFを書き出し"),
                                ("⌘C / ⌘V", "輪郭・部品コピー／貼り付け"),
                                ("⌘⇧D", "選択中コンポーネントを全マスターへ複製"),
                                ("/ / ⌘F", "グリフ検索へフォーカス"),
                                ("Tab / PageUp / PageDown", "前後のグリフへ移動"),
                                ("⌘↑ / ⌘↓", "前後のマスターへ移動"),
                                ("⌘⇧M", "全マスター編集の切り替え"),
                                ("Shift + ドラッグ", "移動軸を水平／垂直に固定"),
                                ("Option + ドラッグ", "選択部品を複製して移動"),
                                ("Command + 回転", "部品を15度刻みで回転"),
                                ("中ボタン + ドラッグ", "ツールを切り替えずにパン"),
                                ("右クリック", "キャンバス操作メニュー"),
                                ("選択 + ドラッグ", "字幅・LSB・RSBをキャンバス上で調整"),
                            ] {
                                ui.label(egui::RichText::new(key).monospace().strong());
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                });
        }
        self.show_tool_bar(ctx);
        self.show_status_bar(ctx);

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

        if self.show_preview {
            egui::TopBottomPanel::bottom("preview_panel")
                .default_height(180.0)
                .resizable(true)
                .height_range(140.0..=360.0)
                .show(ctx, |ui| {
                    let mut preview_feature_tags = vec![
                        "liga", "kern", "mark", "mkmk", "calt", "rvrn", "ccmp", "locl", "rlig",
                        "salt", "frac", "sups", "subs", "vert", "ss01",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                    for (tag, _) in
                        crate::export::extract_feature_blocks(&self.project.feature_source())
                    {
                        let tag = String::from_utf8_lossy(&tag.to_be_bytes()).to_string();
                        if !preview_feature_tags.iter().any(|item| item == &tag) {
                            preview_feature_tags.push(tag);
                        }
                    }
                    preview_feature_tags.sort();
                    // Keep the control strip usable when the side panels leave
                    // only a narrow preview width. Glyphs-style workflows
                    // need feature toggles and spacing controls to remain
                    // discoverable instead of disappearing off-screen.
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("プレビュー");
                        if self.show_interpolation_overlay {
                            let from_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.interpolation_from_master)
                                .map(|master| master.name.as_str())
                                .unwrap_or("始点");
                            let to_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.interpolation_to_master)
                                .map(|master| master.name.as_str())
                                .unwrap_or("終点");
                            ui.label(
                                egui::RichText::new(format!(
                                    "比較: {from_name} → {to_name} ({:.0}%)",
                                    self.interpolation_factor * 100.0
                                ))
                                .small()
                                .color(Color32::LIGHT_BLUE),
                            );
                        }
                        ui.add(
                            egui::TextEdit::multiline(&mut self.preview_text)
                                .desired_width(260.0)
                                .desired_rows(2)
                                .hint_text("テキストを入力（改行対応）"),
                        );
                        ui.label("機能:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.preview_features)
                                .desired_width(150.0)
                                .hint_text("liga,salt,kern"),
                        );
                        if ui
                            .small_button("標準")
                            .on_hover_text("liga・kern・markを有効化")
                            .clicked()
                        {
                            self.preview_features = "liga,kern,mark".to_string();
                        }
                        if ui
                            .small_button("全OFF")
                            .on_hover_text("OpenType機能をすべて無効化")
                            .clicked()
                        {
                            self.preview_features.clear();
                        }
                        for tag in &preview_feature_tags {
                            let enabled = preview_feature_enabled(&self.preview_features, tag);
                            if ui
                                .selectable_label(enabled, tag)
                                .on_hover_text(format!("{tag} のON/OFF"))
                                .clicked()
                            {
                                toggle_preview_feature(&mut self.preview_features, tag);
                            }
                        }
                        ui.add(
                            egui::Slider::new(&mut self.preview_scale, 0.015..=0.12).text("サイズ"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.preview_line_spacing, 0.5..=2.5)
                                .text("行間"),
                        );
                        ui.checkbox(&mut self.preview_vertical_metrics, "縦メトリクス");
                        ui.checkbox(&mut self.preview_dark_background, "暗い背景");
                        ui.label("基準");
                        egui::ComboBox::from_id_salt("spacing_reference")
                            .selected_text(self.spacing_reference.to_string())
                            .show_ui(ui, |ui| {
                                for reference in ['H', 'O', 'n', 'o'] {
                                    ui.selectable_value(
                                        &mut self.spacing_reference,
                                        reference,
                                        reference.to_string(),
                                    );
                                }
                            });
                        for sample in ["HH", "HO", "nn", "oo"] {
                            if ui.small_button(sample).clicked() {
                                self.show_preview = true;
                                self.preview_text = sample.to_string();
                            }
                        }
                        if ui
                            .small_button("左右確認")
                            .on_hover_text("現在グリフを基準字形で挟んでスペーシング確認")
                            .clicked()
                        {
                            self.show_preview = true;
                            let current = self
                                .current_glyph
                                .as_deref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| glyph.unicode)
                                .and_then(char::from_u32)
                                .unwrap_or('□');
                            self.preview_text = format!(
                                "{}{current}{}",
                                self.spacing_reference, self.spacing_reference
                            );
                        }
                    });
                    let preview_background = if self.preview_dark_background {
                        Color32::from_rgb(25, 27, 31)
                    } else {
                        Color32::from_rgb(245, 246, 248)
                    };
                    ui.painter().rect_filled(
                        ui.available_rect_before_wrap(),
                        0.0,
                        preview_background,
                    );
                    let mut preview_clicked: Option<String> = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for preview_line in self.preview_text.split('\n') {
                                    if preview_line.is_empty() {
                                        let blank_line_height = (self.project.metadata.units_per_em
                                            as f32
                                            * self.preview_scale
                                            * self.preview_line_spacing)
                                            .clamp(50.0, 320.0);
                                        ui.allocate_space(Vec2::new(
                                            ui.available_width().max(1.0),
                                            blank_line_height,
                                        ));
                                        continue;
                                    }
                                    egui::ScrollArea::horizontal().show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let mut previous_name: Option<String> = None;
                                            let mut previous_origin: Option<Pos2> = None;
                                            let kern_enabled = preview_feature_enabled(
                                                &self.preview_features,
                                                "kern",
                                            );
                                            let names = preview_glyph_names(
                                                &self.project,
                                                preview_line,
                                                &self.preview_features,
                                            );
                                            for name in names {
                                                let mut drawn_origin = None;
                                                if kern_enabled {
                                                    if let Some(previous) = &previous_name {
                                                        if let Some(kern) = self
                                                            .project
                                                            .kerning_for_glyphs(previous, &name)
                                                        {
                                                            ui.add_space(
                                                                (kern as f32 * 0.04)
                                                                    .clamp(-40.0, 80.0),
                                                            );
                                                        }
                                                    }
                                                }
                                                if let Some(glyph) = self.project.glyphs.get(&name)
                                                {
                                                    let multi_axis_layer =
                                                        self.multi_axis_preview_layer(&name);
                                                    let fallback_layer = if self
                                                        .show_interpolation_overlay
                                                        && self.project.masters.len() >= 2
                                                    {
                                                        let from = self
                                                            .project
                                                            .masters
                                                            .iter()
                                                            .find(|master| {
                                                                master.id
                                                                    == self
                                                                        .interpolation_from_master
                                                            })
                                                            .map(|master| &master.id)
                                                            .unwrap_or(&self.project.masters[0].id);
                                                        let to = self
                                                            .project
                                                            .masters
                                                            .iter()
                                                            .find(|master| {
                                                                master.id
                                                                    == self.interpolation_to_master
                                                            })
                                                            .map(|master| &master.id)
                                                            .unwrap_or(
                                                                &self.project.masters[self
                                                                    .project
                                                                    .masters
                                                                    .len()
                                                                    - 1]
                                                                .id,
                                                            );
                                                        glyph.layers.get(from).and_then(|a| {
                                                            glyph.layers.get(to).and_then(|b| {
                                                                a.interpolate(
                                                                    b,
                                                                    self.interpolation_factor
                                                                        as f64,
                                                                )
                                                            })
                                                        })
                                                    } else {
                                                        glyph
                                                            .layers
                                                            .get(&self.current_master_id)
                                                            .cloned()
                                                    };
                                                    let interpolated_layer =
                                                        multi_axis_layer.or(fallback_layer);
                                                    let preview_layer = interpolated_layer.as_ref();
                                                    let contours = preview_layer
                                                        .map(|layer| &layer.contours)
                                                        .unwrap_or(&glyph.contours);
                                                    let components = preview_layer
                                                        .map(|layer| &layer.components)
                                                        .unwrap_or(&glyph.components);
                                                    let preview_width = preview_layer
                                                        .map(|layer| layer.width)
                                                        .unwrap_or(glyph.width);
                                                    let scale = self.preview_scale;
                                                    let cell_width = (preview_width as f32 * scale
                                                        + 8.0)
                                                        .clamp(20.0, 120.0);
                                                    let line_height =
                                                        (self.project.metadata.units_per_em as f32
                                                            * scale
                                                            * self.preview_line_spacing)
                                                            .clamp(50.0, 320.0);
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        Vec2::new(cell_width, line_height),
                                                        egui::Sense::click(),
                                                    );
                                                    let unicode = glyph
                                                        .unicode
                                                        .map(|value| format!("U+{value:04X}"))
                                                        .unwrap_or_else(|| {
                                                            "Unicode未設定".to_string()
                                                        });
                                                    let response = response
                                                        .on_hover_cursor(
                                                            egui::CursorIcon::PointingHand,
                                                        )
                                                        .on_hover_text(format!(
                                                            "{name} · {unicode}\nクリックで編集"
                                                        ));
                                                    if response.clicked() {
                                                        preview_clicked = Some(name.clone());
                                                    }
                                                    let painter = ui.painter();
                                                    let cell_color = if response.hovered() {
                                                        Color32::from_rgb(58, 68, 82)
                                                    } else if self.current_glyph.as_deref()
                                                        == Some(name.as_str())
                                                    {
                                                        Color32::from_rgb(48, 55, 68)
                                                    } else {
                                                        Color32::from_rgb(40, 40, 45)
                                                    };
                                                    painter.rect_filled(rect, 0.0, cell_color);
                                                    let baseline = rect.center().y + 100.0 * scale;
                                                    painter.line_segment(
                                                        [
                                                            Pos2::new(rect.left(), baseline),
                                                            Pos2::new(rect.right(), baseline),
                                                        ],
                                                        Stroke::new(
                                                            1.0_f32,
                                                            Color32::from_rgb(75, 105, 125),
                                                        ),
                                                    );
                                                    for metric in [
                                                        self.project.metadata.ascender,
                                                        self.project.metadata.descender,
                                                    ] {
                                                        let y = baseline - metric as f32 * scale;
                                                        painter.line_segment(
                                                            [
                                                                Pos2::new(rect.left(), y),
                                                                Pos2::new(rect.right(), y),
                                                            ],
                                                            Stroke::new(
                                                                1.0_f32,
                                                                Color32::from_rgb(65, 80, 90),
                                                            ),
                                                        );
                                                    }
                                                    if self.preview_vertical_metrics {
                                                        let vertical = self
                                                            .project
                                                            .vertical_metrics_for_glyph_in_master(
                                                                &name,
                                                                &self.current_master_id,
                                                            );
                                                        let vertical_origin_y = baseline
                                                            - vertical.top_side_bearing as f32
                                                                * scale;
                                                        let vertical_end_y = vertical_origin_y
                                                            - vertical.advance_height as f32
                                                                * scale;
                                                        let x = rect.right() - 8.0;
                                                        let metric_color =
                                                            Color32::from_rgb(90, 190, 205);
                                                        painter.line_segment(
                                                            [
                                                                Pos2::new(x, vertical_origin_y),
                                                                Pos2::new(x, vertical_end_y),
                                                            ],
                                                            Stroke::new(1.0_f32, metric_color),
                                                        );
                                                        painter.circle_filled(
                                                            Pos2::new(x, vertical_origin_y),
                                                            2.5,
                                                            metric_color,
                                                        );
                                                        painter.text(
                                                            Pos2::new(
                                                                rect.right() - 4.0,
                                                                vertical_end_y,
                                                            ),
                                                            egui::Align2::RIGHT_BOTTOM,
                                                            format!(
                                                                "v {}",
                                                                vertical.advance_height.round()
                                                            ),
                                                            egui::FontId::monospace(9.0),
                                                            metric_color,
                                                        );
                                                    }

                                                    let mut origin =
                                                        Pos2::new(rect.center().x, baseline);
                                                    if let Some(previous) = &previous_name {
                                                        if let Some(previous_origin) =
                                                            previous_origin
                                                        {
                                                            if let Some((dx, dy)) =
                                                                preview_mark_attachment(
                                                                    &self.project,
                                                                    previous,
                                                                    &name,
                                                                )
                                                            {
                                                                origin = Pos2::new(
                                                                    previous_origin.x + dx * scale,
                                                                    previous_origin.y - dy * scale,
                                                                );
                                                            }
                                                        }
                                                    }
                                                    drawn_origin = Some(origin);
                                                    for contour in contours {
                                                        let points = preview_contour_points(
                                                            contour, origin, scale,
                                                        );
                                                        if points.len() >= 3 {
                                                            painter.add(
                                                                egui::Shape::convex_polygon(
                                                                    points,
                                                                    Color32::WHITE,
                                                                    Stroke::NONE,
                                                                ),
                                                            );
                                                        }
                                                    }
                                                    for component in components {
                                                        let mut polygons = Vec::new();
                                                        preview_nested_component_polygons(
                                                            &self.project,
                                                            &component.base,
                                                            origin,
                                                            scale,
                                                            component_transform(component),
                                                            &mut std::collections::HashSet::new(),
                                                            &mut polygons,
                                                        );
                                                        for points in polygons {
                                                            if points.len() >= 3 {
                                                                painter.add(
                                                                    egui::Shape::convex_polygon(
                                                                        points,
                                                                        Color32::from_gray(190),
                                                                        Stroke::NONE,
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    let line_height =
                                                        (self.project.metadata.units_per_em as f32
                                                            * self.preview_scale
                                                            * self.preview_line_spacing)
                                                            .clamp(50.0, 320.0);
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        Vec2::new(50.0, line_height),
                                                        egui::Sense::click(),
                                                    );
                                                    let response = response
                                                        .on_hover_cursor(
                                                            egui::CursorIcon::PointingHand,
                                                        )
                                                        .on_hover_text(format!(
                                                            "{name}\nクリックで編集"
                                                        ));
                                                    if response.clicked() {
                                                        preview_clicked = Some(name.clone());
                                                    }
                                                    let painter = ui.painter();
                                                    let border_color = if response.hovered() {
                                                        Color32::from_rgb(230, 130, 110)
                                                    } else {
                                                        Color32::from_rgb(200, 90, 80)
                                                    };
                                                    painter.rect_stroke(
                                                        rect,
                                                        0.0,
                                                        Stroke::new(1.0_f32, border_color),
                                                        egui::StrokeKind::Outside,
                                                    );
                                                    painter.text(
                                                        rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        "?",
                                                        egui::FontId::proportional(24.0),
                                                        Color32::from_rgb(230, 120, 100),
                                                    );
                                                }
                                                previous_origin = drawn_origin;
                                                previous_name = Some(name);
                                            }
                                        });
                                    });
                                }
                            });
                        });
                    if let Some(name) = preview_clicked {
                        if self.current_glyph.as_deref() != Some(name.as_str()) {
                            self.current_glyph = Some(name.clone());
                            self.glyph_rename_input = name;
                            self.clear_canvas_selection();
                        }
                    }
                });
        }

        self.show_glyph_canvas(ctx);
        self.show_unsaved_open_dialog(ctx);
    }
}
