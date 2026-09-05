#![allow(deprecated)]

use crate::canvas::CanvasState;
use crate::font_data::{Contour, FontProject, GlyphComponent};
use crate::generator;
use crate::glyph_list;
use crate::history::History;
use crate::io;
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
    pub validation_issues: Vec<crate::export::ValidationIssue>,
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

    fn multi_axis_preview_layer(&self, name: &str) -> Option<crate::font_data::GlyphLayer> {
        if !self.show_interpolation_overlay || self.project.masters.len() < 4 {
            return None;
        }
        let mut axes = std::collections::BTreeSet::new();
        for master in &self.project.masters {
            axes.extend(master.axes.keys().cloned());
        }
        let axes: Vec<String> = axes.into_iter().collect();
        if axes.len() < 2 {
            return None;
        }
        let bounds = |tag: &str| {
            self.project
                .masters
                .iter()
                .filter_map(|master| master.axes.get(tag).copied())
                .fold(None::<(f64, f64)>, |result, value| {
                    Some(match result {
                        Some((min, max)) => (min.min(value), max.max(value)),
                        None => (value, value),
                    })
                })
        };
        let x = bounds(&axes[0])?;
        let y = bounds(&axes[1])?;
        self.project.interpolate_glyph_bilinear(
            name,
            &axes[0],
            &axes[1],
            x.0 + (x.1 - x.0) * self.interpolation_x_factor as f64,
            y.0 + (y.1 - y.0) * self.interpolation_y_factor as f64,
        )
    }

    fn save_state(&mut self) {
        self.project.sync_active_layer(&self.current_master_id);
        self.history.push(&self.project, &self.current_glyph);
    }

    fn clear_canvas_selection(&mut self) {
        self.canvas.selected_points.clear();
        self.canvas.selected_nodes.clear();
        self.canvas.selected_contour = None;
        self.canvas.selected_component = None;
        self.canvas.selected_components.clear();
        self.selected_glyphs.clear();
        self.contour_clipboard = None;
        self.component_clipboard = None;
        self.validation_issues.clear();
        self.selected_guideline = None;
    }

    fn clear_geometry_selection(&mut self) {
        self.canvas.selected_points.clear();
        self.canvas.selected_nodes.clear();
        self.canvas.selected_contour = None;
        self.canvas.selected_component = None;
        self.canvas.selected_components.clear();
        self.component_resize = None;
        self.anchor_drag = None;
        self.spacing_drag = None;
    }

    fn select_component(&mut self, index: usize, additive: bool) {
        if additive {
            if self.canvas.selected_components.is_empty() {
                if let Some(current) = self.canvas.selected_component {
                    self.canvas.selected_components.push(current);
                }
            }
            if let Some(position) = self
                .canvas
                .selected_components
                .iter()
                .position(|&selected| selected == index)
            {
                self.canvas.selected_components.remove(position);
            } else {
                self.canvas.selected_components.push(index);
            }
        } else {
            self.canvas.selected_components = vec![index];
        }
        self.canvas.selected_component = self.canvas.selected_components.last().copied();
    }

    fn selected_component_indices(&self) -> Vec<usize> {
        if self.canvas.selected_components.is_empty() {
            self.canvas.selected_component.into_iter().collect()
        } else {
            self.canvas.selected_components.clone()
        }
    }

    fn duplicate_selected_glyphs(&mut self) -> usize {
        let source_names: Vec<String> = if self.selected_glyphs.is_empty() {
            self.current_glyph.clone().into_iter().collect()
        } else {
            self.project
                .glyph_names_sorted()
                .into_iter()
                .filter(|name| {
                    self.selected_glyphs
                        .iter()
                        .any(|selected| selected == *name)
                })
                .map(str::to_string)
                .collect()
        };
        let mut duplicated_names = Vec::new();
        for source_name in source_names {
            if let Some(new_name) = self.project.duplicate_glyph(&source_name) {
                duplicated_names.push(new_name);
            }
        }
        if let Some(new_name) = duplicated_names.last().cloned() {
            self.current_glyph = Some(new_name);
            self.selected_glyphs = duplicated_names.into_iter().collect();
            self.save_state();
        }
        self.selected_glyphs.len()
    }

    fn undo(&mut self) {
        if let Some(entry) = self.history.undo() {
            self.project = entry.project.clone();
            self.current_glyph = entry.current_glyph.clone();
            self.clear_geometry_selection();
            self.selected_glyphs
                .retain(|name| self.project.glyphs.contains_key(name));
            self.status_message = "取り消しました".to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(entry) = self.history.redo() {
            self.project = entry.project.clone();
            self.current_glyph = entry.current_glyph.clone();
            self.clear_geometry_selection();
            self.selected_glyphs
                .retain(|name| self.project.glyphs.contains_key(name));
            self.status_message = "やり直しました".to_string();
        }
    }

    fn save_project_file(&mut self) {
        let path = self.project_path.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Glyph Studio Project", &["json"])
                .save_file()
        });
        if let Some(path) = path {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            let result = if extension.eq_ignore_ascii_case("glyphs") {
                io::save_glyphs(&self.project, &path)
            } else if extension.eq_ignore_ascii_case("ufo") {
                io::save_ufo(&self.project, &path)
            } else {
                io::save_project(&self.project, &path)
            };
            match result {
                Ok(()) => {
                    self.status_message = format!("プロジェクトを保存しました: {}", path.display());
                    self.project_path = Some(path);
                    self.saved_history_index = self.history.current_index;
                    self.saved_project = Some(self.project.clone());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    pub fn open_document_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let loaded = if extension.eq_ignore_ascii_case("glyphs") {
            io::load_glyphs(path)
        } else if extension.eq_ignore_ascii_case("ufo") {
            io::load_ufo(path)
        } else if extension.eq_ignore_ascii_case("woff2") {
            io::load_woff2(path)
        } else if extension.eq_ignore_ascii_case("woff") {
            io::load_woff(path)
        } else if matches!(extension.to_ascii_lowercase().as_str(), "ttf" | "otf") {
            io::load_ttf(path)
        } else {
            io::load_project(path)
        }?;
        self.project = loaded;
        self.clear_canvas_selection();
        self.current_master_id = self.project.default_master_id.clone();
        self.current_glyph = self
            .project
            .glyph_names_sorted()
            .first()
            .map(|name| name.to_string());
        self.glyph_rename_input = self.current_glyph.clone().unwrap_or_default();
        self.history = History::new(100);
        self.history.push(&self.project, &self.current_glyph);
        self.saved_history_index = self.history.current_index;
        self.saved_project = Some(self.project.clone());
        self.project_path = matches!(
            extension.to_ascii_lowercase().as_str(),
            "json" | "glyphs" | "ufo"
        )
        .then(|| path.to_path_buf());
        self.status_message = format!("ファイルを開きました: {}", path.display());
        Ok(())
    }

    fn request_open_document_path(&mut self, path: &std::path::Path) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        if dirty {
            self.pending_open_path = Some(path.to_path_buf());
            self.show_unsaved_open_dialog = true;
        } else if let Err(error) = self.open_document_path(path) {
            self.status_message = format!("ファイルを開けませんでした: {error}");
        }
    }

    fn create_new_project(&mut self) {
        self.project = FontProject::new();
        self.current_glyph = None;
        self.project_path = None;
        self.current_master_id = "regular".to_string();
        self.clear_canvas_selection();
        self.history = History::new(100);
        self.history.push(&self.project, &self.current_glyph);
        self.saved_history_index = self.history.current_index;
        self.saved_project = Some(self.project.clone());
        self.status_message = "新規プロジェクトを作成しました".to_string();
    }

    fn request_new_project(&mut self) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        if dirty {
            self.pending_open_path = None;
            self.pending_new_project = true;
            self.show_unsaved_open_dialog = true;
        } else {
            self.create_new_project();
        }
    }

    fn show_unsaved_open_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_open_dialog {
            return;
        }
        let mut action = None;
        egui::Window::new("未保存の変更")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("現在のプロジェクトに未保存の変更があります。");
                ui.label(if self.pending_new_project {
                    "新規プロジェクトを作成する前に、変更を保存しますか？"
                } else {
                    "別のファイルを開く前に、変更を保存しますか？"
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("保存して開く").clicked() {
                        self.save_project_file();
                        let saved = self
                            .saved_project
                            .as_ref()
                            .is_some_and(|project| project == &self.project);
                        if saved {
                            action = Some(true);
                        }
                    }
                    if ui.button("破棄して開く").clicked() {
                        action = Some(true);
                    }
                    if ui.button("キャンセル").clicked() {
                        action = Some(false);
                    }
                });
            });
        if let Some(open) = action {
            self.show_unsaved_open_dialog = false;
            if open {
                if let Some(path) = self.pending_open_path.take() {
                    if let Err(error) = self.open_document_path(&path) {
                        self.status_message = format!("ファイルを開けませんでした: {error}");
                    }
                } else if self.pending_new_project {
                    self.pending_new_project = false;
                    self.create_new_project();
                }
            } else {
                self.pending_open_path = None;
                self.pending_new_project = false;
            }
        }
    }

    fn export_ttf_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("TrueType Font", &["ttf"])
            .save_file()
        {
            match crate::export::export_ttf(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("TTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    fn export_otf_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenType Font", &["otf"])
            .save_file()
        {
            match crate::export::export_otf(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("OTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    fn export_otf_cff2_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CFF2 OpenType Font", &["otf"])
            .save_file()
        {
            match crate::export::export_otf_cff2(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("CFF2 OTFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    fn export_woff2_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Web Open Font Format 2", &["woff2"])
            .save_file()
        {
            match crate::export::export_woff2(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("WOFF2を書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    fn export_woff_file(&mut self) {
        self.validation_issues = crate::export::validate_project_detailed(&self.project);
        if !self.validation_issues.is_empty() {
            self.status_message = format!(
                "書き出しを停止しました: {}件の問題を確認してください",
                self.validation_issues.len()
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Web Open Font Format", &["woff"])
            .save_file()
        {
            match crate::export::export_woff(&self.project, &path) {
                Ok(()) => {
                    self.status_message = format!("WOFFを書き出しました: {}", path.display());
                }
                Err(error) => self.status_message = error,
            }
        }
    }

    fn show_node_inspector(&mut self, ui: &mut egui::Ui) {
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

    fn translate_selected_nodes_by(&mut self, dx: f64, dy: f64) {
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

    fn apply_selected_node_action(&mut self, action: NodeAction) {
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

    fn show_component_inspector(&mut self, ui: &mut egui::Ui) {
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

    fn show_menu_bar(&mut self, ctx: &egui::Context) {
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
                            match io::load_svg(&path) {
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
                                io::load_woff2(&path)
                            } else if extension.eq_ignore_ascii_case("woff") {
                                io::load_woff(&path)
                            } else {
                                io::load_ttf(&path)
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
                            match io::save_project(&self.project, &path) {
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
                            match io::save_glyphs(&self.project, &path) {
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
                        let mut issues = crate::export::validate_project_detailed(&self.project);
                        if self.show_interpolation_overlay {
                            issues.extend(crate::export::validate_interpolation(
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
                            match io::save_ufo(&self.project, &path) {
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
                            match crate::export::export_ttf(&self.project, &path) {
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
                            match crate::export::export_otf(&self.project, &path) {
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
                            match crate::export::export_otf_for_master(
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
                            match crate::export::export_all_otf_for_masters(
                                &self.project,
                                &directory,
                            ) {
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
                            match crate::export::export_woff(&self.project, &path) {
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
                            match crate::export::export_woff2(&self.project, &path) {
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
                            match crate::export::export_woff2_for_master(
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
                            match crate::export::export_all_woff2_for_masters(
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
                            match crate::export::export_woff_for_master(
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
                            match crate::export::export_all_woff_for_masters(
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
                            match crate::export::export_ttf_for_master(
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
                            match crate::export::export_ttf_at_interpolation(
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
                            match crate::export::export_interpolation_set(
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
                            match crate::export::export_interpolation_set(
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
                            match crate::export::export_all_ttf_for_masters(
                                &self.project,
                                &directory,
                            ) {
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
                                match crate::export::export_svg_with_palette(
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
                            match crate::export::export_all_svg_with_palette(
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
                            match crate::export::export_all_svg_for_master_with_palette(
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

    fn show_tool_bar(&mut self, ctx: &egui::Context) {
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
                    self.validation_issues =
                        crate::export::validate_project_detailed(&self.project);
                    if self.show_interpolation_overlay {
                        self.validation_issues
                            .extend(crate::export::validate_interpolation(
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
                        generator::generate_all_japanese(&mut self.project);
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

    fn transform_selection(&mut self, scale: f64, angle: f64) {
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

    fn resize_component_from_handle(
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

    fn rotate_component_from_handle(
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

    fn rotate_component_by_angle(
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

    fn fit_width_to_outline(&mut self) {
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

    fn select_relative_glyph(&mut self, delta: isize) {
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

    fn select_relative_master(&mut self, delta: isize) {
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

    fn select_edge_glyph(&mut self, last: bool) {
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

    fn align_left_side_bearing(&mut self) {
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

    fn center_outline_in_width(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        if self.project.center_glyphs_in_width(&[name]) > 0 {
            self.save_state();
            self.status_message = "アウトラインを字幅の中央へ配置しました".to_string();
        }
    }

    fn flip_selection(&mut self, horizontal: bool) {
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

    fn component_visual_center(
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

    fn translate_selected_components_by(&mut self, deltas: &[(usize, f64, f64)]) {
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

    fn align_selected_components(&mut self, horizontal: bool) {
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

    fn distribute_selected_components(&mut self, horizontal: bool) {
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

    fn align_selection(&mut self, horizontal: bool) {
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

    fn distribute_selection(&mut self, horizontal: bool) {
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

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        let mut save_requested = false;
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.set_min_height(26.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_message).small());
                if !self.canvas.selected_nodes.is_empty() {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{}ノード選択",
                            self.canvas.selected_nodes.len()
                        ))
                        .small()
                        .color(Color32::LIGHT_BLUE),
                    );
                }
                if self.edit_all_masters {
                    ui.separator();
                    ui.label(
                        egui::RichText::new("⚠ 全マスター編集 ON")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(245, 183, 77)),
                    )
                    .on_hover_text("ノードとコンポーネントのドラッグが全マスターへ反映されます");
                }
                let dirty = self
                    .saved_project
                    .as_ref()
                    .is_none_or(|saved| saved != &self.project);
                if dirty
                    && ui
                        .small_button("保存")
                        .on_hover_text("現在のプロジェクトを保存（⌘S）")
                        .clicked()
                {
                    save_requested = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("⌘Z Undo  ·  ⌘⇧Z Redo")
                            .small()
                            .color(Color32::GRAY),
                    );
                    ui.label(format!("ズーム: {:.0}%", self.canvas.zoom * 100.0));
                    let master_name = self
                        .project
                        .masters
                        .iter()
                        .find(|master| master.id == self.current_master_id)
                        .map(|master| master.name.as_str())
                        .unwrap_or(self.current_master_id.as_str());
                    ui.label(format!("マスター: {master_name}"));
                    if let Some(name) = &self.current_glyph {
                        ui.label(format!("グリフ: {}", name));
                    }
                });
            });
        });
        if save_requested {
            self.save_project_file();
        }
    }

    fn show_glyph_canvas(&mut self, ctx: &egui::Context) {
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
        let mut fit_view_requested = false;
        let mut width_changed = false;
        let mut bearing_request: Option<(String, f64, f64)> = None;

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgb(36, 37, 44))
                    .inner_margin(egui::Margin::symmetric(12, 7))
                    .show(ui, |ui| {
                        let previous_master = self.current_master_id.clone();
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .small_button("‹")
                                .on_hover_text("前のグリフ (Shift+Tab)")
                                .clicked()
                            {
                                self.select_relative_glyph(-1);
                            }
                            ui.label(
                                egui::RichText::new(
                                    self.current_glyph.as_deref().unwrap_or("グリフ未選択"),
                                )
                                .strong(),
                            );
                            if let Some(name) = self.current_glyph.as_deref() {
                                if let Some(glyph) = self.project.glyphs.get(name) {
                                    let unicode = glyph
                                        .unicode
                                        .map(|value| format!("U+{value:04X}"))
                                        .unwrap_or_else(|| "Unicode未設定".to_string());
                                    ui.label(
                                        egui::RichText::new(unicode)
                                            .small()
                                            .color(Color32::LIGHT_GRAY),
                                    );
                                    let layer = glyph
                                        .layers
                                        .get(&self.current_master_id)
                                        .or_else(|| glyph.layers.values().next());
                                    if let Some(layer) = layer {
                                        let min_x = layer
                                            .contours
                                            .iter()
                                            .flat_map(|contour| contour.points.iter())
                                            .map(|point| point.x)
                                            .fold(f64::INFINITY, f64::min);
                                        let max_x = layer
                                            .contours
                                            .iter()
                                            .flat_map(|contour| contour.points.iter())
                                            .map(|point| point.x)
                                            .fold(f64::NEG_INFINITY, f64::max);
                                        let lsb = if min_x.is_finite() { min_x } else { 0.0 };
                                        let rsb = if max_x.is_finite() {
                                            layer.width - max_x
                                        } else {
                                            layer.width
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "幅 {:.0}  L {:.0}  R {:.0}",
                                                layer.width, lsb, rsb
                                            ))
                                            .small()
                                            .color(Color32::from_rgb(170, 190, 205)),
                                        )
                                        .on_hover_text("現在のマスターの字幅・左右サイドベアリング");
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}輪郭 · {}部品",
                                                layer.contours.len(),
                                                layer.components.len()
                                            ))
                                            .small()
                                            .color(Color32::from_gray(150)),
                                        );
                                    }
                                }
                            }
                            if ui
                                .small_button("›")
                                .on_hover_text("次のグリフ (Tab)")
                                .clicked()
                            {
                                self.select_relative_glyph(1);
                            }
                            ui.separator();
                            let master_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.current_master_id)
                                .map(|master| master.name.clone())
                                .unwrap_or_else(|| self.current_master_id.clone());
                            let master_ids: Vec<String> =
                                self.project.masters.iter().map(|master| master.id.clone()).collect();
                            let current_master_index = master_ids
                                .iter()
                                .position(|id| id == &self.current_master_id)
                                .unwrap_or(0);
                            if ui
                                .add_enabled(current_master_index > 0, egui::Button::new("‹"))
                                .on_hover_text("前のマスター（⌘↑）")
                                .clicked()
                            {
                                let index = current_master_index.saturating_sub(1);
                                self.current_master_id = master_ids[index].clone();
                            }
                            egui::ComboBox::from_id_salt("canvas_current_master")
                                .selected_text(format!(
                                    "マスター {} / {}: {master_name}",
                                    current_master_index + 1,
                                    master_ids.len()
                                ))
                                .width(118.0)
                                .show_ui(ui, |ui| {
                                    for master in &self.project.masters {
                                        ui.selectable_value(
                                            &mut self.current_master_id,
                                            master.id.clone(),
                                            &master.name,
                                        );
                                    }
                                });
                            if ui
                                .add_enabled(
                                    current_master_index + 1 < master_ids.len(),
                                    egui::Button::new("›"),
                                )
                                .on_hover_text("次のマスター（⌘↓）")
                                .clicked()
                            {
                                let index = (current_master_index + 1)
                                    .min(master_ids.len().saturating_sub(1));
                                self.current_master_id = master_ids[index].clone();
                            }
                            if self.project.masters.len() >= 2 {
                                if !self.project.masters.iter().any(|master| {
                                    master.id == self.interpolation_from_master
                                }) {
                                    self.interpolation_from_master =
                                        self.project.masters[0].id.clone();
                                }
                                if !self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == self.interpolation_to_master)
                                {
                                    self.interpolation_to_master = self
                                        .project
                                        .masters
                                        .last()
                                        .map(|master| master.id.clone())
                                        .unwrap_or_default();
                                }
                                ui.toggle_value(&mut self.show_interpolation_overlay, "比較")
                                    .on_hover_text("補間結果のオーバーレイ表示");
                                ui.toggle_value(&mut self.show_all_masters_overlay, "全マスター")
                                    .on_hover_text("現在のマスター以外の輪郭を薄く重ねて表示");
                                let all_masters_response = ui
                                    .checkbox(&mut self.edit_all_masters, "全マスターへ反映")
                                    .on_hover_text(
                                        "ノードとコンポーネントのドラッグを全マスターへ同期",
                                    );
                                if all_masters_response.changed() {
                                    self.status_message = if self.edit_all_masters {
                                        "全マスター編集をONにしました。ノード移動は全マスターへ反映されます"
                                            .to_string()
                                    } else {
                                        "全マスター編集をOFFにしました".to_string()
                                    };
                                }
                                if self.show_all_masters_overlay {
                                    let overlay_colors = [
                                        Color32::from_rgb(255, 150, 150),
                                        Color32::from_rgb(130, 210, 255),
                                        Color32::from_rgb(180, 140, 255),
                                    ];
                                    for (index, master) in self.project.masters.iter().enumerate() {
                                        if master.id != self.current_master_id {
                                            ui.colored_label(
                                                overlay_colors[index % overlay_colors.len()],
                                                &master.name,
                                            );
                                        }
                                    }
                                }
                                if self.show_interpolation_overlay {
                                    egui::ComboBox::from_id_salt("canvas_interpolation_from")
                                        .selected_text(
                                            self.project
                                                .masters
                                                .iter()
                                                .find(|master| {
                                                    master.id == self.interpolation_from_master
                                                })
                                                .map(|master| format!("始点 {}", master.name))
                                                .unwrap_or_else(|| "始点".to_string()),
                                        )
                                        .show_ui(ui, |ui| {
                                            for master in &self.project.masters {
                                                ui.add_enabled_ui(
                                                    master.id != self.interpolation_to_master,
                                                    |ui| {
                                                        ui.selectable_value(
                                                            &mut self.interpolation_from_master,
                                                            master.id.clone(),
                                                            &master.name,
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    egui::ComboBox::from_id_salt("canvas_interpolation_to")
                                        .selected_text(
                                            self.project
                                                .masters
                                                .iter()
                                                .find(|master| {
                                                    master.id == self.interpolation_to_master
                                                })
                                                .map(|master| format!("終点 {}", master.name))
                                                .unwrap_or_else(|| "終点".to_string()),
                                        )
                                        .show_ui(ui, |ui| {
                                            for master in &self.project.masters {
                                                ui.add_enabled_ui(
                                                    master.id != self.interpolation_from_master,
                                                    |ui| {
                                                        ui.selectable_value(
                                                            &mut self.interpolation_to_master,
                                                            master.id.clone(),
                                                            &master.name,
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.interpolation_factor,
                                            0.0..=1.0,
                                        )
                                        .text("補間"),
                                    );
                                    let mut overlay_axes = std::collections::BTreeSet::new();
                                    for master in &self.project.masters {
                                        overlay_axes.extend(master.axes.keys().cloned());
                                    }
                                    let overlay_axes: Vec<String> = overlay_axes.into_iter().collect();
                                    if overlay_axes.len() >= 2 {
                                        ui.small(format!(
                                            "2軸補間: {} × {}",
                                            overlay_axes[0], overlay_axes[1]
                                        ));
                                        ui.add(
                                            egui::Slider::new(
                                                &mut self.interpolation_x_factor,
                                                0.0..=1.0,
                                            )
                                            .text(&overlay_axes[0]),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut self.interpolation_y_factor,
                                                0.0..=1.0,
                                            )
                                            .text(&overlay_axes[1]),
                                        );
                                    }
                                }
                            }
                            if let Some(name) = self.current_glyph.clone() {
                                let mut width = self
                                    .project
                                    .glyphs
                                    .get(&name)
                                    .map(|glyph| glyph.width)
                                    .unwrap_or_default();
                                let width_response = ui
                                    .add(
                                        egui::DragValue::new(&mut width)
                                            .prefix("幅 ")
                                            .suffix(" u")
                                            .speed(1.0),
                                    )
                                    .on_hover_text("現在グリフの字幅（全マスターへ反映）");
                                if width_response.drag_started() {
                                    self.width_drag_active = true;
                                }
                                let width_value_changed = width_response.changed();
                                if width_value_changed {
                                    self.project.set_width_for_glyphs(
                                        std::slice::from_ref(&name),
                                        width,
                                    );
                                }
                                if width_response.drag_stopped() {
                                    self.width_drag_active = false;
                                    width_changed = true;
                                } else if width_value_changed && !self.width_drag_active {
                                    width_changed = true;
                                }
                                if let Some((min_x, _, max_x, _)) =
                                    self.project.outline_bounds_for_glyph(&name)
                                {
                                    let width = self
                                        .project
                                        .glyphs
                                        .get(&name)
                                        .map(|glyph| glyph.width)
                                        .unwrap_or_default();
                                    let mut left = min_x;
                                    let mut right = width - max_x;
                                    let left_changed = ui
                                        .add(
                                            egui::DragValue::new(&mut left)
                                                .prefix("LSB ")
                                                .speed(1.0),
                                        )
                                        .on_hover_text("左サイドベアリング（全マスター）\nキャンバス上のオレンジ線をドラッグして調整")
                                        .changed();
                                    let right_changed = ui
                                        .add(
                                            egui::DragValue::new(&mut right)
                                                .prefix("RSB ")
                                                .speed(1.0),
                                        )
                                        .on_hover_text("右サイドベアリング（全マスター）\nキャンバス上のオレンジ線をドラッグして調整")
                                        .changed();
                                    if left_changed || right_changed {
                                        bearing_request = Some((name, left, right));
                                    }
                                }
                            }
                            ui.separator();
                            if ui.small_button("−").on_hover_text("ズームアウト").clicked()
                            {
                                self.canvas.zoom = (self.canvas.zoom / 1.15).clamp(0.05, 20.0);
                            }
                            if ui
                                .small_button("100%")
                                .on_hover_text("ズームを100%に戻す")
                                .clicked()
                            {
                                self.canvas.zoom = 1.0;
                            }
                            if ui.small_button("＋").on_hover_text("ズームイン").clicked() {
                                self.canvas.zoom = (self.canvas.zoom * 1.15).clamp(0.05, 20.0);
                            }
                            let mut zoom_percent = self.canvas.zoom * 100.0;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut zoom_percent)
                                        .suffix("%")
                                        .range(5.0..=2000.0)
                                        .speed(1.0),
                                )
                                .on_hover_text("ズーム倍率を直接入力")
                                .changed()
                            {
                                self.canvas.zoom = (zoom_percent / 100.0).clamp(0.05, 20.0);
                            }
                            if ui
                                .small_button("中央")
                                .on_hover_text("表示位置を中央に戻す")
                                .clicked()
                            {
                                self.canvas.pan = Vec2::ZERO;
                            }
                            if ui
                                .small_button("全体")
                                .on_hover_text("現在のグリフ全体をキャンバスに収める (F)")
                                .clicked()
                            {
                                fit_view_requested = true;
                            }
                            ui.separator();
                            ui.toggle_value(&mut self.canvas.show_grid, "グリッド")
                                .on_hover_text("グリッド表示の切り替え");
                            ui.toggle_value(&mut self.canvas.snap_to_grid, "吸着")
                                .on_hover_text("グリッドへのスナップ切り替え");
                            ui.toggle_value(&mut self.canvas.snap_to_guidelines, "ガイド吸着")
                                .on_hover_text("水平・垂直ガイドへのスナップ切り替え");
                            ui.toggle_value(&mut self.canvas.snap_to_anchors, "アンカー吸着")
                                .on_hover_text("現在のグリフのアンカーへのスナップ切り替え");
                            ui.toggle_value(&mut self.canvas.show_guidelines, "ガイド")
                                .on_hover_text("ガイド表示の切り替え (G)");
                            ui.toggle_value(
                                &mut self.canvas.show_contour_direction,
                                "輪郭方向",
                            )
                            .on_hover_text("輪郭の進行方向を表示");
                            ui.toggle_value(&mut self.canvas.show_node_indices, "ノード番号")
                                .on_hover_text("ノード番号を表示 (N)");
                            ui.toggle_value(&mut self.canvas.show_anchors, "アンカー")
                                .on_hover_text("アンカー表示の切り替え");
                            ui.toggle_value(&mut self.canvas.show_background_images, "背景")
                                .on_hover_text("背景画像表示の切り替え");
                            ui.toggle_value(&mut self.show_side_glyphs, "前後字形")
                                .on_hover_text("現在のグリフの左右に隣接する字形を薄く表示（スペーシング確認）");
                            let has_canvas_selection = !self.canvas.selected_nodes.is_empty()
                                || !self.canvas.selected_points.is_empty()
                                || self.canvas.selected_component.is_some();
                            if has_canvas_selection
                                && ui
                                    .small_button("選択解除")
                                    .on_hover_text("キャンバス上の選択をすべて解除")
                                    .clicked()
                            {
                                self.clear_canvas_selection();
                            }
                            let selection_label = if self.canvas.selected_components.len() > 1 {
                                format!(
                                    "選択: {}部品",
                                    self.canvas.selected_components.len()
                                )
                            } else if let Some(component_index) =
                                self.canvas.selected_component
                            {
                                self.current_glyph
                                    .as_ref()
                                    .and_then(|name| self.project.glyphs.get(name))
                                    .and_then(|glyph| glyph.components.get(component_index))
                                    .map(|component| format!("選択: 部品 {}", component.base))
                                    .unwrap_or_else(|| "選択: 部品".to_string())
                            } else if !self.canvas.selected_nodes.is_empty() {
                                format!("選択: {}ノード", self.canvas.selected_nodes.len())
                            } else if !self.canvas.selected_points.is_empty() {
                                format!("選択: {}ノード", self.canvas.selected_points.len())
                            } else {
                                "選択: なし".to_string()
                            };
                            ui.colored_label(
                                if has_canvas_selection {
                                    Color32::from_rgb(255, 210, 80)
                                } else {
                                    Color32::from_rgb(150, 155, 170)
                                },
                                selection_label,
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(format!("ツール: {}", self.current_tool.name()));
                                },
                            );
                        });
                        if self.current_master_id != previous_master {
                            self.selected_guideline = None;
                            self.guideline_drag = None;
                            self.project
                                .switch_master(&previous_master, &self.current_master_id);
                            self.save_state();
                            let display_name = self
                                .project
                                .masters
                                .iter()
                                .find(|master| master.id == self.current_master_id)
                                .map(|master| master.name.as_str())
                                .unwrap_or(self.current_master_id.as_str());
                            self.status_message =
                                format!("マスターを{}に切り替えました", display_name);
                        }
                        if width_changed {
                            self.save_state();
                            self.status_message = "字幅を変更しました".to_string();
                        }
                        if let Some((name, left, right)) = bearing_request.take() {
                            if self.project.set_side_bearings(&[name], left, right) > 0 {
                                self.save_state();
                                self.status_message = "サイドベアリングを変更しました".to_string();
                            }
                        }
                    });
                let available_size = ui.available_size();
                let (response, painter) =
                    ui.allocate_painter(available_size, egui::Sense::click_and_drag());

                let rect = response.rect;
                let origin = Pos2::new(
                    rect.center().x + self.canvas.pan.x,
                    rect.center().y + self.canvas.pan.y,
                );

                painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));
                let empty_glyph = self
                    .current_glyph
                    .as_ref()
                    .and_then(|name| self.project.glyphs.get(name))
                    .and_then(|glyph| {
                        glyph
                            .layers
                            .get(&self.current_master_id)
                            .or_else(|| glyph.layers.values().next())
                    })
                    .is_some_and(|layer| {
                        layer.contours.is_empty() && layer.components.is_empty()
                    });
                if self.current_glyph.is_none() || empty_glyph {
                    let title = self
                        .current_glyph
                        .as_deref()
                        .map_or("グリフを選択", |name| {
                            if empty_glyph {
                                name
                            } else {
                                "グリフを選択"
                            }
                        });
                    painter.text(
                        rect.center() - Vec2::new(0.0, 18.0),
                        egui::Align2::CENTER_CENTER,
                        title,
                        egui::FontId::proportional(20.0),
                        Color32::from_rgb(205, 210, 222),
                    );
                    painter.text(
                        rect.center() + Vec2::new(0.0, 16.0),
                        egui::Align2::CENTER_CENTER,
                        "ペンツール (P) で描き始める  ·  SVGを読み込むこともできます",
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(135, 143, 160),
                    );
                }
                if self.current_tool == Tool::Select {
                    if let (Some(mouse_pos), Some(name)) =
                        (response.hover_pos(), &self.current_glyph)
                    {
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        let anchor_hit = self.canvas.show_anchors
                            && (glyph
                                .layers
                                .get(&self.current_master_id)
                                .map(|layer| {
                                    layer.anchors.iter().any(|anchor| {
                                        self.canvas
                                            .glyph_to_screen(anchor.x, anchor.y, origin)
                                            .distance(mouse_pos)
                                            <= 9.0
                                    })
                                })
                                .unwrap_or(false)
                                || glyph.anchors.iter().any(|anchor| {
                                    self.canvas
                                        .glyph_to_screen(anchor.x, anchor.y, origin)
                                        .distance(mouse_pos)
                                        <= 9.0
                                }));
                        let advance_x = self.canvas.glyph_to_screen(glyph.width, 0.0, origin).x;
                        let bearing_edge = self
                            .project
                            .outline_bounds_for_glyph(name)
                            .map(|(min_x, _, max_x, _)| {
                                let left_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                                let right_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
                                (mouse_pos.x - left_x).abs() <= 8.0
                                    || (mouse_pos.x - right_x).abs() <= 8.0
                            })
                            .unwrap_or(false);
                        if anchor_hit {
                            ctx.set_cursor_icon(egui::CursorIcon::Move);
                        } else if (mouse_pos.x - advance_x).abs() <= 8.0 || bearing_edge {
                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                    }
                    }
                }
                if let Some(pointer) = response.hover_pos() {
                    let (x, y) = self.canvas.screen_to_glyph(pointer, origin);
                    painter.text(
                        rect.right_bottom() - Vec2::new(12.0, 10.0),
                        egui::Align2::RIGHT_BOTTOM,
                        format!("X {:>7.1}  Y {:>7.1}", x, y),
                        egui::FontId::monospace(11.0),
                        Color32::from_gray(170),
                    );
                }

                self.canvas.draw_grid(&painter, rect, origin);

                self.canvas.draw_metrics(
                    &painter,
                    rect,
                    origin,
                    self.project.metadata.units_per_em,
                    self.project.metadata.ascender,
                    self.project.metadata.descender,
                    self.current_glyph
                        .as_ref()
                        .and_then(|name| self.project.glyphs.get(name))
                        .map(|glyph| glyph.width)
                    .unwrap_or(self.project.metadata.units_per_em),
                );
                if self.show_side_glyphs {
                    if let Some(current_name) = self.current_glyph.as_deref() {
                        let names = self.project.glyph_names_sorted();
                        if let Some(current_index) =
                            names.iter().position(|name| *name == current_name)
                        {
                            let layer_width = |name: &str| {
                                self.project
                                    .glyphs
                                    .get(name)
                                    .and_then(|glyph| {
                                        glyph
                                            .layers
                                            .get(&self.current_master_id)
                                            .map(|layer| layer.width)
                                            .or(Some(glyph.width))
                                    })
                                    .unwrap_or(self.project.metadata.units_per_em)
                            };
                            let side_color = Color32::from_rgba_premultiplied(130, 155, 185, 72);
                            if current_index > 0 {
                                let previous = names[current_index - 1];
                                let previous_origin = Pos2::new(
                                    origin.x
                                        - ((layer_width(previous)
                                            + self
                                                .project
                                                .kerning_for_glyphs(previous, current_name)
                                                .unwrap_or(0.0))
                                            as f32
                                            * self.canvas.zoom),
                                    origin.y,
                                );
                                self.canvas.draw_master_overlay(
                                    &painter,
                                    &self.project,
                                    previous,
                                    &self.current_master_id,
                                    previous_origin,
                                    side_color,
                                );
                                painter.text(
                                    Pos2::new(previous_origin.x, rect.bottom() - 10.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("‹ {previous}"),
                                    egui::FontId::monospace(10.0),
                                    Color32::from_rgba_premultiplied(170, 190, 215, 150),
                                );
                            }
                            if let Some(next) = names.get(current_index + 1) {
                                let next_origin = Pos2::new(
                                    origin.x
                                        + ((layer_width(current_name)
                                            + self
                                                .project
                                                .kerning_for_glyphs(current_name, next)
                                                .unwrap_or(0.0))
                                            as f32
                                            * self.canvas.zoom),
                                    origin.y,
                                );
                                self.canvas.draw_master_overlay(
                                    &painter,
                                    &self.project,
                                    next,
                                    &self.current_master_id,
                                    next_origin,
                                    side_color,
                                );
                                painter.text(
                                    Pos2::new(next_origin.x, rect.bottom() - 10.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("{next} ›"),
                                    egui::FontId::monospace(10.0),
                                    Color32::from_rgba_premultiplied(170, 190, 215, 150),
                                );
                            }
                        }
                    }
                }
                if let Some(name) = &self.current_glyph {
                    if let Some((min_x, _, max_x, _)) = self.project.outline_bounds_for_glyph(name)
                    {
                        let lsb_x = self.canvas.glyph_to_screen(min_x, 0.0, origin).x;
                        let rsb_x = self.canvas.glyph_to_screen(max_x, 0.0, origin).x;
                        let bearing_stroke = Stroke::new(
                            0.8_f32,
                            Color32::from_rgba_premultiplied(255, 180, 80, 130),
                        );
                        for x in [lsb_x, rsb_x] {
                            painter.line_segment(
                                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                                bearing_stroke,
                            );
                        }
                        let width = self
                            .project
                            .glyphs
                            .get(name)
                            .map(|glyph| glyph.width)
                            .unwrap_or_default();
                        painter.text(
                            Pos2::new(lsb_x + 4.0, rect.bottom() - 22.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("LSB {:.0}", min_x),
                            egui::FontId::monospace(10.0),
                            Color32::from_rgb(255, 190, 100),
                        );
                        painter.text(
                            Pos2::new(rsb_x - 4.0, rect.bottom() - 22.0),
                            egui::Align2::RIGHT_BOTTOM,
                            format!("RSB {:.0}", width - max_x),
                            egui::FontId::monospace(10.0),
                            Color32::from_rgb(255, 190, 100),
                        );
                    }
                }
                if self.canvas.show_guidelines {
                    self.canvas
                        .draw_guidelines(
                            &painter,
                            self.project.guidelines_for_master(&self.current_master_id),
                            rect,
                            origin,
                        );
                    if let Some(GuidelineTarget::Global(index)) = self.selected_guideline {
                        if let Some(guide) = self
                            .project
                            .guidelines_for_master(&self.current_master_id)
                            .get(index)
                        {
                            self.canvas
                                .draw_guideline_highlight(&painter, guide, rect, origin);
                        }
                    }
                }

                if let Some(name) = &self.current_glyph {
                    if self.canvas.show_background_images {
                        if let Some(path) = self
                            .project
                            .background_images
                            .get(name)
                            .and_then(|masters| masters.get(&self.current_master_id))
                            .filter(|path| !path.trim().is_empty())
                            .cloned()
                        {
                            let modified = std::fs::metadata(&path)
                                .and_then(|metadata| metadata.modified())
                                .ok();
                            let cache_is_current = self
                                .background_cache
                                .get(&path)
                                .is_some_and(|(cached_modified, _)| *cached_modified == modified);
                            if !cache_is_current {
                                if let Ok(bytes) = std::fs::read(&path) {
                                    let is_svg =
                                        std::path::Path::new(&path).extension().is_some_and(
                                            |extension| extension.eq_ignore_ascii_case("svg"),
                                        );
                                    let image = if is_svg {
                                        egui_extras::RetainedImage::from_svg_bytes(
                                            format!("background:{path}"),
                                            &bytes,
                                        )
                                    } else {
                                        egui_extras::RetainedImage::from_image_bytes(
                                            format!("background:{path}"),
                                            &bytes,
                                        )
                                    };
                                    if let Ok(image) = image {
                                        self.background_cache
                                            .insert(path.clone(), (modified, image));
                                    }
                                }
                            }
                            if let Some((_, image)) = self.background_cache.get(&path) {
                                let opacity = self
                                    .project
                                    .background_opacities
                                    .get(name)
                                    .and_then(|masters| masters.get(&self.current_master_id))
                                    .copied()
                                    .unwrap_or(0.35);
                                let transform = self
                                    .project
                                    .background_transforms
                                    .get(name)
                                    .and_then(|masters| masters.get(&self.current_master_id))
                                    .copied()
                                    .unwrap_or(crate::font_data::BackgroundImageTransform {
                                        x: 0.0,
                                        y: 0.0,
                                        scale: 1.0,
                                        rotation: 0.0,
                                        flip_x: false,
                                        flip_y: false,
                                    });
                                self.canvas.draw_background_image(
                                    &painter,
                                    image.texture_id(ctx),
                                    image.size(),
                                    origin,
                                    opacity,
                                    transform,
                                );
                            }
                        }
                    }
                    if self.canvas.show_guidelines {
                        if let Some(glyph) = self.project.glyphs.get(name) {
                            self.canvas.draw_guidelines(
                                &painter,
                                glyph.guidelines_for_master(&self.current_master_id),
                                rect,
                                origin,
                            );
                            if let Some(GuidelineTarget::Glyph(index)) = self.selected_guideline {
                                if let Some(guide) = glyph
                                    .guidelines_for_master(&self.current_master_id)
                                    .get(index)
                                {
                                    self.canvas
                                        .draw_guideline_highlight(&painter, guide, rect, origin);
                                }
                            }
                        }
                    }
                    if self.show_interpolation_overlay && self.project.masters.len() >= 2 {
                        if let Some(glyph) = self.project.glyphs.get(name) {
                            let mut drew_bilinear = false;
                            let mut axis_tags = std::collections::BTreeSet::new();
                            for master in &self.project.masters {
                                axis_tags.extend(master.axes.keys().cloned());
                            }
                            let axis_tags: Vec<String> = axis_tags.into_iter().collect();
                            if axis_tags.len() >= 2 {
                                let axis_bounds = |tag: &str| {
                                    let values: Vec<f64> = self
                                        .project
                                        .masters
                                        .iter()
                                        .filter_map(|master| master.axes.get(tag).copied())
                                        .collect();
                                    values.iter().copied().fold(
                                        None::<(f64, f64)>,
                                        |bounds, value| {
                                        Some(match bounds {
                                            Some((min, max)) => (min.min(value), max.max(value)),
                                            None => (value, value),
                                        })
                                    })
                                };
                                let x_bounds = axis_bounds(&axis_tags[0]);
                                let y_bounds = axis_bounds(&axis_tags[1]);
                                let target_x = x_bounds.map_or(0.0, |(min, max)| {
                                    min + (max - min) * self.interpolation_x_factor as f64
                                });
                                let target_y = y_bounds.map_or(0.0, |(min, max)| {
                                    min + (max - min) * self.interpolation_y_factor as f64
                                });
                                if let Some(layer) = self.project.interpolate_glyph_bilinear(
                                    name,
                                    &axis_tags[0],
                                    &axis_tags[1],
                                    target_x,
                                    target_y,
                                ) {
                                    self.canvas.draw_layer(
                                        &painter,
                                        &layer,
                                        origin,
                                        Color32::from_rgba_premultiplied(120, 180, 255, 90),
                                    );
                                    if self.canvas.show_anchors {
                                        self.canvas.draw_anchors(&painter, &layer.anchors, origin);
                                    }
                                    drew_bilinear = true;
                                }
                            }
                            if !drew_bilinear {
                                let first = if self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == self.interpolation_from_master)
                                {
                                    &self.interpolation_from_master
                                } else {
                                    &self.project.masters[0].id
                                };
                                let last = if self
                                    .project
                                    .masters
                                    .iter()
                                    .any(|master| master.id == self.interpolation_to_master)
                                {
                                    &self.interpolation_to_master
                                } else {
                                    &self.project.masters[self.project.masters.len() - 1].id
                                };
                                if let (Some(a), Some(b)) =
                                    (glyph.layers.get(first), glyph.layers.get(last))
                                {
                                    if let Some(layer) =
                                        a.interpolate(b, self.interpolation_factor as f64)
                                    {
                                        self.canvas.draw_layer(
                                            &painter,
                                            &layer,
                                            origin,
                                            Color32::from_rgba_premultiplied(120, 180, 255, 90),
                                        );
                                        if self.canvas.show_anchors {
                                            self.canvas.draw_anchors(
                                                &painter,
                                                &layer.anchors,
                                                origin,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if self.show_all_masters_overlay {
                        if let Some(glyph) = self.project.glyphs.get(name) {
                            for (index, master) in self.project.masters.iter().enumerate() {
                                if master.id == self.current_master_id {
                                    continue;
                                }
                                if glyph.layers.contains_key(&master.id) {
                                    let colors = [
                                        Color32::from_rgba_premultiplied(255, 130, 130, 75),
                                        Color32::from_rgba_premultiplied(130, 210, 255, 75),
                                        Color32::from_rgba_premultiplied(180, 140, 255, 75),
                                    ];
                                    self.canvas.draw_master_overlay(
                                        &painter,
                                        &self.project,
                                        name,
                                        &master.id,
                                        origin,
                                        colors[index % colors.len()],
                                    );
                                }
                            }
                        }
                    }
                    let mut axis_values = self
                        .project
                        .masters
                        .iter()
                        .find(|master| master.id == self.current_master_id)
                        .map(|master| master.axes.clone())
                        .unwrap_or_default();
                    if let Some(master) = self
                        .project
                        .masters
                        .iter()
                        .find(|master| master.id == self.current_master_id)
                    {
                        axis_values.entry("wght".into()).or_insert(master.weight);
                        axis_values.entry("wdth".into()).or_insert(master.width);
                    }
                    if let Some(layer) =
                        self.project.conditional_layer_for_glyph(name, &axis_values)
                    {
                        self.canvas.draw_conditional_layer(
                            &painter,
                            &self.project,
                            &layer.layer,
                            &self.current_master_id,
                            origin,
                            Color32::from_rgb(220, 180, 90),
                        );
                        if self.canvas.show_anchors {
                            self.canvas
                                .draw_anchors(&painter, &layer.layer.anchors, origin);
                        }
                    } else if !self.canvas.draw_color_glyph(
                        &painter,
                        &self.project,
                        name,
                        &self.current_master_id,
                        self.preview_color_palette,
                        origin,
                    ) {
                    self.canvas
                        .draw_glyph_recursive(&painter, &self.project, name, origin);
                    if self.canvas.show_contour_direction {
                        if let Some(glyph) = self.project.glyphs.get(name) {
                            self.canvas
                                .draw_contour_directions(&painter, &glyph.contours, origin);
                        }
                    }
                    }
                    if self.canvas.show_anchors {
                        let anchors = self.project.anchors_for_glyph(name);
                        self.canvas.draw_anchors(&painter, &anchors, origin);
                    }
                    let selected_components = self.selected_component_indices();
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        for component in selected_components {
                            self.canvas.draw_component_selection(
                                &painter,
                                &self.project,
                                glyph,
                                component,
                                origin,
                                !self.edit_all_masters,
                            );
                        }
                    }
                }

                self.canvas
                    .draw_pen_preview(&painter, &self.pen_state, origin);
                self.canvas.draw_selection(&painter);
                self.canvas.draw_ruler(&painter, origin);

                // Handle keyboard shortcuts
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
                    } else if !wants_keyboard_input
                        && i.modifiers.command
                        && i.key_pressed(Key::Minus)
                    {
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
                response.context_menu(|ui| {
                    if ui.button("選択を解除").clicked() {
                        self.clear_canvas_selection();
                        ui.close_menu();
                    }
                    if ui.button("グリフの全ノードを選択").clicked() {
                        if let Some(name) = &self.current_glyph {
                            if let Some(glyph) = self.project.glyphs.get(name) {
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
                                    .map(|contour| (0..contour.points.len()).collect())
                                    .unwrap_or_default();
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !self.canvas.selected_nodes.is_empty(),
                            egui::Button::new("スムーズノードにする"),
                        )
                        .clicked()
                    {
                        self.apply_selected_node_action(NodeAction::Smooth);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !self.canvas.selected_nodes.is_empty(),
                            egui::Button::new("コーナーノードにする"),
                        )
                        .clicked()
                    {
                        self.apply_selected_node_action(NodeAction::Corner);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("全体表示").clicked() {
                        self.fit_current_glyph_to_canvas(rect);
                        ui.close_menu();
                    }
                });

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

    fn fit_current_glyph_to_canvas(&mut self, rect: egui::Rect) {
        let Some(name) = self.current_glyph.as_deref() else {
            return;
        };
        let bounds = self.project.outline_bounds_for_glyph(name);
        let Some((min_x, min_y, max_x, max_y)) = bounds else {
            self.canvas.zoom = 1.0;
            self.canvas.pan = Vec2::ZERO;
            return;
        };
        let width = (max_x - min_x).max(1.0) + 200.0;
        let height = (max_y - min_y).max(1.0) + 200.0;
        self.canvas.zoom = ((rect.width() as f64 / width).min(rect.height() as f64 / height))
            .clamp(0.05, 50.0) as f32;
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        self.canvas.pan = Vec2::new(
            (-center_x * self.canvas.zoom as f64) as f32,
            (center_y * self.canvas.zoom as f64) as f32,
        );
    }

    fn decompose_current_components(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(components) = self
            .project
            .glyphs
            .get(&name)
            .map(|glyph| glyph.components.clone())
        else {
            return;
        };
        let mut contours = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        for component in &components {
            collect_decomposed_contours(
                &self.project,
                &component.base,
                component_transform(component),
                &mut visiting,
                &mut contours,
            );
        }
        let master_ids: Vec<String> = self
            .project
            .glyphs
            .get(&name)
            .into_iter()
            .flat_map(|glyph| glyph.layers.keys().cloned())
            .collect();
        let mut layer_contours = Vec::new();
        for master_id in master_ids {
            let mut decomposed = Vec::new();
            let mut visiting = std::collections::HashSet::new();
            for component in &components {
                collect_decomposed_contours_for_master(
                    &self.project,
                    &component.base,
                    &master_id,
                    component_transform(component),
                    &mut visiting,
                    &mut decomposed,
                );
            }
            layer_contours.push((master_id, decomposed));
        }
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            if contours.is_empty() {
                return;
            }
            glyph.contours.extend(contours);
            glyph.components.clear();
            for (master_id, decomposed) in layer_contours {
                if let Some(layer) = glyph.layers.get_mut(&master_id) {
                    layer.contours.extend(decomposed);
                    layer.components.clear();
                }
            }
            self.canvas.selected_contour = None;
            self.canvas.selected_points.clear();
            self.canvas.selected_nodes.clear();
            self.save_state();
            self.status_message = "コンポーネントを輪郭化しました".to_string();
        }
    }

    fn decompose_named_components(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            let Some(components) = self.project.glyphs.get(name).map(|g| g.components.clone())
            else {
                continue;
            };
            if components.is_empty() {
                continue;
            }
            let mut contours = Vec::new();
            let mut visiting = std::collections::HashSet::new();
            for component in &components {
                collect_decomposed_contours(
                    &self.project,
                    &component.base,
                    component_transform(component),
                    &mut visiting,
                    &mut contours,
                );
            }
            let master_ids: Vec<String> = self
                .project
                .glyphs
                .get(name)
                .into_iter()
                .flat_map(|glyph| glyph.layers.keys().cloned())
                .collect();
            let mut layer_contours = Vec::new();
            for master_id in master_ids {
                let mut decomposed = Vec::new();
                let mut visiting = std::collections::HashSet::new();
                for component in &components {
                    collect_decomposed_contours_for_master(
                        &self.project,
                        &component.base,
                        &master_id,
                        component_transform(component),
                        &mut visiting,
                        &mut decomposed,
                    );
                }
                layer_contours.push((master_id, decomposed));
            }
            if let Some(glyph) = self.project.glyphs.get_mut(name) {
                if !contours.is_empty() {
                    glyph.contours.extend(contours);
                    glyph.components.clear();
                    for (master_id, decomposed) in layer_contours {
                        if let Some(layer) = glyph.layers.get_mut(&master_id) {
                            layer.contours.extend(decomposed);
                            layer.components.clear();
                        }
                    }
                    changed += 1;
                }
            }
        }
        changed
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

fn preview_mark_attachment(
    project: &FontProject,
    base_name: &str,
    mark_name: &str,
) -> Option<(f32, f32)> {
    let mark_anchors = project.anchors_for_glyph(mark_name);
    project
        .anchors_for_glyph(base_name)
        .into_iter()
        .filter(|anchor| !anchor.name.starts_with('_'))
        .find_map(|base_anchor| {
            let mark_anchor = mark_anchors
                .iter()
                .find(|anchor| anchor.name == format!("_{}", base_anchor.name))?;
            Some((
                (base_anchor.x - mark_anchor.x) as f32,
                (base_anchor.y - mark_anchor.y) as f32,
            ))
        })
}

fn preview_context_sequences(parts: &[&str]) -> Vec<(Vec<String>, usize)> {
    let mut groups: Vec<(Vec<String>, bool)> = Vec::new();
    let mut logical_parts = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        if parts[index].starts_with('[') && !parts[index].contains(']') {
            let mut combined = parts[index].to_string();
            index += 1;
            while index < parts.len() {
                combined.push(' ');
                combined.push_str(parts[index]);
                if parts[index].contains(']') {
                    break;
                }
                index += 1;
            }
            logical_parts.push(combined);
        } else {
            logical_parts.push(parts[index].to_string());
        }
        index += 1;
    }
    for raw in &logical_parts {
        let mut token = raw.as_str();
        let marked = token.ends_with('\'');
        if marked {
            token = &token[..token.len() - 1];
        }
        if token.starts_with('[') && token.ends_with(']') {
            token = &token[1..token.len() - 1];
        }
        let choices = token
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return Vec::new();
        }
        groups.push((choices, marked));
    }
    let marked_indices = groups
        .iter()
        .enumerate()
        .filter_map(|(index, (_, marked))| marked.then_some(index))
        .collect::<Vec<_>>();
    if marked_indices.len() != 1 {
        return Vec::new();
    }
    let target = marked_indices[0];
    let mut sequences = vec![(Vec::new(), target)];
    for (choices, _) in groups {
        let mut next = Vec::new();
        for (sequence, target_index) in &sequences {
            for choice in &choices {
                let mut expanded = sequence.clone();
                expanded.push(choice.clone());
                next.push((expanded, *target_index));
            }
        }
        sequences = next;
    }
    sequences
}

fn preview_glyph_names(project: &FontProject, text: &str, enabled_features: &str) -> Vec<String> {
    let mut names: Vec<String> = text
        .chars()
        .map(|character| glyph_name_for_project_char(project, character))
        .collect();
    let mut ligatures = Vec::new();
    let mut substitutions = Vec::new();
    let mut multiples = Vec::new();
    let mut alternates = Vec::new();
    let mut contexts = Vec::new();
    let enabled: std::collections::HashSet<&str> = enabled_features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect();
    let feature_source = project.feature_source();
    let expanded_features = crate::export::expand_named_feature_classes(&feature_source);
    let rule_sources = crate::export::extract_feature_blocks(&expanded_features);
    let source = if rule_sources.is_empty() {
        expanded_features
    } else {
        rule_sources
            .into_iter()
            .filter(|(tag, _)| {
                enabled.contains(std::str::from_utf8(&tag.to_be_bytes()).unwrap_or(""))
            })
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join(";")
    };
    for statement in source.split(';') {
        let tokens: Vec<_> = statement.split_whitespace().collect();
        let Some(sub) = tokens.iter().position(|token| *token == "sub") else {
            continue;
        };
        if sub + 2 < tokens.len() && tokens[sub + 2] == "from" {
            let from = tokens[sub + 1].trim_matches(|character: char| "[]".contains(character));
            let choices = tokens[sub + 3..]
                .iter()
                .map(|name| name.trim_matches(|character: char| "[]".contains(character)))
                .filter(|name| project.glyphs.contains_key(*name))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !from.is_empty() && !choices.is_empty() {
                alternates.push((from.to_string(), choices[0].clone()));
            }
        } else {
            let Some(by) = tokens.iter().position(|token| *token == "by") else {
                continue;
            };
            fn clean_token(token: &str) -> &str {
                token.trim_matches(|character: char| "[]'".contains(character))
            }
            let marked: Vec<_> = tokens[sub + 1..by]
                .iter()
                .enumerate()
                .filter(|(_, token)| token.ends_with('\''))
                .collect();
            if marked.len() == 1 && by > sub + 2 && by + 1 < tokens.len() {
                let replacement = clean_token(tokens[by + 1]).to_string();
                let mut parsed_context = false;
                if project.glyphs.contains_key(&replacement) {
                    for (sequence, target_index) in preview_context_sequences(&tokens[sub + 1..by])
                    {
                        if sequence
                            .iter()
                            .all(|name| project.glyphs.contains_key(name))
                        {
                            contexts.push((sequence, target_index, replacement.clone()));
                            parsed_context = true;
                        }
                    }
                }
                if parsed_context {
                    continue;
                }
            }
            if by > sub + 2 && tokens[sub + 1].starts_with('[') && tokens[by + 1].starts_with('[') {
                let from = tokens[sub + 1..by]
                    .iter()
                    .map(|token| clean_token(token).to_string())
                    .collect::<Vec<_>>();
                let to = tokens[by + 1..]
                    .iter()
                    .map(|token| clean_token(token).to_string())
                    .collect::<Vec<_>>();
                if from.len() == to.len() {
                    for (from, to) in from.into_iter().zip(to) {
                        if project.glyphs.contains_key(&from) && project.glyphs.contains_key(&to) {
                            substitutions.push((from, to));
                        }
                    }
                }
                continue;
            }
            if by > sub + 2 && by + 1 < tokens.len() {
                ligatures.push((
                    tokens[sub + 1..by]
                        .iter()
                        .map(|name| (*name).to_string())
                        .collect::<Vec<_>>(),
                    tokens[by + 1]
                        .trim_matches(|character: char| "[]".contains(character))
                        .to_string(),
                ));
            } else if by == sub + 2 && by + 1 < tokens.len() {
                let from = tokens[sub + 1].trim_matches(|character: char| "[]".contains(character));
                let replacements = tokens[by + 1..]
                    .iter()
                    .map(|name| name.trim_matches(|character: char| "[]".contains(character)))
                    .filter(|name| project.glyphs.contains_key(*name))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if project.glyphs.contains_key(from) && replacements.len() == 1 {
                    substitutions.push((from.to_string(), replacements[0].clone()));
                } else if project.glyphs.contains_key(from) && replacements.len() > 1 {
                    multiples.push((from.to_string(), replacements));
                }
            }
        }
    }
    for (components, replacement) in ligatures {
        if components.len() < 2 || !project.glyphs.contains_key(&replacement) {
            continue;
        }
        let mut index = 0;
        while index + components.len() <= names.len() {
            if names[index..index + components.len()] == components[..] {
                names.splice(index..index + components.len(), [replacement.clone()]);
            } else {
                index += 1;
            }
        }
    }
    for (from, to) in substitutions {
        for name in &mut names {
            if *name == from {
                *name = to.clone();
            }
        }
    }
    for (from, to) in alternates {
        for name in &mut names {
            if *name == from {
                *name = to.clone();
            }
        }
    }
    for (sequence, target_index, replacement) in contexts {
        if sequence.len() > names.len() {
            continue;
        }
        let mut index = 0;
        while index + sequence.len() <= names.len() {
            if names[index..index + sequence.len()] == sequence[..] {
                names[index + target_index] = replacement.clone();
                index += sequence.len();
            } else {
                index += 1;
            }
        }
    }
    for (from, replacements) in multiples {
        let mut index = 0;
        while index < names.len() {
            if names[index] == from {
                names.splice(index..=index, replacements.clone());
                index += replacements.len();
            } else {
                index += 1;
            }
        }
    }
    names
}

fn preview_feature_enabled(features: &str, tag: &str) -> bool {
    features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .any(|candidate| candidate == tag)
}

fn toggle_preview_feature(features: &mut String, tag: &str) {
    let mut tags: Vec<String> = features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty() && *candidate != tag)
        .map(str::to_string)
        .collect();
    if !preview_feature_enabled(features, tag) {
        tags.push(tag.to_string());
    }
    *features = tags.join(",");
}

fn preview_contour_points(contour: &Contour, origin: Pos2, scale: f32) -> Vec<Pos2> {
    let mut points = Vec::new();
    flatten(contour.to_bezpath(), 0.5, |element| {
        if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
            points.push(Pos2::new(
                origin.x + point.x as f32 * scale,
                origin.y - point.y as f32 * scale,
            ));
        }
    });
    points
}

type PreviewTransform = (f64, f64, f64, f64, f64, f64);

fn component_transform(component: &GlyphComponent) -> PreviewTransform {
    (
        component.x_scale,
        component.xy_scale,
        component.yx_scale,
        component.y_scale,
        component.x_offset,
        component.y_offset,
    )
}

fn max_projected_outline_x(
    project: &FontProject,
    glyph_name: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
) -> Option<f64> {
    if !visiting.insert(glyph_name.to_string()) {
        return None;
    }
    let mut max_x = None;
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (a, b, _, _, e, _) = transform;
        for contour in &glyph.contours {
            flatten(contour.to_bezpath(), 0.25, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let x = a * point.x + b * point.y + e;
                    if x.is_finite() {
                        max_x = Some(max_x.map_or(x, |current: f64| current.max(x)));
                    }
                }
            });
        }
        for component in &glyph.components {
            let child_max = max_projected_outline_x(
                project,
                &component.base,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
            );
            if let Some(x) = child_max {
                max_x = Some(max_x.map_or(x, |current: f64| current.max(x)));
            }
        }
    }
    visiting.remove(glyph_name);
    max_x
}

fn min_projected_outline_x(
    project: &FontProject,
    glyph_name: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
) -> Option<f64> {
    if !visiting.insert(glyph_name.to_string()) {
        return None;
    }
    let mut min_x = None;
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (a, b, _, _, e, _) = transform;
        for contour in &glyph.contours {
            flatten(contour.to_bezpath(), 0.25, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let x = a * point.x + b * point.y + e;
                    if x.is_finite() {
                        min_x = Some(min_x.map_or(x, |current: f64| current.min(x)));
                    }
                }
            });
        }
        for component in &glyph.components {
            let child_min = min_projected_outline_x(
                project,
                &component.base,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
            );
            if let Some(x) = child_min {
                min_x = Some(min_x.map_or(x, |current: f64| current.min(x)));
            }
        }
    }
    visiting.remove(glyph_name);
    min_x
}

fn compose_preview_transform(
    parent: PreviewTransform,
    child: PreviewTransform,
) -> PreviewTransform {
    let (a, b, c, d, e, f) = parent;
    let (g, h, i, j, k, l) = child;
    (
        a * g + b * i,
        a * h + b * j,
        c * g + d * i,
        c * h + d * j,
        a * k + b * l + e,
        c * k + d * l + f,
    )
}

fn preview_nested_component_polygons(
    project: &FontProject,
    glyph_name: &str,
    origin: Pos2,
    scale: f32,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
    polygons: &mut Vec<Vec<Pos2>>,
) {
    if !visiting.insert(glyph_name.to_string()) {
        return;
    }
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        for contour in &glyph.contours {
            let mut points = Vec::new();
            flatten(contour.to_bezpath(), 0.5, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let (a, b, c, d, e, f) = transform;
                    points.push(Pos2::new(
                        origin.x + (a * point.x + b * point.y + e) as f32 * scale,
                        origin.y - (c * point.x + d * point.y + f) as f32 * scale,
                    ));
                }
            });
            polygons.push(points);
        }
        for component in &glyph.components {
            preview_nested_component_polygons(
                project,
                &component.base,
                origin,
                scale,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
                polygons,
            );
        }
    }
    visiting.remove(glyph_name);
}

fn glyph_name_for_char(ch: char) -> String {
    format!("uni{:04X}", ch as u32)
}

fn glyph_name_for_project_char(project: &FontProject, ch: char) -> String {
    let codepoint = ch as u32;
    project
        .glyphs
        .values()
        .find(|glyph| glyph.unicode == Some(codepoint) || glyph.unicodes.contains(&codepoint))
        .map(|glyph| glyph.name.clone())
        .unwrap_or_else(|| glyph_name_for_char(ch))
}

fn master_compatibility_issues(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
) -> Vec<String> {
    let mut issues = Vec::new();
    for glyph in project.glyphs.values() {
        let Some(from) = glyph.layers.get(from_master_id) else {
            continue;
        };
        let Some(to) = glyph.layers.get(to_master_id) else {
            continue;
        };
        if from.interpolate(to, 0.5).is_none() {
            let reason = if from.contours.len() != to.contours.len() {
                "輪郭数"
            } else if from
                .contours
                .iter()
                .zip(&to.contours)
                .any(|(a, b)| a.points.len() != b.points.len())
            {
                "ノード数"
            } else if from.components.len() != to.components.len() {
                "コンポーネント数"
            } else if from.anchors.len() != to.anchors.len() {
                "アンカー数"
            } else if from
                .components
                .iter()
                .zip(&to.components)
                .any(|(a, b)| a.base != b.base)
            {
                "コンポーネント名"
            } else if from
                .anchors
                .iter()
                .any(|a| !to.anchors.iter().any(|b| a.name == b.name))
            {
                "アンカー名"
            } else if from.contours.iter().zip(&to.contours).any(|(a, b)| {
                a.points
                    .iter()
                    .zip(&b.points)
                    .any(|(from_point, to_point)| from_point.point_type != to_point.point_type)
            }) {
                "ノード種別"
            } else {
                "構成"
            };
            issues.push(format!("{}: {}が不一致", glyph.name, reason));
        }
    }
    issues.sort();
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_resolves_standard_and_alias_glyph_names() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        assert_eq!(glyph_name_for_project_char(&project, 'A'), "A");
        let mut alias = crate::font_data::GlyphData::new("alt-a".into(), None);
        alias.unicodes.push('Å' as u32);
        project.glyphs.insert("alt-a".into(), alias);
        assert_eq!(glyph_name_for_project_char(&project, 'Å'), "alt-a");
    }

    #[test]
    fn nested_preview_transform_composes_translation_and_scale() {
        let parent = (2.0, 0.0, 0.0, 2.0, 10.0, 20.0);
        let child = (1.0, 0.0, 0.0, 1.0, 3.0, 4.0);
        assert_eq!(
            compose_preview_transform(parent, child),
            (2.0, 0.0, 0.0, 2.0, 16.0, 28.0)
        );
    }

    #[test]
    fn preview_mark_attachment_uses_matching_anchor_pair() {
        let mut project = FontProject::new();
        let mut base = crate::font_data::GlyphData::new("A".into(), Some('A' as u32));
        base.anchors.push(crate::font_data::GlyphAnchor {
            name: "top".into(),
            x: 250.0,
            y: 700.0,
        });
        project.glyphs.insert("A".into(), base);
        let mut mark = crate::font_data::GlyphData::new("acutecomb".into(), None);
        mark.anchors.push(crate::font_data::GlyphAnchor {
            name: "_top".into(),
            x: 30.0,
            y: 40.0,
        });
        project.glyphs.insert("acutecomb".into(), mark);
        assert_eq!(
            preview_mark_attachment(&project, "A", "acutecomb"),
            Some((220.0, 660.0))
        );
    }

    #[test]
    fn preview_applies_ligature_rules() {
        let mut project = FontProject::new();
        project.add_glyph("f".into(), Some('f' as u32));
        project.add_glyph("i".into(), Some('i' as u32));
        project.add_glyph("fi".into(), None);
        project.opentype_features = "feature liga { sub f i by fi; } liga;".into();
        assert_eq!(preview_glyph_names(&project, "fi", "liga"), vec!["fi"]);
    }

    #[test]
    fn preview_applies_single_substitution_rules() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.alt".into(), None);
        project.opentype_features = "feature salt { sub A by A.alt; } salt;".into();
        assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
    }

    #[test]
    fn preview_applies_contextual_substitution_rules() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.add_glyph("A.alt".into(), None);
        project.opentype_features = "feature calt { sub A' B by A.alt; } calt;".into();
        assert_eq!(
            preview_glyph_names(&project, "AB", "calt"),
            vec!["A.alt", "B"]
        );
        assert_eq!(
            preview_glyph_names(&project, "AC", "calt"),
            vec!["A", "uni0043"]
        );
    }

    #[test]
    fn preview_applies_multiple_and_alternate_rules() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.add_glyph("A.alt".into(), None);
        project.add_glyph("B.alt".into(), None);
        project.opentype_features = "feature cv01 { sub A by A.alt B.alt; } cv01; feature salt { sub B from [B.alt]; } salt;".into();
        assert_eq!(
            preview_glyph_names(&project, "AB", "cv01,salt"),
            vec!["A.alt", "B.alt", "B.alt"]
        );
    }

    #[test]
    fn preview_applies_class_substitution_rules() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.add_glyph("A.alt".into(), None);
        project.add_glyph("B.alt".into(), None);
        project.opentype_features = "feature ss01 { sub [A B] by [A.alt B.alt]; } ss01;".into();
        assert_eq!(
            preview_glyph_names(&project, "AB", "ss01"),
            vec!["A.alt", "B.alt"]
        );
    }

    #[test]
    fn preview_expands_named_feature_classes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.alt".into(), None);
        project.opentype_features =
            "@caps = [A]; feature salt { sub @caps by A.alt; } salt;".into();
        assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
    }

    #[test]
    fn preview_applies_contextual_target_class() {
        let mut project = FontProject::new();
        for (name, unicode) in [("A", 'A'), ("C", 'C')] {
            project.add_glyph(name.into(), Some(unicode as u32));
        }
        project.add_glyph("C.alt".into(), None);
        project.opentype_features = "feature calt { sub A [C]' by C.alt; } calt;".into();
        assert_eq!(
            preview_glyph_names(&project, "AC", "calt"),
            vec!["A", "C.alt"]
        );
    }

    #[test]
    fn preview_applies_each_choice_in_contextual_target_class() {
        let mut project = FontProject::new();
        for (name, unicode) in [("A", 'A'), ("C", 'C'), ("D", 'D')] {
            project.add_glyph(name.into(), Some(unicode as u32));
        }
        project.add_glyph("C.alt".into(), None);
        project.opentype_features = "feature calt { sub A [C D]' by C.alt; } calt;".into();
        assert_eq!(
            preview_glyph_names(&project, "ACD", "calt"),
            vec!["A", "C.alt", "D"]
        );
        assert_eq!(
            preview_glyph_names(&project, "AD", "calt"),
            vec!["A", "C.alt"]
        );
    }

    #[test]
    fn preview_feature_enabled_matches_comma_or_space_separated_tags() {
        assert!(preview_feature_enabled("liga, kern", "kern"));
        assert!(!preview_feature_enabled("liga salt", "kern"));
    }

    #[test]
    fn toggle_preview_feature_preserves_other_tags_and_toggles_requested_tag() {
        let mut features = "liga, kern".to_string();
        toggle_preview_feature(&mut features, "kern");
        assert_eq!(features, "liga");
        toggle_preview_feature(&mut features, "mark");
        assert_eq!(features, "liga,mark");
    }

    #[test]
    fn preview_ignores_disabled_feature_rules() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.alt".into(), None);
        project.opentype_features = "feature salt { sub A by A.alt; } salt;".into();
        assert_eq!(preview_glyph_names(&project, "A", "liga"), vec!["A"]);
        assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
    }

    #[test]
    fn decomposition_flattens_nested_components_and_stops_cycles() {
        let mut project = FontProject::new();
        let mut base = crate::font_data::GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![crate::font_data::ContourPoint::on_curve(10.0, 20.0)],
        });
        project.glyphs.insert("base".into(), base);

        let mut middle = crate::font_data::GlyphData::new("middle".into(), None);
        middle.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 2.0,
            x_offset: 5.0,
            y_offset: 7.0,
        });
        project.glyphs.insert("middle".into(), middle);

        let mut cycle = crate::font_data::GlyphData::new("cycle".into(), None);
        cycle.components.push(GlyphComponent {
            base: "cycle".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("cycle".into(), cycle);

        let mut output = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        collect_decomposed_contours(
            &project,
            "middle",
            (1.0, 0.0, 0.0, 1.0, 3.0, 4.0),
            &mut visiting,
            &mut output,
        );
        assert_eq!(output.len(), 1);
        assert_eq!((output[0].points[0].x, output[0].points[0].y), (28.0, 51.0));

        output.clear();
        collect_decomposed_contours(
            &project,
            "cycle",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut visiting,
            &mut output,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn projected_outline_bounds_include_nested_component_offsets() {
        let mut project = FontProject::new();
        let mut base = crate::font_data::GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                crate::font_data::ContourPoint::on_curve(-20.0, 0.0),
                crate::font_data::ContourPoint::on_curve(40.0, 0.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = crate::font_data::GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 100.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("composite".into(), composite);
        let mut visiting = std::collections::HashSet::new();
        assert_eq!(
            min_projected_outline_x(
                &project,
                "composite",
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut visiting,
            ),
            Some(80.0)
        );
        assert_eq!(
            max_projected_outline_x(
                &project,
                "composite",
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut visiting,
            ),
            Some(140.0)
        );
    }

    #[test]
    fn relative_glyph_selection_wraps_in_glyph_order() {
        let mut app = GlyphStudioApp::default();
        app.project.add_glyph("A".into(), Some('A' as u32));
        app.project.add_glyph("B".into(), Some('B' as u32));
        app.current_glyph = Some("B".into());
        app.select_relative_glyph(1);
        assert_eq!(app.current_glyph.as_deref(), Some("A"));
        app.select_relative_glyph(-1);
        assert_eq!(app.current_glyph.as_deref(), Some("B"));
        app.select_edge_glyph(false);
        assert_eq!(app.current_glyph.as_deref(), Some("A"));
        app.select_edge_glyph(true);
        assert_eq!(app.current_glyph.as_deref(), Some("B"));
    }

    #[test]
    fn master_compatibility_reports_structural_mismatch() {
        let mut project = FontProject::new();
        let mut glyph = crate::font_data::GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "regular".into(),
            crate::font_data::GlyphLayer {
                width: 600.0,
                contours: vec![Contour {
                    points: vec![
                        crate::font_data::ContourPoint::on_curve(0.0, 0.0),
                        crate::font_data::ContourPoint::on_curve(100.0, 0.0),
                        crate::font_data::ContourPoint::on_curve(0.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            crate::font_data::GlyphLayer {
                width: 600.0,
                contours: vec![],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        let issues = master_compatibility_issues(&project, "regular", "bold");
        assert_eq!(issues, vec!["A: 輪郭数が不一致"]);

        let mut component_glyph = crate::font_data::GlyphData::new("B".into(), None);
        let component = |base: &str| crate::font_data::GlyphComponent {
            base: base.into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        };
        component_glyph.layers.insert(
            "regular".into(),
            crate::font_data::GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: vec![component("acute")],
                anchors: Vec::new(),
            },
        );
        component_glyph.layers.insert(
            "bold".into(),
            crate::font_data::GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: vec![component("grave")],
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("B".into(), component_glyph);
        let issues = master_compatibility_issues(&project, "regular", "bold");
        assert_eq!(
            issues,
            vec!["A: 輪郭数が不一致", "B: コンポーネント名が不一致"]
        );
    }
}
