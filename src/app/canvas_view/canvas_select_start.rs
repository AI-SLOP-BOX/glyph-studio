use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn begin_select_drag(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        origin: egui::Pos2,
        mouse_pos: egui::Pos2,
    ) {
        self.spacing_drag = self
            .current_glyph
            .as_ref()
            .and_then(|name| self.project.glyphs.get(name))
            .and_then(|glyph| {
                let x = self.canvas.glyph_to_screen(glyph.width, 0.0, origin).x;
                ((mouse_pos.x - x).abs() <= 8.0).then_some(SpacingTarget::Advance)
            })
            .or_else(|| {
                self.current_glyph
                    .as_ref()
                    .and_then(|name| self.project.outline_bounds_for_glyph(name))
                    .and_then(|(min_x, _, max_x, _)| {
                        let left_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                        let right_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
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
                            glyph
                                .anchors
                                .iter()
                                .enumerate()
                                .find(|(_, anchor)| {
                                    self.canvas
                                        .glyph_to_screen(anchor.x, anchor.y, origin)
                                        .distance(mouse_pos)
                                        <= 9.0
                                })
                                .map(|(index, _)| AnchorTarget::Glyph(index))
                        })
                })
            })
        } else {
            None
        };
        let near_guide = |guide: &crate::font_data::Guideline| {
            let center = self.canvas.glyph_to_screen(guide.x, guide.y, origin);
            let direction = Vec2::new(
                (-(guide.angle as f32).to_radians()).cos(),
                (-(guide.angle as f32).to_radians()).sin(),
            );
            let offset = mouse_pos - center;
            let distance = (offset.x * direction.y - offset.y * direction.x).abs();
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
            self.project
                .glyphs
                .get(name)
                .and_then(|glyph| self.canvas.hit_test(start, glyph, origin))
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
            if ctx.input(|input| input.modifiers.alt) && !self.component_drag_duplicated {
                if let Some(name) = self.current_glyph.clone() {
                    if self
                        .project
                        .duplicate_component_all_layers(&name, component_index)
                    {
                        let new_index = self
                            .project
                            .glyphs
                            .get(&name)
                            .map(|glyph| glyph.components.len() - 1)
                            .unwrap_or(component_index);
                        component_hit = Some(new_index);
                        self.component_drag_duplicated = true;
                        self.save_state();
                        self.status_message = "部品を複製しました（Optionドラッグ）".to_string();
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
                if let (Some(name), Some(handle)) = (self.current_glyph.as_ref(), handle) {
                    if let Some(component) = self
                        .project
                        .glyphs
                        .get(name)
                        .and_then(|glyph| glyph.components.get(component_index))
                        .cloned()
                    {
                        self.component_resize = Some((component_index, handle, component));
                    }
                }
            }
        }
        if self.guideline_drag.is_none() && self.spacing_drag.is_none() {
            self.canvas.selection_start =
                (hit.is_none() && component_hit.is_none() && self.anchor_drag.is_none())
                    .then(|| response.interact_pointer_pos())
                    .flatten();
            if let Some(component_index) = component_hit {
                self.select_component(component_index, ctx.input(|input| input.modifiers.shift));
            } else {
                self.canvas.selected_component = None;
                self.canvas.selected_components.clear();
            }
        }
        self.canvas.selection_rect = None;
    }
}
