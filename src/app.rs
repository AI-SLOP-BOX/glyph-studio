#![allow(deprecated)]

use crate::canvas::CanvasState;
use crate::font_data::{Contour, FontProject, GlyphComponent};
use crate::glyph_list;
use crate::history::History;
use crate::properties;
use crate::tools::{PenState, Tool};
use eframe::egui;
use egui::{Color32, Key, Pos2, Stroke, Vec2};
use kurbo::{flatten, CubicBez, Line, ParamCurveNearest, PathEl, Point, QuadBez};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

pub struct GlyphStudioApp {
    pub project: FontProject,
    pub current_glyph: Option<String>,
    pub current_tool: Tool,
    pub space_previous_tool: Option<Tool>,
    pub canvas: CanvasState,
    pub pen_state: PenState,
    pub pen_drag_start: Option<(f64, f64)>,
    pub history: History,
    pub show_glyph_list: bool,
    pub show_properties: bool,
    pub show_preview: bool,
    pub show_shortcuts: bool,
    pub properties_filter: String,
    pub show_kerning_window: bool,
    pub preview_text: String,
    pub feature_left: String,
    pub feature_right: String,
    pub feature_replacement: String,
    pub feature_kerning_value: String,
    pub feature_target_tag: String,
    pub feature_anchor_x: String,
    pub feature_anchor_y: String,
    pub preview_features: String,
    pub preview_scale: f32,
    pub preview_line_spacing: f32,
    pub preview_vertical_metrics: bool,
    pub preview_dark_background: bool,
    pub spacing_reference: char,
    pub glyph_search: String,
    pub focus_glyph_search: bool,
    pub selected_glyphs: HashSet<String>,
    pub batch_glyphs_input: String,
    pub batch_unicode_input: String,
    pub batch_width: f64,
    pub batch_left_side_bearing: f64,
    pub batch_right_side_bearing: f64,
    pub batch_dx: f64,
    pub batch_dy: f64,
    pub selection_dx: f64,
    pub selection_dy: f64,
    pub batch_left_kerning_group: String,
    pub batch_right_kerning_group: String,
    pub component_base: String,
    pub component_scale_linked: bool,
    pub kerning_right: String,
    pub kerning_pair_filter: String,
    pub unicode_alias_input: String,
    pub unicode_variation_selector: String,
    pub color_layer_glyph: String,
    pub preview_color_palette: usize,
    pub glyph_rename_input: String,
    pub glyph_sort_by_unicode: bool,
    pub glyph_list_only_unassigned: bool,
    pub glyph_list_grid_view: bool,
    pub master_axis_tag_input: String,
    pub status_message: String,
    pub validation_issues: Vec<crate::core::ValidationIssue>,
    pub validation_glyphs_only: bool,
    pub project_path: Option<PathBuf>,
    pub pending_open_path: Option<PathBuf>,
    pub pending_new_project: bool,
    pub show_unsaved_open_dialog: bool,
    pub saved_history_index: usize,
    pub saved_project: Option<FontProject>,
    pub current_master_id: String,
    pub interpolation_from_master: String,
    pub interpolation_to_master: String,
    pub interpolation_factor: f32,
    pub interpolation_x_factor: f32,
    pub interpolation_y_factor: f32,
    pub interpolation_batch_factors: String,
    pub show_interpolation_overlay: bool,
    pub show_all_masters_overlay: bool,
    pub show_side_glyphs: bool,
    pub edit_all_masters: bool,
    pub contour_clipboard: Option<Contour>,
    pub component_clipboard: Option<GlyphComponent>,
    pub knife_first_cut: Option<(usize, usize)>,
    pub guideline_drag: Option<GuidelineTarget>,
    pub selected_guideline: Option<GuidelineTarget>,
    pub spacing_drag: Option<SpacingTarget>,
    pub component_drag_duplicated: bool,
    pub component_resize: Option<(usize, usize, GlyphComponent)>,
    pub width_drag_active: bool,
    pub anchor_drag: Option<AnchorTarget>,
    pub background_cache: HashMap<String, (Option<SystemTime>, egui_extras::RetainedImage)>,
    pub conditional_layer_axis: String,
    pub conditional_layer_min: String,
    pub conditional_layer_max: String,
    pub conditional_layer_axis_2: String,
    pub conditional_layer_min_2: String,
    pub conditional_layer_max_2: String,
    pub conditional_layer_axis_3: String,
    pub conditional_layer_min_3: String,
    pub conditional_layer_max_3: String,
    pub conditional_layer_axis_4: String,
    pub conditional_layer_min_4: String,
    pub conditional_layer_max_4: String,
    pub conditional_layer_extra: Vec<(String, String, String)>,
    pub master_map_drag: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum GuidelineTarget {
    Global(usize),
    Glyph(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingTarget {
    Advance,
    LeftBearing,
    RightBearing,
}

#[derive(Debug, Clone, Copy)]
pub enum AnchorTarget {
    Glyph(usize),
    Layer(usize),
}

#[derive(Debug, Clone, Copy)]
enum NodeAction {
    Smooth,
    Corner,
    ToggleCurve,
}

impl Default for GlyphStudioApp {
    fn default() -> Self {
        Self {
            project: FontProject::new(),
            current_glyph: None,
            current_tool: Tool::Select,
            space_previous_tool: None,
            canvas: CanvasState::default(),
            pen_state: PenState::new(),
            pen_drag_start: None,
            history: History::new(100),
            show_glyph_list: true,
            show_properties: true,
            // Keep the initial workspace focused on drawing; preview can be
            // enabled from the toolbar or the layout presets when needed.
            show_preview: false,
            show_shortcuts: false,
            properties_filter: String::new(),
            show_kerning_window: false,
            preview_text: "Aa".to_string(),
            feature_left: "A".to_string(),
            feature_right: "B".to_string(),
            feature_replacement: "A.alt".to_string(),
            feature_kerning_value: "-80".to_string(),
            feature_target_tag: "liga".to_string(),
            feature_anchor_x: "300".to_string(),
            feature_anchor_y: "700".to_string(),
            preview_features: "liga,kern".to_string(),
            preview_scale: 0.04,
            preview_line_spacing: 1.0,
            preview_vertical_metrics: false,
            preview_dark_background: false,
            spacing_reference: 'H',
            glyph_search: String::new(),
            focus_glyph_search: false,
            glyph_sort_by_unicode: false,
            glyph_list_only_unassigned: false,
            glyph_list_grid_view: false,
            selected_glyphs: HashSet::new(),
            batch_glyphs_input: String::new(),
            batch_unicode_input: String::new(),
            batch_width: 600.0,
            batch_left_side_bearing: 50.0,
            batch_right_side_bearing: 50.0,
            batch_dx: 0.0,
            batch_dy: 0.0,
            selection_dx: 0.0,
            selection_dy: 0.0,
            batch_left_kerning_group: String::new(),
            batch_right_kerning_group: String::new(),
            component_base: String::new(),
            component_scale_linked: true,
            kerning_right: String::new(),
            kerning_pair_filter: String::new(),
            unicode_alias_input: String::new(),
            unicode_variation_selector: "FE00".to_string(),
            color_layer_glyph: String::new(),
            preview_color_palette: 0,
            glyph_rename_input: String::new(),
            master_axis_tag_input: String::new(),
            status_message: "準備完了".to_string(),
            validation_issues: Vec::new(),
            validation_glyphs_only: false,
            project_path: None,
            pending_open_path: None,
            pending_new_project: false,
            show_unsaved_open_dialog: false,
            saved_history_index: 0,
            saved_project: None,
            current_master_id: "regular".to_string(),
            interpolation_from_master: String::new(),
            interpolation_to_master: String::new(),
            interpolation_factor: 0.5,
            interpolation_x_factor: 0.5,
            interpolation_y_factor: 0.5,
            interpolation_batch_factors: "25,50,75".to_string(),
            show_interpolation_overlay: false,
            show_all_masters_overlay: false,
            show_side_glyphs: true,
            edit_all_masters: false,
            contour_clipboard: None,
            component_clipboard: None,
            knife_first_cut: None,
            guideline_drag: None,
            selected_guideline: None,
            spacing_drag: None,
            component_drag_duplicated: false,
            component_resize: None,
            width_drag_active: false,
            anchor_drag: None,
            background_cache: HashMap::new(),
            conditional_layer_axis: "wght".into(),
            conditional_layer_min: "700".into(),
            conditional_layer_max: String::new(),
            conditional_layer_axis_2: String::new(),
            conditional_layer_min_2: String::new(),
            conditional_layer_max_2: String::new(),
            conditional_layer_axis_3: String::new(),
            conditional_layer_min_3: String::new(),
            conditional_layer_max_3: String::new(),
            conditional_layer_axis_4: String::new(),
            conditional_layer_min_4: String::new(),
            conditional_layer_max_4: String::new(),
            conditional_layer_extra: Vec::new(),
            master_map_drag: None,
        }
    }
}

impl GlyphStudioApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(6.0, 4.0);
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            style.visuals.window_fill = Color32::from_rgb(30, 32, 36);
            style.visuals.panel_fill = Color32::from_rgb(25, 27, 31);
            style.visuals.extreme_bg_color = Color32::from_rgb(18, 20, 23);
            style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 48, 54);
            style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(61, 83, 101);
            style.visuals.widgets.active.bg_fill = Color32::from_rgb(76, 112, 133);
            style.visuals.selection.bg_fill = Color32::from_rgb(55, 104, 132);
            style.visuals.selection.stroke.color = Color32::from_rgb(145, 205, 230);
        });
        // egui's built-in font does not contain Japanese glyphs. On macOS,
        // Arial Unicode is a stable system font and keeps the Japanese UI
        // readable without bundling a large font file into the repository.
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = fs::read("/System/Library/Fonts/Supplemental/Arial Unicode.ttf") {
            fonts.font_data.insert(
                "ui_japanese".into(),
                Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "ui_japanese".into());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "ui_japanese".into());
            cc.egui_ctx.set_fonts(fonts);
        }
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.visuals.panel_fill = Color32::from_rgb(27, 28, 34);
        style.visuals.window_fill = Color32::from_rgb(34, 35, 42);
        style.visuals.extreme_bg_color = Color32::from_rgb(20, 21, 26);
        style.visuals.faint_bg_color = Color32::from_rgb(40, 41, 48);
        style.visuals.selection.bg_fill = Color32::from_rgb(55, 93, 145);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(48, 50, 59);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(64, 79, 103);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(45, 108, 166);
        cc.egui_ctx.set_style(style);
        let mut app = Self::default();

        let default_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789\
                            .,;:!?-";
        for ch in default_chars.chars() {
            let name = glyph_name_for_char(ch);
            let unicode = ch as u32;
            app.project.add_glyph(name, Some(unicode));
        }

        let hiragana = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん";
        for ch in hiragana.chars() {
            let name = glyph_name_for_char(ch);
            let unicode = ch as u32;
            app.project.add_glyph(name, Some(unicode));
        }

        let katakana = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン";
        for ch in katakana.chars() {
            let name = glyph_name_for_char(ch);
            let unicode = ch as u32;
            app.project.add_glyph(name, Some(unicode));
        }

        if let Some(first) = app.project.glyph_names_sorted().first() {
            app.current_glyph = Some(first.to_string());
        }

        app.history.push(&app.project, &app.current_glyph);
        app.saved_history_index = app.history.current_index;
        app.saved_project = Some(app.project.clone());

        app
    }
}

