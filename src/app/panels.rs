use super::*;

impl GlyphStudioApp {
    pub(super) fn show_node_inspector(&mut self, ui: &mut egui::Ui) {
        if self.canvas.selected_nodes.is_empty() {
            return;
        }
        ui.separator();
        let mut batch_node_action = None;
        let mut node_translation = None;
        egui::CollapsingHeader::new("選択ノード")
            .default_open(true)
            .show(ui, |ui| {
                if self.canvas.selected_nodes.len() != 1 {
                    ui.label(format!(
                        "{}個のノードを選択中",
                        self.canvas.selected_nodes.len()
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("スムーズ").clicked() {
                            batch_node_action = Some(NodeAction::Smooth);
                        }
                        if ui.small_button("コーナー").clicked() {
                            batch_node_action = Some(NodeAction::Corner);
                        }
                        if ui.small_button("オン／オフ曲線").clicked() {
                            batch_node_action = Some(NodeAction::ToggleCurve);
                        }
                    });
                    ui.small("変更は全マスターへ反映されます");
                    ui.horizontal(|ui| {
                        ui.label("移動");
                        ui.add(
                            egui::DragValue::new(&mut self.selection_dx)
                                .prefix("X ")
                                .speed(1.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.selection_dy)
                                .prefix("Y ")
                                .speed(1.0),
                        );
                        if ui.small_button("適用").clicked()
                            && (self.selection_dx.abs() > f64::EPSILON
                                || self.selection_dy.abs() > f64::EPSILON)
                        {
                            node_translation = Some((self.selection_dx, self.selection_dy));
                        }
                    });
                    return;
                }
                let (contour_index, point_index) = self.canvas.selected_nodes[0];
                let Some(glyph_name) = self.current_glyph.as_deref() else {
                    return;
                };
                let Some(point) = self
                    .project
                    .glyphs
                    .get(glyph_name)
                    .and_then(|glyph| glyph.contours.get(contour_index))
                    .and_then(|contour| contour.points.get(point_index))
                    .copied()
                else {
                    ui.label("選択ノードが見つかりません");
                    return;
                };
                ui.small(format!(
                    "輪郭 {} / ノード {}・{}",
                    contour_index + 1,
                    point_index + 1,
                    if point.is_on_curve() {
                        "オンカーブ"
                    } else {
                        "オフカーブ"
                    }
                ));
                let mut x = point.x;
                let mut y = point.y;
                let mut smooth = point.smooth;
                let mut on_curve = point.is_on_curve();
                let mut apply_all_layers = false;
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut x).speed(1.0));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut y).speed(1.0));
                });
                ui.checkbox(&mut smooth, "スムーズ");
                if ui
                    .button(if on_curve {
                        "オフカーブ化"
                    } else {
                        "オンカーブ化"
                    })
                    .clicked()
                {
                    on_curve = !on_curve;
                }
                if ui.button("現在のノードを全マスターへ適用").clicked() {
                    apply_all_layers = true;
                }
                if (x - point.x).abs() > f64::EPSILON
                    || (y - point.y).abs() > f64::EPSILON
                    || smooth != point.smooth
                    || on_curve != point.is_on_curve()
                    || apply_all_layers
                {
                    if let Some(target) = self
                        .project
                        .glyphs
                        .get_mut(glyph_name)
                        .and_then(|glyph| glyph.contours.get_mut(contour_index))
                        .and_then(|contour| contour.points.get_mut(point_index))
                    {
                        target.x = x;
                        target.y = y;
                        target.smooth = smooth;
                        target.point_type = if on_curve {
                            crate::font_data::PointType::OnCurve
                        } else {
                            crate::font_data::PointType::OffCurve
                        };
                        if apply_all_layers {
                            if let Some(glyph) = self.project.glyphs.get_mut(glyph_name) {
                                for layer in glyph.layers.values_mut() {
                                    if let Some(target) = layer
                                        .contours
                                        .get_mut(contour_index)
                                        .and_then(|contour| contour.points.get_mut(point_index))
                                    {
                                        target.x = x;
                                        target.y = y;
                                        target.smooth = smooth;
                                        target.point_type = if on_curve {
                                            crate::font_data::PointType::OnCurve
                                        } else {
                                            crate::font_data::PointType::OffCurve
                                        };
                                    }
                                }
                            }
                        }
                        self.save_state();
                    }
                }
            });
        if let Some(action) = batch_node_action {
            self.apply_selected_node_action(action);
        }
        if let Some((dx, dy)) = node_translation {
            self.translate_selected_nodes_by(dx, dy);
        }
    }

    pub(super) fn translate_selected_nodes_by(&mut self, dx: f64, dy: f64) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let nodes = self.canvas.selected_nodes.clone();
        if nodes.is_empty() {
            return;
        }
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if self.edit_all_masters {
            if let Err(error) = glyph.translate_nodes_all_layers(&nodes, dx, dy) {
                self.status_message = error;
                return;
            }
        } else {
            for (contour_index, contour) in glyph.contours.iter_mut().enumerate() {
                let points: Vec<usize> = nodes
                    .iter()
                    .filter_map(|&(selected_contour, point_index)| {
                        (selected_contour == contour_index).then_some(point_index)
                    })
                    .collect();
                if !points.is_empty() {
                    contour.translate_points(&points, dx, dy);
                }
            }
        }
        self.save_state();
        self.status_message = format!("{}個のノードを数値移動しました", nodes.len());
    }

    pub(super) fn apply_selected_node_action(&mut self, action: NodeAction) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let nodes = if !self.canvas.selected_nodes.is_empty() {
            self.canvas.selected_nodes.clone()
        } else if let Some(contour_index) = self.canvas.selected_contour {
            self.canvas
                .selected_points
                .iter()
                .map(|&point_index| (contour_index, point_index))
                .collect()
        } else {
            return;
        };
        if nodes.is_empty() {
            return;
        }
        let result = self
            .project
            .glyphs
            .get_mut(&name)
            .map(|glyph| match action {
                NodeAction::Smooth => glyph.set_smooth_nodes_all_layers(&nodes, true),
                NodeAction::Corner => glyph.set_smooth_nodes_all_layers(&nodes, false),
                NodeAction::ToggleCurve => glyph.toggle_curve_nodes_all_layers(&nodes),
            });
        match result {
            Some(Ok(())) => {
                self.save_state();
                self.status_message = match action {
                    NodeAction::Smooth => "スムーズノードにしました".to_string(),
                    NodeAction::Corner => "コーナーノードにしました".to_string(),
                    NodeAction::ToggleCurve => "オン/オフ曲線を切り替えました".to_string(),
                };
            }
            Some(Err(error)) => self.status_message = error,
            None => {}
        }
    }

    pub(super) fn show_component_inspector(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
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
                                    if let Some((name, mut glyph)) = imported.glyphs.drain().next()
                                    {
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
                                        self.status_message = format!(
                                            "SVGをグリフとして追加しました: {}",
                                            path.display()
                                        );
                                    } else {
                                        self.status_message =
                                            "SVGに有効なグリフがありません".into();
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
                                    self.status_message =
                                        format!("UFOを保存しました: {}", path.display());
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
                            match crate::core::export_all_otf_for_masters(&self.project, &directory)
                            {
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
                            match crate::core::export_all_woff2_for_masters(
                                &self.project,
                                &directory,
                            ) {
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
                            match crate::core::export_all_woff_for_masters(
                                &self.project,
                                &directory,
                            ) {
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
                                    self.status_message = format!(
                                        "補間インスタンスを出力しました: {}",
                                        path.display()
                                    );
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
                            match crate::core::export_all_ttf_for_masters(&self.project, &directory)
                            {
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

                ui.menu_button("編集", |ui| {
                    if ui
                        .add(egui::Button::new("取り消す").shortcut_text("⌘Z"))
                        .clicked()
                    {
                        self.undo();
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new("やり直す").shortcut_text("⌘⇧Z"))
                        .clicked()
                    {
                        self.redo();
                        ui.close_menu();
                    }
                });

                ui.menu_button("表示", |ui| {
                    ui.checkbox(&mut self.show_glyph_list, "グリフ一覧");
                    ui.checkbox(&mut self.show_properties, "プロパティ");
                    ui.checkbox(&mut self.show_preview, "プレビュー");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("レイアウト")
                            .small()
                            .color(Color32::GRAY),
                    );
                    if ui
                        .button("標準")
                        .on_hover_text("一覧・キャンバス・プロパティ・プレビュー")
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
                        .button("プレビュー重視")
                        .on_hover_text("プロパティを隠して組み確認")
                        .clicked()
                    {
                        self.show_glyph_list = false;
                        self.show_properties = false;
                        self.show_preview = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.canvas.show_grid, "グリッド");
                    ui.checkbox(&mut self.canvas.snap_to_grid, "グリッドにスナップ");
                    ui.checkbox(&mut self.canvas.snap_to_guidelines, "ガイドにスナップ");
                    ui.checkbox(&mut self.canvas.snap_to_anchors, "アンカーにスナップ");
                    ui.horizontal(|ui| {
                        ui.label("間隔:");
                        ui.add(
                            egui::DragValue::new(&mut self.canvas.grid_size)
                                .speed(1.0)
                                .range(1.0..=1000.0),
                        );
                    });
                    ui.checkbox(&mut self.canvas.show_metrics, "メトリクス");
                    ui.checkbox(&mut self.canvas.show_guidelines, "ガイド (G)");
                    ui.checkbox(&mut self.canvas.show_background_images, "背景画像");
                    ui.checkbox(&mut self.canvas.show_contour_direction, "輪郭方向");
                    ui.checkbox(&mut self.canvas.show_node_indices, "ノード番号");
                    ui.checkbox(&mut self.canvas.show_anchors, "アンカー");
                });
            });
        });
    }

    pub(super) fn show_tool_bar(&mut self, ctx: &egui::Context) {
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

    pub(super) fn transform_selection(&mut self, scale: f64, angle: f64) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let component_indices = self.selected_component_indices();
        if component_indices.len() > 1 {
            let (sin, cos) = angle.sin_cos();
            let mut changed = false;
            if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                if self.edit_all_masters {
                    for component_index in component_indices {
                        match glyph.transform_component_all_layers(component_index, scale, angle) {
                            Ok(()) => changed = true,
                            Err(error) => self.status_message = error,
                        }
                    }
                } else {
                    for component_index in component_indices {
                        if let Some(component) = glyph.components.get_mut(component_index) {
                            let a = component.x_scale;
                            let b = component.xy_scale;
                            let c = component.yx_scale;
                            let d = component.y_scale;
                            component.x_scale = scale * (cos * a - sin * b);
                            component.xy_scale = scale * (sin * a + cos * b);
                            component.yx_scale = scale * (cos * c - sin * d);
                            component.y_scale = scale * (sin * c + cos * d);
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                self.save_state();
            }
            return;
        }
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            if let Some(index) = self.canvas.selected_component {
                if self.edit_all_masters {
                    match glyph.transform_component_all_layers(index, scale, angle) {
                        Ok(()) => self.save_state(),
                        Err(error) => self.status_message = error,
                    }
                    return;
                }
                if let Some(component) = glyph.components.get_mut(index) {
                    let (sin, cos) = angle.sin_cos();
                    let a = component.x_scale;
                    let b = component.xy_scale;
                    let c = component.yx_scale;
                    let d = component.y_scale;
                    component.x_scale = scale * (cos * a - sin * b);
                    component.xy_scale = scale * (sin * a + cos * b);
                    component.yx_scale = scale * (cos * c - sin * d);
                    component.y_scale = scale * (sin * c + cos * d);
                    self.save_state();
                }
                return;
            }
            let Some(contour_index) = self.canvas.selected_contour else {
                return;
            };
            if self.edit_all_masters {
                let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
                    self.canvas
                        .selected_points
                        .iter()
                        .map(|&point_index| (contour_index, point_index))
                        .collect()
                } else {
                    self.canvas.selected_nodes.clone()
                };
                if !nodes.is_empty() {
                    match glyph.transform_nodes_all_layers(&nodes, scale, angle) {
                        Ok(()) => self.save_state(),
                        Err(error) => self.status_message = error,
                    }
                }
                return;
            }
            let changed = if !self.canvas.selected_nodes.is_empty() {
                self.canvas.transform_selected_nodes(glyph, scale, angle)
            } else {
                self.canvas
                    .transform_selected(glyph, contour_index, scale, angle)
            };
            if changed {
                self.save_state();
            }
        }
    }

    pub(super) fn resize_component_from_handle(
        project: &FontProject,
        original: &GlyphComponent,
        handle: usize,
        target: (f64, f64),
    ) -> Option<GlyphComponent> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&original.base)?;
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
            (max_x, min_y),
        ];
        let handle = handle.min(3);
        let opposite = (handle + 2) % 4;
        let transform = |point: (f64, f64), component: &GlyphComponent| {
            (
                component.x_scale * point.0 + component.yx_scale * point.1 + component.x_offset,
                component.xy_scale * point.0 + component.y_scale * point.1 + component.y_offset,
            )
        };
        let fixed = transform(corners[opposite], original);
        let target_delta = (target.0 - fixed.0, target.1 - fixed.1);
        let local_delta = (
            corners[handle].0 - corners[opposite].0,
            corners[handle].1 - corners[opposite].1,
        );
        let x_axis = (
            original.x_scale * local_delta.0,
            original.xy_scale * local_delta.0,
        );
        let y_axis = (
            original.yx_scale * local_delta.1,
            original.y_scale * local_delta.1,
        );
        let determinant = x_axis.0 * y_axis.1 - x_axis.1 * y_axis.0;
        if determinant.abs() < 1.0e-9 {
            return None;
        }
        let scale_x = (target_delta.0 * y_axis.1 - target_delta.1 * y_axis.0) / determinant;
        let scale_y = (x_axis.0 * target_delta.1 - x_axis.1 * target_delta.0) / determinant;
        if !scale_x.is_finite() || !scale_y.is_finite() {
            return None;
        }
        let scale_x = scale_x.clamp(-100.0, 100.0);
        let scale_y = scale_y.clamp(-100.0, 100.0);
        let new_x_scale = original.x_scale * scale_x;
        let new_xy_scale = original.xy_scale * scale_x;
        let new_yx_scale = original.yx_scale * scale_y;
        let new_y_scale = original.y_scale * scale_y;
        let mut resized = original.clone();
        resized.x_scale = new_x_scale;
        resized.xy_scale = new_xy_scale;
        resized.yx_scale = new_yx_scale;
        resized.y_scale = new_y_scale;
        resized.x_offset = fixed.0
            - resized.x_scale * corners[opposite].0
            - resized.yx_scale * corners[opposite].1;
        resized.y_offset = fixed.1
            - resized.xy_scale * corners[opposite].0
            - resized.y_scale * corners[opposite].1;
        Some(resized)
    }

    pub(super) fn rotate_component_from_handle(
        project: &FontProject,
        original: &GlyphComponent,
        start: (f64, f64),
        target: (f64, f64),
        snap_angle: bool,
    ) -> Option<GlyphComponent> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&original.base)?;
        let center_local = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        let center = (
            original.x_scale * center_local.0
                + original.yx_scale * center_local.1
                + original.x_offset,
            original.xy_scale * center_local.0
                + original.y_scale * center_local.1
                + original.y_offset,
        );
        let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
        let target_angle = (target.1 - center.1).atan2(target.0 - center.0);
        let angle = target_angle - start_angle;
        let angle = if snap_angle {
            (angle / (std::f64::consts::PI / 12.0)).round() * (std::f64::consts::PI / 12.0)
        } else {
            angle
        };
        Self::rotate_component_by_angle(project, original, angle)
    }

    pub(super) fn rotate_component_by_angle(
        project: &FontProject,
        original: &GlyphComponent,
        angle: f64,
    ) -> Option<GlyphComponent> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&original.base)?;
        let center_local = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        let center = (
            original.x_scale * center_local.0
                + original.yx_scale * center_local.1
                + original.x_offset,
            original.xy_scale * center_local.0
                + original.y_scale * center_local.1
                + original.y_offset,
        );
        let (sin, cos) = angle.sin_cos();
        let mut rotated = original.clone();
        rotated.x_scale = cos * original.x_scale - sin * original.xy_scale;
        rotated.xy_scale = sin * original.x_scale + cos * original.xy_scale;
        rotated.yx_scale = cos * original.yx_scale - sin * original.y_scale;
        rotated.y_scale = sin * original.yx_scale + cos * original.y_scale;
        rotated.x_offset =
            center.0 - rotated.x_scale * center_local.0 - rotated.yx_scale * center_local.1;
        rotated.y_offset =
            center.1 - rotated.xy_scale * center_local.0 - rotated.y_scale * center_local.1;
        Some(rotated)
    }

    pub(super) fn fit_width_to_outline(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(max_x) = max_projected_outline_x(
            &self.project,
            &name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
        ) else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if max_x >= 0.0 && (glyph.width - max_x).abs() > f64::EPSILON {
            glyph.width = max_x;
            self.save_state();
            self.status_message = "字幅をアウトラインの右端に合わせました".to_string();
        }
    }

    pub(super) fn select_relative_glyph(&mut self, delta: isize) {
        let names = self.project.glyph_names_sorted();
        if names.is_empty() {
            return;
        }
        let current = self
            .current_glyph
            .as_deref()
            .and_then(|name| names.iter().position(|candidate| *candidate == name))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(names.len() as isize) as usize;
        let next_name = names[next].to_string();
        self.current_glyph = Some(next_name.clone());
        self.glyph_rename_input = next_name.clone();
        self.clear_geometry_selection();
        self.selected_glyphs.clear();
        self.status_message = format!("グリフ: {next_name}");
    }

    pub(super) fn select_relative_master(&mut self, delta: isize) {
        let master_ids: Vec<String> = self
            .project
            .masters
            .iter()
            .map(|master| master.id.clone())
            .collect();
        if master_ids.is_empty() {
            return;
        }
        let current = master_ids
            .iter()
            .position(|id| id == &self.current_master_id)
            .unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, master_ids.len() as isize - 1) as usize;
        if master_ids[next] != self.current_master_id {
            let previous = self.current_master_id.clone();
            self.project.switch_master(&previous, &master_ids[next]);
            self.current_master_id = master_ids[next].clone();
            self.selected_guideline = None;
            self.guideline_drag = None;
            self.project.sync_active_layer(&self.current_master_id);
            self.status_message = format!(
                "マスター: {}",
                self.project
                    .masters
                    .iter()
                    .find(|master| master.id == self.current_master_id)
                    .map(|master| master.name.as_str())
                    .unwrap_or(self.current_master_id.as_str())
            );
        }
    }

    pub(super) fn select_edge_glyph(&mut self, last: bool) {
        let names = self.project.glyph_names_sorted();
        let Some(name) = names.get(if last {
            names.len().saturating_sub(1)
        } else {
            0
        }) else {
            return;
        };
        let edge_name = (*name).to_string();
        self.current_glyph = Some(edge_name.clone());
        self.glyph_rename_input = edge_name.clone();
        self.clear_geometry_selection();
        self.selected_glyphs.clear();
        self.status_message = format!("グリフ: {edge_name}");
    }

    pub(super) fn align_left_side_bearing(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(min_x) = min_projected_outline_x(
            &self.project,
            &name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
        ) else {
            return;
        };
        if min_x.abs() <= f64::EPSILON {
            return;
        }
        let shift = -min_x;
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            glyph.translate_geometry(shift, 0.0);
            glyph.width += shift;
            self.save_state();
            self.status_message = "左余白を0に揃えました".to_string();
        }
    }

    pub(super) fn center_outline_in_width(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        if self.project.center_glyphs_in_width(&[name]) > 0 {
            self.save_state();
            self.status_message = "アウトラインを字幅の中央へ配置しました".to_string();
        }
    }

    pub(super) fn flip_selection(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let component_indices = self.selected_component_indices();
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if !component_indices.is_empty() {
            if self.edit_all_masters {
                for index in component_indices {
                    if let Err(error) = glyph.reflect_component_all_layers(index, horizontal) {
                        self.status_message = error;
                        return;
                    }
                }
            } else {
                for index in component_indices {
                    if let Some(component) = glyph.components.get_mut(index) {
                        if horizontal {
                            component.x_scale = -component.x_scale;
                            component.xy_scale = -component.xy_scale;
                        } else {
                            component.yx_scale = -component.yx_scale;
                            component.y_scale = -component.y_scale;
                        }
                    }
                }
            }
            self.save_state();
            return;
        }
        let Some(ci) = self.canvas.selected_contour else {
            return;
        };
        let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
            self.canvas
                .selected_points
                .iter()
                .map(|&pi| (ci, pi))
                .collect()
        } else {
            self.canvas.selected_nodes.clone()
        };
        if self.edit_all_masters {
            match glyph.reflect_nodes_all_layers(&nodes, horizontal) {
                Ok(()) => self.save_state(),
                Err(error) => self.status_message = error,
            }
            return;
        }
        let points: Vec<(f64, f64)> = nodes
            .iter()
            .filter_map(|&(node_ci, pi)| {
                glyph
                    .contours
                    .get(node_ci)
                    .and_then(|contour| contour.points.get(pi))
                    .map(|point| (point.x, point.y))
            })
            .collect();
        if points.is_empty() {
            return;
        }
        // Match the usual font-editor transform behavior: reflect around the
        // selection bounding box, not around the arithmetic mean of nodes.
        let min_x = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);
        let center = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        for (node_ci, pi) in nodes {
            if let Some(point) = glyph
                .contours
                .get_mut(node_ci)
                .and_then(|contour| contour.points.get_mut(pi))
            {
                if horizontal {
                    point.x = center.0 - (point.x - center.0);
                } else {
                    point.y = center.1 - (point.y - center.1);
                }
            }
        }
        for contour in &mut glyph.contours {
            contour.repair_smooth_handles();
        }
        self.save_state();
    }

    pub(super) fn component_visual_center(
        project: &FontProject,
        component: &GlyphComponent,
    ) -> Option<(f64, f64)> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&component.base)?;
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, min_y),
            (max_x, max_y),
        ];
        let transformed = corners.into_iter().map(|(x, y)| {
            (
                component.x_scale * x + component.yx_scale * y + component.x_offset,
                component.xy_scale * x + component.y_scale * y + component.y_offset,
            )
        });
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (x, y) in transformed {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        Some(((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
    }

    pub(super) fn translate_selected_components_by(&mut self, deltas: &[(usize, f64, f64)]) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        for &(index, dx, dy) in deltas {
            if self.edit_all_masters {
                if let Err(error) = glyph.translate_component_all_layers(index, dx, dy) {
                    self.status_message = error;
                    return;
                }
            } else if let Some(component) = glyph.components.get_mut(index) {
                component.x_offset += dx;
                component.y_offset += dy;
            }
        }
        self.save_state();
    }

    pub(super) fn align_selected_components(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get(&name) else {
            return;
        };
        let selected = self.selected_component_indices();
        let centers: Vec<(usize, f64, f64)> = selected
            .into_iter()
            .filter_map(|index| {
                let component = glyph.components.get(index)?;
                let (x, y) = Self::component_visual_center(&self.project, component)?;
                Some((index, x, y))
            })
            .collect();
        if centers.len() < 2 {
            return;
        }
        let target = centers
            .iter()
            .map(|(_, x, y)| if horizontal { *y } else { *x })
            .sum::<f64>()
            / centers.len() as f64;
        let deltas: Vec<(usize, f64, f64)> = centers
            .into_iter()
            .map(|(index, x, y)| {
                if horizontal {
                    (index, 0.0, target - y)
                } else {
                    (index, target - x, 0.0)
                }
            })
            .collect();
        self.translate_selected_components_by(&deltas);
        self.status_message = "選択部品を整列しました".to_string();
    }

    pub(super) fn distribute_selected_components(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get(&name) else {
            return;
        };
        let selected = self.selected_component_indices();
        let mut centers: Vec<(usize, f64, f64)> = selected
            .into_iter()
            .filter_map(|index| {
                let component = glyph.components.get(index)?;
                let (x, y) = Self::component_visual_center(&self.project, component)?;
                Some((index, x, y))
            })
            .collect();
        if centers.len() < 3 {
            return;
        }
        centers.sort_by(|left, right| {
            let left_value = if horizontal { left.1 } else { left.2 };
            let right_value = if horizontal { right.1 } else { right.2 };
            left_value.total_cmp(&right_value)
        });
        let first = if horizontal {
            centers[0].1
        } else {
            centers[0].2
        };
        let last = if horizontal {
            centers.last().map(|item| item.1).unwrap_or(first)
        } else {
            centers.last().map(|item| item.2).unwrap_or(first)
        };
        let step = (last - first) / (centers.len() - 1) as f64;
        let deltas: Vec<(usize, f64, f64)> = centers
            .into_iter()
            .enumerate()
            .map(|(position, (index, x, y))| {
                let target = first + step * position as f64;
                if horizontal {
                    (index, target - x, 0.0)
                } else {
                    (index, 0.0, target - y)
                }
            })
            .collect();
        self.translate_selected_components_by(&deltas);
        self.status_message = "選択部品を分布しました".to_string();
    }

    pub(super) fn align_selection(&mut self, horizontal: bool) {
        if !self.selected_component_indices().is_empty() {
            self.align_selected_components(horizontal);
            return;
        }
        let (Some(name), Some(ci)) = (self.current_glyph.clone(), self.canvas.selected_contour)
        else {
            return;
        };
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
                self.canvas
                    .selected_points
                    .iter()
                    .map(|&pi| (ci, pi))
                    .collect()
            } else {
                self.canvas.selected_nodes.clone()
            };
            if self.edit_all_masters {
                match glyph.align_nodes_all_layers(&nodes, horizontal) {
                    Ok(()) => self.save_state(),
                    Err(error) => self.status_message = error,
                }
                return;
            }
            let values: Vec<f64> = nodes
                .iter()
                .filter_map(|&(node_ci, pi)| {
                    glyph
                        .contours
                        .get(node_ci)
                        .and_then(|c| c.points.get(pi))
                        .map(|p| if horizontal { p.y } else { p.x })
                })
                .collect();
            if values.is_empty() {
                return;
            }
            let target = values.iter().copied().sum::<f64>() / values.len() as f64;
            for (node_ci, pi) in nodes {
                if let Some(point) = glyph
                    .contours
                    .get_mut(node_ci)
                    .and_then(|c| c.points.get_mut(pi))
                {
                    if horizontal {
                        point.y = target;
                    } else {
                        point.x = target;
                    }
                }
            }
            for contour in &mut glyph.contours {
                contour.repair_smooth_handles();
            }
            self.save_state();
        }
    }

    pub(super) fn distribute_selection(&mut self, horizontal: bool) {
        if !self.selected_component_indices().is_empty() {
            self.distribute_selected_components(horizontal);
            return;
        }
        let (Some(name), Some(ci)) = (self.current_glyph.clone(), self.canvas.selected_contour)
        else {
            return;
        };
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
                self.canvas
                    .selected_points
                    .iter()
                    .map(|&pi| (ci, pi))
                    .collect()
            } else {
                self.canvas.selected_nodes.clone()
            };
            if self.edit_all_masters {
                match glyph.distribute_nodes_all_layers(&nodes, horizontal) {
                    Ok(()) => self.save_state(),
                    Err(error) => self.status_message = error,
                }
                return;
            }
            let mut values: Vec<(f64, usize, usize)> = nodes
                .iter()
                .filter_map(|&(node_ci, pi)| {
                    glyph
                        .contours
                        .get(node_ci)
                        .and_then(|c| c.points.get(pi))
                        .map(|p| (if horizontal { p.x } else { p.y }, node_ci, pi))
                })
                .collect();
            if values.len() < 3 {
                return;
            }
            values.sort_by(|a, b| a.0.total_cmp(&b.0));
            let first = values.first().unwrap().0;
            let last = values.last().unwrap().0;
            let step = (last - first) / (values.len() - 1) as f64;
            for (index, (_, node_ci, pi)) in values.into_iter().enumerate() {
                if let Some(point) = glyph
                    .contours
                    .get_mut(node_ci)
                    .and_then(|c| c.points.get_mut(pi))
                {
                    if horizontal {
                        point.x = first + step * index as f64;
                    } else {
                        point.y = first + step * index as f64;
                    }
                }
            }
            for contour in &mut glyph.contours {
                contour.repair_smooth_handles();
            }
            self.save_state();
        }
    }
}
