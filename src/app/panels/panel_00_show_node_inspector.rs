use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_node_inspector(&mut self, ui: &mut egui::Ui) {
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
}