fn collect_decomposed_contours(
    project: &FontProject,
    glyph_name: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
    output: &mut Vec<Contour>,
) {
    if !visiting.insert(glyph_name.to_string()) {
        return;
    }
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (a, b, c, d, e, f) = transform;
        for contour in &glyph.contours {
            let mut copy = contour.clone();
            for point in &mut copy.points {
                let x = a * point.x + b * point.y + e;
                let y = c * point.x + d * point.y + f;
                point.x = x;
                point.y = y;
            }
            output.push(copy);
        }
        for component in &glyph.components {
            collect_decomposed_contours(
                project,
                &component.base,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
                output,
            );
        }
    }
    visiting.remove(glyph_name);
}

fn collect_decomposed_contours_for_master(
    project: &FontProject,
    glyph_name: &str,
    master_id: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
    output: &mut Vec<Contour>,
) {
    if !visiting.insert(glyph_name.to_string()) {
        return;
    }
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (contours, components) = glyph
            .layers
            .get(master_id)
            .map(|layer| (layer.contours.clone(), layer.components.clone()))
            .unwrap_or_else(|| (glyph.contours.clone(), glyph.components.clone()));
        let (a, b, c, d, e, f) = transform;
        for contour in contours {
            let mut copy = contour;
            for point in &mut copy.points {
                let x = a * point.x + b * point.y + e;
                let y = c * point.x + d * point.y + f;
                point.x = x;
                point.y = y;
            }
            output.push(copy);
        }
        for component in components {
            collect_decomposed_contours_for_master(
                project,
                &component.base,
                master_id,
                compose_preview_transform(transform, component_transform(&component)),
                visiting,
                output,
            );
        }
    }
    visiting.remove(glyph_name);
}

mod preview;
use preview::*;
mod canvas_view;
mod document;
mod panels;
#[cfg(test)]
mod tests;
mod view;
