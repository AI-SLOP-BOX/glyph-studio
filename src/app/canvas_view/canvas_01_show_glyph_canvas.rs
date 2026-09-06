use super::super::*;
use super::canvas_input::CanvasKeyboardRequests;

impl GlyphStudioApp {
    pub(crate) fn show_glyph_canvas(&mut self, ctx: &egui::Context) {
        if self.canvas.selected_component.is_none() {
            self.canvas.selected_components.clear();
        }
        let referenced_backgrounds: HashSet<String> = self
            .project
            .background_images
            .values()
            .flat_map(|masters| masters.values())
            .filter(|path| !path.trim().is_empty())
            .cloned()
            .collect();
        self.background_cache
            .retain(|path, _| referenced_backgrounds.contains(path));
        let panel_frame = egui::Frame::default()
            .fill(Color32::from_rgb(30, 30, 35))
            .inner_margin(0.0);
        egui::CentralPanel::default()
        .frame(panel_frame)
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

            if let Some((delta, mouse_pos)) = zoom_delta {
                self.canvas.zoom_at(delta, mouse_pos, rect.center());
            }
            if reset_view_requested {
                self.canvas.zoom = 1.0;
                self.canvas.pan = Vec2::ZERO;
            }
            if fit_view_requested {
                self.fit_current_glyph_to_canvas(rect);
            }

            // Handle mouse interactions
            let space_down = ctx.input(|input| input.key_down(Key::Space));
            if space_down {
                if self.space_previous_tool.is_none() && self.current_tool != Tool::Hand {
                    self.space_previous_tool = Some(self.current_tool);
                    self.current_tool = Tool::Hand;
                }
            } else if let Some(previous_tool) = self.space_previous_tool.take() {
                self.current_tool = previous_tool;
            }
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
                        if response.drag_started() {
                            self.spacing_drag = self
                                .current_glyph
                                .as_ref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| {
                                    let x =
                                        self.canvas.glyph_to_screen(glyph.width, 0.0, origin).x;
                                    ((mouse_pos.x - x).abs() <= 8.0)
                                        .then_some(SpacingTarget::Advance)
                                })
                                .or_else(|| {
                                    self.current_glyph
                                        .as_ref()
                                        .and_then(|name| {
                                            self.project.outline_bounds_for_glyph(name)
                                        })
                                        .and_then(|(min_x, _, max_x, _)| {
                                            let left_x = self
                                                .canvas
                                                .glyph_to_screen(min_x, 0.0, origin)
                                                .x;
                                            let right_x = self
                                                .canvas
                                                .glyph_to_screen(max_x, 0.0, origin)
                                                .x;
                                            if (mouse_pos.x - left_x).abs() <= 8.0 {
                                                Some(SpacingTarget::LeftBearing)
                                            } else if (mouse_pos.x - right_x).abs() <= 8.0 {
                                                Some(SpacingTarget::RightBearing)
                                            } else {
                                                None
                                            }
                                        })
                                });
                            self.anchor_drag = if self.canvas.show_anchors {
                                self.current_glyph.as_ref().and_then(|name| {
                                self.project.glyphs.get(name).and_then(|glyph| {
                                    glyph
                                        .layers
                                        .get(&self.current_master_id)
                                        .and_then(|layer| {
                                            layer
                                                .anchors
                                                .iter()
                                                .enumerate()
                                                .find(|(_, anchor)| {
                                                    self.canvas
                                                        .glyph_to_screen(anchor.x, anchor.y, origin)
                                                        .distance(mouse_pos)
                                                        <= 9.0
                                                })
                                                .map(|(index, _)| AnchorTarget::Layer(index))
                                        })
                                        .or_else(|| {
                                            glyph.anchors.iter().enumerate().find(|(_, anchor)| {
                                                self.canvas
                                                    .glyph_to_screen(anchor.x, anchor.y, origin)
                                                    .distance(mouse_pos)
                                                    <= 9.0
                                            }).map(|(index, _)| AnchorTarget::Glyph(index))
                                        })
                                })
                                })
                            } else {
                                None
                            };
                            let near_guide = |guide: &crate::font_data::Guideline| {
                                let center =
                                    self.canvas.glyph_to_screen(guide.x, guide.y, origin);
                                let direction = Vec2::new(
                                    (-(guide.angle as f32).to_radians()).cos(),
                                    (-(guide.angle as f32).to_radians()).sin(),
                                );
                                let offset = mouse_pos - center;
                                let distance =
                                    (offset.x * direction.y - offset.y * direction.x).abs();
                                distance <= 8.0
                            };
                            self.guideline_drag = self
                                .current_glyph
                                .as_ref()
                                .and_then(|name| self.project.glyphs.get(name))
                                .and_then(|glyph| {
                                    glyph
                                        .guidelines_for_master(&self.current_master_id)
                                        .iter()
                                        .enumerate()
                                        .find(|(_, guide)| near_guide(guide))
                                })
                                .map(|(index, _)| GuidelineTarget::Glyph(index))
                                .or_else(|| {
                                    self.project
                                        .guidelines
                                        .iter()
                                        .enumerate()
                                        .find(|(_, guide)| near_guide(guide))
                                        .map(|(index, _)| GuidelineTarget::Global(index))
                                });
                            if self.guideline_drag.is_some() {
                                self.selected_guideline = self.guideline_drag;
                            }
                            if self.guideline_drag.is_some() || self.anchor_drag.is_some() {
                                self.spacing_drag = None;
                                self.canvas.selection_start = None;
                                self.canvas.selection_rect = None;
                            }
                            if self.spacing_drag.is_some() {
                                self.canvas.selection_start = None;
                                self.canvas.selection_rect = None;
                                self.canvas.selected_component = None;
                            }
                            let hit = if let (Some(name), Some(start)) =
                                (&self.current_glyph, response.interact_pointer_pos())
                            {
                                self.project.glyphs.get(name).and_then(|glyph| {
                                    self.canvas.hit_test(start, glyph, origin)
                                })
                            } else {
                                None
                            };
                            let component_hit = if hit.is_none() {
                                self.current_glyph
                                    .as_ref()
                                    .and_then(|name| self.project.glyphs.get(name))
                                    .and_then(|glyph| {
                                        self.canvas.hit_test_component(
                                            response.interact_pointer_pos()?,
                                            &self.project,
                                            glyph,
                                            origin,
                                        )
                                    })
                            } else {
                                None
                            };
                            let mut component_hit = component_hit;
                            if let Some(component_index) = component_hit {
                                if ctx.input(|input| input.modifiers.alt)
                                    && !self.component_drag_duplicated
                                {
                                    if let Some(name) = self.current_glyph.clone() {
                                        if self.project.duplicate_component_all_layers(
                                            &name,
                                            component_index,
                                        ) {
                                            let new_index = self
                                                .project
                                                .glyphs
                                                .get(&name)
                                                .map(|glyph| glyph.components.len() - 1)
                                                .unwrap_or(component_index);
                                            component_hit = Some(new_index);
                                            self.component_drag_duplicated = true;
                                            self.save_state();
                                            self.status_message =
                                                "部品を複製しました（Optionドラッグ）".to_string();
                                        }
                                    }
                                }
                                {
                                    let handle = self.current_glyph.as_ref().and_then(|name| {
                                        response.interact_pointer_pos().and_then(|position| {
                                            self.project.glyphs.get(name).and_then(|glyph| {
                                                self.canvas.hit_test_component_handle(
                                                    position,
                                                    &self.project,
                                                    glyph,
                                                    component_index,
                                                    origin,
                                                )
                                            })
                                        })
                                    });
                                    if let (Some(name), Some(handle)) = (
                                        self.current_glyph.as_ref(),
                                        handle,
                                    ) {
                                        if let Some(component) = self
                                            .project
                                            .glyphs
                                            .get(name)
                                            .and_then(|glyph| glyph.components.get(component_index))
                                            .cloned()
                                        {
                                            self.component_resize =
                                                Some((component_index, handle, component));
                                        }
                                }
                            }
                            }
                            if self.guideline_drag.is_none() && self.spacing_drag.is_none() {
                                self.canvas.selection_start = (hit.is_none()
                                    && component_hit.is_none()
                                    && self.anchor_drag.is_none())
                                .then(|| response.interact_pointer_pos())
                                .flatten();
                                if let Some(component_index) = component_hit {
                                    self.select_component(
                                        component_index,
                                        ctx.input(|input| input.modifiers.shift),
                                    );
                                } else {
                                    self.canvas.selected_component = None;
                                    self.canvas.selected_components.clear();
                                }
                            }
                            self.canvas.selection_rect = None;
                        }
                        if response.clicked() {
                            if let Some(name) = &self.current_glyph {
                                if let Some(glyph) = self.project.glyphs.get(name) {
                                    if let Some((ci, pi)) =
                                        self.canvas.hit_test(mouse_pos, glyph, origin)
                                    {
                                        self.canvas.selected_component = None;
                                        self.canvas.selected_components.clear();
                                        let additive = ctx.input(|i| i.modifiers.shift);
                                        if additive {
                                            if let Some(pos) = self
                                                .canvas
                                                .selected_nodes
                                                .iter()
                                                .position(|&selected| selected == (ci, pi))
                                            {
                                                self.canvas.selected_nodes.remove(pos);
                                            } else {
                                                self.canvas.selected_nodes.push((ci, pi));
                                            }
                                        } else {
                                            self.canvas.selected_nodes = vec![(ci, pi)];
                                        }
                                        self.canvas.selected_points = self
                                            .canvas
                                            .selected_nodes
                                            .iter()
                                            .filter_map(|&(selected_ci, selected_pi)| {
                                                (selected_ci == ci).then_some(selected_pi)
                                            })
                                            .collect();
                                        self.canvas.selected_contour = Some(ci);
                                    } else {
                                        let component_hit = self
                                            .current_glyph
                                            .as_ref()
                                            .and_then(|name| self.project.glyphs.get(name))
                                            .and_then(|glyph| {
                                                self.canvas.hit_test_component(
                                                    mouse_pos,
                                                    &self.project,
                                                    glyph,
                                                    origin,
                                                )
                                            });
                                        if let Some(component_index) = component_hit {
                                            self.select_component(
                                                component_index,
                                                ctx.input(|input| input.modifiers.shift),
                                            );
                                        } else {
                                            self.canvas.selected_component = None;
                                            self.canvas.selected_components.clear();
                                        }
                                        self.canvas.selected_points.clear();
                                        self.canvas.selected_nodes.clear();
                                        self.canvas.selected_contour = None;
                                    }
                                }
                            }
                        }
                        if response.double_clicked() {
                            if let Some(name) = self.current_glyph.clone() {
                                let hit = self.project.glyphs.get(&name).and_then(|glyph| {
                                    self.canvas.hit_test(mouse_pos, glyph, origin)
                                });
                                let segment_hit = self.project.glyphs.get(&name).and_then(|glyph| {
                                    self.canvas.hit_test_segment(mouse_pos, glyph, origin)
                                });
                                if let Some((ci, pi)) = hit {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let smooth = glyph
                                            .contours
                                            .get(ci)
                                            .and_then(|contour| contour.points.get(pi))
                                            .map(|point| !point.smooth);
                                        if let Some(smooth) = smooth {
                                            match glyph.set_smooth_nodes_all_layers(&[(ci, pi)], smooth) {
                                                Ok(()) => {
                                                    self.canvas.selected_contour = Some(ci);
                                                    self.canvas.selected_points = vec![pi];
                                                    self.canvas.selected_nodes = vec![(ci, pi)];
                                                    self.save_state();
                                                }
                                                Err(error) => {
                                                    self.status_message = error;
                                                }
                                            }
                                        }
                                    }
                                } else if let Some((ci, start_index, factor)) = segment_hit {
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        match glyph.split_segment_all_layers(
                                            ci,
                                            start_index,
                                            factor,
                                        ) {
                                            Ok(inserted_index) => {
                                                self.canvas.selected_component = None;
                                                self.canvas.selected_components.clear();
                                                self.canvas.selected_contour = Some(ci);
                                                self.canvas.selected_points =
                                                    vec![inserted_index];
                                                self.canvas.selected_nodes =
                                                    vec![(ci, inserted_index)];
                                                self.save_state();
                                                self.status_message =
                                                    "曲線上にノードを追加しました".to_string();
                                            }
                                            Err(error) => self.status_message = error,
                                        }
                                    }
                                } else if let Some(component_index) = self
                                    .project
                                    .glyphs
                                    .get(&name)
                                    .and_then(|glyph| {
                                        self.canvas.hit_test_component(
                                            mouse_pos,
                                            &self.project,
                                            glyph,
                                            origin,
                                        )
                                    })
                                {
                                    if let Some(base) = self
                                        .project
                                        .glyphs
                                        .get(&name)
                                        .and_then(|glyph| glyph.components.get(component_index))
                                        .map(|component| component.base.clone())
                                    {
                                        self.current_glyph = Some(base.clone());
                                        self.glyph_rename_input = base.clone();
                                        self.clear_canvas_selection();
                                        self.status_message =
                                            format!("参照先グリフを開きました: {base}");
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(target) = self.anchor_drag {
                                if let Some(name) = self.current_glyph.clone() {
                                    let mut anchor_guidelines = self
                                        .project
                                        .guidelines_for_master(&self.current_master_id)
                                        .to_vec();
                                    if let Some(glyph) = self.project.glyphs.get(&name) {
                                        anchor_guidelines.extend(
                                            glyph
                                                .guidelines_for_master(&self.current_master_id)
                                                .iter()
                                                .cloned(),
                                        );
                                    }
                                    let (anchor_x, anchor_y) = self
                                        .canvas
                                        .snap_point_to_guidelines(gx, gy, &anchor_guidelines);
                                    if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                                        let anchor = match target {
                                            AnchorTarget::Glyph(index) => glyph.anchors.get_mut(index),
                                            AnchorTarget::Layer(index) => glyph
                                                .layers
                                                .get_mut(&self.current_master_id)
                                                .and_then(|layer| layer.anchors.get_mut(index)),
                                        };
                                        if let Some(anchor) = anchor {
                                            anchor.x = anchor_x;
                                            anchor.y = anchor_y;
                                        }
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(target) = self.guideline_drag {
                                let guide = match target {
                                    GuidelineTarget::Global(index) => {
                                        self.project
                                            .guidelines_for_master_mut(&self.current_master_id)
                                            .get_mut(index)
                                    }
                                    GuidelineTarget::Glyph(index) => self
                                        .current_glyph
                                        .as_ref()
                                        .and_then(|name| self.project.glyphs.get_mut(name))
                                        .and_then(|glyph| {
                                            glyph
                                                .guidelines_for_master_mut(&self.current_master_id)
                                                .get_mut(index)
                                        }),
                                };
                                let mut delta = None;
                                if let Some(guide) = guide {
                                    let before = (guide.x, guide.y, guide.angle);
                                    if ctx.input(|input| input.modifiers.shift) {
                                        let center = self
                                            .canvas
                                            .glyph_to_screen(guide.x, guide.y, origin);
                                        let delta = mouse_pos - center;
                                        if delta.length_sq() > 1.0 {
                                            let mut angle =
                                                -(delta.y.atan2(delta.x).to_degrees() as f64);
                                            if ctx.input(|input| input.modifiers.command) {
                                                angle = (angle / 15.0).round() * 15.0;
                                            }
                                            guide.angle = angle;
                                        }
                                    } else {
                                        guide.x = gx;
                                        guide.y = gy;
                                    }
                                    delta = Some((
                                        guide.x - before.0,
                                        guide.y - before.1,
                                        guide.angle - before.2,
                                    ));
                                }
                                if self.edit_all_masters {
                                    if let Some((dx, dy, dangle)) = delta {
                                        match target {
                                            GuidelineTarget::Global(index) => {
                                                for (master_id, guides) in
                                                    &mut self.project.guidelines_by_master
                                                {
                                                    if master_id != &self.current_master_id {
                                                        if let Some(guide) = guides.get_mut(index) {
                                                            guide.x += dx;
                                                            guide.y += dy;
                                                            guide.angle += dangle;
                                                        }
                                                    }
                                                }
                                            }
                                            GuidelineTarget::Glyph(index) => {
                                                if let Some(name) = self.current_glyph.as_ref() {
                                                    if let Some(glyph) =
                                                        self.project.glyphs.get_mut(name)
                                                    {
                                                        for (master_id, guides) in
                                                            &mut glyph.master_guidelines
                                                        {
                                                            if master_id != &self.current_master_id {
                                                                if let Some(guide) = guides.get_mut(index) {
                                                                    guide.x += dx;
                                                                    guide.y += dy;
                                                                    guide.angle += dangle;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if response.dragged()
                            && self.guideline_drag.is_none()
                            && self.anchor_drag.is_none()
                        {
                            if let Some(target) = self.spacing_drag {
                                if let Some(name) = self.current_glyph.clone() {
                                    match target {
                                        SpacingTarget::Advance => {
                                            self.project
                                                .set_width_for_glyphs(&[name], gx.max(0.0));
                                        }
                                        SpacingTarget::LeftBearing
                                        | SpacingTarget::RightBearing => {
                                            if let Some((min_x, _, max_x, _)) =
                                                self.project.outline_bounds_for_glyph(&name)
                                            {
                                                let width = self
                                                    .project
                                                    .glyphs
                                                    .get(&name)
                                                    .map(|glyph| glyph.width)
                                                    .unwrap_or_default();
                                                let (left, right) = match target {
                                                    SpacingTarget::LeftBearing => {
                                                        (gx.max(0.0), (width - max_x).max(0.0))
                                                    }
                                                    SpacingTarget::RightBearing => (
                                                        (min_x).max(0.0),
                                                        (width - gx).max(0.0),
                                                    ),
                                                    SpacingTarget::Advance => unreachable!(),
                                                };
                                                self.project.set_side_bearings(
                                                    &[name],
                                                    left,
                                                    right,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            if self.spacing_drag.is_some() {
                                // The advance-width handle consumes this drag.
                            } else if let Some(start) = self.canvas.selection_start {
                                let selection = egui::Rect::from_two_pos(start, mouse_pos);
                                self.canvas.selection_rect = Some(selection);
                                if let Some(name) = &self.current_glyph {
                                    if let Some(glyph) = self.project.glyphs.get(name) {
                                        let mut selected_contour = None;
                                        let mut selected_points = Vec::new();
                                        let mut selected_nodes = Vec::new();
                                        for (ci, contour) in glyph.contours.iter().enumerate() {
                                            let inside: Vec<usize> = contour
                                                .points
                                                .iter()
                                                .enumerate()
                                                .filter_map(|(pi, point)| {
                                                    let screen = self.canvas.glyph_to_screen(
                                                        point.x, point.y, origin,
                                                    );
                                                    selection.contains(screen).then_some(pi)
                                                })
                                                .collect();
                                            if !inside.is_empty() {
                                                if selected_contour.is_none() {
                                                    selected_contour = Some(ci);
                                                    selected_points = inside.clone();
                                                }
                                                selected_nodes.extend(
                                                    inside.into_iter().map(|pi| (ci, pi)),
                                                );
                                            }
                                        }
                                        let selected_components: Vec<usize> = glyph
                                            .components
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(component_index, component)| {
                                                let (min_x, min_y, max_x, max_y) = self
                                                    .project
                                                    .outline_bounds_for_glyph(&component.base)?;
                                                let corners = [
                                                    (min_x, min_y),
                                                    (min_x, max_y),
                                                    (max_x, max_y),
                                                    (max_x, min_y),
                                                ];
                                                let intersects = corners.into_iter().any(|(x, y)| {
                                                    selection.contains(self.canvas.glyph_to_screen(
                                                        component.x_scale * x
                                                            + component.yx_scale * y
                                                            + component.x_offset,
                                                        component.xy_scale * x
                                                            + component.y_scale * y
                                                            + component.y_offset,
                                                        origin,
                                                    ))
                                                });
                                                intersects.then_some(component_index)
                                            })
                                            .collect();
                                        if !selected_components.is_empty() {
                                            self.canvas.selected_nodes.clear();
                                            self.canvas.selected_points.clear();
                                            self.canvas.selected_contour = None;
                                            self.canvas.selected_components =
                                                selected_components;
                                            self.canvas.selected_component =
                                                self.canvas.selected_components.last().copied();
                                        }
                                        if self.canvas.selected_components.is_empty() {
                                            self.canvas.selected_contour = selected_contour;
                                            self.canvas.selected_points = selected_points;
                                            self.canvas.selected_nodes = selected_nodes;
                                        } else {
                                            self.canvas.selected_contour = None;
                                        }
                                    }
                                }
                            }
                            if self.canvas.selection_start.is_none() {
                                let delta = self
                                    .canvas
                                    .screen_to_glyph(mouse_pos - response.drag_delta(), origin);
                                let (dx, dy) = self.canvas.screen_to_glyph(mouse_pos, origin);
                                let (ddx, ddy) = {
                                    let raw_dx = dx - delta.0;
                                    let raw_dy = dy - delta.1;
                                    if ctx.input(|input| input.modifiers.shift) {
                                        if raw_dx.abs() >= raw_dy.abs() {
                                            (raw_dx, 0.0)
                                        } else {
                                            (0.0, raw_dy)
                                        }
                                    } else {
                                        (raw_dx, raw_dy)
                                    }
                                };

                                let transformed_component = self
                                    .component_resize
                                    .clone()
                                    .and_then(|(resize_index, handle, original)| {
                                        (Some(resize_index) == self.canvas.selected_component)
                                            .then(|| {
                                                if handle == 4 {
                                                    let start = self.canvas.screen_to_glyph(
                                                        mouse_pos - response.drag_delta(),
                                                        origin,
                                                    );
                                                    Self::rotate_component_from_handle(
                                                        &self.project,
                                                        &original,
                                                        start,
                                                        (gx, gy),
                                                        ctx.input(|input| input.modifiers.command),
                                                    )
                                                } else {
                                                    Self::resize_component_from_handle(
                                                        &self.project,
                                                        &original,
                                                        handle,
                                                        (gx, gy),
                                                    )
                                                }
                                            })
                                            .flatten()
                                    });
                                let master_rotation_updates =
                                    if self.edit_all_masters {
                                        self.component_resize
                                            .as_ref()
                                            .filter(|(_, handle, _)| *handle == 4)
                                            .and_then(|(_, _, original)| {
                                                let resized = transformed_component.as_ref()?;
                                                let angle = resized.xy_scale.atan2(resized.x_scale)
                                                    - original.xy_scale.atan2(original.x_scale);
                                                let name = self.current_glyph.as_ref()?;
                                                let glyph = self.project.glyphs.get(name)?;
                                                Some(
                                                    glyph
                                                        .layers
                                                        .iter()
                                                        .filter_map(|(layer_id, layer)| {
                                                            let component =
                                                                layer.components.get(self.canvas.selected_component?)?;
                                                            Some((
                                                                layer_id.clone(),
                                                                Self::rotate_component_by_angle(
                                                                    &self.project,
                                                                    component,
                                                                    angle,
                                                                )?,
                                                            ))
                                                        })
                                                        .collect::<Vec<_>>(),
                                                )
                                            })
                                    } else {
                                        None
                                    };

                                let component_indices = self.selected_component_indices();
                                let global_guidelines = self
                                    .project
                                    .guidelines_for_master(&self.current_master_id)
                                    .to_vec();
                                if let Some(name) = &self.current_glyph {
                                    if let Some(glyph) = self.project.glyphs.get_mut(name) {
                                        if component_indices.len() > 1 {
                                            if self.edit_all_masters {
                                                for component_index in &component_indices {
                                                    if let Err(error) = glyph
                                                        .translate_component_all_layers(
                                                            *component_index,
                                                            ddx,
                                                            ddy,
                                                        )
                                                    {
                                                        self.status_message = error;
                                                    }
                                                }
                                            } else {
                                                for component_index in component_indices {
                                                    if let Some(component) =
                                                        glyph.components.get_mut(component_index)
                                                    {
                                                        component.x_offset += ddx;
                                                        component.y_offset += ddy;
                                                    }
                                                }
                                            }
                                        } else if let Some(component_index) =
                                            self.canvas.selected_component
                                        {
                                            if let Some(resized) = transformed_component {
                                                let resize_original = self
                                                    .component_resize
                                                    .as_ref()
                                                    .map(|(_, _, original)| original.clone());
                                                let scale_x = resize_original
                                                    .as_ref()
                                                    .and_then(|original| {
                                                        (original.x_scale.abs() > f64::EPSILON)
                                                            .then_some(
                                                                resized.x_scale / original.x_scale,
                                                            )
                                                    })
                                                    .unwrap_or(1.0);
                                                let scale_y = resize_original
                                                    .as_ref()
                                                    .and_then(|original| {
                                                        (original.y_scale.abs() > f64::EPSILON)
                                                            .then_some(
                                                                resized.y_scale / original.y_scale,
                                                            )
                                                    })
                                                    .unwrap_or(1.0);
                                                let offset_dx = resize_original
                                                    .as_ref()
                                                    .map_or(0.0, |original| {
                                                        resized.x_offset - original.x_offset
                                                    });
                                                let offset_dy = resize_original
                                                    .as_ref()
                                                    .map_or(0.0, |original| {
                                                        resized.y_offset - original.y_offset
                                                    });
                                                if let Some(component) =
                                                    glyph.components.get_mut(component_index)
                                                {
                                                    *component = resized;
                                                }
                                                if self.edit_all_masters {
                                                    if let Some(updates) =
                                                        master_rotation_updates
                                                    {
                                                        for (layer_id, component) in updates {
                                                            if let Some(layer_component) = glyph
                                                                .layers
                                                                .get_mut(&layer_id)
                                                                .and_then(|layer| {
                                                                    layer.components.get_mut(component_index)
                                                                })
                                                            {
                                                                *layer_component = component;
                                                            }
                                                        }
                                                    } else {
                                                        for layer in glyph.layers.values_mut() {
                                                            if let Some(component) =
                                                                layer.components.get_mut(component_index)
                                                            {
                                                                component.x_scale *= scale_x;
                                                                component.xy_scale *= scale_x;
                                                                component.yx_scale *= scale_y;
                                                                component.y_scale *= scale_y;
                                                                component.x_offset += offset_dx;
                                                                component.y_offset += offset_dy;
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if self.edit_all_masters {
                                                if let Err(error) = glyph
                                                    .translate_component_all_layers(
                                                        component_index,
                                                        ddx,
                                                        ddy,
                                                    )
                                                {
                                                    self.status_message = error;
                                                }
                                            } else if let Some(component) =
                                                glyph.components.get_mut(component_index)
                                            {
                                                component.x_offset += ddx;
                                                component.y_offset += ddy;
                                            }
                                        } else {
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
                                            if self.edit_all_masters {
                                                if let Err(error) =
                                                    glyph.translate_nodes_all_layers(&nodes, ddx, ddy)
                                                {
                                                    self.status_message = error;
                                                }
                                            } else {
                                                let glyph_guidelines = glyph
                                                    .guidelines_for_master(
                                                        &self.current_master_id,
                                                    )
                                                    .to_vec();
                                                for (ci, contour) in
                                                    glyph.contours.iter_mut().enumerate()
                                                {
                                                    let indices: Vec<usize> = nodes
                                                        .iter()
                                                        .filter_map(|&(selected_ci, pi)| {
                                                            (selected_ci == ci).then_some(pi)
                                                        })
                                                        .collect();
                                                    if indices.is_empty() {
                                                        continue;
                                                    }
                                                    contour.translate_points(&indices, ddx, ddy);
                                                    for pi in indices {
                                                        if let Some(point) =
                                                            contour.points.get_mut(pi)
                                                        {
                                                            let (x, y) = self.canvas.snap_point(
                                                                point.x,
                                                                point.y,
                                                            );
                                                            let mut guidelines =
                                                                global_guidelines.clone();
                                                            guidelines.extend(
                                                                glyph_guidelines.iter().cloned(),
                                                            );
                                                            let (x, y) = self.canvas
                                                                .snap_point_to_guidelines(
                                                                    x, y, &guidelines,
                                                                );
                                                            let (x, y) = self.canvas
                                                                .snap_point_to_anchors(
                                                                    x,
                                                                    y,
                                                                    &glyph.anchors,
                                                                );
                                                            point.x = x;
                                                            point.y = y;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if response.drag_stopped() && self.guideline_drag.take().is_some() {
                            self.save_state();
                        }
                        if response.drag_stopped() && self.anchor_drag.take().is_some() {
                            self.save_state();
                            self.status_message = "アンカーを移動しました".to_string();
                        }
                        if response.drag_stopped()
                            && (!self.canvas.selected_points.is_empty()
                                || self.canvas.selected_component.is_some())
                        {
                            self.save_state();
                        }
                        if response.drag_stopped() {
                            self.component_drag_duplicated = false;
                            self.component_resize = None;
                            if self.spacing_drag.is_some() {
                                self.save_state();
                                self.spacing_drag = None;
                                self.status_message =
                                    "キャンバス上で字幅を変更しました".to_string();
                            }
                            self.canvas.selection_start = None;
                            self.canvas.selection_rect = None;
                        }
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
                                        // The final segment closes the cyclic contour and
                                        // may contain one or two off-curve controls.
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

            if let Some(mouse_pos) = response.hover_pos() {
                painter.circle_filled(
                    mouse_pos,
                    3.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 100),
                );
                if self.current_tool != Tool::Hand {
                    let (cursor_x, cursor_y) =
                        self.canvas.screen_to_glyph(mouse_pos, origin);
                    painter.text(
                        mouse_pos + Vec2::new(10.0, 12.0),
                        egui::Align2::LEFT_TOP,
                        format!("{cursor_x:.0}, {cursor_y:.0}"),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgba_premultiplied(220, 225, 235, 190),
                    );
                }
            }
        });
    }
}
