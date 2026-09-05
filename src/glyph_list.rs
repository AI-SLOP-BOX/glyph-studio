use crate::font_data::{FontProject, GlyphData};
use egui::{Color32, Pos2, RichText, ScrollArea, Stroke, Ui, Vec2};
use kurbo::{flatten, PathEl};
use std::collections::HashSet;

fn draw_thumbnail(
    painter: &egui::Painter,
    rect: egui::Rect,
    project: &FontProject,
    glyph: &GlyphData,
) {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include_point = |x: f64, y: f64| {
        bounds = Some(match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    };
    for contour in &glyph.contours {
        for point in &contour.points {
            include_point(point.x, point.y);
        }
    }
    let mut visited = HashSet::new();
    for component in &glyph.components {
        include_component_bounds(
            project,
            &component.base,
            [
                component.x_scale,
                component.xy_scale,
                component.yx_scale,
                component.y_scale,
                component.x_offset,
                component.y_offset,
            ],
            4,
            &mut bounds,
            &mut visited,
        );
    }
    let Some((min_x, min_y, max_x, max_y)) = bounds else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "·",
            egui::FontId::proportional(22.0),
            Color32::from_gray(100),
        );
        return;
    };
    let scale = (rect.width() as f64 / (max_x - min_x).max(1.0))
        .min(rect.height() as f64 / (max_y - min_y).max(1.0))
        * 0.82;
    let map = |x: f64, y: f64| {
        Pos2::new(
            rect.center().x + ((x - (min_x + max_x) * 0.5) * scale) as f32,
            rect.center().y - ((y - (min_y + max_y) * 0.5) * scale) as f32,
        )
    };
    for contour in &glyph.contours {
        let mut previous = None;
        let mut subpath_start = None;
        flatten(contour.to_bezpath(), 0.5, |element| match element {
            PathEl::MoveTo(point) => {
                let mapped = map(point.x, point.y);
                previous = Some(mapped);
                subpath_start = Some(mapped);
            }
            PathEl::LineTo(point) => {
                let current = map(point.x, point.y);
                if let Some(start) = previous {
                    painter.line_segment(
                        [start, current],
                        Stroke::new(1.2_f32, Color32::from_rgb(210, 215, 225)),
                    );
                }
                previous = Some(current);
            }
            PathEl::ClosePath => {
                if let (Some(start), Some(end)) = (subpath_start, previous) {
                    painter.line_segment(
                        [end, start],
                        Stroke::new(1.2_f32, Color32::from_rgb(210, 215, 225)),
                    );
                }
                previous = subpath_start;
            }
            PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
        });
    }
    for component in &glyph.components {
        draw_component_thumbnail(
            painter,
            project,
            &component.base,
            [
                component.x_scale,
                component.xy_scale,
                component.yx_scale,
                component.y_scale,
                component.x_offset,
                component.y_offset,
            ],
            &map,
            4,
            &mut HashSet::new(),
        );
    }
}

