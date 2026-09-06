use super::super::*;
use super::canvas_input::CanvasKeyboardRequests;

impl GlyphStudioApp {
    pub(crate) fn show_glyph_canvas(&mut self, ctx: &egui::Context) {
        self.prepare_canvas_state();
        egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(Color32::from_rgb(30, 30, 35)).inner_margin(0.0))
        .show(ctx, |ui| {
                let fit_view_requested = self.show_canvas_header(ui);
            let (response, painter, rect, origin) = self.draw_canvas_scene(ui, ctx);
            let CanvasKeyboardRequests {
                fit_view_requested,
                toggle_background_requested,
                toggle_side_glyphs_requested,
                undo_requested,
                redo_requested,
                delete_requested,
                escape_pressed,
                select_all_requested,
                select_all_glyphs_requested,
                toggle_guides_requested,
                toggle_contour_direction_requested,
                toggle_metrics_requested,
                toggle_node_indices_requested,
                toggle_all_masters_requested,
                node_action_requested,
                nudge,
                reset_view_requested,
                zoom_delta,
                new_tool,
            } = self.read_canvas_keyboard(ctx, rect, &response, fit_view_requested);
            if let Some(tool) = new_tool {
                self.current_tool = tool;
                if tool != Tool::Pen {
                    self.pen_state.cancel();
                    self.pen_drag_start = None;
                }
                if tool != Tool::Knife {
                    self.knife_first_cut = None;
                }
                self.master_map_drag = None;
            }

            if undo_requested {
                self.undo();
            }
            if redo_requested {
                self.redo();
            }
            if toggle_guides_requested {
                self.canvas.show_guidelines = !self.canvas.show_guidelines;
            }
            if toggle_background_requested {
                self.canvas.show_background_images = !self.canvas.show_background_images;
            }
            if toggle_side_glyphs_requested {
                self.show_side_glyphs = !self.show_side_glyphs;
                self.status_message = if self.show_side_glyphs {
                    "前後字形を表示しました (B)".to_string()
                } else {
                    "前後字形を非表示にしました (B)".to_string()
                };
            }
            if toggle_contour_direction_requested {
                self.canvas.show_contour_direction = !self.canvas.show_contour_direction;
            }
            if toggle_metrics_requested {
                self.canvas.show_metrics = !self.canvas.show_metrics;
            }
            if toggle_node_indices_requested {
                self.canvas.show_node_indices = !self.canvas.show_node_indices;
            }
            if toggle_all_masters_requested {
                if self.project.masters.len() >= 2 {
                    self.edit_all_masters = !self.edit_all_masters;
                    self.status_message = if self.edit_all_masters {
                        "全マスター編集を有効にしました (⌘⇧M)".to_string()
                    } else {
                        "全マスター編集を無効にしました (⌘⇧M)".to_string()
                    };
                } else {
                    self.status_message =
                        "全マスター編集には2つ以上のマスターが必要です".to_string();
                }
            }
            if self.current_tool == Tool::Select {
                if let Some(action) = node_action_requested {
                    self.apply_selected_node_action(action);
                }
            }

            if delete_requested {
                if let Some(target) = self.selected_guideline.take() {
                    let removed = match target {
                        GuidelineTarget::Global(index) => {
                            let removed = (index
                                < self
                                    .project
                                    .guidelines_for_master(&self.current_master_id)
                                    .len())
                                .then(|| {
                                    self.project
                                        .guidelines_for_master_mut(&self.current_master_id)
                                        .remove(index)
                                })
                                .is_some();
                            if removed && self.edit_all_masters {
                                for (master_id, guides) in
                                    &mut self.project.guidelines_by_master
                                {
                                    if master_id != &self.current_master_id
                                        && index < guides.len()
                                    {
                                        guides.remove(index);
                                    }
                                }
                            }
                            removed
                        }
                        GuidelineTarget::Glyph(index) => self
                            .current_glyph
                            .as_ref()
                            .and_then(|name| self.project.glyphs.get_mut(name))
                            .map(|glyph| {
                                let guides = glyph
                                    .guidelines_for_master_mut(&self.current_master_id);
                                let removed =
                                    (index < guides.len()).then(|| guides.remove(index)).is_some();
                                if removed && self.edit_all_masters {
                                    for (master_id, other_guides) in
                                        &mut glyph.master_guidelines
                                    {
                                        if master_id != &self.current_master_id
                                            && index < other_guides.len()
                                        {
                                            other_guides.remove(index);
                                        }
                                    }
                                }
                                removed
                            })
                            .unwrap_or(false),
                    };
                    if removed {
                        self.guideline_drag = None;
                        self.save_state();
                        self.status_message = "ガイドを削除しました".to_string();
                        return;
                    }
                }
            }
            if delete_requested
                && (!self.canvas.selected_points.is_empty()
                || self.canvas.selected_component.is_some())
            {
                if let Some(name) = self.current_glyph.clone() {
                    let mut component_indices = self.selected_component_indices();
                    component_indices.sort_unstable_by(|left, right| right.cmp(left));
                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                        if !component_indices.is_empty() {
                            for component_index in component_indices {
                                if component_index < glyph.components.len() {
                                    glyph.components.remove(component_index);
                                    for layer in glyph.layers.values_mut() {
                                        if component_index < layer.components.len() {
                                            layer.components.remove(component_index);
                                        }
                                    }
                                }
                            }
                            self.clear_canvas_selection();
                            self.save_state();
                            return;
                        }
                        let mut selected = self.canvas.selected_nodes.clone();
                        if selected.is_empty() {
                            if let Some(ci) = self.canvas.selected_contour {
                                selected = self
                                    .canvas
                                    .selected_points
                                    .iter()
                                    .map(|&pi| (ci, pi))
                                    .collect();
                            }
                        }
                        match glyph.remove_nodes_all_layers(&selected) {
                            Ok(()) => {
                                self.clear_geometry_selection();
                                self.save_state();
                            }
                            Err(error) => self.status_message = error,
                        }
                    }
                }
            }

            if escape_pressed {
                self.pen_state.cancel();
                self.pen_drag_start = None;
                self.knife_first_cut = None;
                self.guideline_drag = None;
                self.master_map_drag = None;
                self.clear_geometry_selection();
                self.selected_guideline = None;
            }

            if select_all_glyphs_requested {
                self.selected_glyphs = self
                    .project
                    .glyph_names_sorted()
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                self.clear_geometry_selection();
                self.status_message =
                    format!("{}グリフを選択しました", self.selected_glyphs.len());
            }

            if select_all_requested && self.current_tool == Tool::Select {
                if let Some(name) = &self.current_glyph {
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        self.canvas.selected_component = None;
                        self.canvas.selected_components.clear();
                        self.canvas.selected_nodes = glyph
                            .contours
                            .iter()
                            .enumerate()
                            .flat_map(|(ci, contour)| {
                                (0..contour.points.len()).map(move |pi| (ci, pi))
                            })
                            .collect();
                        self.canvas.selected_contour = glyph.contours.first().map(|_| 0);
                        self.canvas.selected_points = glyph
                            .contours
                            .first()
                            .map(|c| (0..c.points.len()).collect())
                            .unwrap_or_default();
                    }
                }
            }

