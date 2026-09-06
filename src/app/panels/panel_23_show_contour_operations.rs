use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_contour_operations(&mut self, ui: &mut egui::Ui) {
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
                            .add_enabled(can_copy, egui::Button::new("コンポーネントをコピー"))
                            .clicked()
                        {
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
                        if ui
                            .add_enabled(
                                self.component_clipboard.is_some() && self.current_glyph.is_some(),
                                egui::Button::new("コンポーネントを貼り付け"),
                            )
                            .clicked()
                        {
                            if let (Some(name), Some(component)) =
                                (self.current_glyph.clone(), self.component_clipboard.clone())
                            {
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
                                    self.canvas.selected_points = (0..point_count).collect();
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
                                    self.canvas.selected_points = (0..point_count).collect();
                                    self.canvas.selected_nodes = self
                                        .canvas
                                        .selected_points
                                        .iter()
                                        .map(|&pi| (new_ci, pi))
                                        .collect();
                                    self.save_state();
                                    self.status_message = "輪郭を貼り付けました".to_string();
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
                                self.status_message = "全輪郭の方向を調整しました".to_string();
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
                            self.status_message = format!("重複ノードを{}個整理しました", removed);
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
                                            "輪郭を全マスターで差分処理しました".to_string();
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
    }
}