fn include_component_bounds(
    project: &FontProject,
    glyph_name: &str,
    transform: [f64; 6],
    depth: usize,
    bounds: &mut Option<(f64, f64, f64, f64)>,
    visited: &mut HashSet<String>,
) {
    if !visited.insert(glyph_name.to_string()) {
        return;
    }
    let Some(glyph) = project.glyphs.get(glyph_name) else {
        visited.remove(glyph_name);
        return;
    };
    let mut include = |x: f64, y: f64| {
        *bounds = Some(match *bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    };
    for contour in &glyph.contours {
        for point in &contour.points {
            include(
                transform[0] * point.x + transform[2] * point.y + transform[4],
                transform[1] * point.x + transform[3] * point.y + transform[5],
            );
        }
    }
    if depth == 0 {
        visited.remove(glyph_name);
        return;
    }
    for component in &glyph.components {
        let child = [
            component.x_scale,
            component.xy_scale,
            component.yx_scale,
            component.y_scale,
            component.x_offset,
            component.y_offset,
        ];
        let composed = [
            transform[0] * child[0] + transform[2] * child[1],
            transform[1] * child[0] + transform[3] * child[1],
            transform[0] * child[2] + transform[2] * child[3],
            transform[1] * child[2] + transform[3] * child[3],
            transform[0] * child[4] + transform[2] * child[5] + transform[4],
            transform[1] * child[4] + transform[3] * child[5] + transform[5],
        ];
        include_component_bounds(
            project,
            &component.base,
            composed,
            depth - 1,
            bounds,
            visited,
        );
    }
    visited.remove(glyph_name);
}

fn draw_component_thumbnail(
    painter: &egui::Painter,
    project: &FontProject,
    glyph_name: &str,
    transform: [f64; 6],
    map: &dyn Fn(f64, f64) -> Pos2,
    depth: usize,
    visited: &mut HashSet<String>,
) {
    if !visited.insert(glyph_name.to_string()) {
        return;
    }
    let Some(glyph) = project.glyphs.get(glyph_name) else {
        visited.remove(glyph_name);
        return;
    };
    let apply = |x: f64, y: f64| {
        (
            transform[0] * x + transform[2] * y + transform[4],
            transform[1] * x + transform[3] * y + transform[5],
        )
    };
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(170, 200, 225));
    for contour in &glyph.contours {
        let mut previous = None;
        let mut subpath_start = None;
        flatten(contour.to_bezpath(), 0.5, |element| match element {
            PathEl::MoveTo(point) => {
                let mapped = apply(point.x, point.y);
                let mapped = map(mapped.0, mapped.1);
                previous = Some(mapped);
                subpath_start = Some(mapped);
            }
            PathEl::LineTo(point) => {
                let mapped = apply(point.x, point.y);
                let mapped = map(mapped.0, mapped.1);
                if let Some(start) = previous {
                    painter.line_segment([start, mapped], stroke);
                }
                previous = Some(mapped);
            }
            PathEl::ClosePath => {
                if let (Some(start), Some(end)) = (subpath_start, previous) {
                    painter.line_segment([end, start], stroke);
                }
                previous = subpath_start;
            }
            PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => {}
        });
    }
    if depth == 0 {
        visited.remove(glyph_name);
        return;
    }
    for component in &glyph.components {
        let child = [
            component.x_scale,
            component.xy_scale,
            component.yx_scale,
            component.y_scale,
            component.x_offset,
            component.y_offset,
        ];
        let composed = [
            transform[0] * child[0] + transform[2] * child[1],
            transform[1] * child[0] + transform[3] * child[1],
            transform[0] * child[2] + transform[2] * child[3],
            transform[1] * child[2] + transform[3] * child[3],
            transform[0] * child[4] + transform[2] * child[5] + transform[4],
            transform[1] * child[4] + transform[3] * child[5] + transform[5],
        ];
        draw_component_thumbnail(
            painter,
            project,
            &component.base,
            composed,
            map,
            depth - 1,
            visited,
        );
    }
    visited.remove(glyph_name);
}

fn select_glyph(
    selected: &mut Option<String>,
    selected_glyphs: &mut HashSet<String>,
    current_glyph: &Option<String>,
    visible_names: &[&str],
    name: &str,
    shift: bool,
    command: bool,
) {
    *selected = Some(name.to_string());
    if shift {
        if let (Some(anchor), Some(clicked)) = (
            current_glyph.as_deref(),
            visible_names.iter().position(|item| *item == name),
        ) {
            if let Some(start) = visible_names.iter().position(|item| *item == anchor) {
                let (low, high) = if start <= clicked {
                    (start, clicked)
                } else {
                    (clicked, start)
                };
                selected_glyphs.clear();
                selected_glyphs.extend(
                    visible_names[low..=high]
                        .iter()
                        .map(|item| (*item).to_string()),
                );
            }
        } else {
            selected_glyphs.insert(name.to_string());
        }
    } else if command {
        if !selected_glyphs.remove(name) {
            selected_glyphs.insert(name.to_string());
        }
    } else {
        selected_glyphs.clear();
        selected_glyphs.insert(name.to_string());
    }
}

