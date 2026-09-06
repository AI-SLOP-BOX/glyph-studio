use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn handle_select_tool(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        origin: egui::Pos2,
        mouse_pos: egui::Pos2,
        gx: f64,
        gy: f64,
    ) {
        self.begin_select_drag(ctx, response, origin, mouse_pos);
        self.handle_select_click(ctx, response, origin, mouse_pos);
        if response.double_clicked() {
            if let Some(name) = self.current_glyph.clone() {
                let hit = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| self.canvas.hit_test(mouse_pos, glyph, origin));
                let segment_hit = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| self.canvas.hit_test_segment(mouse_pos, glyph, origin));
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
                        match glyph.split_segment_all_layers(ci, start_index, factor) {
                            Ok(inserted_index) => {
                                self.canvas.selected_component = None;
                                self.canvas.selected_components.clear();
                                self.canvas.selected_contour = Some(ci);
                                self.canvas.selected_points = vec![inserted_index];
                                self.canvas.selected_nodes = vec![(ci, inserted_index)];
                                self.save_state();
                                self.status_message = "曲線上にノードを追加しました".to_string();
                            }
                            Err(error) => self.status_message = error,
                        }
                    }
                } else if let Some(component_index) =
                    self.project.glyphs.get(&name).and_then(|glyph| {
                        self.canvas
                            .hit_test_component(mouse_pos, &self.project, glyph, origin)
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
                        self.status_message = format!("参照先グリフを開きました: {base}");
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
                    let (anchor_x, anchor_y) =
                        self.canvas
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
                    GuidelineTarget::Global(index) => self
                        .project
                        .guidelines_for_master_mut(&self.current_master_id)
                        .get_mut(index),
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
                        let center = self.canvas.glyph_to_screen(guide.x, guide.y, origin);
                        let delta = mouse_pos - center;
                        if delta.length_sq() > 1.0 {
                            let mut angle = -(delta.y.atan2(delta.x).to_degrees() as f64);
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
                                for (master_id, guides) in &mut self.project.guidelines_by_master {
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
                                    if let Some(glyph) = self.project.glyphs.get_mut(name) {
                                        for (master_id, guides) in &mut glyph.master_guidelines {
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
        if response.dragged() && self.guideline_drag.is_none() && self.anchor_drag.is_none() {
            if let Some(target) = self.spacing_drag {
                if let Some(name) = self.current_glyph.clone() {
                    match target {
                        SpacingTarget::Advance => {
                            self.project.set_width_for_glyphs(&[name], gx.max(0.0));
                        }
                        SpacingTarget::LeftBearing | SpacingTarget::RightBearing => {
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
                                    SpacingTarget::RightBearing => {
                                        ((min_x).max(0.0), (width - gx).max(0.0))
                                    }
                                    SpacingTarget::Advance => unreachable!(),
                                };
                                self.project.set_side_bearings(&[name], left, right);
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
                                    let screen =
                                        self.canvas.glyph_to_screen(point.x, point.y, origin);
                                    selection.contains(screen).then_some(pi)
                                })
                                .collect();
                            if !inside.is_empty() {
                                if selected_contour.is_none() {
                                    selected_contour = Some(ci);
                                    selected_points = inside.clone();
                                }
                                selected_nodes.extend(inside.into_iter().map(|pi| (ci, pi)));
                            }
                        }
                        let selected_components: Vec<usize> = glyph
                            .components
                            .iter()
                            .enumerate()
                            .filter_map(|(component_index, component)| {
                                let (min_x, min_y, max_x, max_y) =
                                    self.project.outline_bounds_for_glyph(&component.base)?;
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
                            self.canvas.selected_components = selected_components;
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

                let transformed_component =
                    self.component_resize
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
                let master_rotation_updates = if self.edit_all_masters {
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
                                        let component = layer
                                            .components
                                            .get(self.canvas.selected_component?)?;
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
                                    if let Err(error) = glyph.translate_component_all_layers(
                                        *component_index,
                                        ddx,
                                        ddy,
                                    ) {
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
                        } else if let Some(component_index) = self.canvas.selected_component {
                            if let Some(resized) = transformed_component {
                                let resize_original = self
                                    .component_resize
                                    .as_ref()
                                    .map(|(_, _, original)| original.clone());
                                let scale_x = resize_original
                                    .as_ref()
                                    .and_then(|original| {
                                        (original.x_scale.abs() > f64::EPSILON)
                                            .then_some(resized.x_scale / original.x_scale)
                                    })
                                    .unwrap_or(1.0);
                                let scale_y = resize_original
                                    .as_ref()
                                    .and_then(|original| {
                                        (original.y_scale.abs() > f64::EPSILON)
                                            .then_some(resized.y_scale / original.y_scale)
                                    })
                                    .unwrap_or(1.0);
                                let offset_dx = resize_original
                                    .as_ref()
                                    .map_or(0.0, |original| resized.x_offset - original.x_offset);
                                let offset_dy = resize_original
                                    .as_ref()
                                    .map_or(0.0, |original| resized.y_offset - original.y_offset);
                                if let Some(component) = glyph.components.get_mut(component_index) {
                                    *component = resized;
                                }
                                if self.edit_all_masters {
                                    if let Some(updates) = master_rotation_updates {
                                        for (layer_id, component) in updates {
                                            if let Some(layer_component) =
                                                glyph.layers.get_mut(&layer_id).and_then(|layer| {
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
                                if let Err(error) =
                                    glyph.translate_component_all_layers(component_index, ddx, ddy)
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
                                    .guidelines_for_master(&self.current_master_id)
                                    .to_vec();
                                for (ci, contour) in glyph.contours.iter_mut().enumerate() {
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
                                        if let Some(point) = contour.points.get_mut(pi) {
                                            let (x, y) = self.canvas.snap_point(point.x, point.y);
                                            let mut guidelines = global_guidelines.clone();
                                            guidelines.extend(glyph_guidelines.iter().cloned());
                                            let (x, y) = self.canvas.snap_point_to_guidelines(
                                                x,
                                                y,
                                                &guidelines,
                                            );
                                            let (x, y) = self.canvas.snap_point_to_anchors(
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
            && (!self.canvas.selected_points.is_empty() || self.canvas.selected_component.is_some())
        {
            self.save_state();
        }
        if response.drag_stopped() {
            self.component_drag_duplicated = false;
            self.component_resize = None;
            if self.spacing_drag.is_some() {
                self.save_state();
                self.spacing_drag = None;
                self.status_message = "キャンバス上で字幅を変更しました".to_string();
            }
            self.canvas.selection_start = None;
            self.canvas.selection_rect = None;
        }
    }
}