            if let Some((dx, dy)) = nudge {
                if self.current_tool == Tool::Select
                    && (self.canvas.selected_component.is_some()
                        || !self.canvas.selected_points.is_empty())
                {
                    if let Some(name) = self.current_glyph.clone() {
                        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                            if let Some(component_index) = self.canvas.selected_component {
                                if self.edit_all_masters {
                                    if let Err(error) = glyph.translate_component_all_layers(
                                        component_index,
                                        dx,
                                        dy,
                                    ) {
                                        self.status_message = error;
                                    } else {
                                        self.save_state();
                                    }
                                } else if let Some(component) =
                                    glyph.components.get_mut(component_index)
                                {
                                    component.x_offset += dx;
                                    component.y_offset += dy;
                                    self.save_state();
                                }
                            } else {
                                let nodes: Vec<(usize, usize)> =
                                    if !self.canvas.selected_nodes.is_empty() {
                                        self.canvas.selected_nodes.clone()
                                    } else if let Some(ci) = self.canvas.selected_contour {
                                        self.canvas
                                            .selected_points
                                            .iter()
                                            .map(|&pi| (ci, pi))
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                if self.edit_all_masters && !nodes.is_empty() {
                                    if let Err(error) =
                                        glyph.translate_nodes_all_layers(&nodes, dx, dy)
                                    {
                                        self.status_message = error;
                                    } else {
                                        self.save_state();
                                    }
                                } else if !nodes.is_empty() {
                                    for (ci, contour) in glyph.contours.iter_mut().enumerate() {
                                        let indices: Vec<usize> = nodes
                                            .iter()
                                            .filter_map(|&(selected_ci, pi)| {
                                                (selected_ci == ci).then_some(pi)
                                            })
                                            .collect();
                                        if !indices.is_empty() {
                                            contour.translate_points(&indices, dx, dy);
                                        }
                                    }
                                    self.save_state();
                                }
                            }
                        }
                    }
                }
            }

            self.apply_canvas_view_requests(rect, zoom_delta, reset_view_requested, fit_view_requested);

            self.update_space_tool(ctx);
            self.show_canvas_context_menu(&response, rect);