#[allow(clippy::too_many_arguments)]
pub fn show_glyph_list(
    ui: &mut Ui,
    project: &FontProject,
    current_glyph: &Option<String>,
    search: &mut String,
    focus_search: &mut bool,
    sort_by_unicode: &mut bool,
    only_unassigned: &mut bool,
    grid_view: &mut bool,
    selected_glyphs: &mut HashSet<String>,
) -> Option<String> {
    let mut selected = current_glyph.clone();

    let mut names = project.glyph_names_sorted();
    if *sort_by_unicode {
        names.sort_by_key(|name| project.glyphs[*name].unicode.unwrap_or(u32::MAX));
    }

    let visible_names: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| {
            let glyph = &project.glyphs[*name];
            if *only_unassigned && (glyph.unicode.is_some() || !glyph.unicodes.is_empty()) {
                return false;
            }
            let query = search.trim();
            if query.is_empty() {
                return true;
            }
            let query_lower = query.to_ascii_lowercase();
            let character = glyph
                .unicode
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
                .unwrap_or_default();
            let extra_codepoints = glyph
                .unicodes
                .iter()
                .map(|unicode| format!(" U+{unicode:04X}"))
                .collect::<String>();
            let extra_characters = glyph
                .unicodes
                .iter()
                .filter_map(|unicode| char::from_u32(*unicode))
                .collect::<String>();
            name.to_ascii_lowercase().contains(&query_lower)
                || format!("U+{:04X}", glyph.unicode.unwrap_or(0))
                    .to_ascii_lowercase()
                    .contains(&query_lower)
                || character.contains(query)
                || extra_codepoints.to_ascii_lowercase().contains(&query_lower)
                || extra_characters.contains(query)
        })
        .collect();

    ui.horizontal(|ui| {
        ui.label("検索:");
        let response = ui.add(
            egui::TextEdit::singleline(search)
                .id_salt("glyph_search_input")
                .hint_text("名前 / Unicode / 文字"),
        );
        if *focus_search {
            response.request_focus();
            *focus_search = false;
        }
        if !search.is_empty() && ui.small_button("×").clicked() {
            search.clear();
        }
        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            search.clear();
        }
        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            if let Some(name) = visible_names.first() {
                selected = Some((*name).to_string());
            }
        }
    });

    if let Some(name) = current_glyph {
        if let Some(glyph) = project.glyphs.get(name) {
            let display_unicode = glyph.unicode.or_else(|| glyph.unicodes.first().copied());
            let character = display_unicode.and_then(char::from_u32).unwrap_or('·');
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(43, 45, 52))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(character.to_string()).size(28.0).strong());
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).strong());
                            if let Some(unicode) = display_unicode {
                                ui.label(
                                    RichText::new(format!("U+{unicode:04X}"))
                                        .small()
                                        .color(Color32::GRAY),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Unicode未設定").small().color(Color32::GRAY),
                                );
                            }
                            if !glyph.unicodes.is_empty() {
                                ui.label(
                                    RichText::new(format!(
                                        "別名 {}",
                                        glyph
                                            .unicodes
                                            .iter()
                                            .map(|unicode| format!("U+{unicode:04X}"))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ))
                                    .small()
                                    .color(Color32::GRAY),
                                );
                            }
                            let node_count: usize = glyph
                                .contours
                                .iter()
                                .map(|contour| contour.points.len())
                                .sum();
                            ui.label(
                                RichText::new(format!(
                                    "幅 {:.0}  輪郭 {}  ノード {}",
                                    glyph.width,
                                    glyph.contours.len(),
                                    node_count
                                ))
                                .small()
                                .color(Color32::GRAY),
                            );
                        });
                    });
                });
        }
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(if visible_names.len() == names.len() {
                format!("{} glyphs", names.len())
            } else {
                format!("表示 {} / {} glyphs", visible_names.len(), names.len())
            })
            .small()
            .color(Color32::GRAY),
        );
        ui.separator();
        if ui.small_button("全選択").clicked() {
            selected_glyphs.clear();
            selected_glyphs.extend(visible_names.iter().map(|name| (*name).to_string()));
        }
        if ui.small_button("選択解除").clicked() {
            selected_glyphs.clear();
        }
        if ui.small_button("表示を反転").clicked() {
            let visible_set: HashSet<&str> = visible_names.iter().copied().collect();
            let previously_selected: HashSet<String> = selected_glyphs
                .iter()
                .filter(|name| visible_set.contains(name.as_str()))
                .cloned()
                .collect();
            selected_glyphs.retain(|name| !visible_set.contains(name.as_str()));
            for name in &visible_names {
                if !previously_selected.contains(*name) {
                    selected_glyphs.insert((*name).to_string());
                }
            }
        }
        if !selected_glyphs.is_empty() {
            ui.label(format!("{}件選択", selected_glyphs.len()));
        }
        egui::ComboBox::from_id_salt("glyph_list_sort")
            .selected_text(if *sort_by_unicode {
                "Unicode順"
            } else {
                "名前順"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(sort_by_unicode, false, "名前順");
                ui.selectable_value(sort_by_unicode, true, "Unicode順");
            });
        ui.toggle_value(only_unassigned, "Unicode未設定");
        ui.toggle_value(grid_view, "グリッド");
    });

    let visible_count = visible_names.len();
    if visible_names.is_empty() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("該当するグリフがありません").strong());
                if !search.trim().is_empty() {
                    ui.label(format!("「{}」に一致する項目はありません", search.trim()));
                }
                if ui.button("検索をクリア").clicked() {
                    search.clear();
                }
            });
        });
    }
    ScrollArea::vertical().show(ui, |ui| {
        if *grid_view {
            let columns = ((ui.available_width() / 68.0).floor() as usize).clamp(1, 8);
            egui::Grid::new("glyph_grid")
                .num_columns(columns)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    for (index, name) in visible_names.iter().enumerate() {
                        let glyph = &project.glyphs[*name];
                        let display_unicode =
                            glyph.unicode.or_else(|| glyph.unicodes.first().copied());
                        let character = display_unicode.and_then(char::from_u32).unwrap_or('·');
                        let is_current = current_glyph.as_deref() == Some(*name);
                        let is_selected = selected_glyphs.contains(*name);
                        let (cell, response) =
                            ui.allocate_exact_size(Vec2::new(62.0, 68.0), egui::Sense::click());
                        let fill = if is_current {
                            Color32::from_rgb(75, 70, 35)
                        } else if is_selected {
                            Color32::from_rgb(45, 65, 85)
                        } else {
                            Color32::from_rgb(48, 49, 57)
                        };
                        ui.painter().rect_filled(cell, 4.0, fill);
                        draw_thumbnail(
                            ui.painter(),
                            egui::Rect::from_min_max(
                                cell.left_top() + Vec2::new(5.0, 4.0),
                                cell.right_top() + Vec2::new(-5.0, 39.0),
                            ),
                            project,
                            glyph,
                        );
                        ui.painter().text(
                            Pos2::new(cell.center().x, cell.bottom() - 12.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{character}  {name}"),
                            egui::FontId::proportional(11.0),
                            if is_current {
                                Color32::YELLOW
                            } else {
                                Color32::WHITE
                            },
                        );
                        let response = response.on_hover_text({
                            let missing = glyph
                                .components
                                .iter()
                                .filter(|component| !project.glyphs.contains_key(&component.base))
                                .map(|component| component.base.as_str())
                                .collect::<Vec<_>>();
                            let warning = if missing.is_empty() {
                                String::new()
                            } else {
                                format!("\n⚠ 未解決: {}", missing.join(", "))
                            };
                            format!(
                                "{}  幅 {:.0}\n輪郭 {}・ノード {}・コンポーネント {}{}",
                                name,
                                glyph.width,
                                glyph.contours.len(),
                                glyph
                                    .contours
                                    .iter()
                                    .map(|contour| contour.points.len())
                                    .sum::<usize>(),
                                glyph.components.len(),
                                warning
                            )
                        });
                        if response.clicked() {
                            let modifiers = ui.input(|input| input.modifiers);
                            select_glyph(
                                &mut selected,
                                selected_glyphs,
                                current_glyph,
                                &visible_names,
                                name,
                                modifiers.shift,
                                modifiers.command || modifiers.ctrl,
                            );
                        }
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
            return;
        }
        for name in &visible_names {
            let glyph = &project.glyphs[*name];
            let extra_codepoints = glyph
                .unicodes
                .iter()
                .map(|unicode| format!(" U+{unicode:04X}"))
                .collect::<String>();
            let is_current = current_glyph.as_deref() == Some(*name);
            let is_selected = selected_glyphs.contains(*name);

            let display_unicode = glyph.unicode.or_else(|| glyph.unicodes.first().copied());
            let metric_badge = if !glyph.left_metrics_key.trim().is_empty()
                || !glyph.right_metrics_key.trim().is_empty()
            {
                "  ↔"
            } else {
                ""
            };
            let text = if let Some(unicode) = display_unicode {
                let ch = char::from_u32(unicode).unwrap_or('\0');
                format!(
                    "{}   {}  U+{:04X}{}{}",
                    ch, name, unicode, extra_codepoints, metric_badge
                )
            } else {
                format!("·   {name}{metric_badge}")
            };

            let mut rt = RichText::new(&text).size(16.0);
            if is_current {
                rt = rt.color(Color32::YELLOW).strong();
            } else if is_selected {
                rt = rt.color(Color32::LIGHT_BLUE);
            }

            let node_count: usize = glyph
                .contours
                .iter()
                .map(|contour| contour.points.len())
                .sum();
            let mut details = format!(
                "幅 {:.0}\n輪郭 {}・ノード {}",
                glyph.width,
                glyph.contours.len(),
                node_count
            );
            if !glyph.unicodes.is_empty() {
                details.push_str(&format!(
                    "\n別名: {}",
                    glyph
                        .unicodes
                        .iter()
                        .map(|unicode| format!("U+{unicode:04X}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !glyph.left_kerning_group.is_empty() || !glyph.right_kerning_group.is_empty() {
                details.push_str(&format!(
                    "\nカーニング: {} / {}",
                    if glyph.left_kerning_group.is_empty() {
                        "—"
                    } else {
                        &glyph.left_kerning_group
                    },
                    if glyph.right_kerning_group.is_empty() {
                        "—"
                    } else {
                        &glyph.right_kerning_group
                    }
                ));
            }
            if !glyph.left_metrics_key.trim().is_empty()
                || !glyph.right_metrics_key.trim().is_empty()
            {
                details.push_str(&format!(
                    "\nメトリクスキー: {} / {}",
                    if glyph.left_metrics_key.trim().is_empty() {
                        "—"
                    } else {
                        &glyph.left_metrics_key
                    },
                    if glyph.right_metrics_key.trim().is_empty() {
                        "—"
                    } else {
                        &glyph.right_metrics_key
                    }
                ));
            }
            let missing_components = glyph
                .components
                .iter()
                .filter(|component| !project.glyphs.contains_key(&component.base))
                .map(|component| component.base.as_str())
                .collect::<Vec<_>>();
            if !missing_components.is_empty() {
                details.push_str(&format!(
                    "\n⚠ 未解決コンポーネント: {}",
                    missing_components.join(", ")
                ));
            }
            let response = ui
                .selectable_label(is_current || is_selected, rt)
                .on_hover_text(details);
            if response.clicked() {
                let modifiers = ui.input(|input| input.modifiers);
                select_glyph(
                    &mut selected,
                    selected_glyphs,
                    current_glyph,
                    &visible_names,
                    name,
                    modifiers.shift,
                    modifiers.command || modifiers.ctrl,
                );
            }
        }
    });
    ui.label(
        RichText::new(format!("表示 {} / {}", visible_count, names.len()))
            .small()
            .color(Color32::GRAY),
    );

    selected
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlyphAction {
    Add(String),
    Duplicate(String, String),
    DuplicateMany(Vec<String>),
    Delete(String),
    DeleteMany(Vec<String>),
    Move(String, isize),
    Rename(String, String),
    MetricsKeysApplied(usize),
}

pub fn show_glyph_actions(
    ui: &mut Ui,
    project: &mut FontProject,
    current_glyph: &Option<String>,
    rename_input: &mut String,
    selected_glyphs: &mut HashSet<String>,
) -> Option<GlyphAction> {
    let mut action = None;
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("+ 新規グリフ").clicked() {
            let mut index = project.glyphs.len();
            while project.glyphs.contains_key(&format!("glyph_{index}")) {
                index += 1;
            }
            let name = format!("glyph_{index}");
            project.add_glyph(name.clone(), None);
            action = Some(GlyphAction::Add(name));
        }
        if ui.button("複製").clicked() {
            if selected_glyphs.len() > 1 {
                let source_names: Vec<String> = project
                    .glyph_names_sorted()
                    .into_iter()
                    .filter(|name| selected_glyphs.iter().any(|selected| selected == *name))
                    .map(str::to_string)
                    .collect();
                let mut duplicated = Vec::new();
                for source_name in source_names {
                    if let Some(name) = project.duplicate_glyph(&source_name) {
                        duplicated.push(name);
                    }
                }
                if !duplicated.is_empty() {
                    action = Some(GlyphAction::DuplicateMany(duplicated));
                }
            } else if let Some(source_name) = current_glyph {
                if let Some(source) = project.glyphs.get(source_name).cloned() {
                    let mut index = project.glyphs.len();
                    let name = loop {
                        let candidate = format!("{}_copy{index}", source_name);
                        if !project.glyphs.contains_key(&candidate) {
                            break candidate;
                        }
                        index += 1;
                    };
                    let mut duplicate = source;
                    duplicate.name = name.clone();
                    duplicate.unicode = None;
                    duplicate.unicodes.clear();
                    project.glyphs.insert(name.clone(), duplicate);
                    project.glyph_order.push(name.clone());
                    action = Some(GlyphAction::Duplicate(source_name.clone(), name));
                }
            }
        }
        if ui.button("🗑 削除").clicked() {
            if selected_glyphs.len() > 1 {
                let mut names: Vec<String> = selected_glyphs.iter().cloned().collect();
                names.sort();
                for name in &names {
                    project.remove_glyph(name);
                }
                action = Some(GlyphAction::DeleteMany(names));
            } else if let Some(name) = current_glyph {
                project.remove_glyph(name);
                action = Some(GlyphAction::Delete(name.clone()));
            }
        }
        if let Some(name) = current_glyph {
            if ui.small_button("↑").clicked() {
                project.move_glyph(name, -1);
                action = Some(GlyphAction::Move(name.clone(), -1));
            }
            if ui.small_button("↓").clicked() {
                project.move_glyph(name, 1);
                action = Some(GlyphAction::Move(name.clone(), 1));
            }
        }
        let metric_targets: Vec<String> = if selected_glyphs.is_empty() {
            current_glyph.iter().cloned().collect()
        } else {
            selected_glyphs.iter().cloned().collect()
        };
        if !metric_targets.is_empty()
            && ui
                .small_button("↔ キー適用")
                .on_hover_text("選択中のグリフへメトリクスキーを全マスター適用")
                .clicked()
        {
            match project.apply_metrics_keys(&metric_targets) {
                Ok(count) => action = Some(GlyphAction::MetricsKeysApplied(count)),
                Err(error) => {
                    ui.colored_label(Color32::from_rgb(230, 130, 100), error);
                }
            }
        }
    });
    if let Some(name) = current_glyph {
        ui.horizontal(|ui| {
            ui.label("名前:");
            if rename_input.is_empty() {
                rename_input.push_str(name);
            }
            ui.text_edit_singleline(rename_input);
            if ui.button("変更").clicked() && project.rename_glyph(name, rename_input.clone()) {
                action = Some(GlyphAction::Rename(name.clone(), rename_input.clone()));
            }
        });
    }
    action
}
