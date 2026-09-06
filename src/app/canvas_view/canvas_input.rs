use super::super::*;

#[derive(Default)]
pub(crate) struct CanvasKeyboardRequests {
    pub(crate) fit_view_requested: bool,
    pub(crate) toggle_background_requested: bool,
    pub(crate) toggle_side_glyphs_requested: bool,
    pub(crate) undo_requested: bool,
    pub(crate) redo_requested: bool,
    pub(crate) delete_requested: bool,
    pub(crate) escape_pressed: bool,
    pub(crate) select_all_requested: bool,
    pub(crate) select_all_glyphs_requested: bool,
    pub(crate) toggle_guides_requested: bool,
    pub(crate) toggle_contour_direction_requested: bool,
    pub(crate) toggle_metrics_requested: bool,
    pub(crate) toggle_node_indices_requested: bool,
    pub(crate) toggle_all_masters_requested: bool,
    pub(crate) node_action_requested: Option<NodeAction>,
    pub(crate) nudge: Option<(f64, f64)>,
    pub(crate) reset_view_requested: bool,
    pub(crate) zoom_delta: Option<(f32, Pos2)>,
    pub(crate) new_tool: Option<Tool>,
}

impl GlyphStudioApp {
    pub(crate) fn read_canvas_keyboard(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        response: &egui::Response,
        mut fit_view_requested: bool,
    ) -> CanvasKeyboardRequests {
        let mut toggle_background_requested = false;
        let mut toggle_side_glyphs_requested = false;
        let mut undo_requested = false;
        let mut redo_requested = false;
        let mut delete_requested = false;
        let mut escape_pressed = false;
        let mut select_all_requested = false;
        let mut select_all_glyphs_requested = false;
        let mut toggle_guides_requested = false;
        let mut toggle_contour_direction_requested = false;
        let mut toggle_metrics_requested = false;
        let mut toggle_node_indices_requested = false;
        let mut toggle_all_masters_requested = false;
        let mut node_action_requested: Option<NodeAction> = None;
        let mut nudge: Option<(f64, f64)> = None;
        let mut reset_view_requested = false;
        let mut zoom_delta: Option<(f32, Pos2)> = None;
        let mut new_tool: Option<Tool> = None;
        let wants_keyboard_input = ctx.wants_keyboard_input();

        ctx.input(|i| {
            if !wants_keyboard_input && i.key_pressed(Key::V) {
                new_tool = Some(Tool::Select);
            }
            if !wants_keyboard_input && i.key_pressed(Key::P) {
                new_tool = Some(Tool::Pen);
            }
            if !wants_keyboard_input && i.key_pressed(Key::H) {
                new_tool = Some(Tool::Hand);
            }
            if !wants_keyboard_input && i.key_pressed(Key::K) {
                new_tool = Some(Tool::Knife);
            }

            if !wants_keyboard_input
                && i.modifiers.command
                && i.key_pressed(Key::Z)
                && !i.modifiers.shift
            {
                undo_requested = true;
            }
            if !wants_keyboard_input
                && i.modifiers.command
                && i.modifiers.shift
                && i.key_pressed(Key::Z)
            {
                redo_requested = true;
            }
            if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::Y) {
                redo_requested = true;
            }

            if !wants_keyboard_input
                && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
            {
                delete_requested = true;
            }

            if !wants_keyboard_input && i.key_pressed(Key::Escape) {
                escape_pressed = true;
            }

            if !wants_keyboard_input
                && (i.modifiers.command || i.modifiers.ctrl)
                && i.key_pressed(Key::A)
            {
                if i.modifiers.shift {
                    select_all_glyphs_requested = true;
                } else {
                    select_all_requested = true;
                }
            }
            if !wants_keyboard_input && i.key_pressed(Key::Tab) {
                let delta = if i.modifiers.shift { -1 } else { 1 };
                self.select_relative_glyph(delta);
            }
            if !wants_keyboard_input && i.key_pressed(Key::PageUp) {
                self.select_relative_glyph(-1);
            }
            if !wants_keyboard_input && i.key_pressed(Key::PageDown) {
                self.select_relative_glyph(1);
            }
            if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::ArrowUp) {
                self.select_relative_master(-1);
            }
            if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::ArrowDown) {
                self.select_relative_master(1);
            }
            if !wants_keyboard_input && i.key_pressed(Key::Home) {
                self.select_edge_glyph(false);
            }
            if !wants_keyboard_input && i.key_pressed(Key::End) {
                self.select_edge_glyph(true);
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::G)
                && !i.modifiers.command
                && !i.modifiers.alt
            {
                toggle_guides_requested = true;
            }
            if !wants_keyboard_input && i.key_pressed(Key::I) {
                toggle_background_requested = true;
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::B)
                && !i.modifiers.command
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                toggle_side_glyphs_requested = true;
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::D)
                && !i.modifiers.command
                && !i.modifiers.alt
            {
                toggle_contour_direction_requested = true;
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::M)
                && !i.modifiers.command
                && !i.modifiers.alt
            {
                toggle_metrics_requested = true;
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::N)
                && !i.modifiers.command
                && !i.modifiers.alt
            {
                toggle_node_indices_requested = true;
            }
            if !wants_keyboard_input
                && i.modifiers.command
                && i.modifiers.shift
                && i.key_pressed(Key::M)
            {
                toggle_all_masters_requested = true;
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::S)
                && !i.modifiers.command
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                node_action_requested = Some(NodeAction::Smooth);
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::C)
                && !i.modifiers.command
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                node_action_requested = Some(NodeAction::Corner);
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::T)
                && !i.modifiers.command
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                node_action_requested = Some(NodeAction::ToggleCurve);
            }
            if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::Num0) {
                reset_view_requested = true;
            }
            if !wants_keyboard_input
                && i.modifiers.command
                && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals))
            {
                zoom_delta = Some((3.0, rect.center()));
            } else if !wants_keyboard_input && i.modifiers.command && i.key_pressed(Key::Minus) {
                zoom_delta = Some((-3.0, rect.center()));
            }
            if !wants_keyboard_input
                && i.key_pressed(Key::F)
                && !i.modifiers.command
                && !i.modifiers.alt
            {
                fit_view_requested = true;
            }
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            if !wants_keyboard_input && i.key_pressed(Key::ArrowLeft) {
                nudge = Some((-step, 0.0));
            } else if !wants_keyboard_input && i.key_pressed(Key::ArrowRight) {
                nudge = Some((step, 0.0));
            } else if !wants_keyboard_input && i.key_pressed(Key::ArrowUp) {
                nudge = Some((0.0, step));
            } else if !wants_keyboard_input && i.key_pressed(Key::ArrowDown) {
                nudge = Some((0.0, -step));
            }

            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    if let Some(mouse_pos) = response.hover_pos() {
                        zoom_delta = Some((delta.y, mouse_pos));
                    }
                }
            }
        });

        CanvasKeyboardRequests {
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
        }
    }
}