            if let Some(mouse_pos) = response.hover_pos() {
                let (gx, gy) = self.canvas.screen_to_glyph(mouse_pos, origin);
                let (gx, gy) = self.canvas.snap_point(gx, gy);

                let middle_pan = ctx.input(|input| input.pointer.middle_down());
                if middle_pan {
                    if response.dragged() {
                        self.canvas.pan += response.drag_delta();
                    }
                } else {
                    match self.current_tool {
                    Tool::Select => {
                        self.handle_select_tool(
                            ctx,
                            &response,
                            origin,
                            mouse_pos,
                            gx,
                            gy,
                        );
                    }
                    Tool::Pen => {
                        if response.drag_started() {
                            if let Some(start) = response.interact_pointer_pos() {
                                let point = self.canvas.screen_to_glyph(start, origin);
                                self.pen_drag_start = Some(point);
                                self.pen_state.begin_drag(point.0, point.1);
                            }
                        }
                        if response.dragged() {
                            self.pen_state.update_drag(gx, gy);
                        }
                        if response.drag_stopped() {
                            if let (Some((start_x, start_y)), Some(end)) = (
                                self.pen_drag_start.take(),
                                response.interact_pointer_pos(),
                            ) {
                                let (handle_x, handle_y) =
                                    self.canvas.screen_to_glyph(end, origin);
                                self.pen_state.add_dragged_anchor(
                                    start_x, start_y, handle_x, handle_y,
                                );
                            }
                        } else if response.clicked() && !response.dragged() {
                            let is_off_curve = ctx.input(|i| i.modifiers.shift);

                            if !self.pen_state.is_drawing {
                                self.pen_state.start_path(gx, gy);
                            } else {
                                self.pen_state.add_point(gx, gy, is_off_curve);
                            }
                        }

                        if response.double_clicked() {
                            if let Some(points) = self.pen_state.finish_path() {
                                let on_curve_count =
                                    points.iter().filter(|point| point.is_on_curve()).count();
                                if on_curve_count < 3 {
                                    self.status_message =
                                        "輪郭には3つ以上のオンカーブ点が必要です".to_string();
                                } else if let Some(name) = &self.current_glyph {
                                    if let Some(contour_index) = self
                                        .project
                                        .add_contour_all_layers(name, Contour { points })
                                    {
                                        self.canvas.selected_contour = Some(contour_index);
                                        let point_count = self
                                            .project
                                            .glyphs
                                            .get(name)
                                            .and_then(|glyph| glyph.contours.get(contour_index))
                                            .map_or(0, |contour| contour.points.len());
                                        self.canvas.selected_points = (0..point_count).collect();
                                        self.canvas.selected_nodes = self
                                            .canvas
                                            .selected_points
                                            .iter()
                                            .map(|&point_index| (contour_index, point_index))
                                            .collect();
                                        self.save_state();
                                        self.status_message = "輪郭を作成しました".to_string();
                                    }
                                }
                            }
                        }
                    }
                    Tool::Knife => {
                        if response.clicked() {
                            if let Some(name) = self.current_glyph.clone() {
                                if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                    let mut best: Option<(usize, usize, f64, f32)> = None;
                                    for (ci, contour) in glyph.contours.iter().enumerate() {
                                        for start in 0..contour.points.len().saturating_sub(1) {
                                            if !contour.points[start].is_on_curve() {
                                                continue;
                                            }
                                            let mut end = start + 1;
                                            while end < contour.points.len()
                                                && !contour.points[end].is_on_curve()
                                            {
                                                end += 1;
                                            }
                                            if end >= contour.points.len() || end - start > 3 {
                                                continue;
                                            }
                                            let p = |i: usize| {
                                                Point::new(
                                                    contour.points[i].x,
                                                    contour.points[i].y,
                                                )
                                            };
                                            let mouse = Point::new(gx, gy);
                                            let nearest = match end - start {
                                                1 => Line::new(p(start), p(end))
                                                    .nearest(mouse, 0.01),
                                                2 => {
                                                    QuadBez::new(p(start), p(start + 1), p(end))
                                                        .nearest(mouse, 0.01)
                                                }
                                                3 => CubicBez::new(
                                                    p(start),
                                                    p(start + 1),
                                                    p(start + 2),
                                                    p(end),
                                                )
                                                .nearest(mouse, 0.01),
                                                _ => continue,
                                            };
                                            let distance = (nearest.distance_sq.sqrt()
                                                * self.canvas.zoom as f64)
                                                as f32;
                                            if best.is_none_or(|(_, _, _, d)| distance < d) {
                                                best = Some((ci, start, nearest.t, distance));
                                            }
                                        }
                                        if let Some(start) = contour
                                            .points
                                            .iter()
                                            .rposition(|point| point.is_on_curve())
                                        {
                                            let mut indices = vec![start];
                                            let mut index = (start + 1) % contour.points.len();
                                            while index != 0
                                                && !contour.points[index].is_on_curve()
                                                && indices.len() <= 3
                                            {
                                                indices.push(index);
                                                index = (index + 1) % contour.points.len();
                                            }
                                            if index == 0
                                                && indices.len() <= 3
                                                && contour.points[0].is_on_curve()
                                            {
                                                indices.push(0);
                                                let p = |i: usize| {
                                                    Point::new(
                                                        contour.points[i].x,
                                                        contour.points[i].y,
                                                    )
                                                };
                                                let mouse = Point::new(gx, gy);
                                                let nearest = match indices.len() - 1 {
                                                    1 => Line::new(p(indices[0]), p(0))
                                                        .nearest(mouse, 0.01),
                                                    2 => QuadBez::new(
                                                        p(indices[0]),
                                                        p(indices[1]),
                                                        p(0),
                                                    )
                                                    .nearest(mouse, 0.01),
                                                    3 => CubicBez::new(
                                                        p(indices[0]),
                                                        p(indices[1]),
                                                        p(indices[2]),
                                                        p(0),
                                                    )
                                                    .nearest(mouse, 0.01),
                                                    _ => unreachable!(),
                                                };
                                                let distance = (nearest.distance_sq.sqrt()
                                                    * self.canvas.zoom as f64)
                                                    as f32;
                                                if best.is_none_or(|(_, _, _, d)| distance < d)
                                                {
                                                    best =
                                                        Some((ci, start, nearest.t, distance));
                                                }
                                            }
                                        }
                                    }
                                    if let Some((ci, segment_start, t, distance)) = best {
                                        if distance <= 18.0_f32.powi(2) {
                                            {
                                                let Ok(insert_at) = glyph.split_segment_all_layers(
                                                    ci,
                                                    segment_start,
                                                    t,
                                                ) else {
                                                    return;
                                                };
                                                if let Some((first_ci, first_index)) =
                                                    self.knife_first_cut.take()
                                                {
                                                    if first_ci == ci {
                                                        let first_index =
                                                            (first_index + usize::from(insert_at <= first_index))
                                                                .min(glyph.contours[ci].points.len().saturating_sub(1));
                                                        if glyph
                                                            .cut_contour_all_layers(
                                                                ci,
                                                                first_index,
                                                                insert_at,
                                                            )
                                                            .is_ok()
                                                        {
                                                            self.canvas.selected_contour = Some(ci);
                                                            self.canvas.selected_points.clear();
                                                            self.canvas.selected_nodes.clear();
                                                            self.save_state();
                                                            self.status_message =
                                                                "輪郭を2つに分割しました".to_string();
                                                        } else {
                                                            self.knife_first_cut = Some((ci, insert_at));
                                                            self.status_message =
                                                                "この位置では分割できません。別の輪郭上をクリックしてください"
                                                                    .to_string();
                                                        }
                                                    } else {
                                                        self.knife_first_cut = Some((ci, insert_at));
                                                        self.status_message =
                                                            "1点目と同じ輪郭上をクリックしてください"
                                                                .to_string();
                                                    }
                                                } else {
                                                    self.knife_first_cut = Some((ci, insert_at));
                                                    self.canvas.selected_contour = Some(ci);
                                                    self.canvas.selected_points = vec![insert_at];
                                                    self.canvas.selected_nodes = vec![(ci, insert_at)];
                                                    self.save_state();
                                                    self.status_message =
                                                        "1点目を追加しました。もう1点クリックして輪郭を分割"
                                                            .to_string();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Tool::Hand => {
                        if response.dragged() {
                            self.canvas.pan += response.drag_delta();
                        }
                    }
                    Tool::Ruler => {
                        if response.drag_started() {
                            self.canvas.ruler_start = response.interact_pointer_pos();
                            self.canvas.ruler_end = self.canvas.ruler_start;
                        }
                        if response.dragged() {
                            self.canvas.ruler_end = response.interact_pointer_pos();
                        }
                        if response.drag_stopped() {
                            if let (Some(start), Some(end)) =
                                (self.canvas.ruler_start, self.canvas.ruler_end)
                            {
                                let (sx, sy) = self.canvas.screen_to_glyph(start, origin);
                                let (ex, ey) = self.canvas.screen_to_glyph(end, origin);
                                self.status_message = format!(
                                    "測定: Δx {:.1}, Δy {:.1}, 距離 {:.1}",
                                    ex - sx,
                                    ey - sy,
                                    ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt()
                                );
                            }
                        }
                    }
                }
                    }
                }

            self.show_canvas_cursor(&painter, &response, origin);
        });
    }
}
