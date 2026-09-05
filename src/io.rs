use crate::font_data::{ColorGradient, FontMaster, FontProject, GlyphLayer};
use kurbo::{Arc, Point, SvgArc, Vec2};
use read_fonts::tables::gpos::{PairPos, PositionSubtables, SinglePos, ValueContext, ValueRecord};
use read_fonts::tables::gsub::{SingleSubst, SubstitutionSubtables};
use read_fonts::types::{GlyphId, NameId};
use read_fonts::{FontRef, Offset, TableProvider};
use std::io::Read;
use std::path::Path;

fn parse_color_gradient(info: &plist::Dictionary) -> Option<ColorGradient> {
    let value = info.get("gradient")?.as_dictionary()?;
    let integer = |key: &str| {
        value
            .get(key)
            .and_then(|item| {
                item.as_signed_integer()
                    .or_else(|| item.as_unsigned_integer().map(|v| v as i64))
            })
            .and_then(|item| u16::try_from(item).ok())
    };
    let real = |key: &str| {
        value.get(key).and_then(|item| {
            item.as_real()
                .or_else(|| item.as_signed_integer().map(|v| v as f64))
        })
    };
    let stops = value
        .get("stops")
        .and_then(plist::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(plist::Value::as_dictionary)
        .filter_map(|stop| {
            Some(crate::font_data::ColorGradientStop {
                offset: real_from(stop, "offset")?,
                palette_index: integer_from(stop, "paletteIndex")?,
                alpha: real_from(stop, "alpha").unwrap_or(1.0),
            })
        })
        .collect();
    Some(ColorGradient {
        start_palette_index: integer("startPaletteIndex")?,
        end_palette_index: integer("endPaletteIndex")?,
        kind: match value.get("kind").and_then(plist::Value::as_string) {
            Some("radial") => crate::font_data::ColorGradientKind::Radial,
            Some("sweep") => crate::font_data::ColorGradientKind::Sweep,
            _ => crate::font_data::ColorGradientKind::Linear,
        },
        extend: match value.get("extend").and_then(plist::Value::as_string) {
            Some("repeat") => crate::font_data::ColorGradientExtend::Repeat,
            Some("reflect") => crate::font_data::ColorGradientExtend::Reflect,
            _ => crate::font_data::ColorGradientExtend::Pad,
        },
        x0: real("x0")?,
        y0: real("y0")?,
        x1: real("x1")?,
        y1: real("y1")?,
        x2: real("x2").unwrap_or_else(|| real("x1").unwrap_or(0.0)),
        y2: real("y2").unwrap_or_else(|| real("y1").unwrap_or(0.0)),
        stops,
        radius0: real("radius0").unwrap_or(0.0),
        radius1: real("radius1").unwrap_or(0.0),
        start_angle: real("startAngle").unwrap_or(0.0),
        end_angle: real("endAngle").unwrap_or(360.0),
    })
}

fn integer_from(info: &plist::Dictionary, key: &str) -> Option<u16> {
    info.get(key)
        .and_then(|item| {
            item.as_signed_integer()
                .or_else(|| item.as_unsigned_integer().map(|v| v as i64))
        })
        .and_then(|item| u16::try_from(item).ok())
}

fn real_from(info: &plist::Dictionary, key: &str) -> Option<f64> {
    info.get(key).and_then(|item| {
        item.as_real()
            .or_else(|| item.as_signed_integer().map(|v| v as f64))
    })
}

struct OutlineCollector {
    contours: Vec<crate::font_data::Contour>,
    current: crate::font_data::Contour,
}

impl OutlineCollector {
    fn new() -> Self {
        Self {
            contours: Vec::new(),
            current: crate::font_data::Contour::new(),
        }
    }
}

impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = crate::font_data::Contour::new();
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x1 as f64, y1 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x1 as f64, y1 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x2 as f64, y2 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn close(&mut self) {
        if !self.current.points.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}

pub fn save_project(project: &FontProject, path: &Path) -> Result<(), String> {
    let mut normalized = project.clone();
    normalized.normalize_glyph_order();
    normalized.normalize_masters();
    let json = serde_json::to_vec_pretty(&normalized)
        .map_err(|e| format!("プロジェクト変換エラー: {e}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "プロジェクト保存先のファイル名が不正です".to_string())?;
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    std::fs::write(&temporary, json).map_err(|e| format!("プロジェクト保存エラー: {e}"))?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("プロジェクト保存エラー: {error}"));
    }
    Ok(())
}

/// Exports the common editable subset as a Glyphs-compatible XML Property List.
pub fn save_glyphs(project: &FontProject, path: &Path) -> Result<(), String> {
    let mut root = plist::Dictionary::new();
    root.insert("formatVersion".into(), plist::Value::Integer(3.into()));
    root.insert(
        "familyName".into(),
        plist::Value::String(project.metadata.family_name.clone()),
    );
    root.insert(
        "styleName".into(),
        plist::Value::String(project.metadata.style_name.clone()),
    );
    root.insert(
        "copyright".into(),
        plist::Value::String(project.metadata.copyright.clone()),
    );
    root.insert(
        "designer".into(),
        plist::Value::String(project.metadata.designer.clone()),
    );
    let version_major = project.metadata.font_revision.floor() as i64;
    let version_minor =
        ((project.metadata.font_revision - version_major as f64) * 100.0).round() as i64;
    root.insert(
        "versionMajor".into(),
        plist::Value::Integer(version_major.into()),
    );
    root.insert(
        "versionMinor".into(),
        plist::Value::Integer(version_minor.into()),
    );
    let mut axis_tags = std::collections::BTreeSet::new();
    for master in &project.masters {
        axis_tags.extend(master.axes.keys().cloned());
    }
    let axis_tags = axis_tags.into_iter().collect::<Vec<_>>();
    let axes = axis_tags
        .iter()
        .map(|tag| {
            let mut dict = plist::Dictionary::new();
            dict.insert("tag".into(), plist::Value::String(tag.clone()));
            dict.insert(
                "name".into(),
                plist::Value::String(
                    project
                        .axis_names
                        .get(tag)
                        .cloned()
                        .unwrap_or_else(|| tag.clone()),
                ),
            );
            plist::Value::Dictionary(dict)
        })
        .collect();
    root.insert("axes".into(), plist::Value::Array(axes));
    root.insert(
        "unitsPerEm".into(),
        plist::Value::Real(project.metadata.units_per_em),
    );
    let masters = project
        .masters
        .iter()
        .map(|master| {
            let mut dict = plist::Dictionary::new();
            dict.insert("id".into(), plist::Value::String(master.id.clone()));
            dict.insert("name".into(), plist::Value::String(master.name.clone()));
            dict.insert("weightValue".into(), plist::Value::Real(master.weight));
            dict.insert("widthValue".into(), plist::Value::Real(master.width));
            dict.insert("isBracket".into(), plist::Value::Boolean(master.is_bracket));
            let metrics = project.master_metrics_for(&master.id);
            dict.insert("ascender".into(), plist::Value::Real(metrics.ascender));
            dict.insert("descender".into(), plist::Value::Real(metrics.descender));
            dict.insert("lineGap".into(), plist::Value::Real(metrics.line_gap));
            dict.insert(
                "axes".into(),
                plist::Value::Array(
                    axis_tags
                        .iter()
                        .map(|tag| plist::Value::Real(*master.axes.get(tag).unwrap_or(&0.0)))
                        .collect(),
                ),
            );
            let guides = project
                .guidelines_by_master
                .get(&master.id)
                .cloned()
                .or_else(|| {
                    (master.id == project.default_master_id).then(|| project.guidelines.clone())
                })
                .unwrap_or_default()
                .into_iter()
                .map(|guide| {
                    let mut value = plist::Dictionary::new();
                    value.insert("x".into(), plist::Value::Real(guide.x));
                    value.insert("y".into(), plist::Value::Real(guide.y));
                    value.insert("angle".into(), plist::Value::Real(guide.angle));
                    if !guide.name.is_empty() {
                        value.insert("name".into(), plist::Value::String(guide.name));
                    }
                    plist::Value::Dictionary(value)
                })
                .collect::<Vec<_>>();
            if !guides.is_empty() {
                dict.insert("guides".into(), plist::Value::Array(guides));
            }
            plist::Value::Dictionary(dict)
        })
        .collect();
    root.insert("fontMaster".into(), plist::Value::Array(masters));
    let instances = project
        .instances
        .iter()
        .map(|instance| {
            let mut dict = plist::Dictionary::new();
            dict.insert("name".into(), plist::Value::String(instance.name.clone()));
            dict.insert(
                "familyName".into(),
                plist::Value::String(project.metadata.family_name.clone()),
            );
            dict.insert("weightValue".into(), plist::Value::Real(instance.weight));
            dict.insert("widthValue".into(), plist::Value::Real(instance.width));
            dict.insert(
                "axes".into(),
                plist::Value::Array(
                    axis_tags
                        .iter()
                        .map(|tag| {
                            let value = instance.axes.get(tag).copied().unwrap_or_else(|| {
                                if tag.eq_ignore_ascii_case("wght") {
                                    instance.weight
                                } else if tag.eq_ignore_ascii_case("wdth") {
                                    instance.width
                                } else {
                                    0.0
                                }
                            });
                            plist::Value::Real(value)
                        })
                        .collect(),
                ),
            );
            plist::Value::Dictionary(dict)
        })
        .collect();
    root.insert("instances".into(), plist::Value::Array(instances));
    let classes = project
        .opentype_classes
        .lines()
        .filter_map(parse_glyphs_class_line)
        .map(|(name, code)| {
            let mut dict = plist::Dictionary::new();
            dict.insert("name".into(), plist::Value::String(name));
            dict.insert("code".into(), plist::Value::String(code));
            plist::Value::Dictionary(dict)
        })
        .collect::<Vec<_>>();
    root.insert("classes".into(), plist::Value::Array(classes));
    let features = parse_glyphs_feature_source(&project.opentype_features)
        .into_iter()
        .map(|(tag, code)| {
            let mut dict = plist::Dictionary::new();
            dict.insert("tag".into(), plist::Value::String(tag));
            dict.insert("code".into(), plist::Value::String(code));
            plist::Value::Dictionary(dict)
        })
        .collect::<Vec<_>>();
    root.insert("features".into(), plist::Value::Array(features));
    // Glyphs' public `features` array only models feature blocks and drops
    // source-level declarations such as `languagesystem` and named lookups.
    // Keep the complete source in a namespaced key so Glyph Studio round-trips
    // advanced OpenType files without losing data, while other Glyphs tools
    // can still consume the standard array above.
    root.insert(
        "com.glyph-studio.opentypeFeatures".into(),
        plist::Value::String(project.opentype_features.clone()),
    );
    let unicode_variation_sequences = serde_json::to_string(&project.unicode_variation_sequences)
        .map_err(|error| format!("IVS保存エラー: {error}"))?;
    root.insert(
        "com.glyph-studio.unicodeVariationSequences".into(),
        plist::Value::String(unicode_variation_sequences),
    );
    let axis_mappings = serde_json::to_string(&project.axis_mappings)
        .map_err(|error| format!("avar軸マッピング保存エラー: {error}"))?;
    root.insert(
        "com.glyph-studio.axisMappings".into(),
        plist::Value::String(axis_mappings),
    );
    let preserved_tables = serde_json::to_string(&project.preserved_tables)
        .map_err(|error| format!("OpenTypeテーブル保存エラー: {error}"))?;
    root.insert(
        "com.glyph-studio.preservedTables".into(),
        plist::Value::String(preserved_tables),
    );
    if let Some(source) = &project.preserved_layout_source {
        root.insert(
            "com.glyph-studio.preservedLayoutSource".into(),
            plist::Value::String(source.clone()),
        );
    }
    if let Some(fingerprint) = project.preserved_layout_fingerprint {
        root.insert(
            "com.glyph-studio.preservedLayoutFingerprint".into(),
            plist::Value::String(fingerprint.to_string()),
        );
    }
    let glyphs = project
        .glyph_names_sorted()
        .into_iter()
        .filter_map(|name| project.glyphs.get(name))
        .map(|glyph| {
            let mut dict = plist::Dictionary::new();
            dict.insert("glyphname".into(), plist::Value::String(glyph.name.clone()));
            if let Some(unicode) = glyph.unicode {
                dict.insert(
                    "unicode".into(),
                    plist::Value::String(format!("{unicode:04X}")),
                );
            }
            if !glyph.left_kerning_group.is_empty() {
                dict.insert(
                    "leftKerningGroup".into(),
                    plist::Value::String(glyph.left_kerning_group.clone()),
                );
            }
            if !glyph.right_kerning_group.is_empty() {
                dict.insert(
                    "rightKerningGroup".into(),
                    plist::Value::String(glyph.right_kerning_group.clone()),
                );
            }
            if !glyph.left_metrics_key.is_empty() {
                dict.insert(
                    "leftMetricsKey".into(),
                    plist::Value::String(glyph.left_metrics_key.clone()),
                );
            }
            if !glyph.right_metrics_key.is_empty() {
                dict.insert(
                    "rightMetricsKey".into(),
                    plist::Value::String(glyph.right_metrics_key.clone()),
                );
            }
            let layers = glyph
                .layers
                .iter()
                .map(|(id, layer)| {
                    let mut layer_dict = plist::Dictionary::new();
                    layer_dict.insert("layerId".into(), plist::Value::String(id.clone()));
                    layer_dict.insert("width".into(), plist::Value::Real(layer.width));
                    let shapes = layer
                        .contours
                        .iter()
                        .map(|contour| {
                            let mut shape = plist::Dictionary::new();
                            let nodes = contour
                                .points
                                .iter()
                                .map(|point| {
                                    let kind = if point.is_on_curve() {
                                        "LINE"
                                    } else {
                                        "OFFCURVE"
                                    };
                                    let smooth = if point.smooth { " SMOOTH" } else { "" };
                                    plist::Value::String(format!(
                                        "{} {} {kind}{smooth}",
                                        point.x, point.y
                                    ))
                                })
                                .collect();
                            shape.insert("nodes".into(), plist::Value::Array(nodes));
                            plist::Value::Dictionary(shape)
                        })
                        .chain(layer.components.iter().map(|component| {
                            let mut shape = plist::Dictionary::new();
                            shape
                                .insert("ref".into(), plist::Value::String(component.base.clone()));
                            shape.insert(
                                "transform".into(),
                                plist::Value::Array(vec![
                                    plist::Value::Real(component.x_scale),
                                    plist::Value::Real(component.xy_scale),
                                    plist::Value::Real(component.yx_scale),
                                    plist::Value::Real(component.y_scale),
                                    plist::Value::Real(component.x_offset),
                                    plist::Value::Real(component.y_offset),
                                ]),
                            );
                            plist::Value::Dictionary(shape)
                        }))
                        .collect();
                    layer_dict.insert("shapes".into(), plist::Value::Array(shapes));
                    let anchors = layer
                        .anchors
                        .iter()
                        .map(|anchor| {
                            let mut dict = plist::Dictionary::new();
                            dict.insert("name".into(), plist::Value::String(anchor.name.clone()));
                            dict.insert(
                                "pos".into(),
                                plist::Value::String(format!("{{{}, {}}}", anchor.x, anchor.y)),
                            );
                            plist::Value::Dictionary(dict)
                        })
                        .collect();
                    if !layer.anchors.is_empty() {
                        layer_dict.insert("anchors".into(), plist::Value::Array(anchors));
                    }
                    let guides = glyph
                        .master_guidelines
                        .get(id)
                        .cloned()
                        .or_else(|| {
                            (id == &project.default_master_id).then(|| glyph.guidelines.clone())
                        })
                        .unwrap_or_default()
                        .into_iter()
                        .map(|guide| {
                            let mut dict = plist::Dictionary::new();
                            dict.insert(
                                "pos".into(),
                                plist::Value::String(format!("{{{}, {}}}", guide.x, guide.y)),
                            );
                            dict.insert("angle".into(), plist::Value::Real(guide.angle));
                            if !guide.name.is_empty() {
                                dict.insert("name".into(), plist::Value::String(guide.name));
                            }
                            plist::Value::Dictionary(dict)
                        })
                        .collect::<Vec<_>>();
                    if !guides.is_empty() {
                        layer_dict.insert("guides".into(), plist::Value::Array(guides));
                    }
                    plist::Value::Dictionary(layer_dict)
                })
                .collect();
            dict.insert("layers".into(), plist::Value::Array(layers));
            plist::Value::Dictionary(dict)
        })
        .collect();
    root.insert("glyphs".into(), plist::Value::Array(glyphs));
    let mut kerning = plist::Dictionary::new();
    let mut kerning_maps = project.kerning_by_master.clone();
    kerning_maps
        .entry(project.default_master_id.clone())
        .or_insert_with(|| project.kerning.clone());
    for (master_id, pairs) in kerning_maps {
        let mut master_pairs = plist::Dictionary::new();
        for ((left, right), value) in &pairs {
            let glyphs_left_key = glyphs_kerning_key(project, left, true);
            let glyphs_right_key = glyphs_kerning_key(project, right, false);
            if !master_pairs.contains_key(&glyphs_left_key) {
                master_pairs.insert(
                    glyphs_left_key.clone(),
                    plist::Value::Dictionary(plist::Dictionary::new()),
                );
            }
            if let Some(plist::Value::Dictionary(right_pairs)) =
                master_pairs.get_mut(&glyphs_left_key)
            {
                right_pairs.insert(glyphs_right_key, plist::Value::Real(*value));
            }
        }
        kerning.insert(master_id, plist::Value::Dictionary(master_pairs));
    }
    root.insert("kerning".into(), plist::Value::Dictionary(kerning));
    let mut user_data = plist::Dictionary::new();
    if !project.color_palettes.is_empty() {
        user_data.insert(
            "com.glyph-studio.colorPalettes".into(),
            plist::Value::String(
                serde_json::to_string(&project.color_palettes)
                    .map_err(|error| format!("カラー情報のGlyphs保存に失敗しました: {error}"))?,
            ),
        );
    }
    if project
        .color_palette_names
        .iter()
        .any(|name| !name.trim().is_empty())
    {
        user_data.insert(
            "com.glyph-studio.colorPaletteNames".into(),
            plist::Value::String(
                serde_json::to_string(&project.color_palette_names).map_err(|error| {
                    format!("カラーパレット名のGlyphs保存に失敗しました: {error}")
                })?,
            ),
        );
    }
    if project
        .color_palette_types
        .iter()
        .any(|palette_type| *palette_type != 0)
    {
        user_data.insert(
            "com.glyph-studio.colorPaletteTypes".into(),
            plist::Value::String(serde_json::to_string(&project.color_palette_types).map_err(
                |error| format!("カラーパレット用途のGlyphs保存に失敗しました: {error}"),
            )?),
        );
    }
    if project
        .color_palette_entry_names
        .iter()
        .any(|name| !name.trim().is_empty())
    {
        user_data.insert(
            "com.glyph-studio.colorPaletteEntryNames".into(),
            plist::Value::String(
                serde_json::to_string(&project.color_palette_entry_names).map_err(|error| {
                    format!("カラーパレット項目名のGlyphs保存に失敗しました: {error}")
                })?,
            ),
        );
    }
    if !project.color_layers.is_empty() {
        user_data.insert(
            "com.glyph-studio.colorLayers".into(),
            plist::Value::String(
                serde_json::to_string(&project.color_layers)
                    .map_err(|error| format!("カラー層のGlyphs保存に失敗しました: {error}"))?,
            ),
        );
    }
    if !project.color_layer_transforms.is_empty() {
        user_data.insert(
            "com.glyph-studio.colorLayerTransforms".into(),
            plist::Value::String(
                serde_json::to_string(&project.color_layer_transforms)
                    .map_err(|error| format!("カラー層変形のGlyphs保存に失敗しました: {error}"))?,
            ),
        );
    }
    if !user_data.is_empty() {
        root.insert("userData".into(), plist::Value::Dictionary(user_data));
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Glyphs保存先のファイル名が不正です".to_string())?;
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    let file =
        std::fs::File::create(&temporary).map_err(|e| format!("Glyphs書き出しエラー: {e}"))?;
    if let Err(error) = plist::to_writer_xml(file, &plist::Value::Dictionary(root)) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("Glyphs書き出しエラー: {error}"));
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("Glyphs書き出しエラー: {error}"));
    }
    Ok(())
}

pub fn load_project(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("プロジェクト読み込みエラー: {e}"))?;
    let mut project: FontProject =
        serde_json::from_slice(&bytes).map_err(|e| format!("プロジェクト形式エラー: {e}"))?;
    project.normalize_glyph_order();
    project.normalize_masters();
    Ok(project)
}

/// Imports the portable, ASCII Property List form written by Glyphs.
/// The importer intentionally keeps unknown keys intact at the file boundary and
/// converts the common outline/layer data into the native project model.
pub fn load_glyphs(path: &Path) -> Result<FontProject, String> {
    let mut bytes = std::fs::read(path).map_err(|e| format!("Glyphs読み込みエラー: {e}"))?;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
    let trimmed = bytes
        .iter()
        .copied()
        .skip_while(u8::is_ascii_whitespace)
        .collect::<Vec<_>>();
    let value = if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<plist") {
        plist::Value::from_reader_xml(std::io::Cursor::new(bytes))
    } else {
        plist::Value::from_reader_ascii(std::io::Cursor::new(bytes))
    }
    .map_err(|e| format!("Glyphs形式エラー: {e}"))?;
    let root = value
        .as_dictionary()
        .ok_or_else(|| "Glyphsファイルのルートが辞書ではありません".to_string())?;
    if !root.contains_key("glyphs") {
        return Err("Glyphsファイルにglyphs配列がありません".to_string());
    }
    let mut project = FontProject::new();
    if let Some(value) = root.get("familyName").and_then(plist_string) {
        project.metadata.family_name = value;
    }
    if let Some(value) = root.get("styleName").and_then(plist_string) {
        project.metadata.style_name = value;
    }
    if let Some(value) = root.get("copyright").and_then(plist_string) {
        project.metadata.copyright = value;
    }
    if let Some(value) = root.get("designer").and_then(plist_string) {
        project.metadata.designer = value;
    }
    if let Some(major) = root.get("versionMajor").and_then(plist_number) {
        let minor = root
            .get("versionMinor")
            .and_then(plist_number)
            .unwrap_or(0.0);
        project.metadata.font_revision = major + minor / 100.0;
    }
    let glyphs_axis_tags = root
        .get("axes")
        .and_then(plist_array)
        .map(|axes| {
            axes.iter()
                .filter_map(|value| value.as_dictionary())
                .filter_map(|axis| axis.get("tag").and_then(plist_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(axes) = root.get("axes").and_then(plist_array) {
        for axis in axes.iter().filter_map(|value| value.as_dictionary()) {
            if let (Some(tag), Some(name)) = (
                axis.get("tag").and_then(plist_string),
                axis.get("name").and_then(plist_string),
            ) {
                project.axis_names.insert(tag, name);
            }
        }
    }
    if let Some(value) = root.get("unitsPerEm").and_then(plist_number) {
        project.metadata.units_per_em = value;
    }
    if let Some(masters) = root.get("fontMaster").and_then(plist_array) {
        project.masters.clear();
        for (index, master) in masters.iter().filter_map(|v| v.as_dictionary()).enumerate() {
            let id = master
                .get("id")
                .and_then(plist_string)
                .unwrap_or_else(|| format!("master{}", index + 1));
            let axes = master
                .get("axes")
                .and_then(plist_array)
                .map(|values| {
                    glyphs_axis_tags
                        .iter()
                        .zip(values.iter().filter_map(plist_number))
                        .map(|(tag, value)| (tag.clone(), value))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(guides) = master.get("guides").and_then(plist_array) {
                let parsed = guides
                    .iter()
                    .filter_map(|value| value.as_dictionary())
                    .filter_map(|guide| {
                        Some(crate::font_data::Guideline {
                            x: guide.get("x").and_then(plist_number)?,
                            y: guide.get("y").and_then(plist_number)?,
                            angle: guide.get("angle").and_then(plist_number).unwrap_or(0.0),
                            name: guide.get("name").and_then(plist_string).unwrap_or_default(),
                        })
                    })
                    .collect();
                project.guidelines_by_master.insert(id.clone(), parsed);
            }
            let master_id = id.clone();
            let master_metrics = (
                master.get("ascender").and_then(plist_number),
                master.get("descender").and_then(plist_number),
            );
            project.masters.push(crate::font_data::FontMaster {
                id,
                name: master
                    .get("name")
                    .and_then(plist_string)
                    .unwrap_or_else(|| format!("Master {}", index + 1)),
                weight: master
                    .get("weightValue")
                    .and_then(plist_number)
                    .unwrap_or(400.0),
                width: master
                    .get("widthValue")
                    .and_then(plist_number)
                    .unwrap_or(100.0),
                is_bracket: master
                    .get("isBracket")
                    .and_then(plist_bool)
                    .unwrap_or(false),
                axes,
            });
            if let (Some(ascender), Some(descender)) = master_metrics {
                project.metrics_by_master.insert(
                    master_id,
                    crate::font_data::MasterMetrics {
                        ascender,
                        descender,
                        line_gap: master.get("lineGap").and_then(plist_number).unwrap_or(0.0),
                    },
                );
            }
        }
        if project.masters.is_empty() {
            project.masters = FontProject::new().masters;
        }
        project.default_master_id = project.masters[0].id.clone();
    }
    if let Some(instances) = root.get("instances").and_then(plist_array) {
        project.instances = instances
            .iter()
            .filter_map(|value| value.as_dictionary())
            .filter_map(|instance| {
                let name = instance
                    .get("name")
                    .and_then(plist_string)
                    .or_else(|| instance.get("familyName").and_then(plist_string))?;
                let values = instance.get("axes").and_then(plist_array);
                let axes = values
                    .map(|values| {
                        glyphs_axis_tags
                            .iter()
                            .zip(values.iter().filter_map(plist_number))
                            .map(|(tag, value)| (tag.clone(), value))
                            .collect::<std::collections::HashMap<_, _>>()
                    })
                    .unwrap_or_default();
                let weight = axes
                    .iter()
                    .find(|(tag, _)| tag.eq_ignore_ascii_case("wght"))
                    .map(|(_, value)| *value)
                    .or_else(|| instance.get("weightValue").and_then(plist_number))
                    .unwrap_or(400.0);
                let width = axes
                    .iter()
                    .find(|(tag, _)| tag.eq_ignore_ascii_case("wdth"))
                    .map(|(_, value)| *value)
                    .or_else(|| instance.get("widthValue").and_then(plist_number))
                    .unwrap_or(100.0);
                Some(crate::font_data::FontInstance {
                    name,
                    axes,
                    weight,
                    width,
                })
            })
            .collect();
    }
    if let Some(classes) = root.get("classes").and_then(plist_array) {
        project.opentype_classes = classes
            .iter()
            .filter_map(|value| value.as_dictionary())
            .filter_map(|class| {
                let name = class.get("name").and_then(plist_string)?;
                let code = class.get("code").and_then(plist_string)?;
                Some(format!("@{name} = [{code}];"))
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    if let Some(features) = root.get("features").and_then(plist_array) {
        project.opentype_features = features
            .iter()
            .filter_map(|value| value.as_dictionary())
            .filter_map(|feature| {
                let tag = feature.get("tag").and_then(plist_string)?;
                let code = feature.get("code").and_then(plist_string)?;
                Some(format!("feature {tag} {{\n{code}\n}} {tag};"))
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Some(features) = root
        .get("com.glyph-studio.opentypeFeatures")
        .and_then(plist_string)
    {
        project.opentype_features = features;
    }
    if let Some(serialized) = root
        .get("com.glyph-studio.unicodeVariationSequences")
        .and_then(plist_string)
    {
        if let Ok(sequences) = serde_json::from_str(&serialized) {
            project.unicode_variation_sequences = sequences;
        }
    }
    if let Some(serialized) = root
        .get("com.glyph-studio.axisMappings")
        .and_then(plist_string)
    {
        if let Ok(mappings) = serde_json::from_str(&serialized) {
            project.axis_mappings = mappings;
        }
    }
    if let Some(serialized) = root
        .get("com.glyph-studio.preservedTables")
        .and_then(plist_string)
    {
        if let Ok(tables) = serde_json::from_str(&serialized) {
            project.preserved_tables = tables;
        }
    }
    project.preserved_layout_source = root
        .get("com.glyph-studio.preservedLayoutSource")
        .and_then(plist_string);
    project.preserved_layout_fingerprint = root
        .get("com.glyph-studio.preservedLayoutFingerprint")
        .and_then(plist_string)
        .and_then(|value| value.parse().ok());
    let master_id = project.default_master_id.clone();
    if let Some(glyphs) = root.get("glyphs").and_then(plist_array) {
        for glyph_dict in glyphs.iter().filter_map(|v| v.as_dictionary()) {
            let Some(name) = glyph_dict.get("glyphname").and_then(plist_string) else {
                continue;
            };
            let unicode = glyph_dict.get("unicode").and_then(plist_u32);
            let mut glyph = crate::font_data::GlyphData::new(name.clone(), unicode);
            glyph.width = glyph_dict
                .get("width")
                .and_then(plist_number)
                .unwrap_or(glyph.width);
            glyph.left_kerning_group = glyph_dict
                .get("leftKerningGroup")
                .and_then(plist_string)
                .unwrap_or_default();
            glyph.right_kerning_group = glyph_dict
                .get("rightKerningGroup")
                .and_then(plist_string)
                .unwrap_or_default();
            glyph.left_metrics_key = glyph_dict
                .get("leftMetricsKey")
                .and_then(plist_string)
                .unwrap_or_default();
            glyph.right_metrics_key = glyph_dict
                .get("rightMetricsKey")
                .and_then(plist_string)
                .unwrap_or_default();
            if let Some(layers) = glyph_dict.get("layers").and_then(plist_array) {
                for layer_dict in layers.iter().filter_map(|v| v.as_dictionary()) {
                    let id = layer_dict
                        .get("layerId")
                        .and_then(plist_string)
                        .unwrap_or_else(|| master_id.clone());
                    let mut layer = crate::font_data::GlyphLayer {
                        width: layer_dict
                            .get("width")
                            .and_then(plist_number)
                            .unwrap_or(glyph.width),
                        contours: Vec::new(),
                        components: Vec::new(),
                        anchors: Vec::new(),
                    };
                    if let Some(anchors) = layer_dict.get("anchors").and_then(plist_array) {
                        for anchor in anchors.iter().filter_map(|value| value.as_dictionary()) {
                            let Some(anchor_name) = anchor.get("name").and_then(plist_string)
                            else {
                                continue;
                            };
                            let Some((x, y)) = anchor
                                .get("pos")
                                .and_then(plist_string)
                                .and_then(|position| parse_glyphs_position(&position))
                            else {
                                continue;
                            };
                            layer.anchors.push(crate::font_data::GlyphAnchor {
                                name: anchor_name,
                                x,
                                y,
                            });
                        }
                    }
                    if let Some(guides) = layer_dict.get("guides").and_then(plist_array) {
                        let parsed = guides
                            .iter()
                            .filter_map(|value| value.as_dictionary())
                            .filter_map(|guide| {
                                let (x, y) = guide
                                    .get("pos")
                                    .and_then(plist_string)
                                    .and_then(|position| parse_glyphs_position(&position))?;
                                Some(crate::font_data::Guideline {
                                    x,
                                    y,
                                    angle: guide.get("angle").and_then(plist_number).unwrap_or(0.0),
                                    name: guide
                                        .get("name")
                                        .and_then(plist_string)
                                        .unwrap_or_default(),
                                })
                            })
                            .collect::<Vec<_>>();
                        glyph.master_guidelines.insert(id.clone(), parsed.clone());
                        if id == master_id {
                            glyph.guidelines = parsed;
                        }
                    }
                    if let Some(shapes) = layer_dict.get("shapes").and_then(plist_array) {
                        for shape in shapes.iter().filter_map(|v| v.as_dictionary()) {
                            if let Some(nodes) = shape.get("nodes").and_then(plist_array) {
                                let mut contour = crate::font_data::Contour::new();
                                for node in nodes
                                    .iter()
                                    .filter_map(plist_string)
                                    .filter_map(|value| parse_glyphs_node(&value))
                                {
                                    contour.points.push(node);
                                }
                                if !contour.points.is_empty() {
                                    layer.contours.push(contour);
                                }
                            } else if let Some(base) = shape.get("ref").and_then(plist_string) {
                                let transform = shape
                                    .get("transform")
                                    .and_then(plist_array)
                                    .map(|values| {
                                        values.iter().filter_map(plist_number).collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                layer.components.push(crate::font_data::GlyphComponent {
                                    base,
                                    x_scale: transform.first().copied().unwrap_or(1.0),
                                    xy_scale: transform.get(1).copied().unwrap_or(0.0),
                                    yx_scale: transform.get(2).copied().unwrap_or(0.0),
                                    y_scale: transform.get(3).copied().unwrap_or(1.0),
                                    x_offset: transform.get(4).copied().unwrap_or(0.0),
                                    y_offset: transform.get(5).copied().unwrap_or(0.0),
                                });
                            }
                        }
                    }
                    glyph.layers.insert(id, layer);
                }
            }
            if glyph.layers.is_empty() {
                glyph.layers.insert(
                    master_id.clone(),
                    crate::font_data::GlyphLayer {
                        width: glyph.width,
                        contours: glyph.contours.clone(),
                        components: glyph.components.clone(),
                        anchors: glyph.anchors.clone(),
                    },
                );
            }
            if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                glyph.width = layer.width;
                glyph.contours = layer.contours;
                glyph.components = layer.components;
                glyph.anchors = layer.anchors;
            }
            project.glyph_order.push(name.clone());
            project.glyphs.insert(name, glyph);
        }
    }
    if let Some(kerning) = root.get("kerning").and_then(|value| value.as_dictionary()) {
        for (master_id, master_value) in kerning {
            let Some(master_pairs) = master_value.as_dictionary() else {
                continue;
            };
            let mut master_kerning = std::collections::HashMap::new();
            for (left, right_values) in master_pairs {
                let Some(right_values) = right_values.as_dictionary() else {
                    continue;
                };
                let left = normalize_glyphs_kerning_key(left, true);
                for (right, value) in right_values {
                    if let Some(value) = plist_number(value) {
                        master_kerning.insert(
                            (left.clone(), normalize_glyphs_kerning_key(right, false)),
                            value,
                        );
                    }
                }
            }
            project
                .kerning_by_master
                .insert(master_id.clone(), master_kerning);
        }
        project.kerning = project
            .kerning_by_master
            .get(&project.default_master_id)
            .cloned()
            .or_else(|| project.kerning_by_master.values().next().cloned())
            .unwrap_or_default();
    }
    if let Some(user_data) = root.get("userData").and_then(|value| value.as_dictionary()) {
        if let Some(value) = user_data
            .get("com.glyph-studio.colorPalettes")
            .and_then(plist_string)
        {
            if let Ok(palettes) = serde_json::from_str(&value) {
                project.color_palettes = palettes;
            }
        }
        if let Some(value) = user_data
            .get("com.glyph-studio.colorPaletteNames")
            .and_then(plist_string)
        {
            if let Ok(names) = serde_json::from_str(&value) {
                project.color_palette_names = names;
            }
        }
        if let Some(value) = user_data
            .get("com.glyph-studio.colorPaletteTypes")
            .and_then(plist_string)
        {
            if let Ok(types) = serde_json::from_str(&value) {
                project.color_palette_types = types;
            }
        }
        if let Some(value) = user_data
            .get("com.glyph-studio.colorPaletteEntryNames")
            .and_then(plist_string)
        {
            if let Ok(names) = serde_json::from_str(&value) {
                project.color_palette_entry_names = names;
            }
        }
        if let Some(value) = user_data
            .get("com.glyph-studio.colorLayers")
            .and_then(plist_string)
        {
            if let Ok(layers) = serde_json::from_str(&value) {
                project.color_layers = layers;
            }
        }
        if let Some(value) = user_data
            .get("com.glyph-studio.colorLayerTransforms")
            .and_then(plist_string)
        {
            if let Ok(transforms) = serde_json::from_str(&value) {
                project.color_layer_transforms = transforms;
            }
        }
    }
    project.normalize_glyph_order();
    project.normalize_masters();
    Ok(project)
}

fn plist_array(value: &plist::Value) -> Option<&Vec<plist::Value>> {
    value.as_array()
}
fn plist_string(value: &plist::Value) -> Option<String> {
    value.as_string().map(str::to_string)
}
fn plist_number(value: &plist::Value) -> Option<f64> {
    value
        .as_real()
        .or_else(|| value.as_signed_integer().map(|n| n as f64))
}

fn plist_bool(value: &plist::Value) -> Option<bool> {
    value
        .as_boolean()
        .or_else(|| value.as_signed_integer().map(|number| number != 0))
        .or_else(|| {
            value.as_string().and_then(|text| match text {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
        })
}
fn plist_u32(value: &plist::Value) -> Option<u32> {
    if let Some(number) = value.as_signed_integer() {
        return u32::try_from(number).ok();
    }
    value
        .as_string()
        .and_then(|text| u32::from_str_radix(text.trim().trim_start_matches("0x"), 16).ok())
}
fn normalize_glyphs_kerning_key(value: &str, left: bool) -> String {
    let prefix = if left { "@MMK_L_" } else { "@MMK_R_" };
    value.strip_prefix(prefix).unwrap_or(value).to_string()
}
fn glyphs_kerning_key(project: &FontProject, value: &str, left: bool) -> String {
    let is_group = project.glyphs.values().any(|glyph| {
        if left {
            glyph.left_kerning_group == value
        } else {
            glyph.right_kerning_group == value
        }
    });
    if is_group {
        format!("@MMK_{}_{}", if left { "L" } else { "R" }, value)
    } else {
        value.to_string()
    }
}
fn parse_glyphs_class_line(line: &str) -> Option<(String, String)> {
    let (name, code) = line.trim().strip_prefix('@')?.split_once('=')?;
    Some((
        name.trim().to_string(),
        code.trim()
            .trim_matches(';')
            .trim()
            .trim_matches(['[', ']'])
            .trim()
            .to_string(),
    ))
}
fn parse_glyphs_feature_source(source: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("feature ") {
        let candidate = &rest[start + "feature ".len()..];
        let Some(open) = candidate.find('{') else {
            break;
        };
        let tag = candidate[..open].trim();
        let Some(close) = candidate[open + 1..].find('}') else {
            break;
        };
        result.push((
            tag.to_string(),
            candidate[open + 1..open + 1 + close].trim().to_string(),
        ));
        rest = &candidate[open + 1 + close + 1..];
    }
    result
}
fn parse_glyphs_node(value: &str) -> Option<crate::font_data::ContourPoint> {
    let mut parts = value.split_whitespace();
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let kind = parts.next().unwrap_or("LINE");
    let smooth = parts.any(|part| part == "SMOOTH");
    let mut point = if kind == "OFFCURVE" {
        crate::font_data::ContourPoint::off_curve(x, y)
    } else {
        crate::font_data::ContourPoint::on_curve(x, y)
    };
    point.smooth = smooth;
    Some(point)
}

fn parse_glyphs_position(value: &str) -> Option<(f64, f64)> {
    let cleaned = value
        .trim()
        .trim_start_matches(['{', '('])
        .trim_end_matches(['}', ')']);
    let mut parts = cleaned.split(',').map(str::trim);
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// Captures the raw payload of SFNT tables that the project model does not
/// regenerate. This lets newer or specialised OpenType/AAT tables survive an
/// import/export cycle without duplicating core outline/layout tables.
fn preserved_sfnt_tables(bytes: &[u8]) -> std::collections::HashMap<String, Vec<u8>> {
    let Ok(font) = FontRef::new(bytes) else {
        return std::collections::HashMap::new();
    };
    font.table_directory
        .table_records()
        .iter()
        .filter_map(|record| {
            let tag = record.tag().into_bytes();
            let tag = String::from_utf8(tag.to_vec()).ok()?;
            // These are regenerated from the project model during export.
            // In particular, retaining CFF alongside generated glyf/loca or
            // retaining stale outline/variation data would create an invalid
            // or misleading output font. Layout tables are retained as a
            // fallback when the source cannot be reconstructed; generated
            // GSUB/GPOS/GDEF always replace them when available.
            if matches!(
                tag.as_str(),
                "CFF "
                    | "CFF2"
                    | "glyf"
                    | "loca"
                    | "fvar"
                    | "gvar"
                    | "avar"
                    | "HVAR"
                    | "VVAR"
                    | "MVAR"
                    | "cmap"
                    | "head"
                    | "hhea"
                    | "hmtx"
                    | "maxp"
                    | "name"
                    | "OS/2"
                    | "post"
                    | "vhea"
                    | "vmtx"
                    | "gasp"
                    | "kern"
            ) {
                return None;
            }
            let data = font.table_data(record.tag())?.as_bytes().to_vec();
            Some((tag, data))
        })
        .collect()
}

fn decode_name_string(platform: u16, bytes: &[u8]) -> Option<String> {
    if platform == 0 || platform == 3 {
        let (chunks, remainder) = bytes.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        let units = chunks
            .iter()
            .map(|chunk| u16::from_be_bytes(*chunk))
            .collect::<Vec<_>>();
        String::from_utf16(&units).ok()
    } else {
        None
    }
}

fn escape_feature_name(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}

fn feature_glyph_name(names: &[String], glyph_id: ttf_parser::GlyphId) -> Option<&str> {
    let name = names.get(glyph_id.0 as usize)?.as_str();
    (!name.is_empty()
        && !name
            .chars()
            .any(|character| character.is_whitespace() || "[]{}();,'\"".contains(character)))
    .then_some(name)
}

macro_rules! imported_lookup_flag_source {
    ($lookup:expr) => {{
        let lookup = $lookup;
        let flags = lookup.flags;
        let mut values = Vec::new();
        if flags.right_to_left() {
            values.push("RightToLeft".to_string());
        }
        if flags.ignore_base_glyphs() {
            values.push("IgnoreBaseGlyphs".to_string());
        }
        if flags.ignore_ligatures() {
            values.push("IgnoreLigatures".to_string());
        }
        if flags.ignore_marks() {
            values.push("IgnoreMarks".to_string());
        }
        let mark_attachment_type = flags.mark_attachment_type();
        if mark_attachment_type != 0 {
            values.push(format!("MarkAttachmentType {mark_attachment_type}"));
        }
        let mark_filtering_set = lookup
            .mark_filtering_set
            .map(|index| format!(" UseMarkFilteringSet @GSMarkSet{index}"))
            .unwrap_or_default();
        (!values.is_empty() || !mark_filtering_set.is_empty())
            .then(|| format!("lookupflag {}{};", values.join(" "), mark_filtering_set))
    }};
}

macro_rules! collect_imported_languagesystems {
    ($table:expr, $systems:expr) => {
        for script in $table.scripts {
            if let Some(language) = script.default_language {
                $systems.insert(format!("languagesystem {} {};", script.tag, language.tag));
            }
            for language in script.languages {
                $systems.insert(format!("languagesystem {} {};", script.tag, language.tag));
            }
        }
    };
}

macro_rules! imported_coverage_class {
    ($coverage:expr, $names:expr, $definitions:expr, $serial:expr, $prefix:expr) => {{
        let mut glyphs = Vec::new();
        for raw_id in 0..$names.len() {
            let glyph_id = ttf_parser::GlyphId(raw_id as u16);
            if $coverage.get(glyph_id).is_some() {
                if let Some(name) = feature_glyph_name($names, glyph_id) {
                    glyphs.push(name.to_string());
                }
            }
        }
        if glyphs.is_empty() {
            None
        } else {
            $serial += 1;
            let class_name = format!("@{}{}", $prefix, $serial);
            $definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
            Some(class_name)
        }
    }};
}

fn imported_single_substitution_map(
    lookup: &read_fonts::tables::gsub::SubstitutionLookup<'_>,
) -> std::collections::HashMap<u16, u16> {
    let mut substitutions = std::collections::HashMap::new();
    let Ok(SubstitutionSubtables::Single(subtables)) = lookup.subtables() else {
        return substitutions;
    };
    for subtable in subtables.iter().flatten() {
        match subtable {
            SingleSubst::Format1(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for glyph in coverage.iter() {
                    let source = glyph.to_u16();
                    let target = i32::from(source) + i32::from(table.delta_glyph_id());
                    if let Ok(target) = u16::try_from(target) {
                        substitutions.insert(source, target);
                    }
                }
            }
            SingleSubst::Format2(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for (index, glyph) in coverage.iter().enumerate() {
                    if let Some(target) = table.substitute_glyph_ids().get(index) {
                        substitutions.insert(glyph.to_u16(), target.get().to_u16());
                    }
                }
            }
        }
    }
    substitutions
}

fn imported_single_positioning_map(
    lookup: &read_fonts::tables::gpos::PositionLookup<'_>,
) -> std::collections::HashMap<u16, [i16; 4]> {
    let mut positions = std::collections::HashMap::new();
    let Ok(PositionSubtables::Single(subtables)) = lookup.subtables() else {
        return positions;
    };
    let value = |record: &ValueRecord| {
        Some([
            record.x_placement().unwrap_or(0),
            record.y_placement().unwrap_or(0),
            record.x_advance().unwrap_or(0),
            record.y_advance().unwrap_or(0),
        ])
    };
    for subtable in subtables.iter().flatten() {
        match subtable {
            SinglePos::Format1(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                let Some(value) = value(&table.value_record()) else {
                    continue;
                };
                for glyph in coverage.iter() {
                    positions.insert(glyph.to_u16(), value);
                }
            }
            SinglePos::Format2(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for (index, glyph) in coverage.iter().enumerate() {
                    let Ok(record) = table.value_records().get(index) else {
                        continue;
                    };
                    if let Some(value) = value(&record) {
                        positions.insert(glyph.to_u16(), value);
                    }
                }
            }
        }
    }
    positions
}

fn imported_read_lookup_flag_source(
    flag: read_fonts::tables::layout::LookupFlag,
) -> Option<String> {
    let mut values = Vec::new();
    if flag.contains(read_fonts::tables::layout::LookupFlag::RIGHT_TO_LEFT) {
        values.push("RightToLeft".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_BASE_GLYPHS) {
        values.push("IgnoreBaseGlyphs".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_LIGATURES) {
        values.push("IgnoreLigatures".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_MARKS) {
        values.push("IgnoreMarks".to_string());
    }
    if let Some(class) = flag.mark_attachment_class() {
        values.push(format!("MarkAttachmentType {class}"));
    }
    (!values.is_empty()).then(|| format!("lookupflag {};", values.join(" ")))
}

fn imported_contextual_gpos_features(font: &FontRef<'_>, names: &[String]) -> String {
    let Ok(gpos) = font.gpos() else {
        return String::new();
    };
    let (Ok(features), Ok(lookups)) = (gpos.feature_list(), gpos.lookup_list()) else {
        return String::new();
    };
    let mut feature_rules = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    let coverage_source = |coverage: &read_fonts::tables::layout::CoverageTable<'_>,
                           prefix: &str,
                           definitions: &mut Vec<String>,
                           serial: &mut usize|
     -> Option<String> {
        let glyphs = coverage
            .iter()
            .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
            .cloned()
            .collect::<Vec<_>>();
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    let class_source = |glyphs: Vec<String>,
                        prefix: &str,
                        definitions: &mut Vec<String>,
                        serial: &mut usize|
     -> Option<String> {
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    let value_source =
        |value: [i16; 4]| format!("<{} {} {} {}>", value[0], value[1], value[2], value[3]);
    for record in features.feature_records() {
        let tag = record.feature_tag().to_string();
        if tag.len() != 4 || !tag.is_ascii() || tag == "kern" {
            continue;
        }
        let Ok(feature) = record.feature(features.offset_data()) else {
            continue;
        };
        let rules = feature_rules.entry(tag).or_default();
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.lookups().get(usize::from(lookup_index.get())) else {
                continue;
            };
            if let Some(flags) = imported_read_lookup_flag_source(lookup.lookup_flag()) {
                rules.push(flags);
            }
            let Ok(PositionSubtables::Contextual(subtables)) = lookup.subtables() else {
                continue;
            };
            for table in subtables.iter().flatten() {
                let (records, mut tokens, _target_coverage, class_based) = match table {
                    read_fonts::tables::layout::SequenceContext::Format2(context) => {
                        let (Ok(coverage), Ok(class_def)) =
                            (context.coverage(), context.class_def())
                        else {
                            continue;
                        };
                        let mut found = None;
                        for (first_glyph, rule_set) in coverage
                            .iter()
                            .zip(context.class_seq_rule_sets().iter().flatten())
                        {
                            let Ok(rule_set) = rule_set else {
                                continue;
                            };
                            let first_class = class_def.get(first_glyph);
                            for rule in rule_set.class_seq_rules().iter() {
                                let Ok(rule) = rule else {
                                    continue;
                                };
                                if rule.seq_lookup_records().len() == 1 {
                                    found = Some((first_class, rule, class_def.clone()));
                                    break;
                                }
                            }
                            if found.is_some() {
                                break;
                            }
                        }
                        let Some((first_class, rule, class_def)) = found else {
                            continue;
                        };
                        let records = rule.seq_lookup_records();
                        let mut tokens = Vec::new();
                        let first_glyphs = coverage
                            .iter()
                            .filter(|glyph| class_def.get(*glyph) == first_class)
                            .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
                            .cloned()
                            .collect::<Vec<_>>();
                        let Some(first) = class_source(
                            first_glyphs,
                            "GPClass",
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            continue;
                        };
                        tokens.push(first);
                        for class in rule.input_sequence() {
                            let class = class.get();
                            let glyphs = (0..names.len())
                                .filter_map(|raw_id| {
                                    let glyph = GlyphId::new(raw_id as u32);
                                    (class_def.get(glyph) == class)
                                        .then(|| names.get(raw_id).cloned())
                                        .flatten()
                                })
                                .collect::<Vec<_>>();
                            let Some(token) = class_source(
                                glyphs,
                                "GPClass",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(token);
                        }
                        (records, tokens, coverage, true)
                    }
                    read_fonts::tables::layout::SequenceContext::Format3(context) => {
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let coverages = context.coverages().iter().flatten().collect::<Vec<_>>();
                        let mut tokens = Vec::new();
                        for coverage in &coverages {
                            let Some(class) = coverage_source(
                                coverage,
                                "GPContext",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(class);
                        }
                        let target_index = usize::from(records[0].sequence_index());
                        if target_index >= coverages.len() {
                            continue;
                        }
                        (records, tokens, coverages[target_index].clone(), false)
                    }
                    read_fonts::tables::layout::SequenceContext::Format1(context) => {
                        let Ok(coverage) = context.coverage() else {
                            continue;
                        };
                        let mut found = None;
                        for rule_set in context.seq_rule_sets().iter().flatten() {
                            let Ok(rule_set) = rule_set else {
                                continue;
                            };
                            for rule in rule_set.seq_rules().iter() {
                                let Ok(rule) = rule else {
                                    continue;
                                };
                                if rule.seq_lookup_records().len() == 1 {
                                    found = Some((rule, coverage.clone()));
                                    break;
                                }
                            }
                            if found.is_some() {
                                break;
                            }
                        }
                        let Some((rule, target_coverage)) = found else {
                            continue;
                        };
                        let records = rule.seq_lookup_records();
                        let mut tokens = Vec::new();
                        let Some(first) = coverage_source(
                            &target_coverage,
                            "GPContext",
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            continue;
                        };
                        tokens.push(first);
                        for glyph in rule.input_sequence() {
                            let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push((*name).clone());
                        }
                        (records, tokens, target_coverage, false)
                    }
                };
                if tokens.is_empty() {
                    continue;
                }
                let lookup_record = records[0];
                let Ok(target_lookup) = lookups
                    .lookups()
                    .get(usize::from(lookup_record.lookup_list_index()))
                else {
                    continue;
                };
                let positions = imported_single_positioning_map(&target_lookup);
                let target_index = usize::from(lookup_record.sequence_index());
                if positions.is_empty() || target_index >= tokens.len() {
                    continue;
                }
                for (source, value) in positions {
                    let Some(name) = names.get(usize::from(source)) else {
                        continue;
                    };
                    if class_based {
                        tokens[target_index] =
                            format!("{}'", tokens[target_index].trim_end_matches('\''));
                    } else {
                        tokens[target_index] = format!("{name}'");
                    }
                    rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                }
            }
        }
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.lookups().get(usize::from(lookup_index.get())) else {
                continue;
            };
            if let Some(flags) = imported_read_lookup_flag_source(lookup.lookup_flag()) {
                rules.push(flags);
            }
            let Ok(PositionSubtables::ChainContextual(subtables)) = lookup.subtables() else {
                continue;
            };
            for table in subtables.iter().flatten() {
                if let read_fonts::tables::layout::ChainedSequenceContext::Format1(context) = table
                {
                    let Ok(coverage) = context.coverage() else {
                        continue;
                    };
                    let mut found = None;
                    for (first_glyph, rule_set) in coverage
                        .iter()
                        .zip(context.chained_seq_rule_sets().iter().flatten())
                    {
                        let Ok(rule_set) = rule_set else {
                            continue;
                        };
                        for rule in rule_set.chained_seq_rules().iter() {
                            let Ok(rule) = rule else {
                                continue;
                            };
                            if rule.seq_lookup_records().len() == 1 {
                                found = Some((first_glyph, rule));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    let Some((first_glyph, rule)) = found else {
                        continue;
                    };
                    let records = rule.seq_lookup_records();
                    let target_index = usize::from(records[0].sequence_index());
                    let mut tokens = Vec::new();
                    for glyph in rule.backtrack_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    let Some(first_name) = names.get(usize::from(first_glyph.to_u16())) else {
                        continue;
                    };
                    tokens.push((*first_name).clone());
                    for glyph in rule.input_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    for glyph in rule.lookahead_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    if tokens.is_empty() {
                        continue;
                    }
                    let Ok(target_lookup) = lookups
                        .lookups()
                        .get(usize::from(records[0].lookup_list_index()))
                    else {
                        continue;
                    };
                    let positions = imported_single_positioning_map(&target_lookup);
                    let token_index = rule.backtrack_sequence().len() + target_index;
                    for (source, value) in positions {
                        let Some(name) = names.get(usize::from(source)) else {
                            continue;
                        };
                        if token_index >= tokens.len() {
                            continue;
                        }
                        tokens[token_index] = format!("{name}'");
                        rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                    }
                    continue;
                }
                if let read_fonts::tables::layout::ChainedSequenceContext::Format2(context) = table
                {
                    let (Ok(coverage), Ok(backtrack_def), Ok(input_def), Ok(lookahead_def)) = (
                        context.coverage(),
                        context.backtrack_class_def(),
                        context.input_class_def(),
                        context.lookahead_class_def(),
                    ) else {
                        continue;
                    };
                    let mut found = None;
                    for (first_glyph, rule_set) in coverage
                        .iter()
                        .zip(context.chained_class_seq_rule_sets().iter().flatten())
                    {
                        let Ok(rule_set) = rule_set else {
                            continue;
                        };
                        for rule in rule_set.chained_class_seq_rules().iter() {
                            let Ok(rule) = rule else {
                                continue;
                            };
                            if rule.seq_lookup_records().len() == 1 {
                                found = Some((first_glyph, rule));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    let Some((first_glyph, rule)) = found else {
                        continue;
                    };
                    let records = rule.seq_lookup_records();
                    let class_glyphs = |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                        class: u16|
                     -> Vec<String> {
                        (0..names.len())
                            .filter_map(|raw_id| {
                                let glyph = GlyphId::new(raw_id as u32);
                                (class_def.get(glyph) == class)
                                    .then(|| names.get(raw_id).cloned())
                                    .flatten()
                            })
                            .collect()
                    };
                    let class_token = |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                       class: u16,
                                       definitions: &mut Vec<String>,
                                       serial: &mut usize|
                     -> Option<String> {
                        class_source(
                            class_glyphs(class_def, class),
                            "GPChainClass",
                            definitions,
                            serial,
                        )
                    };
                    let mut tokens = Vec::new();
                    for class in rule.backtrack_sequence() {
                        let Some(token) = class_token(
                            &backtrack_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    let first_class = input_def.get(first_glyph);
                    let Some(token) = class_token(
                        &input_def,
                        first_class,
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        continue;
                    };
                    tokens.push(token);
                    for class in rule.input_sequence() {
                        let Some(token) = class_token(
                            &input_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    for class in rule.lookahead_sequence() {
                        let Some(token) = class_token(
                            &lookahead_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    if tokens.is_empty() {
                        continue;
                    }
                    let Ok(target_lookup) = lookups
                        .lookups()
                        .get(usize::from(records[0].lookup_list_index()))
                    else {
                        continue;
                    };
                    let positions = imported_single_positioning_map(&target_lookup);
                    let target_index =
                        rule.backtrack_sequence().len() + usize::from(records[0].sequence_index());
                    if target_index >= tokens.len() {
                        continue;
                    }
                    tokens[target_index] = format!("{}'", tokens[target_index]);
                    let target_class = if records[0].sequence_index() == 0 {
                        first_class
                    } else {
                        let Some(class) = rule
                            .input_sequence()
                            .get(usize::from(records[0].sequence_index()) - 1)
                        else {
                            continue;
                        };
                        class.get()
                    };
                    for (source, value) in positions {
                        if input_def.get(GlyphId::new(u32::from(source))) != target_class {
                            continue;
                        }
                        rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                    }
                    continue;
                }
                let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) = table
                else {
                    continue;
                };
                let records = context.seq_lookup_records();
                if records.len() != 1 {
                    continue;
                }
                let input_coverages = context
                    .input_coverages()
                    .iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let target_index = usize::from(records[0].sequence_index());
                if input_coverages.is_empty() || target_index >= input_coverages.len() {
                    continue;
                }
                let mut tokens = Vec::new();
                for coverage in context.backtrack_coverages().iter().flatten() {
                    let Some(class) = coverage_source(
                        &coverage,
                        "GPChainB",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                if tokens.is_empty() && context.backtrack_glyph_count() != 0 {
                    continue;
                }
                for coverage in &input_coverages {
                    let Some(class) = coverage_source(
                        coverage,
                        "GPChainI",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                for coverage in context.lookahead_coverages().iter().flatten() {
                    let Some(class) = coverage_source(
                        &coverage,
                        "GPChainL",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                if tokens.is_empty() {
                    continue;
                }
                let Ok(target_lookup) = lookups
                    .lookups()
                    .get(usize::from(records[0].lookup_list_index()))
                else {
                    continue;
                };
                let positions = imported_single_positioning_map(&target_lookup);
                let token_index = usize::from(context.backtrack_glyph_count()) + target_index;
                for (source, value) in positions {
                    if input_coverages[target_index]
                        .get(GlyphId::new(u32::from(source)))
                        .is_none()
                    {
                        continue;
                    }
                    let Some(name) = names.get(usize::from(source)) else {
                        continue;
                    };
                    if token_index >= tokens.len() {
                        continue;
                    }
                    tokens[token_index] = format!("{name}'");
                    rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                }
            }
        }
    }
    let features = feature_rules
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>();
    class_definitions
        .into_iter()
        .chain(features)
        .collect::<Vec<_>>()
        .join(" ")
}

fn imported_contextual_gsub_features(font: &FontRef<'_>, names: &[String]) -> String {
    let Ok(gsub) = font.gsub() else {
        return String::new();
    };
    let (Ok(features), Ok(lookups)) = (gsub.feature_list(), gsub.lookup_list()) else {
        return String::new();
    };
    let mut feature_rules = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    let coverage_source = |coverage: &read_fonts::tables::layout::CoverageTable<'_>,
                           prefix: &str,
                           definitions: &mut Vec<String>,
                           serial: &mut usize|
     -> Option<String> {
        let glyphs = coverage
            .iter()
            .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
            .cloned()
            .collect::<Vec<_>>();
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    let class_source = |glyphs: Vec<String>,
                        prefix: &str,
                        definitions: &mut Vec<String>,
                        serial: &mut usize|
     -> Option<String> {
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    for record in features.feature_records() {
        let tag = record.feature_tag().to_string();
        if tag.len() != 4 || !tag.is_ascii() {
            continue;
        }
        let Ok(feature) = record.feature(features.offset_data()) else {
            continue;
        };
        let rules = feature_rules.entry(tag).or_default();
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.lookups().get(usize::from(lookup_index.get())) else {
                continue;
            };
            if let Some(flags) = imported_read_lookup_flag_source(lookup.lookup_flag()) {
                rules.push(flags);
            }
            let Ok(subtables) = lookup.subtables() else {
                continue;
            };
            match subtables {
                SubstitutionSubtables::Contextual(subtables) => {
                    for table in subtables.iter().flatten() {
                        if let read_fonts::tables::layout::SequenceContext::Format2(context) = table
                        {
                            let (Ok(coverage), Ok(class_def)) =
                                (context.coverage(), context.class_def())
                            else {
                                continue;
                            };
                            let mut found = None;
                            for (first_glyph, rule_set) in coverage
                                .iter()
                                .zip(context.class_seq_rule_sets().iter().flatten())
                            {
                                let Ok(rule_set) = rule_set else {
                                    continue;
                                };
                                let first_class = class_def.get(first_glyph);
                                for rule in rule_set.class_seq_rules().iter() {
                                    let Ok(rule) = rule else {
                                        continue;
                                    };
                                    if rule.seq_lookup_records().len() == 1 {
                                        found = Some((first_class, rule, class_def.clone()));
                                        break;
                                    }
                                }
                                if found.is_some() {
                                    break;
                                }
                            }
                            let Some((first_class, rule, class_def)) = found else {
                                continue;
                            };
                            let records = rule.seq_lookup_records();
                            let Some(first) = class_source(
                                coverage
                                    .iter()
                                    .filter(|glyph| class_def.get(*glyph) == first_class)
                                    .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
                                    .cloned()
                                    .collect(),
                                "GSClass",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                continue;
                            };
                            let mut tokens = vec![first];
                            for class in rule.input_sequence() {
                                let glyphs = (0..names.len())
                                    .filter_map(|raw_id| {
                                        let glyph = GlyphId::new(raw_id as u32);
                                        (class_def.get(glyph) == class.get())
                                            .then(|| names.get(raw_id).cloned())
                                            .flatten()
                                    })
                                    .collect();
                                let Some(token) = class_source(
                                    glyphs,
                                    "GSClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            let target_index = usize::from(records[0].sequence_index());
                            let target_class = if target_index == 0 {
                                first_class
                            } else {
                                let Some(class) = rule.input_sequence().get(target_index - 1)
                                else {
                                    continue;
                                };
                                class.get()
                            };
                            if tokens.is_empty() || target_index >= tokens.len() {
                                continue;
                            }
                            let Ok(target_lookup) = lookups
                                .lookups()
                                .get(usize::from(records[0].lookup_list_index()))
                            else {
                                continue;
                            };
                            for (source, target) in imported_single_substitution_map(&target_lookup)
                            {
                                if class_def.get(GlyphId::new(u32::from(source))) != target_class {
                                    continue;
                                }
                                let Some(target_name) = names.get(usize::from(target)) else {
                                    continue;
                                };
                                tokens[target_index] =
                                    format!("{}'", tokens[target_index].trim_end_matches('\''));
                                rules.push(format!("sub {} by {target_name};", tokens.join(" ")));
                            }
                            continue;
                        }
                        if let read_fonts::tables::layout::SequenceContext::Format1(context) = table
                        {
                            let Ok(coverage) = context.coverage() else {
                                continue;
                            };
                            for (first_glyph, rule_set) in coverage
                                .iter()
                                .zip(context.seq_rule_sets().iter().flatten())
                            {
                                let Ok(rule_set) = rule_set else {
                                    continue;
                                };
                                for rule in rule_set.seq_rules().iter() {
                                    let Ok(rule) = rule else {
                                        continue;
                                    };
                                    let records = rule.seq_lookup_records();
                                    if records.len() != 1 {
                                        continue;
                                    }
                                    let Ok(target_lookup) = lookups
                                        .lookups()
                                        .get(usize::from(records[0].lookup_list_index()))
                                    else {
                                        continue;
                                    };
                                    let substitutions =
                                        imported_single_substitution_map(&target_lookup);
                                    if substitutions.is_empty() {
                                        continue;
                                    }
                                    let Some(first_name) =
                                        names.get(usize::from(first_glyph.to_u16()))
                                    else {
                                        continue;
                                    };
                                    let mut tokens = vec![first_name.clone()];
                                    for glyph in rule.input_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    let target_index = usize::from(records[0].sequence_index());
                                    if tokens.is_empty() || target_index >= tokens.len() {
                                        continue;
                                    }
                                    for (source, target) in substitutions {
                                        let Some(source_name) = names.get(usize::from(source))
                                        else {
                                            continue;
                                        };
                                        let Some(target_name) = names.get(usize::from(target))
                                        else {
                                            continue;
                                        };
                                        tokens[target_index] = format!("{source_name}'");
                                        rules.push(format!(
                                            "sub {} by {target_name};",
                                            tokens.join(" ")
                                        ));
                                    }
                                }
                            }
                            continue;
                        }
                        let read_fonts::tables::layout::SequenceContext::Format3(context) = table
                        else {
                            continue;
                        };
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let lookup_record = records[0];
                        let Ok(target_lookup) = lookups
                            .lookups()
                            .get(usize::from(lookup_record.lookup_list_index()))
                        else {
                            continue;
                        };
                        let substitutions = imported_single_substitution_map(&target_lookup);
                        if substitutions.is_empty() {
                            continue;
                        }
                        let coverages = context.coverages().iter().flatten().collect::<Vec<_>>();
                        let target_index = usize::from(lookup_record.sequence_index());
                        if target_index >= coverages.len() {
                            continue;
                        }
                        let mut context_tokens = Vec::new();
                        for coverage in coverages {
                            let Some(class) = coverage_source(
                                &coverage,
                                "GSCtx",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                context_tokens.clear();
                                break;
                            };
                            context_tokens.push(class);
                        }
                        if context_tokens.is_empty() {
                            continue;
                        }
                        for (source, target) in substitutions {
                            if context
                                .coverages()
                                .iter()
                                .nth(target_index)
                                .and_then(Result::ok)
                                .is_none_or(|coverage| {
                                    coverage.get(GlyphId::new(u32::from(source))).is_none()
                                })
                            {
                                continue;
                            }
                            let (Some(source), Some(target)) = (
                                names.get(usize::from(source)),
                                names.get(usize::from(target)),
                            ) else {
                                continue;
                            };
                            context_tokens[target_index] = format!("{source}'");
                            rules.push(format!("sub {} by {target};", context_tokens.join(" ")));
                        }
                    }
                }
                SubstitutionSubtables::ChainContextual(subtables) => {
                    for table in subtables.iter().flatten() {
                        if let read_fonts::tables::layout::ChainedSequenceContext::Format2(
                            context,
                        ) = table
                        {
                            let (Ok(coverage), Ok(backtrack_def), Ok(input_def), Ok(lookahead_def)) = (
                                context.coverage(),
                                context.backtrack_class_def(),
                                context.input_class_def(),
                                context.lookahead_class_def(),
                            ) else {
                                continue;
                            };
                            let mut found = None;
                            for (first_glyph, rule_set) in coverage
                                .iter()
                                .zip(context.chained_class_seq_rule_sets().iter().flatten())
                            {
                                let Ok(rule_set) = rule_set else {
                                    continue;
                                };
                                for rule in rule_set.chained_class_seq_rules().iter() {
                                    let Ok(rule) = rule else {
                                        continue;
                                    };
                                    if rule.seq_lookup_records().len() == 1 {
                                        found = Some((first_glyph, rule));
                                        break;
                                    }
                                }
                                if found.is_some() {
                                    break;
                                }
                            }
                            let Some((first_glyph, rule)) = found else {
                                continue;
                            };
                            let records = rule.seq_lookup_records();
                            let class_glyphs =
                                |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                 class: u16|
                                 -> Vec<String> {
                                    (0..names.len())
                                        .filter_map(|raw_id| {
                                            let glyph = GlyphId::new(raw_id as u32);
                                            (class_def.get(glyph) == class)
                                                .then(|| names.get(raw_id).cloned())
                                                .flatten()
                                        })
                                        .collect()
                                };
                            let class_token =
                                |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                 class: u16,
                                 prefix: &str,
                                 definitions: &mut Vec<String>,
                                 serial: &mut usize|
                                 -> Option<String> {
                                    class_source(
                                        class_glyphs(class_def, class),
                                        prefix,
                                        definitions,
                                        serial,
                                    )
                                };
                            let mut tokens = Vec::new();
                            for class in rule.backtrack_sequence() {
                                let Some(token) = class_token(
                                    &backtrack_def,
                                    class.get(),
                                    "GSChainClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            let first_class = input_def.get(first_glyph);
                            let Some(token) = class_token(
                                &input_def,
                                first_class,
                                "GSChainClass",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                continue;
                            };
                            tokens.push(token);
                            for class in rule.input_sequence() {
                                let Some(token) = class_token(
                                    &input_def,
                                    class.get(),
                                    "GSChainClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            for class in rule.lookahead_sequence() {
                                let Some(token) = class_token(
                                    &lookahead_def,
                                    class.get(),
                                    "GSChainClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            let target_index = rule.backtrack_sequence().len()
                                + usize::from(records[0].sequence_index());
                            let target_class = if records[0].sequence_index() == 0 {
                                first_class
                            } else {
                                let Some(class) = rule
                                    .input_sequence()
                                    .get(usize::from(records[0].sequence_index()) - 1)
                                else {
                                    continue;
                                };
                                class.get()
                            };
                            if tokens.is_empty() || target_index >= tokens.len() {
                                continue;
                            }
                            let Ok(target_lookup) = lookups
                                .lookups()
                                .get(usize::from(records[0].lookup_list_index()))
                            else {
                                continue;
                            };
                            for (source, target) in imported_single_substitution_map(&target_lookup)
                            {
                                if input_def.get(GlyphId::new(u32::from(source))) != target_class {
                                    continue;
                                }
                                let Some(target_name) = names.get(usize::from(target)) else {
                                    continue;
                                };
                                tokens[target_index] =
                                    format!("{}'", tokens[target_index].trim_end_matches('\''));
                                rules.push(format!("sub {} by {target_name};", tokens.join(" ")));
                            }
                            continue;
                        }
                        if let read_fonts::tables::layout::ChainedSequenceContext::Format1(
                            context,
                        ) = table
                        {
                            let Ok(coverage) = context.coverage() else {
                                continue;
                            };
                            for (first_glyph, rule_set) in coverage
                                .iter()
                                .zip(context.chained_seq_rule_sets().iter().flatten())
                            {
                                let Ok(rule_set) = rule_set else {
                                    continue;
                                };
                                for rule in rule_set.chained_seq_rules().iter() {
                                    let Ok(rule) = rule else {
                                        continue;
                                    };
                                    let records = rule.seq_lookup_records();
                                    if records.len() != 1 {
                                        continue;
                                    }
                                    let Ok(target_lookup) = lookups
                                        .lookups()
                                        .get(usize::from(records[0].lookup_list_index()))
                                    else {
                                        continue;
                                    };
                                    let substitutions =
                                        imported_single_substitution_map(&target_lookup);
                                    if substitutions.is_empty() {
                                        continue;
                                    }
                                    let Some(first_name) =
                                        names.get(usize::from(first_glyph.to_u16()))
                                    else {
                                        continue;
                                    };
                                    let mut tokens = Vec::new();
                                    for glyph in rule.backtrack_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    tokens.push(first_name.clone());
                                    for glyph in rule.input_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    for glyph in rule.lookahead_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    let target_index = rule.backtrack_sequence().len()
                                        + usize::from(records[0].sequence_index());
                                    if tokens.is_empty() || target_index >= tokens.len() {
                                        continue;
                                    }
                                    for (source, target) in substitutions {
                                        let (Some(source_name), Some(target_name)) = (
                                            names.get(usize::from(source)),
                                            names.get(usize::from(target)),
                                        ) else {
                                            continue;
                                        };
                                        tokens[target_index] = format!("{source_name}'");
                                        rules.push(format!(
                                            "sub {} by {target_name};",
                                            tokens.join(" ")
                                        ));
                                    }
                                }
                            }
                            continue;
                        }
                        let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) =
                            table
                        else {
                            continue;
                        };
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let lookup_record = records[0];
                        let Ok(target_lookup) = lookups
                            .lookups()
                            .get(usize::from(lookup_record.lookup_list_index()))
                        else {
                            continue;
                        };
                        let substitutions = imported_single_substitution_map(&target_lookup);
                        let input_coverages = context
                            .input_coverages()
                            .iter()
                            .flatten()
                            .collect::<Vec<_>>();
                        let target_index = usize::from(lookup_record.sequence_index());
                        if substitutions.is_empty() || target_index >= input_coverages.len() {
                            continue;
                        }
                        let mut tokens = Vec::new();
                        for coverage in context.backtrack_coverages().iter().flatten() {
                            let Some(class) = coverage_source(
                                &coverage,
                                "GSChainB",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(class);
                        }
                        if tokens.is_empty() && context.backtrack_glyph_count() != 0 {
                            continue;
                        }
                        for coverage in &input_coverages {
                            let Some(class) = coverage_source(
                                coverage,
                                "GSChainI",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(class);
                        }
                        for coverage in context.lookahead_coverages().iter().flatten() {
                            let Some(class) = coverage_source(
                                &coverage,
                                "GSChainL",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(class);
                        }
                        if tokens.is_empty() {
                            continue;
                        }
                        let input_start = usize::from(context.backtrack_glyph_count());
                        for (source, target) in substitutions {
                            if input_coverages[target_index]
                                .get(GlyphId::new(u32::from(source)))
                                .is_none()
                            {
                                continue;
                            }
                            let (Some(source), Some(target)) = (
                                names.get(usize::from(source)),
                                names.get(usize::from(target)),
                            ) else {
                                continue;
                            };
                            tokens[input_start + target_index] = format!("{source}'");
                            rules.push(format!("sub {} by {target};", tokens.join(" ")));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let features = feature_rules
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>();
    class_definitions
        .into_iter()
        .chain(features)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Restores the simple, lossless GSUB lookup forms that are commonly found in
/// production fonts as editable Feature File rules. More complex lookups stay
/// in `preserved_tables` until their full semantic editor is available.
fn import_simple_gsub_features(face: &ttf_parser::Face<'_>, names: &[String]) -> String {
    let Some(gsub) = face.tables().gsub else {
        return String::new();
    };
    let mut features = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    for feature in gsub.features {
        let tag = feature.tag.to_string();
        if tag.len() != 4 || !tag.is_ascii() {
            continue;
        }
        let rules = features.entry(tag).or_default();
        if let Some(first_lookup_index) = feature.lookup_indices.get(0) {
            if let Some(lookup) = gsub.lookups.get(first_lookup_index) {
                if let Some(flags) = imported_lookup_flag_source!(lookup) {
                    rules.push(flags);
                }
            }
        }
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gsub.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gsub::SubstitutionSubtable>()
            {
                match subtable {
                    ttf_parser::gsub::SubstitutionSubtable::Single(
                        ttf_parser::gsub::SingleSubstitution::Format1 { coverage, delta },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            if coverage.get(source).is_none() {
                                continue;
                            }
                            let target = u16::try_from(i32::from(source.0) + i32::from(delta))
                                .ok()
                                .map(ttf_parser::GlyphId);
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source),
                                target.and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            rules.push(format!("sub {source} by {target};"));
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Single(
                        ttf_parser::gsub::SingleSubstitution::Format2 {
                            coverage,
                            substitutes,
                        },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = coverage.get(source) else {
                                continue;
                            };
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source),
                                substitutes
                                    .get(index)
                                    .and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            rules.push(format!("sub {source} by {target};"));
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Multiple(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(sequence) = table.sequences.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            let targets = sequence
                                .substitutes
                                .into_iter()
                                .filter_map(|id| feature_glyph_name(names, id))
                                .collect::<Vec<_>>();
                            if targets.len() == usize::from(sequence.substitutes.len())
                                && !targets.is_empty()
                            {
                                rules.push(format!("sub {source} by [{}];", targets.join(" ")));
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Alternate(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(alternates) = table.alternate_sets.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            let targets = alternates
                                .alternates
                                .into_iter()
                                .filter_map(|id| feature_glyph_name(names, id))
                                .collect::<Vec<_>>();
                            if targets.len() == usize::from(alternates.alternates.len())
                                && !targets.is_empty()
                            {
                                rules.push(format!("sub {source} from [{}];", targets.join(" ")));
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Ligature(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(set) = table.ligature_sets.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            for ligature in set {
                                let Some(target) = feature_glyph_name(names, ligature.glyph) else {
                                    continue;
                                };
                                let components = ligature
                                    .components
                                    .into_iter()
                                    .filter_map(|id| feature_glyph_name(names, id))
                                    .collect::<Vec<_>>();
                                if components.len() == usize::from(ligature.components.len()) {
                                    rules.push(format!(
                                        "sub {source} {} by {target};",
                                        components.join(" ")
                                    ));
                                }
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::ReverseChainSingle(table) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = table.coverage.get(source_id) else {
                                continue;
                            };
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source_id),
                                table
                                    .substitutes
                                    .get(coverage_index)
                                    .and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            let backtrack = table
                                .backtrack_coverages
                                .into_iter()
                                .filter_map(|coverage| {
                                    imported_coverage_class!(
                                        coverage,
                                        names,
                                        class_definitions,
                                        class_serial,
                                        "GSRevB"
                                    )
                                })
                                .collect::<Vec<_>>();
                            let lookahead = table
                                .lookahead_coverages
                                .into_iter()
                                .filter_map(|coverage| {
                                    imported_coverage_class!(
                                        coverage,
                                        names,
                                        class_definitions,
                                        class_serial,
                                        "GSRevL"
                                    )
                                })
                                .collect::<Vec<_>>();
                            let mut groups = backtrack;
                            groups.push(format!("{source}'"));
                            groups.extend(lookahead);
                            rules.push(format!("reversesub {} by {target};", groups.join(" ")));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let feature_source = features
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>()
        .join(" ");
    class_definitions
        .into_iter()
        .chain((!feature_source.is_empty()).then_some(feature_source))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_gpos_value(value: ttf_parser::gpos::ValueRecord<'_>) -> Option<String> {
    if value.x_placement_device.is_some()
        || value.y_placement_device.is_some()
        || value.x_advance_device.is_some()
        || value.y_advance_device.is_some()
    {
        return None;
    }
    if value.x_placement == 0
        && value.y_placement == 0
        && value.x_advance == 0
        && value.y_advance == 0
    {
        return None;
    }
    Some(format!(
        "<{} {} {} {}>",
        value.x_placement, value.y_placement, value.x_advance, value.y_advance
    ))
}

fn format_gpos_pair(
    first: ttf_parser::gpos::ValueRecord<'_>,
    second: ttf_parser::gpos::ValueRecord<'_>,
) -> Option<String> {
    let has_device = |value: ttf_parser::gpos::ValueRecord<'_>| {
        value.x_placement_device.is_some()
            || value.y_placement_device.is_some()
            || value.x_advance_device.is_some()
            || value.y_advance_device.is_some()
    };
    if has_device(first) || has_device(second) {
        return None;
    }
    let first = format_gpos_value(first).unwrap_or_else(|| "<0 0 0 0>".to_string());
    let second = format_gpos_value(second).unwrap_or_else(|| "<0 0 0 0>".to_string());
    (first != "<0 0 0 0>" || second != "<0 0 0 0>").then(|| format!("{first} {second}"))
}

/// Restores simple GPOS SinglePos lookups as editable four-value `pos` rules.
/// Device tables and more complex positioning remain preserved byte-for-byte.
fn import_simple_gpos_features(face: &ttf_parser::Face<'_>, names: &[String]) -> String {
    let Some(gpos) = face.tables().gpos else {
        return String::new();
    };
    let mut features = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    for feature in gpos.features {
        let tag = feature.tag.to_string();
        if tag.len() != 4 || !tag.is_ascii() || tag.eq_ignore_ascii_case("kern") {
            continue;
        }
        let rules = features.entry(tag).or_default();
        if let Some(first_lookup_index) = feature.lookup_indices.get(0) {
            if let Some(lookup) = gpos.lookups.get(first_lookup_index) {
                if let Some(flags) = imported_lookup_flag_source!(lookup) {
                    rules.push(flags);
                }
            }
        }
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                match subtable {
                    ttf_parser::gpos::PositioningSubtable::Single(
                        ttf_parser::gpos::SingleAdjustment::Format1 { coverage, value },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            if coverage.get(source_id).is_none() {
                                continue;
                            }
                            let (Some(source), Some(value)) = (
                                feature_glyph_name(names, source_id),
                                format_gpos_value(value),
                            ) else {
                                continue;
                            };
                            rules.push(format!("pos {source} {value};"));
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Single(
                        ttf_parser::gpos::SingleAdjustment::Format2 { coverage, values },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = coverage.get(source_id) else {
                                continue;
                            };
                            let (Some(source), Some(value)) = (
                                feature_glyph_name(names, source_id),
                                values.get(coverage_index).and_then(format_gpos_value),
                            ) else {
                                continue;
                            };
                            rules.push(format!("pos {source} {value};"));
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format1 { coverage, sets },
                    ) => {
                        for raw_id in 0..names.len() {
                            let first_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = coverage.get(first_id) else {
                                continue;
                            };
                            let Some(pair_set) = sets.get(coverage_index) else {
                                continue;
                            };
                            let Some(first_name) = feature_glyph_name(names, first_id) else {
                                continue;
                            };
                            for second_raw_id in 0..names.len() {
                                let second_id = ttf_parser::GlyphId(second_raw_id as u16);
                                let Some((first_value, second_value)) = pair_set.get(second_id)
                                else {
                                    continue;
                                };
                                let (Some(second_name), Some(value)) = (
                                    feature_glyph_name(names, second_id),
                                    format_gpos_pair(first_value, second_value),
                                ) else {
                                    continue;
                                };
                                rules.push(format!("pos {first_name} {second_name} {value};"));
                            }
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format2 {
                            coverage,
                            classes: (left_classes, right_classes),
                            matrix,
                        },
                    ) => {
                        let mut left_members = vec![Vec::<String>::new(); names.len()];
                        let mut right_members = vec![Vec::<String>::new(); names.len()];
                        let mut max_left_class = 0_u16;
                        let mut max_right_class = 0_u16;
                        for raw_id in 0..names.len() {
                            let glyph_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(name) = feature_glyph_name(names, glyph_id) else {
                                continue;
                            };
                            let left_class = left_classes.get(glyph_id);
                            let right_class = right_classes.get(glyph_id);
                            max_left_class = max_left_class.max(left_class);
                            max_right_class = max_right_class.max(right_class);
                            if coverage.get(glyph_id).is_some() {
                                left_members[usize::from(left_class)].push(name.to_string());
                            }
                            right_members[usize::from(right_class)].push(name.to_string());
                        }
                        let mut left_names = Vec::new();
                        let mut right_names = Vec::new();
                        class_serial += 1;
                        for class in 0..=max_left_class {
                            let members = &left_members[usize::from(class)];
                            if members.is_empty() {
                                left_names.push(None);
                            } else {
                                let class_name = format!("@GS{class_serial}L{class}");
                                class_definitions
                                    .push(format!("{class_name} = [{}];", members.join(" ")));
                                left_names.push(Some(class_name));
                            }
                        }
                        for class in 0..=max_right_class {
                            let members = &right_members[usize::from(class)];
                            if members.is_empty() {
                                right_names.push(None);
                            } else {
                                let class_name = format!("@GS{class_serial}R{class}");
                                class_definitions
                                    .push(format!("{class_name} = [{}];", members.join(" ")));
                                right_names.push(Some(class_name));
                            }
                        }
                        for left_class in 0..=max_left_class {
                            for right_class in 0..=max_right_class {
                                let Some((first, second)) = matrix.get((left_class, right_class))
                                else {
                                    continue;
                                };
                                let Some(value) = format_gpos_pair(first, second) else {
                                    continue;
                                };
                                let (Some(left_name), Some(right_name)) = (
                                    left_names[usize::from(left_class)].as_deref(),
                                    right_names[usize::from(right_class)].as_deref(),
                                ) else {
                                    continue;
                                };
                                rules.push(format!("pos {left_name} {right_name} {value};"));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let feature_source = features
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>()
        .join(" ");
    class_definitions
        .into_iter()
        .chain((!feature_source.is_empty()).then_some(feature_source))
        .collect::<Vec<_>>()
        .join(" ")
}

fn add_imported_anchor(project: &mut FontProject, glyph_name: &str, name: String, x: i16, y: i16) {
    let Some(glyph) = project.glyphs.get_mut(glyph_name) else {
        return;
    };
    if glyph
        .anchors
        .iter()
        .any(|anchor| anchor.name == name && anchor.x == f64::from(x) && anchor.y == f64::from(y))
    {
        return;
    }
    glyph.anchors.push(crate::font_data::GlyphAnchor {
        name,
        x: f64::from(x),
        y: f64::from(y),
    });
}

/// Imports MarkToBase anchors into the editable glyph model. Class numbers
/// are given stable names so the exporter can rebuild a valid mark class and
/// base attachment lookup on the next export.
fn import_mark_to_base_anchors(
    face: &ttf_parser::Face<'_>,
    names: &[String],
    project: &mut FontProject,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for feature in gpos.features {
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                let ttf_parser::gpos::PositioningSubtable::MarkToBase(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let mark_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark_index) = table.mark_coverage.get(mark_id) else {
                        continue;
                    };
                    let Some((class, anchor)) = table.marks.get(mark_index) else {
                        continue;
                    };
                    if anchor.x_device.is_some() || anchor.y_device.is_some() {
                        continue;
                    }
                    let Some(name) = feature_glyph_name(names, mark_id) else {
                        continue;
                    };
                    add_imported_anchor(
                        project,
                        name,
                        format!("_class{class}"),
                        anchor.x,
                        anchor.y,
                    );
                }
                for raw_id in 0..names.len() {
                    let base_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(base_index) = table.base_coverage.get(base_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, base_id) else {
                        continue;
                    };
                    for class in 0..table.anchors.cols {
                        let Some(anchor) = table.anchors.get(base_index, class) else {
                            continue;
                        };
                        if anchor.x_device.is_some() || anchor.y_device.is_some() {
                            continue;
                        }
                        add_imported_anchor(
                            project,
                            name,
                            format!("class{class}"),
                            anchor.x,
                            anchor.y,
                        );
                    }
                }
            }
        }
    }
}

fn import_cursive_anchors(
    face: &ttf_parser::Face<'_>,
    names: &[String],
    project: &mut FontProject,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for feature in gpos.features {
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                let ttf_parser::gpos::PositioningSubtable::Cursive(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let glyph_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(index) = table.coverage.get(glyph_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, glyph_id) else {
                        continue;
                    };
                    if let Some(anchor) = table.sets.entry(index) {
                        if anchor.x_device.is_none() && anchor.y_device.is_none() {
                            add_imported_anchor(project, name, "entry".into(), anchor.x, anchor.y);
                        }
                    }
                    if let Some(anchor) = table.sets.exit(index) {
                        if anchor.x_device.is_none() && anchor.y_device.is_none() {
                            add_imported_anchor(project, name, "exit".into(), anchor.x, anchor.y);
                        }
                    }
                }
            }
        }
    }
}

fn import_mark_to_mark_anchors(
    face: &ttf_parser::Face<'_>,
    names: &[String],
    project: &mut FontProject,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for feature in gpos.features {
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                let ttf_parser::gpos::PositioningSubtable::MarkToMark(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let mark_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark_index) = table.mark1_coverage.get(mark_id) else {
                        continue;
                    };
                    let Some((class, anchor)) = table.marks.get(mark_index) else {
                        continue;
                    };
                    if anchor.x_device.is_some() || anchor.y_device.is_some() {
                        continue;
                    }
                    let Some(name) = feature_glyph_name(names, mark_id) else {
                        continue;
                    };
                    add_imported_anchor(
                        project,
                        name,
                        format!("_class{class}"),
                        anchor.x,
                        anchor.y,
                    );
                }
                for raw_id in 0..names.len() {
                    let mark2_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark2_index) = table.mark2_coverage.get(mark2_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, mark2_id) else {
                        continue;
                    };
                    for class in 0..table.mark2_matrix.cols {
                        let Some(anchor) = table.mark2_matrix.get(mark2_index, class) else {
                            continue;
                        };
                        if anchor.x_device.is_some() || anchor.y_device.is_some() {
                            continue;
                        }
                        add_imported_anchor(
                            project,
                            name,
                            format!("class{class}"),
                            anchor.x,
                            anchor.y,
                        );
                    }
                }
            }
        }
    }
}

fn import_mark_to_ligature_anchors(
    face: &ttf_parser::Face<'_>,
    names: &[String],
    project: &mut FontProject,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for feature in gpos.features {
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                let ttf_parser::gpos::PositioningSubtable::MarkToLigature(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let mark_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark_index) = table.mark_coverage.get(mark_id) else {
                        continue;
                    };
                    let Some((class, anchor)) = table.marks.get(mark_index) else {
                        continue;
                    };
                    if anchor.x_device.is_some() || anchor.y_device.is_some() {
                        continue;
                    }
                    let Some(name) = feature_glyph_name(names, mark_id) else {
                        continue;
                    };
                    add_imported_anchor(
                        project,
                        name,
                        format!("_class{class}"),
                        anchor.x,
                        anchor.y,
                    );
                }
                for raw_id in 0..names.len() {
                    let ligature_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(ligature_index) = table.ligature_coverage.get(ligature_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, ligature_id) else {
                        continue;
                    };
                    let Some(components) = table.ligature_array.get(ligature_index) else {
                        continue;
                    };
                    for component in 0..components.rows {
                        for class in 0..components.cols {
                            let Some(anchor) = components.get(component, class) else {
                                continue;
                            };
                            if anchor.x_device.is_some() || anchor.y_device.is_some() {
                                continue;
                            }
                            add_imported_anchor(
                                project,
                                name,
                                format!("class{class}_{}", component + 1),
                                anchor.x,
                                anchor.y,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn push_imported_color_layer(
    names: &[String],
    leaf_glyph: Option<u16>,
    transform: Option<crate::font_data::ColorLayerTransform>,
    palette_index: u16,
    gradient: Option<crate::font_data::ColorGradient>,
    output: &mut Vec<crate::font_data::ColorLayer>,
    transforms: &mut Vec<Option<crate::font_data::ColorLayerTransform>>,
) {
    let Some(glyph) = leaf_glyph.and_then(|id| names.get(usize::from(id))) else {
        return;
    };
    output.push(crate::font_data::ColorLayer {
        glyph: glyph.clone(),
        palette_index,
        gradient,
        alpha: 1.0,
    });
    transforms.push(transform);
}

#[allow(clippy::too_many_arguments)]
fn imported_color_line(
    line: &read_fonts::tables::colr::ColorLine<'_>,
    kind: crate::font_data::ColorGradientKind,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    radius0: f64,
    radius1: f64,
    start_angle: f64,
    end_angle: f64,
) -> crate::font_data::ColorGradient {
    let stops = line
        .color_stops()
        .iter()
        .map(|stop| crate::font_data::ColorGradientStop {
            offset: f64::from(stop.stop_offset().to_f32()),
            palette_index: stop.palette_index(),
            alpha: f64::from(stop.alpha().to_f32()),
        })
        .collect::<Vec<_>>();
    let start_palette_index = stops.first().map_or(0, |stop| stop.palette_index);
    let end_palette_index = stops
        .last()
        .map_or(start_palette_index, |stop| stop.palette_index);
    let extend = match line.extend() {
        read_fonts::tables::colr::Extend::Repeat => crate::font_data::ColorGradientExtend::Repeat,
        read_fonts::tables::colr::Extend::Reflect => crate::font_data::ColorGradientExtend::Reflect,
        _ => crate::font_data::ColorGradientExtend::Pad,
    };
    crate::font_data::ColorGradient {
        start_palette_index,
        end_palette_index,
        kind,
        extend,
        x0,
        y0,
        x1,
        y1,
        x2,
        y2,
        stops,
        radius0,
        radius1,
        start_angle,
        end_angle,
    }
}

#[allow(clippy::too_many_arguments)]
fn import_colr_v1_paint(
    colr: &read_fonts::tables::colr::Colr<'_>,
    paint: read_fonts::tables::colr::Paint<'_>,
    names: &[String],
    leaf_glyph: Option<u16>,
    transform: Option<crate::font_data::ColorLayerTransform>,
    output: &mut Vec<crate::font_data::ColorLayer>,
    transforms: &mut Vec<Option<crate::font_data::ColorLayerTransform>>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    use read_fonts::tables::colr::Paint;
    match paint {
        Paint::ColrLayers(layers) => {
            let first = layers.first_layer_index() as usize;
            for index in first..first.saturating_add(usize::from(layers.num_layers())) {
                if let Ok((child, _)) = colr.v1_layer(index) {
                    import_colr_v1_paint(
                        colr,
                        child,
                        names,
                        leaf_glyph,
                        transform,
                        output,
                        transforms,
                        depth + 1,
                    );
                }
            }
        }
        Paint::Glyph(glyph) => {
            if let Ok(child) = glyph.paint() {
                import_colr_v1_paint(
                    colr,
                    child,
                    names,
                    Some(glyph.glyph_id().to_u16()),
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        Paint::ColrGlyph(glyph) => {
            if let Ok(Some((child, _))) = colr.v1_base_glyph(glyph.glyph_id().to_u32().into()) {
                import_colr_v1_paint(
                    colr,
                    child,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        Paint::Transform(item) => {
            if let Ok(matrix) = item.transform() {
                let transform = crate::font_data::ColorLayerTransform {
                    xx: matrix.xx().to_f32() as f64,
                    yx: matrix.yx().to_f32() as f64,
                    xy: matrix.xy().to_f32() as f64,
                    yy: matrix.yy().to_f32() as f64,
                    dx: matrix.dx().to_f32() as f64,
                    dy: matrix.dy().to_f32() as f64,
                };
                if let Ok(child) = item.paint() {
                    import_colr_v1_paint(
                        colr,
                        child,
                        names,
                        leaf_glyph,
                        Some(transform),
                        output,
                        transforms,
                        depth + 1,
                    );
                }
            }
        }
        Paint::Solid(item) => {
            if let Some(glyph_id) = leaf_glyph {
                if let Some(glyph) = names.get(usize::from(glyph_id)) {
                    output.push(crate::font_data::ColorLayer {
                        glyph: glyph.clone(),
                        palette_index: item.palette_index(),
                        gradient: None,
                        alpha: f64::from(item.alpha().to_f32()),
                    });
                    transforms.push(transform);
                }
            }
        }
        Paint::LinearGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Linear,
                    f64::from(item.x0().to_i16()),
                    f64::from(item.y0().to_i16()),
                    f64::from(item.x1().to_i16()),
                    f64::from(item.y1().to_i16()),
                    f64::from(item.x2().to_i16()),
                    f64::from(item.y2().to_i16()),
                    0.0,
                    0.0,
                    0.0,
                    360.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::RadialGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Radial,
                    f64::from(item.x0().to_i16()),
                    f64::from(item.y0().to_i16()),
                    f64::from(item.x1().to_i16()),
                    f64::from(item.y1().to_i16()),
                    0.0,
                    0.0,
                    f64::from(item.radius0().to_u16()),
                    f64::from(item.radius1().to_u16()),
                    0.0,
                    360.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::SweepGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Sweep,
                    f64::from(item.center_x().to_i16()),
                    f64::from(item.center_y().to_i16()),
                    f64::from(item.center_x().to_i16()),
                    f64::from(item.center_y().to_i16()),
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    f64::from(item.start_angle().to_f32()) * 180.0,
                    f64::from(item.end_angle().to_f32()) * 180.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::Composite(item) => {
            // The editable model currently represents a color glyph as an
            // ordered layer list rather than a general paint graph. Flatten
            // the backdrop first and the source second, preserving the
            // compositing order so common COLR v1 composites remain editable
            // instead of being discarded as opaque raw data. The exact blend
            // mode is intentionally not guessed here; unsupported blend modes
            // are represented by the same ordered layers and can be rebuilt
            // safely as SrcOver on export.
            if let Ok(backdrop) = item.backdrop_paint() {
                import_colr_v1_paint(
                    colr,
                    backdrop,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
            if let Ok(source) = item.source_paint() {
                import_colr_v1_paint(
                    colr,
                    source,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        _ => {}
    }
}

pub fn load_ttf(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("TTF読み込みエラー: {e}"))?;
    let mut face =
        ttf_parser::Face::parse(&bytes, 0).map_err(|e| format!("フォント解析エラー: {e:?}"))?;
    let mut project = FontProject::new();
    project.preserved_tables = preserved_sfnt_tables(&bytes);
    for index in 0..face.names().len() {
        if let Some(name) = face.names().get(index) {
            if let Some(value) = name.to_string() {
                match name.name_id {
                    0 => project.metadata.copyright = value,
                    1 => project.metadata.family_name = value,
                    2 => project.metadata.style_name = value,
                    8 => project.metadata.manufacturer = value,
                    9 => project.metadata.designer = value,
                    5 => {
                        if let Some(version) = value
                            .strip_prefix("Version ")
                            .and_then(|value| value.trim().parse::<f64>().ok())
                        {
                            project.metadata.font_revision = version;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    project.metadata.units_per_em = face.units_per_em() as f64;
    project.metadata.ascender = face.ascender() as f64;
    project.metadata.descender = face.descender() as f64;
    project.metadata.line_gap = face.line_gap() as f64;
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(head) = font.head() {
            project.metadata.head_flags = head.flags().bits();
            project.metadata.head_mac_style = head.mac_style().bits();
            project.metadata.lowest_rec_ppem = head.lowest_rec_ppem();
            project.metadata.font_direction_hint = head.font_direction_hint();
        }
        if let Ok(post) = font.post() {
            project.metadata.italic_angle = post.italic_angle().to_f64();
            project.metadata.underline_position = i16::from(post.underline_position()) as f64;
            project.metadata.underline_thickness = i16::from(post.underline_thickness()) as f64;
            project.metadata.is_fixed_pitch = post.is_fixed_pitch() != 0;
        }
    }
    project.metadata.weight_class = face.weight().to_number();
    project.metadata.width_class = face.width().to_number();
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(hhea) = font.hhea() {
            project.metadata.caret_slope_rise = hhea.caret_slope_rise();
            project.metadata.caret_slope_run = hhea.caret_slope_run();
            project.metadata.caret_offset = hhea.caret_offset();
        }
        if let Ok(os2) = font.os2() {
            project.metadata.x_height = os2.sx_height().unwrap_or(0) as f64;
            project.metadata.cap_height = os2.s_cap_height().unwrap_or(0) as f64;
            project.metadata.win_ascent = os2.us_win_ascent();
            project.metadata.win_descent = os2.us_win_descent();
            project.metadata.vendor_id = os2.ach_vend_id().to_string();
            project.metadata.fs_type = os2.fs_type();
            project.metadata.fs_selection = os2.fs_selection().bits();
            project.metadata.subscript_x_size = os2.y_subscript_x_size();
            project.metadata.subscript_y_size = os2.y_subscript_y_size();
            project.metadata.subscript_x_offset = os2.y_subscript_x_offset();
            project.metadata.subscript_y_offset = os2.y_subscript_y_offset();
            project.metadata.superscript_x_size = os2.y_superscript_x_size();
            project.metadata.superscript_y_size = os2.y_superscript_y_size();
            project.metadata.superscript_x_offset = os2.y_superscript_x_offset();
            project.metadata.superscript_y_offset = os2.y_superscript_y_offset();
            project.metadata.strikeout_size = os2.y_strikeout_size();
            project.metadata.strikeout_position = os2.y_strikeout_position();
            project.metadata.family_class = os2.s_family_class();
            project.metadata.lower_optical_point_size =
                os2.us_lower_optical_point_size().unwrap_or(0);
            project.metadata.upper_optical_point_size =
                os2.us_upper_optical_point_size().unwrap_or(0);
            if let Ok(panose) = os2.panose_10().try_into() {
                project.metadata.panose = panose;
            }
            project.metadata.default_char = os2.us_default_char().unwrap_or(0);
            project.metadata.break_char = os2.us_break_char().unwrap_or(0);
            project.metadata.max_context = os2.us_max_context().unwrap_or(0);
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axes = fvar
            .axes
            .into_iter()
            .map(|axis| (axis.tag.to_string(), axis.def_value as f64))
            .collect();
        if let Some(master) = project.masters.first_mut() {
            master.axes = axes;
        }
    }
    // Some variable fonts omit STAT but still provide localized axis labels
    // through fvar.axisNameID. Use those labels as a fallback.
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(fvar), Ok(names_table)) = (font.fvar(), font.name()) {
            let string_data = names_table.string_data();
            if let Ok(arrays) = fvar.axis_instance_arrays() {
                for axis in arrays.axes() {
                    let tag = axis.axis_tag().to_string();
                    project.axis_flags.insert(tag.clone(), axis.flags());
                    if project.axis_names.contains_key(&tag) {
                        continue;
                    }
                    let axis_name = names_table
                        .name_record()
                        .iter()
                        .find(|record| record.name_id() == axis.axis_name_id())
                        .and_then(|record| record.string(string_data).ok())
                        .map(|name| name.chars().collect::<String>())
                        .filter(|name| !name.trim().is_empty());
                    if let Some(name) = axis_name {
                        project.axis_names.insert(tag, name);
                    }
                }
            }
        }
    }
    // STAT is the authoritative source for human-readable axis labels in
    // many production variable fonts. Preserve those labels instead of
    // falling back to the four-character axis tags on import.
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(stat), Ok(names_table)) = (font.stat(), font.name()) {
            let string_data = names_table.string_data();
            for axis in stat.design_axes().into_iter().flatten() {
                let tag = axis.axis_tag().to_string();
                let axis_name = names_table
                    .name_record()
                    .iter()
                    .find(|record| record.name_id() == axis.axis_name_id())
                    .and_then(|record| record.string(string_data).ok())
                    .map(|name| name.chars().collect::<String>())
                    .filter(|name| !name.trim().is_empty());
                if let Some(name) = axis_name {
                    project.axis_names.insert(tag, name);
                }
            }
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axis_tags = fvar
            .axes
            .into_iter()
            .map(|axis| axis.tag.to_string())
            .collect::<Vec<_>>();
        if let Ok(font) = FontRef::new(&bytes) {
            if let Ok(avar) = font.avar() {
                if avar.axis_count() as usize == axis_tags.len() {
                    for (tag, segment_maps) in axis_tags
                        .into_iter()
                        .zip(avar.axis_segment_maps().iter().filter_map(Result::ok))
                    {
                        let points = segment_maps
                            .axis_value_maps()
                            .iter()
                            .map(|point| crate::font_data::AxisMappingPoint {
                                input: point.from_coordinate().to_f32() as f64,
                                output: point.to_coordinate().to_f32() as f64,
                            })
                            .collect::<Vec<_>>();
                        if !points.is_empty()
                            && points
                                .iter()
                                .any(|point| (point.input - point.output).abs() > f64::EPSILON)
                        {
                            project.axis_mappings.insert(tag, points);
                        }
                    }
                }
            }
        }
    }
    let mut unicodes = std::collections::HashMap::<u16, Vec<u32>>::new();
    for codepoint in 0..=0x10FFFF_u32 {
        if let Some(character) = char::from_u32(codepoint) {
            if let Some(glyph_id) = face.glyph_index(character) {
                let mappings = unicodes.entry(glyph_id.0).or_default();
                if !mappings.contains(&codepoint) {
                    mappings.push(codepoint);
                }
            }
        }
    }
    for raw_id in 0..face.number_of_glyphs() {
        let glyph_id = ttf_parser::GlyphId(raw_id);
        let name = face
            .glyph_name(glyph_id)
            .map(str::to_string)
            .unwrap_or_else(|| format!("glyph{raw_id}"));
        let unicode = unicodes
            .get(&raw_id)
            .and_then(|values| values.first())
            .copied();
        let mut glyph = crate::font_data::GlyphData::new(name.clone(), unicode);
        if let Some(values) = unicodes.get(&raw_id) {
            glyph.unicodes = values.iter().copied().skip(1).collect();
        }
        glyph.width = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64;
        let mut collector = OutlineCollector::new();
        face.outline_glyph(glyph_id, &mut collector);
        if !collector.current.points.is_empty() {
            collector.contours.push(collector.current);
        }
        glyph.contours = collector.contours;
        project.glyphs.insert(name.clone(), glyph);
        project.glyph_order.push(name);
    }
    let names: Vec<String> = (0..face.number_of_glyphs())
        .map(|raw_id| {
            face.glyph_name(ttf_parser::GlyphId(raw_id))
                .map(str::to_string)
                .unwrap_or_else(|| format!("glyph{raw_id}"))
        })
        .collect();
    // Convert an imported GDEF glyph classification into an explicit
    // Feature File declaration. The exporter can then retain Ligature/Mark/
    // Component semantics even when the original font has no editable source.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(gdef) = font.gdef() {
            let mut gdef_source = String::new();
            let mut mark_set_source = String::new();
            if let Some(Ok(class_def)) = gdef.glyph_class_def() {
                let mut groups = [Vec::<String>::new(), Vec::new(), Vec::new(), Vec::new()];
                for (glyph_id, class) in class_def.iter() {
                    let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                        continue;
                    };
                    let index = match class {
                        1..=4 => usize::from(class - 1),
                        _ => continue,
                    };
                    groups[index].push(name.clone());
                }
                if groups.iter().any(|group| !group.is_empty()) {
                    let definitions = groups
                        .iter()
                        .map(|group| format!("[{}]", group.join(" ")))
                        .collect::<Vec<_>>()
                        .join(", ");
                    gdef_source.push_str(&format!("GlyphClassDef {definitions}; "));
                }
            }
            if let Some(Ok(attach_list)) = gdef.attach_list() {
                let glyphs = attach_list.coverage().ok().map(|coverage| coverage.iter());
                if let Some(glyphs) = glyphs {
                    for (glyph_id, point_table) in glyphs.zip(attach_list.attach_points().iter()) {
                        let Ok(point_table) = point_table else {
                            continue;
                        };
                        let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                            continue;
                        };
                        let points = point_table
                            .point_indices()
                            .iter()
                            .map(|point| point.get().to_string())
                            .collect::<Vec<_>>();
                        if !points.is_empty() {
                            gdef_source.push_str(&format!("Attach {name} {}; ", points.join(" ")));
                        }
                    }
                }
            }
            if let Some(Ok(lig_caret_list)) = gdef.lig_caret_list() {
                let glyphs = lig_caret_list
                    .coverage()
                    .ok()
                    .map(|coverage| coverage.iter());
                if let Some(glyphs) = glyphs {
                    for (glyph_id, lig_glyph) in glyphs.zip(lig_caret_list.lig_glyphs().iter()) {
                        let Ok(lig_glyph) = lig_glyph else {
                            continue;
                        };
                        let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                            continue;
                        };
                        let values = lig_glyph
                            .caret_values()
                            .iter()
                            .filter_map(Result::ok)
                            .filter_map(|caret| match caret {
                                read_fonts::tables::gdef::CaretValue::Format1(value) => Some(
                                    format!("LigatureCaretByPos {name} {}", value.coordinate()),
                                ),
                                read_fonts::tables::gdef::CaretValue::Format2(value) => {
                                    Some(format!(
                                        "LigatureCaretByIndex {name} {}",
                                        value.caret_value_point_index()
                                    ))
                                }
                                read_fonts::tables::gdef::CaretValue::Format3(value)
                                    if value.device_offset().to_usize() == 0 =>
                                {
                                    Some(format!(
                                        "LigatureCaretByPos {name} {}",
                                        value.coordinate()
                                    ))
                                }
                                read_fonts::tables::gdef::CaretValue::Format3(_) => None,
                            })
                            .collect::<Vec<_>>();
                        if !values.is_empty() {
                            gdef_source.push_str(&format!("{}; ", values.join("; ")));
                        }
                    }
                }
            }
            if let Some(Ok(mark_sets)) = gdef.mark_glyph_sets_def() {
                for (index, coverage) in mark_sets.coverages().iter().enumerate() {
                    let Ok(coverage) = coverage else {
                        continue;
                    };
                    let glyphs = coverage
                        .iter()
                        .filter_map(|glyph_id| names.get(glyph_id.to_u32() as usize))
                        .cloned()
                        .collect::<Vec<_>>();
                    if !glyphs.is_empty() {
                        mark_set_source
                            .push_str(&format!("@GSMarkSet{index} = [{}]; ", glyphs.join(" ")));
                    }
                }
            }
            if !gdef_source.is_empty() {
                project.opentype_features =
                    format!("{mark_set_source}table GDEF {{ {gdef_source}}} GDEF;");
            } else if !mark_set_source.is_empty() {
                project.opentype_features = mark_set_source;
            }
        }
        if let Ok(name_table) = font.name() {
            let raw_name = font
                .table_data(read_fonts::types::Tag::new(b"name"))
                .map(|data| data.as_bytes());
            if let Some(raw_name) = raw_name {
                let storage_start = usize::from(name_table.storage_offset());
                let mut records = Vec::new();
                for record in name_table.name_record() {
                    let name_id = record.name_id().to_u16();
                    if name_id < 23 {
                        continue;
                    }
                    let start = storage_start + record.string_offset().to_usize();
                    let end = start + usize::from(record.length());
                    let Some(value) = decode_name_string(
                        record.platform_id(),
                        raw_name.get(start..end).unwrap_or_default(),
                    ) else {
                        continue;
                    };
                    if value.is_empty() {
                        continue;
                    }
                    records.push(format!(
                        "nameid {} {} {} 0x{:04X} \"{}\";",
                        name_id,
                        record.platform_id(),
                        record.encoding_id(),
                        record.language_id(),
                        escape_feature_name(&value)
                    ));
                }
                if !records.is_empty() {
                    if !project.opentype_features.is_empty() {
                        project.opentype_features.push('\n');
                    }
                    project.opentype_features.push_str("table name { ");
                    project.opentype_features.push_str(&records.join(" "));
                    project.opentype_features.push_str(" } name;");
                }
            }
        }
    }
    let imported_gsub = import_simple_gsub_features(&face, &names);
    if !imported_gsub.is_empty() {
        if !project.opentype_features.is_empty() {
            project.opentype_features.push('\n');
        }
        project.opentype_features.push_str(&imported_gsub);
    }
    if let Ok(font) = FontRef::new(&bytes) {
        let imported_contextual = imported_contextual_gsub_features(&font, &names);
        if !imported_contextual.is_empty() {
            if !project.opentype_features.is_empty() {
                project.opentype_features.push('\n');
            }
            project.opentype_features.push_str(&imported_contextual);
        }
    }
    let imported_gpos = import_simple_gpos_features(&face, &names);
    if !imported_gpos.is_empty() {
        if !project.opentype_features.is_empty() {
            project.opentype_features.push('\n');
        }
        project.opentype_features.push_str(&imported_gpos);
    }
    if let Ok(font) = FontRef::new(&bytes) {
        let imported_contextual = imported_contextual_gpos_features(&font, &names);
        if !imported_contextual.is_empty() {
            if !project.opentype_features.is_empty() {
                project.opentype_features.push('\n');
            }
            project.opentype_features.push_str(&imported_contextual);
        }
    }
    let mut imported_languagesystems = std::collections::BTreeSet::new();
    if let Some(gsub) = face.tables().gsub {
        collect_imported_languagesystems!(gsub, imported_languagesystems);
    }
    if let Some(gpos) = face.tables().gpos {
        collect_imported_languagesystems!(gpos, imported_languagesystems);
    }
    if !imported_languagesystems.is_empty() {
        let declarations = imported_languagesystems
            .into_iter()
            .collect::<Vec<_>>()
            .join(" ");
        project.opentype_features = if project.opentype_features.is_empty() {
            declarations
        } else {
            format!("{declarations}\n{}", project.opentype_features)
        };
    }
    import_mark_to_base_anchors(&face, &names, &mut project);
    import_cursive_anchors(&face, &names, &mut project);
    import_mark_to_mark_anchors(&face, &names, &mut project);
    import_mark_to_ligature_anchors(&face, &names, &mut project);
    // Import non-default Unicode Variation Sequences from cmap format 14.
    // Default UVS entries intentionally do not become project mappings: they
    // mean “use the ordinary cmap glyph”, not a separate glyph assignment.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(cmap) = font.cmap() {
            if let Some((_, uvs)) = cmap.uvs_subtable() {
                for (base, selector, variant) in uvs.iter() {
                    let read_fonts::tables::cmap::MapVariant::Variant(glyph_id) = variant else {
                        continue;
                    };
                    let Some(glyph) = names.get(glyph_id.to_u32() as usize) else {
                        continue;
                    };
                    if !project
                        .unicode_variation_sequences
                        .iter()
                        .any(|mapping| mapping.base == base && mapping.selector == selector)
                    {
                        project.unicode_variation_sequences.push(
                            crate::font_data::UnicodeVariationSequence {
                                base,
                                selector,
                                glyph: glyph.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(colr), Ok(cpal)) = (font.colr(), font.cpal()) {
            if let Some(Ok(records)) = cpal.color_records_array() {
                let entries = usize::from(cpal.num_palette_entries());
                let palettes = usize::from(cpal.num_palettes());
                project.color_palettes = (0..palettes)
                    .map(|palette| {
                        (0..entries)
                            .filter_map(|index| {
                                records.get(palette * entries + index).map(|record| {
                                    [record.red(), record.green(), record.blue(), record.alpha()]
                                })
                            })
                            .collect()
                    })
                    .collect();
            }
            if cpal.version() >= 1 {
                if let Some(Ok(types)) = cpal.palette_types_array() {
                    project.color_palette_types = types
                        .iter()
                        .map(|palette_type| palette_type.get().bits())
                        .collect();
                }
                if let (Some(Ok(labels)), Ok(name_table)) =
                    (cpal.palette_entry_labels_array(), font.name())
                {
                    let string_data = name_table.string_data();
                    project.color_palette_entry_names = labels
                        .iter()
                        .map(|label| {
                            let name_id = label.get().to_u16();
                            if name_id == u16::MAX {
                                return String::new();
                            }
                            name_table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == NameId::new(name_id))
                                .and_then(|record| record.string(string_data).ok())
                                .map(|name| name.chars().collect())
                                .unwrap_or_default()
                        })
                        .collect();
                }
                if let (Some(Ok(labels)), Ok(name_table)) =
                    (cpal.palette_labels_array(), font.name())
                {
                    let string_data = name_table.string_data();
                    project.color_palette_names = labels
                        .iter()
                        .map(|label| {
                            let name_id = label.get().to_u16();
                            if name_id == u16::MAX {
                                return String::new();
                            }
                            name_table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == NameId::new(name_id))
                                .and_then(|record| record.string(string_data).ok())
                                .map(|name| name.chars().collect())
                                .unwrap_or_default()
                        })
                        .collect();
                }
            }
            for (raw_id, name) in names.iter().enumerate() {
                let Ok(raw_id) = u16::try_from(raw_id) else {
                    continue;
                };
                let Ok(Some(range)) = colr.v0_base_glyph(GlyphId::new(raw_id as u32)) else {
                    continue;
                };
                let layers = range
                    .filter_map(|index| colr.v0_layer(index).ok())
                    .filter_map(|(glyph, palette_index)| {
                        names.get(usize::from(glyph.to_u16())).map(|glyph| {
                            crate::font_data::ColorLayer {
                                glyph: glyph.clone(),
                                palette_index,
                                gradient: None,
                                alpha: 1.0,
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                if !layers.is_empty() {
                    project.color_layers.insert(name.clone(), layers);
                }
            }
            // COLR v1 paints are flattened into the editable layer model for
            // PaintGlyph/Solid/gradient/transform and composite graphs. A
            // composite's blend mode is approximated by ordered SrcOver
            // layers because the project model does not yet store a general
            // paint graph.
            for (raw_id, name) in names.iter().enumerate() {
                let Ok(raw_id) = u16::try_from(raw_id) else {
                    continue;
                };
                let Ok(Some((paint, _))) = colr.v1_base_glyph(GlyphId::new(u32::from(raw_id)))
                else {
                    continue;
                };
                let mut imported_layers = Vec::new();
                let mut imported_transforms = Vec::new();
                import_colr_v1_paint(
                    &colr,
                    paint,
                    &names,
                    None,
                    None,
                    &mut imported_layers,
                    &mut imported_transforms,
                    0,
                );
                if !imported_layers.is_empty() {
                    project.color_layers.insert(name.clone(), imported_layers);
                    if imported_transforms.iter().any(Option::is_some) {
                        project
                            .color_layer_transforms
                            .insert(name.clone(), imported_transforms);
                    }
                }
            }
        }
        if let (Ok(vhea), Ok(vmtx)) = (font.vhea(), font.vmtx()) {
            project.metadata.vertical_caret_slope_rise = vhea.caret_slope_rise();
            project.metadata.vertical_caret_slope_run = vhea.caret_slope_run();
            project.metadata.vertical_caret_offset = vhea.caret_offset();
            let long_metrics = vmtx.v_metrics();
            let top_bearings = vmtx.top_side_bearings();
            let long_count = usize::from(vhea.number_of_long_ver_metrics());
            for (raw_id, name) in names.iter().enumerate() {
                let (advance, bearing) = if raw_id < long_metrics.len() {
                    let metric = long_metrics[raw_id];
                    (metric.advance(), metric.side_bearing())
                } else if let Some(metric) = long_metrics.last() {
                    let bearing = top_bearings
                        .get(raw_id.saturating_sub(long_count))
                        .map(|value| value.get())
                        .unwrap_or_else(|| metric.side_bearing());
                    (metric.advance(), bearing)
                } else {
                    continue;
                };
                project.vertical_metrics.insert(
                    name.clone(),
                    crate::font_data::VerticalMetrics {
                        advance_height: f64::from(advance),
                        top_side_bearing: f64::from(bearing),
                    },
                );
            }
        }
    }
    if let Some(kern) = face.tables().kern {
        for subtable in kern.subtables {
            if !subtable.horizontal || subtable.has_cross_stream || subtable.has_state_machine {
                continue;
            }
            for left in 0..face.number_of_glyphs() {
                for right in 0..face.number_of_glyphs() {
                    if let Some(value) = subtable
                        .glyphs_kerning(ttf_parser::GlyphId(left), ttf_parser::GlyphId(right))
                    {
                        if value != 0 {
                            project.kerning.insert(
                                (names[left as usize].clone(), names[right as usize].clone()),
                                value as f64,
                            );
                        }
                    }
                }
            }
        }
    }
    // Modern OpenType fonts commonly store kerning in GPOS rather than the
    // legacy `kern` table. Import explicit PairPos format 1 pairs as well.
    if let Some(gpos) = face.tables().gpos {
        for feature in gpos.features.into_iter() {
            if feature.tag != ttf_parser::Tag::from_bytes(b"kern") {
                continue;
            }
            for lookup_index in feature.lookup_indices.into_iter() {
                let Some(lookup) = gpos.lookups.get(lookup_index) else {
                    continue;
                };
                for subtable in lookup
                    .subtables
                    .into_iter::<ttf_parser::gpos::PositioningSubtable>()
                {
                    let ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format1 { coverage, sets },
                    ) = subtable
                    else {
                        continue;
                    };
                    for left in 0..face.number_of_glyphs() {
                        let left_id = ttf_parser::GlyphId(left);
                        let Some(coverage_index) = coverage.get(left_id) else {
                            continue;
                        };
                        let Some(pair_set) = sets.get(coverage_index) else {
                            continue;
                        };
                        for right in 0..face.number_of_glyphs() {
                            let Some((first, _second)) = pair_set.get(ttf_parser::GlyphId(right))
                            else {
                                continue;
                            };
                            if first.x_advance != 0 {
                                project.kerning.insert(
                                    (names[left as usize].clone(), names[right as usize].clone()),
                                    first.x_advance as f64,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // Also expand GPOS PairPos format 2 (class-based kerning). This is the
    // compact form used by many production fonts and is not exposed by the
    // high-level ttf-parser GPOS API.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(gpos) = font.gpos() {
            if let (Ok(features), Ok(lookups)) = (gpos.feature_list(), gpos.lookup_list()) {
                for record in features.feature_records() {
                    if record.feature_tag() != read_fonts::types::Tag::new(b"kern") {
                        continue;
                    }
                    let Ok(feature) = record.feature(features.offset_data()) else {
                        continue;
                    };
                    for lookup_index in feature.lookup_list_indices() {
                        let Ok(lookup) = lookups.lookups().get(lookup_index.get() as usize) else {
                            continue;
                        };
                        let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                            continue;
                        };
                        for subtable in subtables.iter().flatten() {
                            let PairPos::Format2(pair) = subtable else {
                                continue;
                            };
                            let Ok(coverage) = pair.coverage() else {
                                continue;
                            };
                            let Ok(class_def1) = pair.class_def1() else {
                                continue;
                            };
                            let Ok(class_def2) = pair.class_def2() else {
                                continue;
                            };
                            let context = ValueContext::default();
                            for left in 0..face.number_of_glyphs() {
                                let left_id = read_fonts::types::GlyphId::new(left.into());
                                if coverage.get(left_id).is_none() {
                                    continue;
                                }
                                let class1 = class_def1.get(left_id);
                                for right in 0..face.number_of_glyphs() {
                                    let right_id = read_fonts::types::GlyphId::new(right.into());
                                    let class2 = class_def2.get(right_id);
                                    let Ok(values) = pair.values(class1, class2, &context) else {
                                        continue;
                                    };
                                    if values[0].x_advance != 0 {
                                        project.kerning.insert(
                                            (
                                                names[left as usize].clone(),
                                                names[right as usize].clone(),
                                            ),
                                            values[0].x_advance as f64,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axes: Vec<_> = fvar.axes.into_iter().collect();
        for axis in &axes {
            for (suffix, value) in [("min", axis.min_value), ("max", axis.max_value)] {
                let id = format!("{}-{}", axis.tag, suffix);
                if project.masters.iter().any(|master| master.id == id) {
                    continue;
                }
                for default_axis in &axes {
                    face.set_variation(default_axis.tag, default_axis.def_value);
                }
                face.set_variation(axis.tag, value);
                let mut master = FontMaster {
                    id: id.clone(),
                    name: id.clone(),
                    ..FontMaster::default()
                };
                for default_axis in &axes {
                    master
                        .axes
                        .insert(default_axis.tag.to_string(), default_axis.def_value as f64);
                }
                master.axes.insert(axis.tag.to_string(), value as f64);
                project.masters.push(master);
                for raw_id in 0..face.number_of_glyphs() {
                    let glyph_id = ttf_parser::GlyphId(raw_id);
                    let Some(name) = project.glyph_order.get(raw_id as usize) else {
                        continue;
                    };
                    let Some(glyph) = project.glyphs.get_mut(name) else {
                        continue;
                    };
                    let mut collector = OutlineCollector::new();
                    face.outline_glyph(glyph_id, &mut collector);
                    if !collector.current.points.is_empty() {
                        collector.contours.push(collector.current);
                    }
                    glyph.layers.insert(
                        id.clone(),
                        GlyphLayer {
                            width: face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64,
                            contours: collector.contours,
                            components: Vec::new(),
                            anchors: Vec::new(),
                        },
                    );
                }
            }
        }
        if let Ok(font_ref) = FontRef::new(&bytes) {
            if let Ok(variable) = font_ref.fvar() {
                let names_table = font_ref.name().ok();
                let name_data = names_table.as_ref().map(|table| table.string_data());
                let instances = variable
                    .instances()
                    .ok()
                    .into_iter()
                    .flat_map(|items| items.iter().filter_map(Result::ok));
                for (instance_index, instance) in instances.enumerate() {
                    let instance_name = names_table
                        .as_ref()
                        .and_then(|table| {
                            table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == instance.subfamily_name_id)
                        })
                        .and_then(|record| {
                            name_data
                                .as_ref()
                                .and_then(|data| record.string(*data).ok())
                        })
                        .map(|name| name.chars().collect::<String>())
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| format!("Instance {}", instance_index + 1));
                    let mut instance_axes = std::collections::HashMap::new();
                    let mut instance_weight = 400.0;
                    let mut instance_width = 100.0;
                    for (axis, value) in axes.iter().zip(instance.coordinates.iter()) {
                        let value = value.get().to_f32() as f64;
                        let tag = axis.tag.to_string();
                        if tag.eq_ignore_ascii_case("wght") {
                            instance_weight = value;
                        } else if tag.eq_ignore_ascii_case("wdth") {
                            instance_width = value;
                        }
                        instance_axes.insert(tag, value);
                    }
                    project.instances.push(crate::font_data::FontInstance {
                        name: instance_name.clone(),
                        axes: instance_axes,
                        weight: instance_weight,
                        width: instance_width,
                    });
                    let id = format!("instance-{}", instance_index + 1);
                    if project.masters.iter().any(|master| master.id == id) {
                        continue;
                    }
                    for axis in &axes {
                        face.set_variation(axis.tag, axis.def_value);
                    }
                    let mut master = FontMaster {
                        id: id.clone(),
                        name: instance_name,
                        ..FontMaster::default()
                    };
                    for (axis, value) in axes.iter().zip(instance.coordinates.iter()) {
                        let value = value.get().to_f32();
                        face.set_variation(axis.tag, value);
                        master.axes.insert(axis.tag.to_string(), value as f64);
                    }
                    project.masters.push(master);
                    for raw_id in 0..face.number_of_glyphs() {
                        let glyph_id = ttf_parser::GlyphId(raw_id);
                        let Some(name) = project.glyph_order.get(raw_id as usize) else {
                            continue;
                        };
                        let Some(glyph) = project.glyphs.get_mut(name) else {
                            continue;
                        };
                        let mut collector = OutlineCollector::new();
                        face.outline_glyph(glyph_id, &mut collector);
                        if !collector.current.points.is_empty() {
                            collector.contours.push(collector.current);
                        }
                        glyph.layers.insert(
                            id.clone(),
                            GlyphLayer {
                                width: face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64,
                                contours: collector.contours,
                                components: Vec::new(),
                                anchors: Vec::new(),
                            },
                        );
                    }
                }
            }
        }
        for axis in &axes {
            face.set_variation(axis.tag, axis.def_value);
        }
    }
    project.normalize_glyph_order();
    project.normalize_masters();
    project.preserved_layout_source = Some(project.feature_source());
    project.preserved_layout_fingerprint = Some(crate::export::layout_input_fingerprint(&project));
    Ok(project)
}

/// Loads a WOFF 1.0 file by rebuilding its uncompressed SFNT payload.
pub fn load_woff(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("WOFF読み込みエラー: {e}"))?;
    if bytes.len() < 44 || &bytes[0..4] != b"wOFF" {
        return Err("WOFFヘッダーが不正です".into());
    }
    let read_u16 = |offset: usize| -> Result<u16, String> {
        let end = offset.checked_add(2).ok_or("WOFFヘッダーが不正です")?;
        bytes
            .get(offset..end)
            .and_then(|value| value.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or_else(|| "WOFFヘッダーが不正です".into())
    };
    let read_u32 = |offset: usize| -> Result<u32, String> {
        let end = offset.checked_add(4).ok_or("WOFFヘッダーが不正です")?;
        bytes
            .get(offset..end)
            .and_then(|value| value.try_into().ok())
            .map(u32::from_be_bytes)
            .ok_or_else(|| "WOFFヘッダーが不正です".into())
    };
    let flavor = &bytes[4..8];
    let table_count = read_u16(12)? as usize;
    let directory_end = 44usize
        .checked_add(
            table_count
                .checked_mul(20)
                .ok_or("WOFFテーブル数が不正です")?,
        )
        .ok_or("WOFFディレクトリが不正です")?;
    if directory_end > bytes.len() {
        return Err("WOFFテーブルディレクトリが不正です".into());
    }
    let sfnt_header_len = 12usize
        .checked_add(
            table_count
                .checked_mul(16)
                .ok_or("SFNTテーブル数が不正です")?,
        )
        .ok_or("SFNTヘッダーが不正です")?;
    let mut sfnt = Vec::with_capacity(read_u32(16)? as usize);
    sfnt.extend_from_slice(flavor);
    sfnt.extend_from_slice(&(table_count as u16).to_be_bytes());
    sfnt.extend_from_slice(&[0; 6]);
    let mut payloads = Vec::with_capacity(table_count);
    for index in 0..table_count {
        let base = 44 + index * 20;
        let tag = &bytes[base..base + 4];
        let offset = read_u32(base + 4)? as usize;
        let compressed_len = read_u32(base + 8)? as usize;
        let original_len = read_u32(base + 12)? as usize;
        let checksum = &bytes[base + 16..base + 20];
        let end = offset
            .checked_add(compressed_len)
            .ok_or("WOFFテーブル範囲が不正です")?;
        if end > bytes.len() {
            return Err("WOFFテーブル範囲が不正です".into());
        }
        let compressed = &bytes[offset..end];
        let data = if compressed_len < original_len {
            let mut decoder = flate2::read::ZlibDecoder::new(compressed);
            let mut data = Vec::with_capacity(original_len);
            decoder
                .read_to_end(&mut data)
                .map_err(|error| format!("WOFF圧縮データの展開に失敗しました: {error}"))?;
            data
        } else {
            compressed.to_vec()
        };
        if data.len() != original_len {
            return Err("WOFFテーブル長が不一致です".into());
        }
        payloads.push((tag.to_vec(), checksum.to_vec(), data));
    }
    let mut offset = sfnt_header_len;
    for (tag, checksum, data) in &payloads {
        sfnt.extend_from_slice(tag);
        sfnt.extend_from_slice(checksum);
        sfnt.extend_from_slice(&(offset as u32).to_be_bytes());
        sfnt.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset = offset
            .checked_add((data.len() + 3) & !3)
            .ok_or("SFNTサイズが大きすぎます")?;
    }
    for (_, _, data) in payloads {
        sfnt.extend_from_slice(&data);
        while sfnt.len() % 4 != 0 {
            sfnt.push(0);
        }
    }
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff-import-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&temp, sfnt).map_err(|error| error.to_string())?;
    let result = load_ttf(&temp);
    let _ = std::fs::remove_file(temp);
    result
}

/// Loads a WOFF2 file by decoding it to an SFNT font first.
pub fn load_woff2(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("WOFF2読み込みエラー: {e}"))?;
    let sfnt = oxifont_webfont::decode_woff2(&bytes)
        .map_err(|error| format!("WOFF2の展開に失敗しました: {error}"))?;
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff2-import-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&temp, sfnt).map_err(|error| error.to_string())?;
    let result = load_ttf(&temp);
    let _ = std::fs::remove_file(temp);
    result
}

/// Imports all SVG path elements as glyph contours.
pub fn load_svg(path: &Path) -> Result<FontProject, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("SVG読み込みエラー: {e}"))?;
    let mut path_data = Vec::new();
    for (offset, _) in source.match_indices("d=") {
        if offset > 0
            && source[..offset]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            continue;
        }
        let rest = &source[offset + 2..];
        let Some(quote) = rest.chars().next() else {
            continue;
        };
        if quote != '"' && quote != '\'' {
            continue;
        }
        if let Some((value, _)) = rest[quote.len_utf8()..].split_once(quote) {
            path_data.push(value);
        }
    }
    if path_data.is_empty() {
        return Err("SVGにパスがありません".into());
    }
    let d = path_data.join(" ");
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut current = (0.0, 0.0);
    let mut has_on_curve = 0;
    let mut last_cubic_control = None;
    let mut last_quad_control = None;
    let flush = |points: &mut Vec<crate::font_data::ContourPoint>,
                 contours: &mut Vec<crate::font_data::Contour>,
                 has_on_curve: &mut usize| {
        if *has_on_curve >= 3 {
            contours.push(crate::font_data::Contour {
                points: std::mem::take(points),
            });
        } else {
            points.clear();
        }
        *has_on_curve = 0;
    };
    for segment in svgtypes::PathParser::from(d.as_str()) {
        let segment = segment.map_err(|error| format!("SVGパス解析エラー: {error}"))?;
        match segment {
            svgtypes::PathSegment::MoveTo { abs, x, y } => {
                flush(&mut points, &mut contours, &mut has_on_curve);
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::LineTo { abs, x, y } => {
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::HorizontalLineTo { abs, x } => {
                current.0 = if abs { x } else { current.0 + x };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::VerticalLineTo { abs, y } => {
                current.1 = if abs { y } else { current.1 + y };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let control = if abs {
                    (x1, y1)
                } else {
                    (current.0 + x1, current.1 + y1)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(
                    control.0, control.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_quad_control = Some(control);
                last_cubic_control = None;
            }
            svgtypes::PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let first = if abs {
                    (x1, y1)
                } else {
                    (current.0 + x1, current.1 + y1)
                };
                let second = if abs {
                    (x2, y2)
                } else {
                    (current.0 + x2, current.1 + y2)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(first.0, first.1));
                points.push(crate::font_data::ContourPoint::off_curve(
                    second.0, second.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = Some(second);
                last_quad_control = None;
            }
            svgtypes::PathSegment::SmoothQuadratic { abs, x, y } => {
                let control = last_quad_control
                    .map(|(cx, cy)| (2.0 * current.0 - cx, 2.0 * current.1 - cy))
                    .unwrap_or(current);
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(
                    control.0, control.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_quad_control = Some(control);
                last_cubic_control = None;
            }
            svgtypes::PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let first = last_cubic_control
                    .map(|(cx, cy)| (2.0 * current.0 - cx, 2.0 * current.1 - cy))
                    .unwrap_or(current);
                let second = if abs {
                    (x2, y2)
                } else {
                    (current.0 + x2, current.1 + y2)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(first.0, first.1));
                points.push(crate::font_data::ContourPoint::off_curve(
                    second.0, second.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = Some(second);
                last_quad_control = None;
            }
            svgtypes::PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let endpoint = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                let svg_arc = SvgArc {
                    from: Point::new(current.0, current.1),
                    to: Point::new(endpoint.0, endpoint.1),
                    radii: Vec2::new(rx, ry),
                    x_rotation: x_axis_rotation.to_radians(),
                    large_arc,
                    sweep,
                };
                if let Some(arc) = Arc::from_svg_arc(&svg_arc) {
                    arc.to_cubic_beziers(0.1, |first, second, end| {
                        points.push(crate::font_data::ContourPoint::off_curve(first.x, first.y));
                        points.push(crate::font_data::ContourPoint::off_curve(
                            second.x, second.y,
                        ));
                        points.push(crate::font_data::ContourPoint::on_curve(end.x, end.y));
                        has_on_curve += 1;
                    });
                } else {
                    points.push(crate::font_data::ContourPoint::on_curve(
                        endpoint.0, endpoint.1,
                    ));
                    has_on_curve += 1;
                }
                current = endpoint;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::ClosePath { .. } => {
                flush(&mut points, &mut contours, &mut has_on_curve);
                last_cubic_control = None;
                last_quad_control = None;
            }
        }
    }
    flush(&mut points, &mut contours, &mut has_on_curve);
    if contours.is_empty() {
        return Err("有効なSVG輪郭がありません".into());
    }
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("imported")
        .to_string();
    let mut project = FontProject::new();
    let mut glyph = crate::font_data::GlyphData::new(name.clone(), None);
    glyph.contours = contours;
    glyph.width = project.metadata.units_per_em;
    project.glyphs.insert(name.clone(), glyph);
    project.glyph_order = vec![name];
    Ok(project)
}

pub fn save_ufo(project: &FontProject, path: &Path) -> Result<(), String> {
    let font = project.to_norad()?;
    font.save(path).map_err(|e| format!("UFO保存エラー: {}", e))
}

pub fn load_ufo(path: &Path) -> Result<FontProject, String> {
    let font = norad::Font::load(path).map_err(|e| format!("UFO読み込みエラー: {}", e))?;
    let mut project = FontProject::new();
    project.opentype_features = font.features.clone();
    if let Some(plist::Value::String(serialized)) =
        font.lib.get("com.glyph-studio.unicodeVariationSequences")
    {
        if let Ok(sequences) = serde_json::from_str(serialized) {
            project.unicode_variation_sequences = sequences;
        }
    }
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.axisMappings") {
        if let Ok(mappings) = serde_json::from_str(serialized) {
            project.axis_mappings = mappings;
        }
    }
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.preservedTables")
    {
        if let Ok(tables) = serde_json::from_str(serialized) {
            project.preserved_tables = tables;
        }
    }
    if let Some(plist::Value::String(source)) =
        font.lib.get("com.glyph-studio.preservedLayoutSource")
    {
        project.preserved_layout_source = Some(source.clone());
    }
    if let Some(plist::Value::String(fingerprint)) =
        font.lib.get("com.glyph-studio.preservedLayoutFingerprint")
    {
        project.preserved_layout_fingerprint = fingerprint.parse().ok();
    }
    if let Some(plist::Value::String(classes)) = font.lib.get("com.glyph-studio.opentypeClasses") {
        project.opentype_classes = classes.clone();
        if !project.opentype_classes.trim().is_empty() {
            let class_source = project.opentype_classes.clone();
            project.opentype_features = project
                .opentype_features
                .strip_prefix(&class_source)
                .unwrap_or(&project.opentype_features)
                .trim()
                .to_string();
        }
    }
    if let Some(plist::Value::String(serialized)) =
        font.lib.get("com.glyph-studio.conditionalLayers")
    {
        if let Ok(layers) = serde_json::from_str(serialized) {
            project.conditional_layers = layers;
        }
    }
    if let Some(plist::Value::Dictionary(axis_names)) = font.lib.get("com.glyph-studio.axisNames") {
        for (tag, name) in axis_names {
            if let Some(name) = name.as_string() {
                project.axis_names.insert(tag.clone(), name.to_string());
            }
        }
    }
    project.metadata.family_name = font
        .font_info
        .family_name
        .clone()
        .unwrap_or(project.metadata.family_name);
    project.metadata.style_name = font
        .font_info
        .style_name
        .clone()
        .unwrap_or(project.metadata.style_name);
    project.metadata.copyright = font.font_info.copyright.clone().unwrap_or_default();
    project.metadata.designer = font
        .font_info
        .open_type_name_designer
        .clone()
        .unwrap_or_default();
    project.metadata.manufacturer = font
        .font_info
        .open_type_name_manufacturer
        .clone()
        .unwrap_or_default();
    project.metadata.x_height = font.font_info.x_height.unwrap_or(0.0);
    project.metadata.cap_height = font.font_info.cap_height.unwrap_or(0.0);
    project.metadata.italic_angle = font.font_info.italic_angle.unwrap_or(0.0);
    project.metadata.underline_position = font
        .font_info
        .postscript_underline_position
        .unwrap_or(-100.0);
    project.metadata.underline_thickness = font
        .font_info
        .postscript_underline_thickness
        .unwrap_or(50.0);
    project.metadata.is_fixed_pitch = font.font_info.postscript_is_fixed_pitch.unwrap_or(false);
    if let Some(plist::Value::Dictionary(metadata)) = font.lib.get("com.glyph-studio.metadata") {
        if let Some(revision) = metadata.get("fontRevision").and_then(plist::Value::as_real) {
            project.metadata.font_revision = revision;
        }
    }
    if let Some(version) = font.font_info.open_type_name_version.as_deref() {
        if let Some(revision) = version
            .strip_prefix("Version ")
            .and_then(|value| value.trim().parse::<f64>().ok())
        {
            project.metadata.font_revision = revision;
        }
    }
    if let Some(plist::Value::Array(guidelines)) = font.lib.get("com.glyph-studio.guidelines") {
        project.guidelines = guidelines
            .iter()
            .filter_map(|value| {
                let plist::Value::Dictionary(info) = value else {
                    return None;
                };
                Some(crate::font_data::Guideline {
                    x: info.get("x").and_then(plist::Value::as_real)?,
                    y: info.get("y").and_then(plist::Value::as_real)?,
                    angle: info
                        .get("angle")
                        .and_then(plist::Value::as_real)
                        .unwrap_or(0.0),
                    name: info
                        .get("name")
                        .and_then(plist::Value::as_string)
                        .unwrap_or_default()
                        .to_string(),
                })
            })
            .collect();
    }
    if let Some(guidelines) = &font.font_info.guidelines {
        project.guidelines = guidelines
            .iter()
            .map(|guide| {
                let (x, y, angle) = match guide.line {
                    norad::Line::Horizontal(y) => (0.0, y, 0.0),
                    norad::Line::Vertical(x) => (x, 0.0, 90.0),
                    norad::Line::Angle { x, y, degrees } => (x, y, degrees),
                };
                crate::font_data::Guideline {
                    x,
                    y,
                    angle,
                    name: guide
                        .name
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                }
            })
            .collect();
    }
    if let Some(plist::Value::Dictionary(by_master)) =
        font.lib.get("com.glyph-studio.guidelinesByMaster")
    {
        for (master_id, value) in by_master {
            let Some(values) = value.as_array() else {
                continue;
            };
            let guides = values
                .iter()
                .filter_map(|value| {
                    let info = value.as_dictionary()?;
                    Some(crate::font_data::Guideline {
                        x: info.get("x").and_then(plist::Value::as_real)?,
                        y: info.get("y").and_then(plist::Value::as_real)?,
                        angle: info
                            .get("angle")
                            .and_then(plist::Value::as_real)
                            .unwrap_or(0.0),
                        name: info
                            .get("name")
                            .and_then(plist::Value::as_string)
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect();
            project
                .guidelines_by_master
                .insert(master_id.clone(), guides);
        }
    }
    if let Some(plist::Value::Dictionary(master_info)) = font.lib.get("com.glyph-studio.masters") {
        project.masters.clear();
        for (name, value) in master_info {
            let plist::Value::Dictionary(info) = value else {
                continue;
            };
            let id = info
                .get("id")
                .and_then(plist::Value::as_string)
                .unwrap_or(name)
                .to_string();
            let weight = info
                .get("weight")
                .and_then(plist::Value::as_real)
                .unwrap_or(400.0);
            let width = info
                .get("width")
                .and_then(plist::Value::as_real)
                .unwrap_or(100.0);
            let is_bracket = info
                .get("bracket")
                .and_then(plist::Value::as_boolean)
                .unwrap_or(false);
            let axes = info
                .get("axes")
                .and_then(plist::Value::as_dictionary)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|(tag, value)| Some((tag.clone(), value.as_real()?)))
                        .collect()
                })
                .unwrap_or_default();
            project.masters.push(FontMaster {
                id,
                name: name.clone(),
                weight,
                width,
                is_bracket,
                axes,
            });
        }
        if let Some(first) = project.masters.first() {
            project.default_master_id = first.id.clone();
        }
    }
    if let Some(plist::Value::Dictionary(vertical_metrics)) =
        font.lib.get("com.glyph-studio.verticalMetrics")
    {
        for (name, value) in vertical_metrics {
            let plist::Value::Dictionary(info) = value else {
                continue;
            };
            let Some(advance_height) = info.get("advanceHeight").and_then(plist::Value::as_real)
            else {
                continue;
            };
            let top_side_bearing = info
                .get("topSideBearing")
                .and_then(plist::Value::as_real)
                .unwrap_or(0.0);
            project.vertical_metrics.insert(
                name.clone(),
                crate::font_data::VerticalMetrics {
                    advance_height,
                    top_side_bearing,
                },
            );
        }
    }
    if let Some(plist::Value::Dictionary(by_master)) =
        font.lib.get("com.glyph-studio.verticalMetricsByMaster")
    {
        for (master_id, values) in by_master {
            let plist::Value::Dictionary(values) = values else {
                continue;
            };
            let metrics = project
                .vertical_metrics_by_master
                .entry(master_id.clone())
                .or_default();
            for (name, value) in values {
                let plist::Value::Dictionary(info) = value else {
                    continue;
                };
                let Some(advance_height) =
                    info.get("advanceHeight").and_then(plist::Value::as_real)
                else {
                    continue;
                };
                metrics.insert(
                    name.clone(),
                    crate::font_data::VerticalMetrics {
                        advance_height,
                        top_side_bearing: info
                            .get("topSideBearing")
                            .and_then(plist::Value::as_real)
                            .unwrap_or(0.0),
                    },
                );
            }
        }
    }
    if let Some(plist::Value::Dictionary(by_master)) =
        font.lib.get("com.glyph-studio.metricsByMaster")
    {
        for (master_id, value) in by_master {
            let Some(info) = value.as_dictionary() else {
                continue;
            };
            let Some(ascender) = info.get("ascender").and_then(plist::Value::as_real) else {
                continue;
            };
            let Some(descender) = info.get("descender").and_then(plist::Value::as_real) else {
                continue;
            };
            let line_gap = info
                .get("lineGap")
                .and_then(plist::Value::as_real)
                .unwrap_or(0.0);
            project.metrics_by_master.insert(
                master_id.clone(),
                crate::font_data::MasterMetrics {
                    ascender,
                    descender,
                    line_gap,
                },
            );
        }
    }
    if let Some(plist::Value::Dictionary(images)) =
        font.lib.get("com.glyph-studio.backgroundImages")
    {
        for (glyph, masters) in images {
            let plist::Value::Dictionary(masters) = masters else {
                continue;
            };
            let target = project.background_images.entry(glyph.clone()).or_default();
            for (master, path) in masters {
                if let Some(path) = path.as_string() {
                    target.insert(master.clone(), path.to_string());
                }
            }
        }
    }
    if let Some(plist::Value::Dictionary(opacities)) =
        font.lib.get("com.glyph-studio.backgroundOpacities")
    {
        for (glyph, masters) in opacities {
            let plist::Value::Dictionary(masters) = masters else {
                continue;
            };
            let target = project
                .background_opacities
                .entry(glyph.clone())
                .or_default();
            for (master, opacity) in masters {
                if let Some(opacity) = opacity.as_real() {
                    target.insert(master.clone(), opacity as f32);
                }
            }
        }
    }
    if let Some(plist::Value::Dictionary(transforms)) =
        font.lib.get("com.glyph-studio.backgroundTransforms")
    {
        for (glyph, masters) in transforms {
            let plist::Value::Dictionary(masters) = masters else {
                continue;
            };
            let target = project
                .background_transforms
                .entry(glyph.clone())
                .or_default();
            for (master, value) in masters {
                let plist::Value::Dictionary(value) = value else {
                    continue;
                };
                let Some(x) = value.get("x").and_then(plist::Value::as_real) else {
                    continue;
                };
                let Some(y) = value.get("y").and_then(plist::Value::as_real) else {
                    continue;
                };
                let scale = value
                    .get("scale")
                    .and_then(plist::Value::as_real)
                    .unwrap_or(1.0);
                target.insert(
                    master.clone(),
                    crate::font_data::BackgroundImageTransform {
                        x: x as f32,
                        y: y as f32,
                        scale: scale as f32,
                        rotation: value
                            .get("rotation")
                            .and_then(plist::Value::as_real)
                            .unwrap_or(0.0) as f32,
                        flip_x: value
                            .get("flipX")
                            .and_then(plist::Value::as_boolean)
                            .unwrap_or(false),
                        flip_y: value
                            .get("flipY")
                            .and_then(plist::Value::as_boolean)
                            .unwrap_or(false),
                    },
                );
            }
        }
    }
    if let Some(plist::Value::Array(palettes)) = font.lib.get("com.glyph-studio.colorPalettes") {
        project.color_palettes = palettes
            .iter()
            .filter_map(plist::Value::as_array)
            .map(|palette| {
                let channels: Vec<u8> = palette
                    .iter()
                    .filter_map(plist::Value::as_array)
                    .flatten()
                    .filter_map(|value| {
                        value
                            .as_unsigned_integer()
                            .or_else(|| {
                                value
                                    .as_signed_integer()
                                    .and_then(|v| u64::try_from(v).ok())
                            })
                            .and_then(|v| u8::try_from(v).ok())
                    })
                    .collect();
                channels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
                    .collect()
            })
            .collect();
    }
    if let Some(plist::Value::Array(names)) = font.lib.get("com.glyph-studio.colorPaletteNames") {
        project.color_palette_names = names
            .iter()
            .map(|value| value.as_string().unwrap_or_default().to_string())
            .collect();
    }
    if let Some(plist::Value::Array(types)) = font.lib.get("com.glyph-studio.colorPaletteTypes") {
        project.color_palette_types = types
            .iter()
            .filter_map(|value| {
                value
                    .as_unsigned_integer()
                    .or_else(|| {
                        value
                            .as_signed_integer()
                            .and_then(|v| u64::try_from(v).ok())
                    })
                    .and_then(|value| u32::try_from(value).ok())
            })
            .collect();
    }
    if let Some(plist::Value::Array(names)) =
        font.lib.get("com.glyph-studio.colorPaletteEntryNames")
    {
        project.color_palette_entry_names = names
            .iter()
            .map(|value| value.as_string().unwrap_or_default().to_string())
            .collect();
    }
    if let Some(plist::Value::Dictionary(color_layers)) =
        font.lib.get("com.glyph-studio.colorLayers")
    {
        for (name, values) in color_layers {
            let layers = values
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(plist::Value::as_dictionary)
                .filter_map(|info| {
                    Some(crate::font_data::ColorLayer {
                        glyph: info.get("glyph")?.as_string()?.to_string(),
                        palette_index: info
                            .get("paletteIndex")
                            .and_then(|value| {
                                value.as_unsigned_integer().or_else(|| {
                                    value
                                        .as_signed_integer()
                                        .and_then(|v| u64::try_from(v).ok())
                                })
                            })
                            .and_then(|value| u16::try_from(value).ok())?,
                        gradient: parse_color_gradient(info),
                        alpha: info
                            .get("alpha")
                            .and_then(plist::Value::as_real)
                            .unwrap_or(1.0),
                    })
                })
                .collect();
            project.color_layers.insert(name.clone(), layers);
        }
    }

    if let Some(plist::Value::Dictionary(color_layer_transforms)) =
        font.lib.get("com.glyph-studio.colorLayerTransforms")
    {
        for (name, values) in color_layer_transforms {
            let transforms = values
                .as_array()
                .into_iter()
                .flatten()
                .map(|value| {
                    let info = value.as_dictionary()?;
                    Some(crate::font_data::ColorLayerTransform {
                        xx: plist_number(info.get("xx")?)?,
                        yx: plist_number(info.get("yx")?)?,
                        xy: plist_number(info.get("xy")?)?,
                        yy: plist_number(info.get("yy")?)?,
                        dx: plist_number(info.get("dx")?)?,
                        dy: plist_number(info.get("dy")?)?,
                    })
                })
                .collect();
            project
                .color_layer_transforms
                .insert(name.clone(), transforms);
        }
    }

    let info = &font.font_info;
    if let Some(ref family) = info.family_name {
        project.metadata.family_name = family.clone();
    }
    if let Some(ref style) = info.style_name {
        project.metadata.style_name = style.clone();
    }
    if let Some(upm) = info.units_per_em {
        project.metadata.units_per_em = *upm;
    }
    if let Some(asc) = info.ascender {
        project.metadata.ascender = asc;
    }
    if let Some(desc) = &info.descender {
        project.metadata.descender = *desc;
    }

    let default_layer_name = font.default_layer().name().to_string();
    for layer in font.iter_layers().skip(1) {
        if !project.masters.iter().any(|master| {
            master.id == layer.name().to_string() || master.name == layer.name().to_string()
        }) {
            project.masters.push(FontMaster {
                id: layer.name().to_string(),
                name: layer.name().to_string(),
                ..FontMaster::default()
            });
        }
    }
    for layer in font.iter_layers() {
        let is_default = layer.name().to_string() == default_layer_name;
        let master_id = layer.name().to_string();
        for glyph in layer.iter() {
            let name = glyph.name().to_string();
            let unicode = glyph.codepoints.iter().next().map(|c| c as u32);
            let unicode = if unicode == Some(0) { None } else { unicode };

            let mut glyph_data = crate::font_data::GlyphData::new(name.clone(), unicode);
            glyph_data.unicodes = glyph.codepoints.iter().map(|c| c as u32).collect();
            glyph_data.unicodes.retain(|code| Some(*code) != unicode);
            glyph_data.width = glyph.width;
            glyph_data.anchors = glyph
                .anchors
                .iter()
                .filter_map(|anchor| {
                    Some(crate::font_data::GlyphAnchor {
                        name: anchor.name.as_ref()?.to_string(),
                        x: anchor.x,
                        y: anchor.y,
                    })
                })
                .collect();
            glyph_data.left_kerning_group = glyph
                .lib
                .get("com.glyph-studio.leftKerningGroup")
                .and_then(plist::Value::as_string)
                .unwrap_or_default()
                .to_string();
            glyph_data.right_kerning_group = glyph
                .lib
                .get("com.glyph-studio.rightKerningGroup")
                .and_then(plist::Value::as_string)
                .unwrap_or_default()
                .to_string();
            glyph_data.left_metrics_key = glyph
                .lib
                .get("com.glyph-studio.leftMetricsKey")
                .and_then(plist::Value::as_string)
                .unwrap_or_default()
                .to_string();
            glyph_data.right_metrics_key = glyph
                .lib
                .get("com.glyph-studio.rightMetricsKey")
                .and_then(plist::Value::as_string)
                .unwrap_or_default()
                .to_string();
            if let Some(plist::Value::Dictionary(master_guidelines)) =
                glyph.lib.get("com.glyph-studio.masterGuidelines")
            {
                glyph_data.master_guidelines = master_guidelines
                    .iter()
                    .filter_map(|(master_id, value)| {
                        let plist::Value::Array(guidelines) = value else {
                            return None;
                        };
                        let guides = guidelines
                            .iter()
                            .filter_map(|value| {
                                let plist::Value::Dictionary(info) = value else {
                                    return None;
                                };
                                Some(crate::font_data::Guideline {
                                    x: info.get("x").and_then(plist::Value::as_real)?,
                                    y: info.get("y").and_then(plist::Value::as_real)?,
                                    angle: info
                                        .get("angle")
                                        .and_then(plist::Value::as_real)
                                        .unwrap_or(0.0),
                                    name: info
                                        .get("name")
                                        .and_then(plist::Value::as_string)
                                        .unwrap_or_default()
                                        .to_string(),
                                })
                            })
                            .collect::<Vec<_>>();
                        Some((master_id.clone(), guides))
                    })
                    .collect();
            }
            if is_default {
                glyph_data.guidelines = glyph
                    .guidelines
                    .iter()
                    .map(|guide| {
                        let (x, y, angle) = match guide.line {
                            norad::Line::Horizontal(y) => (0.0, y, 0.0),
                            norad::Line::Vertical(x) => (x, 0.0, 90.0),
                            norad::Line::Angle { x, y, degrees } => (x, y, degrees),
                        };
                        crate::font_data::Guideline {
                            x,
                            y,
                            angle,
                            name: guide
                                .name
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_default(),
                        }
                    })
                    .collect();
            }

            for contour in &glyph.contours {
                let mut contour_data = crate::font_data::Contour::new();
                for point in &contour.points {
                    let point_type = match point.typ {
                        norad::PointType::Move
                        | norad::PointType::Line
                        | norad::PointType::Curve
                        | norad::PointType::QCurve => crate::font_data::PointType::OnCurve,
                        norad::PointType::OffCurve => crate::font_data::PointType::OffCurve,
                    };
                    contour_data.points.push(crate::font_data::ContourPoint {
                        x: point.x,
                        y: point.y,
                        point_type,
                        smooth: point.smooth,
                    });
                }
                glyph_data.contours.push(contour_data);
            }
            for component in &glyph.components {
                glyph_data
                    .components
                    .push(crate::font_data::GlyphComponent {
                        base: component.base.to_string(),
                        x_scale: component.transform.x_scale,
                        xy_scale: component.transform.xy_scale,
                        yx_scale: component.transform.yx_scale,
                        y_scale: component.transform.y_scale,
                        x_offset: component.transform.x_offset,
                        y_offset: component.transform.y_offset,
                    });
            }

            if is_default {
                if let Some(existing) = project.glyphs.remove(&name) {
                    glyph_data.layers = existing.layers;
                }
                project.glyphs.insert(name, glyph_data);
            } else {
                let layer = crate::font_data::GlyphLayer {
                    width: glyph_data.width,
                    contours: glyph_data.contours,
                    components: glyph_data.components,
                    anchors: glyph_data.anchors,
                };
                let existing = project
                    .glyphs
                    .entry(name.clone())
                    .or_insert_with(|| crate::font_data::GlyphData::new(name, unicode));
                existing.layers.insert(master_id.clone(), layer);
            }
        }
    }

    for (group_name, members) in &font.groups {
        let group_name = group_name.to_string();
        let (is_left, group) = if let Some(name) = group_name.strip_prefix("public.kern1.") {
            (true, name.to_string())
        } else if let Some(name) = group_name.strip_prefix("public.kern2.") {
            (false, name.to_string())
        } else {
            continue;
        };
        for member in members {
            if let Some(glyph) = project.glyphs.get_mut(member.as_str()) {
                if is_left {
                    glyph.left_kerning_group = group.clone();
                } else {
                    glyph.right_kerning_group = group.clone();
                }
            }
        }
    }

    for (left, pairs) in &font.kerning {
        for (right, value) in pairs {
            let left_names: Vec<String> = if let Some(members) = font.groups.get(left) {
                members.iter().map(ToString::to_string).collect()
            } else {
                vec![left.to_string()]
            };
            let right_names: Vec<String> = if let Some(members) = font.groups.get(right) {
                members.iter().map(ToString::to_string).collect()
            } else {
                vec![right.to_string()]
            };
            for expanded_left in &left_names {
                for expanded_right in &right_names {
                    if project.glyphs.contains_key(expanded_left)
                        && project.glyphs.contains_key(expanded_right)
                    {
                        project
                            .kerning
                            .insert((expanded_left.clone(), expanded_right.clone()), *value);
                    }
                }
            }
        }
    }

    if let Some(plist::Value::Dictionary(by_master)) =
        font.lib.get("com.glyph-studio.kerningByMaster")
    {
        for (master_id, value) in by_master {
            let Some(entries) = value.as_array() else {
                continue;
            };
            let pairs = entries
                .iter()
                .filter_map(|entry| {
                    let entry = entry.as_dictionary()?;
                    Some((
                        (
                            entry.get("left").and_then(plist_string)?,
                            entry.get("right").and_then(plist_string)?,
                        ),
                        entry.get("value").and_then(plist_number)?,
                    ))
                })
                .collect();
            project.kerning_by_master.insert(master_id.clone(), pairs);
        }
        if let Some(pairs) = project.kerning_by_master.get(&project.default_master_id) {
            project.kerning = pairs.clone();
        }
    }

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_data::{
        Contour, ContourPoint, FontMaster, GlyphComponent, GlyphData, GlyphLayer,
    };

    #[test]
    fn project_json_round_trip_preserves_font_data() {
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.json", std::process::id()));
        let mut project = FontProject::new();
        project.metadata.x_height = 500.0;
        project.metadata.cap_height = 700.0;
        project.metadata.family_name = "Round Trip".into();
        project.guidelines.push(crate::font_data::Guideline {
            x: 10.0,
            y: 700.0,
            angle: 15.0,
            name: "global".into(),
        });
        let mut glyph = GlyphData::new("A".into(), Some(65));
        glyph.guidelines.push(crate::font_data::Guideline {
            x: 20.0,
            y: 300.0,
            angle: 90.0,
            name: "glyph".into(),
        });
        glyph.unicodes = vec![0xFF21];
        glyph.left_kerning_group = "A-group".into();
        glyph.right_kerning_group = "V-group".into();
        glyph.left_kerning_group = "A-group".into();
        glyph.right_kerning_group = "V-group".into();
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        glyph.components.push(GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        project.glyphs.insert("A".into(), glyph);
        project.glyph_order.push("A".into());
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                alpha: 1.0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::default(),
                    x0: 0.0,
                    y0: 0.0,
                    x1: 1000.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 1000.0,
                    stops: Vec::new(),
                    radius0: 0.0,
                    radius1: 500.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
            }],
        );
        project.normalize_masters();
        save_project(&project, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded, project);
        assert_eq!(loaded.metadata.x_height, 500.0);
        assert_eq!(loaded.metadata.cap_height, 700.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn legacy_metadata_defaults_are_restored() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-legacy-{}.json", std::process::id()));
        std::fs::write(&path, r#"{
            "metadata":{"family_name":"Legacy","style_name":"Regular","units_per_em":1000.0,"ascender":800.0,"descender":-200.0,"line_gap":0.0},
            "glyphs":{},"kerning":{}
        }"#).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.metadata.weight_class, 400);
        assert_eq!(loaded.metadata.width_class, 5);
        assert_eq!(loaded.metadata.x_height, 0.0);
        assert_eq!(loaded.metadata.cap_height, 0.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn variable_font_round_trip_restores_named_instances() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-variable-import-{}.ttf",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), Some(66));
        project.add_glyph("A.alt".into(), None);
        project.opentype_features =
            "feature calt { sub A' B by A.alt; } calt; feature ccmp { pos A' B <0 0 -40 0>; } ccmp; feature dist { pos A B' A <0 0 -30 0>; } dist;"
                .into();
        project
            .axis_names
            .insert("opsz".into(), "Optical Size Custom".into());
        project.axis_flags.insert("opsz".into(), 1);
        project.metadata.head_mac_style = 3;
        project.metadata.panose = [2, 11, 6, 3, 5, 4, 2, 2, 2, 4];
        project.metadata.default_char = 0x25A1;
        project.metadata.break_char = 0x20;
        project.metadata.max_context = 7;
        project.metadata.caret_slope_rise = 2;
        project.metadata.caret_slope_run = -1;
        project.metadata.caret_offset = 3;
        project.metadata.vertical_caret_slope_rise = 4;
        project.metadata.vertical_caret_slope_run = -2;
        project.metadata.vertical_caret_offset = 6;
        project.metadata.subscript_x_size = 300;
        project.metadata.subscript_y_size = 280;
        project.metadata.subscript_x_offset = 12;
        project.metadata.subscript_y_offset = -18;
        project.metadata.superscript_x_size = 310;
        project.metadata.superscript_y_size = 290;
        project.metadata.superscript_x_offset = 14;
        project.metadata.superscript_y_offset = 420;
        project.metadata.strikeout_size = 35;
        project.metadata.strikeout_position = 310;
        project.metadata.family_class = 0x1234;
        project.metadata.lower_optical_point_size = 9;
        project.metadata.upper_optical_point_size = 72;
        project.metadata.win_ascent = 1200;
        project.metadata.win_descent = 350;
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        project.masters[0].axes.insert("opsz".into(), 10.0);
        project.masters[1].axes.insert("opsz".into(), 14.0);
        let regular = GlyphLayer {
            width: 600.0,
            contours: vec![Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let mut bold = regular.clone();
        bold.contours[0].points[1].x = 160.0;
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.layers.insert("regular".into(), regular);
        glyph.layers.insert("bold".into(), bold);
        project.instances.push(crate::font_data::FontInstance {
            name: "Text Medium".into(),
            axes: [("opsz".into(), 12.0)].into_iter().collect(),
            weight: 550.0,
            width: 100.0,
        });
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("GlyphClassDef"));
        assert!(loaded.opentype_features.contains("feature calt"));
        assert!(loaded.opentype_features.contains("A'"));
        assert!(loaded.opentype_features.contains("GSCtx"));
        assert!(loaded.opentype_features.contains("feature ccmp"));
        assert!(loaded.opentype_features.contains("GPContext"));
        assert!(loaded.opentype_features.contains("feature dist"));
        assert!(loaded.masters.iter().any(|master| master.name == "Regular"));
        assert_eq!(
            loaded.axis_names.get("opsz"),
            Some(&"Optical Size Custom".to_string())
        );
        assert_eq!(loaded.axis_flags.get("opsz"), Some(&1));
        assert_eq!(loaded.metadata.head_mac_style, 3);
        assert_eq!(loaded.metadata.panose, [2, 11, 6, 3, 5, 4, 2, 2, 2, 4]);
        assert_eq!(loaded.metadata.default_char, 0x25A1);
        assert_eq!(loaded.metadata.break_char, 0x20);
        assert_eq!(loaded.metadata.max_context, 7);
        assert_eq!(loaded.metadata.caret_slope_rise, 2);
        assert_eq!(loaded.metadata.caret_slope_run, -1);
        assert_eq!(loaded.metadata.caret_offset, 3);
        assert_eq!(loaded.metadata.vertical_caret_slope_rise, 4);
        assert_eq!(loaded.metadata.vertical_caret_slope_run, -2);
        assert_eq!(loaded.metadata.vertical_caret_offset, 6);
        assert_eq!(loaded.metadata.subscript_x_size, 300);
        assert_eq!(loaded.metadata.subscript_y_size, 280);
        assert_eq!(loaded.metadata.subscript_x_offset, 12);
        assert_eq!(loaded.metadata.subscript_y_offset, -18);
        assert_eq!(loaded.metadata.superscript_x_size, 310);
        assert_eq!(loaded.metadata.superscript_y_size, 290);
        assert_eq!(loaded.metadata.superscript_x_offset, 14);
        assert_eq!(loaded.metadata.superscript_y_offset, 420);
        assert_eq!(loaded.metadata.strikeout_size, 35);
        assert_eq!(loaded.metadata.strikeout_position, 310);
        assert_eq!(loaded.metadata.family_class, 0x1234);
        assert_eq!(loaded.metadata.lower_optical_point_size, 9);
        assert_eq!(loaded.metadata.upper_optical_point_size, 72);
        assert_eq!(loaded.metadata.win_ascent, 1200);
        assert_eq!(loaded.metadata.win_descent, 350);
        assert!(loaded
            .instances
            .iter()
            .any(|instance| instance.name == "Text Medium"));
        assert!(loaded.instances.iter().any(|instance| {
            (instance.axes.get("opsz").copied().unwrap_or_default() - 12.0).abs() < f64::EPSILON
        }));
        assert!(loaded
            .glyphs
            .get("A")
            .is_some_and(|glyph| glyph.layers.keys().any(|id| id.starts_with("instance-"))));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_round_trip_restores_gdef_attach_points() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-gdef-attach-import-{}.ttf",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.opentype_features = "table GDEF { Attach A 7 2 7; } GDEF;".to_string();
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("Attach A 2 7"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_round_trip_restores_gdef_ligature_carets() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-gdef-caret-import-{}.ttf",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("f_i".into(), None);
        project.opentype_features =
            "table GDEF { LigatureCaretByPos f_i 300 600; } GDEF;".to_string();
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded
            .opentype_features
            .contains("LigatureCaretByPos f_i 300"));
        assert!(loaded
            .opentype_features
            .contains("LigatureCaretByPos f_i 600"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_round_trip_restores_mark_filtering_sets() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mark-filtering-import-{}.ttf",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), Some(0x00B4));
        project.add_glyph("grave".into(), Some(0x0060));
        project.opentype_features = "@Marks = [acute grave]; feature mark { lookupflag UseMarkFilteringSet @Marks; pos acute <0 0 10 0>; } mark;".into();
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("@GSMarkSet0"));
        assert!(loaded
            .opentype_features
            .contains("UseMarkFilteringSet @GSMarkSet0"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ufo_round_trip_preserves_named_master_layers() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-layers-{}.ufo", std::process::id()));
        let mut project = FontProject::new();
        project.metadata.font_revision = 3.125;
        project.metadata.x_height = 510.0;
        project.metadata.cap_height = 720.0;
        project.metadata.italic_angle = -9.0;
        project.metadata.underline_position = -110.0;
        project.metadata.underline_thickness = 45.0;
        project.opentype_features = "feature calt { sub A' by A.alt; } calt;".into();
        project
            .preserved_tables
            .insert("GSUB".into(), vec![1, 2, 3]);
        project.preserved_layout_source = Some(project.feature_source());
        project.preserved_layout_fingerprint =
            Some(crate::export::layout_input_fingerprint(&project));
        project.vertical_metrics.insert(
            "A".into(),
            crate::font_data::VerticalMetrics {
                advance_height: 1200.0,
                top_side_bearing: 640.0,
            },
        );
        project.metadata.copyright = "Copyright 2026".into();
        project.metadata.designer = "Designer".into();
        project.metadata.manufacturer = "Manufacturer".into();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "bold".into(),
            weight: 700.0,
            width: 112.0,
            is_bracket: true,
            axes: std::collections::HashMap::new(),
        });
        project
            .masters
            .last_mut()
            .unwrap()
            .axes
            .insert("wght".into(), 700.0);
        let regular = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let bold = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(140.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), Some(65));
        glyph.guidelines.push(crate::font_data::Guideline {
            x: 30.0,
            y: 400.0,
            angle: 22.0,
            name: "glyph guide".into(),
        });
        glyph.master_guidelines.insert(
            "bold".into(),
            vec![crate::font_data::Guideline {
                x: 45.0,
                y: 500.0,
                angle: 90.0,
                name: "bold guide".into(),
            }],
        );
        glyph.unicodes = vec![0xFF21];
        glyph.left_kerning_group = "A-group".into();
        glyph.right_kerning_group = "V-group".into();
        glyph.anchors.push(crate::font_data::GlyphAnchor {
            name: "top".into(),
            x: 250.0,
            y: 700.0,
        });
        glyph.contours.push(regular.clone());
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![regular],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 700.0,
                contours: vec![bold],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project.conditional_layers.insert(
            "A".into(),
            vec![crate::font_data::ConditionalLayer {
                id: "bold-bracket".into(),
                conditions: std::collections::HashMap::from([(
                    "wght".into(),
                    crate::font_data::AxisRange {
                        min: Some(700.0),
                        max: None,
                    },
                )]),
                layer: project.glyphs["A"].layers["bold"].clone(),
            }],
        );
        project
            .set_vertical_metrics_for_master("A", "bold", 1300.0, 600.0)
            .unwrap();
        project
            .set_master_metrics(
                "bold",
                crate::font_data::MasterMetrics {
                    ascender: 920.0,
                    descender: -260.0,
                    line_gap: 40.0,
                },
            )
            .unwrap();
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_palette_names = vec!["Light".into()];
        project.color_palette_types = vec![1];
        project.color_palette_entry_names = vec!["Fill".into(), "Outline".into()];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                alpha: 1.0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::default(),
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: vec![
                        crate::font_data::ColorGradientStop {
                            offset: 0.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 0.5,
                            palette_index: 1,
                            alpha: 0.6,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 1.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                    ],
                    radius0: 0.0,
                    radius1: 100.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
            }],
        );
        project.opentype_classes = "@Upper = [A A.red];".into();
        project.opentype_features = "languagesystem latn dflt;\nlookup L { sub A by A.alt; } L;\nfeature liga { lookup L; } liga;\nfeature mark { pos base A <anchor 300 700> mark @top <anchor 300 0> mark @bottom; } mark;".into();
        project
            .unicode_variation_sequences
            .push(crate::font_data::UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xE0100,
                glyph: "A.ivs".into(),
            });
        project
            .axis_names
            .insert("wght".into(), "Weight Japanese".into());
        project
            .background_images
            .entry("A".into())
            .or_default()
            .insert("bold".into(), "/tmp/reference.png".into());
        project
            .background_opacities
            .entry("A".into())
            .or_default()
            .insert("bold".into(), 0.42);
        project
            .background_transforms
            .entry("A".into())
            .or_default()
            .insert(
                "bold".into(),
                crate::font_data::BackgroundImageTransform {
                    x: 12.0,
                    y: -8.0,
                    scale: 0.75,
                    rotation: 12.0,
                    flip_x: true,
                    flip_y: true,
                },
            );
        let mut bold_only = GlyphData::new("boldOnly".into(), None);
        bold_only.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 400.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(50.0, 0.0),
                        ContourPoint::on_curve(0.0, 50.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("boldOnly".into(), bold_only);
        project.guidelines.push(crate::font_data::Guideline {
            x: 0.0,
            y: 700.0,
            angle: 12.5,
            name: "cap guide".into(),
        });
        project.kerning_by_master.insert(
            "bold".into(),
            std::collections::HashMap::from([(("A".into(), "A".into()), -90.0)]),
        );
        let norad = project.to_norad().unwrap();
        assert!(norad.groups.contains_key("public.kern1.A-group"));
        assert!(norad.groups.contains_key("public.kern2.V-group"));
        save_ufo(&project, &path).unwrap();
        let loaded = load_ufo(&path).unwrap();
        assert!((loaded.metadata.font_revision - 3.125).abs() < f64::EPSILON);
        assert_eq!(loaded.metadata.x_height, 510.0);
        assert_eq!(loaded.metadata.cap_height, 720.0);
        assert_eq!(loaded.metadata.italic_angle, -9.0);
        assert_eq!(loaded.metadata.underline_position, -110.0);
        assert_eq!(loaded.metadata.underline_thickness, 45.0);
        assert_eq!(loaded.color_palette_names, vec!["Light"]);
        assert_eq!(loaded.color_palette_types, vec![1]);
        assert_eq!(loaded.color_palette_entry_names, vec!["Fill", "Outline"]);
        assert_eq!(loaded.preserved_tables.get("GSUB"), Some(&vec![1, 2, 3]));
        assert_eq!(
            loaded.preserved_layout_source,
            project.preserved_layout_source
        );
        assert_eq!(
            loaded.preserved_layout_fingerprint,
            project.preserved_layout_fingerprint
        );
        assert_eq!(
            loaded.color_layers["A"][0]
                .gradient
                .as_ref()
                .unwrap()
                .stops
                .len(),
            3
        );
        assert_eq!(
            loaded.color_layers["A"][0].gradient.as_ref().unwrap().stops[1].alpha,
            0.6
        );
        assert_eq!(
            loaded.vertical_metrics.get("A").unwrap().advance_height,
            1200.0
        );
        assert_eq!(
            loaded.vertical_metrics.get("A").unwrap().top_side_bearing,
            640.0
        );
        assert_eq!(
            loaded.vertical_metrics_by_master["bold"]["A"].advance_height,
            1300.0
        );
        assert_eq!(loaded.master_metrics_for("bold").ascender, 920.0);
        assert_eq!(loaded.master_metrics_for("bold").descender, -260.0);
        assert_eq!(loaded.master_metrics_for("bold").line_gap, 40.0);
        assert_eq!(loaded.color_layers["A"][0].glyph, "A.red");
        assert_eq!(loaded.color_palettes[0][0], [255, 0, 0, 255]);
        assert_eq!(loaded.opentype_classes, "@Upper = [A A.red];");
        assert_eq!(loaded.opentype_features, project.opentype_features);
        assert_eq!(
            loaded.unicode_variation_sequences,
            project.unicode_variation_sequences
        );
        assert_eq!(loaded.axis_names["wght"], "Weight Japanese");
        assert_eq!(loaded.conditional_layers["A"][0].id, "bold-bracket");
        assert_eq!(
            loaded.conditional_layers["A"][0].conditions["wght"].min,
            Some(700.0)
        );
        assert_eq!(loaded.background_images["A"]["bold"], "/tmp/reference.png");
        assert!((loaded.background_opacities["A"]["bold"] - 0.42).abs() < f32::EPSILON);
        assert_eq!(
            loaded.background_transforms["A"]["bold"],
            crate::font_data::BackgroundImageTransform {
                x: 12.0,
                y: -8.0,
                scale: 0.75,
                rotation: 12.0,
                flip_x: true,
                flip_y: true,
            }
        );
        assert_eq!(loaded.metadata.copyright, "Copyright 2026");
        assert_eq!(loaded.metadata.designer, "Designer");
        assert_eq!(loaded.metadata.manufacturer, "Manufacturer");
        assert_eq!(loaded.guidelines, project.guidelines);
        assert_eq!(
            loaded.kerning_by_master["bold"][&("A".into(), "A".into())],
            -90.0
        );
        let loaded_glyph = loaded.glyphs.get("A").unwrap();
        assert_eq!(
            loaded_glyph.guidelines,
            project.glyphs.get("A").unwrap().guidelines
        );
        assert_eq!(
            loaded_glyph.master_guidelines,
            project.glyphs.get("A").unwrap().master_guidelines
        );
        assert_eq!(loaded_glyph.anchors[0].name, "top");
        assert_eq!(loaded_glyph.anchors[0].x, 250.0);
        assert_eq!(loaded_glyph.anchors[0].y, 700.0);
        let mut loaded_codepoints = loaded_glyph.unicodes.clone();
        loaded_codepoints.push(loaded_glyph.unicode.unwrap());
        loaded_codepoints.sort_unstable();
        assert_eq!(loaded_codepoints, vec![65, 0xFF21]);
        assert_eq!(loaded_glyph.left_kerning_group, "A-group");
        assert_eq!(loaded_glyph.right_kerning_group, "V-group");
        let loaded_bold = loaded_glyph.layers.get("bold").unwrap();
        assert_eq!(loaded_bold.width, 700.0);
        assert_eq!(loaded_bold.contours[0].points[1].x, 140.0);
        assert!(loaded_bold.anchors.is_empty());
        let loaded_master = loaded
            .masters
            .iter()
            .find(|master| master.id == "bold")
            .unwrap();
        assert_eq!(loaded_master.weight, 700.0);
        assert_eq!(loaded_master.width, 112.0);
        assert!(loaded_master.is_bracket);
        assert_eq!(loaded_master.axes.get("wght"), Some(&700.0));
        assert!(loaded
            .glyphs
            .get("boldOnly")
            .unwrap()
            .layers
            .contains_key("bold"));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn ttf_round_trip_imports_names_unicode_and_metrics() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-import-{}.ttf", std::process::id()));
        let mut project = FontProject::new();
        project.metadata.font_revision = 2.375;
        project.metadata.italic_angle = -12.5;
        project.metadata.is_fixed_pitch = true;
        project.vertical_metrics.insert(
            "A".into(),
            crate::font_data::VerticalMetrics {
                advance_height: 1200.0,
                top_side_bearing: 640.0,
            },
        );
        project.metadata.underline_position = -115.0;
        project.metadata.underline_thickness = 42.0;
        project.metadata.x_height = 480.0;
        project.metadata.cap_height = 710.0;
        project.metadata.copyright = "Copyright 2026 Glyph Studio".into();
        project.metadata.designer = "Typeface Team".into();
        project.metadata.manufacturer = "Studio Foundry".into();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 720.0;
        project.glyphs.insert("A".into(), glyph);
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 20.0,
                y_offset: 30.0,
            });
        project.kerning.insert(("A".into(), "base".into()), -80.0);
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert_eq!(loaded.metadata.family_name, "My Font");
        assert_eq!(loaded.metadata.style_name, "Regular");
        assert_eq!(loaded.metadata.copyright, "Copyright 2026 Glyph Studio");
        assert_eq!(loaded.metadata.designer, "Typeface Team");
        assert_eq!(loaded.metadata.manufacturer, "Studio Foundry");
        assert!((loaded.metadata.font_revision - 2.375).abs() < 0.001);
        assert!((loaded.metadata.italic_angle + 12.5).abs() < 0.01);
        assert!(loaded.metadata.is_fixed_pitch);
        assert_eq!(loaded.vertical_metrics["A"].advance_height, 1200.0);
        assert_eq!(loaded.vertical_metrics["A"].top_side_bearing, 640.0);
        assert_eq!(loaded.metadata.underline_position, -115.0);
        assert_eq!(loaded.metadata.underline_thickness, 42.0);
        assert_eq!(loaded.metadata.weight_class, 400);
        assert_eq!(loaded.metadata.width_class, 5);
        assert_eq!(loaded.metadata.x_height, 480.0);
        assert_eq!(loaded.metadata.cap_height, 710.0);
        let imported = loaded.glyphs.get("A").unwrap();
        assert_eq!(imported.unicode, Some('A' as u32));
        assert_eq!(imported.width, 720.0);
        assert!(!imported.contours.is_empty());
        assert_eq!(
            loaded.kerning.get(&("A".into(), "base".into())),
            Some(&-80.0)
        );
        let bytes = std::fs::read(&path).unwrap();
        let font = FontRef::new(&bytes).unwrap();
        let vhea = font.vhea().unwrap();
        assert_eq!(i16::from(vhea.ascender()), 800);
        assert_eq!(font.vmtx().unwrap().v_metrics().len(), 3);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn otf_round_trip_imports_cff_outlines_and_metrics() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-import-{}.otf", std::process::id()));
        let mut project = FontProject::new();
        project.metadata.family_name = "CFF Round Trip".into();
        project.metadata.style_name = "Book".into();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 680.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(50.0, 0.0),
                ContourPoint::on_curve(300.0, 700.0),
                ContourPoint::on_curve(550.0, 0.0),
            ],
        });
        project.glyphs.insert("A".into(), glyph);

        crate::export::export_otf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert_eq!(loaded.metadata.family_name, "CFF Round Trip");
        assert_eq!(loaded.metadata.style_name, "Book");
        let imported = loaded.glyphs.get("A").unwrap();
        assert_eq!(imported.unicode, Some('A' as u32));
        assert_eq!(imported.width, 680.0);
        assert_eq!(imported.contours.len(), 1);
        assert_eq!(imported.contours[0].points.len(), 3);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn woff_round_trip_imports_compressed_tables() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-import-{}.woff", std::process::id()));
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 640.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(40.0, 0.0),
                ContourPoint::on_curve(320.0, 700.0),
                ContourPoint::on_curve(600.0, 0.0),
            ],
        });
        project.glyphs.insert("A".into(), glyph);
        crate::export::export_woff(&project, &path).unwrap();
        let loaded = load_woff(&path).unwrap();
        assert_eq!(loaded.glyphs["A"].unicode, Some('A' as u32));
        assert_eq!(loaded.glyphs["A"].width, 640.0);
        assert_eq!(loaded.glyphs["A"].contours.len(), 1);
        assert!(loaded.glyphs["A"].contours[0].points.len() >= 3);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn woff2_round_trip_imports_compressed_tables() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-import-{}.woff2", std::process::id()));
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 640.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(40.0, 0.0),
                ContourPoint::on_curve(320.0, 700.0),
                ContourPoint::on_curve(600.0, 0.0),
            ],
        });
        project.glyphs.insert("A".into(), glyph);
        crate::export::export_woff2(&project, &path).unwrap();
        let loaded = load_woff2(&path).unwrap();
        assert_eq!(loaded.glyphs["A"].unicode, Some('A' as u32));
        assert_eq!(loaded.glyphs["A"].width, 640.0);
        assert_eq!(loaded.glyphs["A"].contours.len(), 1);
        assert!(loaded.glyphs["A"].contours[0].points.len() >= 3);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn static_master_woff2_round_trip_imports_selected_layer() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-master-{}.woff2",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(crate::font_data::FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..Default::default()
        });
        let bold = project.glyphs.get_mut("A").unwrap();
        bold.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 720.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(60.0, 0.0),
                        ContourPoint::on_curve(360.0, 760.0),
                        ContourPoint::on_curve(660.0, 0.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        crate::export::export_woff2_for_master(&project, "bold", &path).unwrap();
        let loaded = load_woff2(&path).unwrap();
        assert_eq!(loaded.glyphs["A"].width, 720.0);
        assert_eq!(loaded.glyphs["A"].contours[0].points.len(), 3);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn variable_woff2_round_trip_preserves_variation_tables() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-variable-{}.woff2",
            std::process::id()
        ));
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.width = 640.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(40.0, 0.0),
                ContourPoint::on_curve(320.0, 700.0),
                ContourPoint::on_curve(600.0, 0.0),
            ],
        });
        glyph.layers.insert(
            second.id,
            GlyphLayer {
                width: 720.0,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        crate::export::export_woff2(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let sfnt = oxifont_webfont::decode_woff2(&bytes).unwrap();
        let face = ttf_parser::Face::parse(&sfnt, 0).unwrap();
        assert!(face.tables().fvar.is_some());
        assert!(face.tables().hvar.is_some());
        let loaded = load_woff2(&path).unwrap();
        assert_eq!(loaded.glyphs["A"].width, 640.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_import_converts_relative_lines_and_curves() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-{}-{:?}.svg",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            &path,
            r#"<svg><path d="M 10 20 l 90 0 q 20 40 40 0 c 10 -30 30 -30 40 0 z"/></svg>"#,
        )
        .unwrap();
        let project = load_svg(&path).unwrap();
        let glyph = project.glyphs.values().next().unwrap();
        assert_eq!(glyph.contours.len(), 1);
        assert!(glyph.contours[0].points.len() >= 5);
        assert!(glyph.contours[0]
            .points
            .iter()
            .any(|point| !point.is_on_curve()));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_import_accepts_single_quoted_path_attributes() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-import-{}.svg", std::process::id()));
        std::fs::write(&path, "<svg><path d='M0 0 L100 0 L50 100 Z'/></svg>").unwrap();
        let project = load_svg(&path).unwrap();
        assert_eq!(project.glyphs.values().next().unwrap().contours.len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_import_combines_multiple_path_elements() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-{}-{:?}.svg",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            &path,
            r#"<svg><path data-id="ignored" d="M0 0 L100 0 L50 100 Z"/><path d="M200 0 L300 0 L250 100 Z"/></svg>"#,
        )
        .unwrap();
        let project = load_svg(&path).unwrap();
        assert_eq!(project.glyphs.values().next().unwrap().contours.len(), 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_import_preserves_smooth_quadratic_and_cubic_controls() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-{}-{:?}.svg",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            &path,
            r#"<svg><path d="M0 0 Q25 100 50 0 T100 0 C110 50 140 50 150 0 S190 -50 200 0 Z"/></svg>"#,
        )
        .unwrap();
        let project = load_svg(&path).unwrap();
        let glyph = project.glyphs.values().next().unwrap();
        assert_eq!(glyph.contours.len(), 1);
        assert!(
            glyph.contours[0]
                .points
                .iter()
                .filter(|point| !point.is_on_curve())
                .count()
                >= 4
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_import_converts_elliptical_arcs_to_cubic_controls() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-{}-{:?}.svg",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            &path,
            r#"<svg><path d="M0 0 A50 50 0 0 1 100 0 L100 100 L0 100 Z"/></svg>"#,
        )
        .unwrap();
        let project = load_svg(&path).unwrap();
        let glyph = project.glyphs.values().next().unwrap();
        assert_eq!(glyph.contours.len(), 1);
        assert!(glyph.contours[0]
            .points
            .iter()
            .any(|point| !point.is_on_curve()));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn glyphs_import_reads_common_outline_component_and_kerning_data() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-import-{}-{:?}.glyphs",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            &path,
                r#"{
familyName = Test;
styleName = Regular;
copyright = "Copyright Test";
designer = Designer;
versionMajor = 1;
versionMinor = 25;
unitsPerEm = 1000;
axes = ({ tag = wght; name = Weight; });
fontMaster = ({ id = M1; name = Regular; weightValue = 400; axes = (400); isBracket = true; });
instances = ({ name = "Text Medium"; familyName = Test; weightValue = 550; widthValue = 100; axes = (550); });
classes = ({ name = Upper; code = "A B"; });
features = ({ tag = liga; code = "sub A B by A;"; });
glyphs = (
 { glyphname = A; unicode = 65; layers = ({ layerId = M1; width = 600; shapes = ({ nodes = ("0 0 LINE SMOOTH", "100 0 LINE", "100 100 LINE"); }); }); },
 { glyphname = Aacute; layers = ({ layerId = M1; shapes = ({ ref = A; transform = (1, 0, 0, 1, 20, 30); }); }); }
);
kerning = { M1 = { A = { V = -80; }; }; };
}"#,
        )
        .unwrap();
        let mut project = load_glyphs(&path).unwrap();
        assert_eq!(project.metadata.family_name, "Test");
        assert_eq!(project.metadata.style_name, "Regular");
        assert_eq!(project.metadata.font_revision, 1.25);
        assert_eq!(project.masters[0].axes.get("wght"), Some(&400.0));
        assert!(project.masters[0].is_bracket);
        assert_eq!(project.instances.len(), 1);
        assert_eq!(project.instances[0].name, "Text Medium");
        assert_eq!(project.instances[0].weight, 550.0);
        assert_eq!(project.axis_names.get("wght"), Some(&"Weight".to_string()));
        assert!(project.opentype_classes.contains("@Upper = [A B];"));
        assert!(project.opentype_features.contains("feature liga"));
        assert_eq!(project.glyphs["A"].unicode, Some(65));
        assert_eq!(project.glyphs["A"].layers["M1"].contours[0].points.len(), 3);
        assert!(project.glyphs["A"].layers["M1"].contours[0].points[0].smooth);
        let component = &project.glyphs["Aacute"].layers["M1"].components[0];
        assert_eq!(component.x_offset, 20.0);
        assert_eq!(component.y_offset, 30.0);
        assert_eq!(project.kerning.get(&("A".into(), "V".into())), Some(&-80.0));
        let roundtrip_path = path.with_file_name(format!(
            "glyph-studio-roundtrip-{}-{:?}.glyphs",
            std::process::id(),
            std::thread::current().id()
        ));
        project.opentype_features = "languagesystem latn dflt;\nlookup L { sub A by A.alt; } L;\nfeature liga { lookup L; } liga;".into();
        project.unicode_variation_sequences = vec![crate::font_data::UnicodeVariationSequence {
            base: 0x4E00,
            selector: 0xFE00,
            glyph: "Aacute".into(),
        }];
        project.axis_mappings.insert(
            "wght".into(),
            vec![crate::font_data::AxisMappingPoint {
                input: 0.5,
                output: 0.25,
            }],
        );
        save_glyphs(&project, &roundtrip_path).unwrap();
        let roundtripped = load_glyphs(&roundtrip_path).unwrap();
        assert_eq!(roundtripped.glyphs["A"].unicode, Some(65));
        assert_eq!(roundtripped.masters[0].axes.get("wght"), Some(&400.0));
        assert!(roundtripped.masters[0].is_bracket);
        assert_eq!(roundtripped.instances.len(), 1);
        assert_eq!(roundtripped.instances[0].name, "Text Medium");
        assert_eq!(roundtripped.instances[0].axes.get("wght"), Some(&550.0));
        assert_eq!(
            roundtripped.axis_names.get("wght"),
            Some(&"Weight".to_string())
        );
        assert!(roundtripped.opentype_classes.contains("@Upper = [A B];"));
        assert!(roundtripped.opentype_features.contains("feature liga"));
        assert!(roundtripped
            .opentype_features
            .contains("languagesystem latn dflt;"));
        assert!(roundtripped.opentype_features.contains("lookup L {"));
        assert_eq!(
            roundtripped.unicode_variation_sequences,
            project.unicode_variation_sequences
        );
        assert_eq!(roundtripped.axis_mappings, project.axis_mappings);
        assert_eq!(
            roundtripped.glyphs["Aacute"].layers["M1"].components[0].x_offset,
            20.0
        );
        assert_eq!(
            roundtripped.kerning.get(&("A".into(), "V".into())),
            Some(&-80.0)
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(roundtrip_path).unwrap();
    }

    #[test]
    fn glyphs_import_rejects_property_lists_without_glyphs() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-invalid-{}-{:?}.glyphs",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&path, b"{ familyName = NotAGlyphsFile; }").unwrap();
        let error = load_glyphs(&path).unwrap_err();
        assert!(error.contains("glyphs配列"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn glyphs_roundtrip_preserves_multiple_masters_and_layers() {
        let mut project = FontProject::new();
        project.metadata.family_name = "MultiMaster".into();
        project.axis_names.insert("wght".into(), "Weight".into());
        project.masters[0].id = "M1".into();
        project.masters[0].axes.insert("wght".into(), 400.0);
        project.masters.push(crate::font_data::FontMaster {
            id: "M2".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: [("wght".into(), 700.0)].into_iter().collect(),
        });
        project
            .set_master_metrics(
                "M2",
                crate::font_data::MasterMetrics {
                    ascender: 930.0,
                    descender: -260.0,
                    line_gap: 24.0,
                },
            )
            .unwrap();
        project.add_glyph("A".into(), Some(65));
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 1.1,
                yx: 0.0,
                xy: 0.0,
                yy: 0.9,
                dx: 12.0,
                dy: -8.0,
            })],
        );
        project.color_palettes = vec![vec![[255, 32, 0, 255], [0, 64, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                alpha: 1.0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Sweep,
                    extend: crate::font_data::ColorGradientExtend::default(),
                    x0: 300.0,
                    y0: 400.0,
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 0.0,
                    stops: Vec::new(),
                    radius0: 0.0,
                    radius1: 500.0,
                    start_angle: 45.0,
                    end_angle: 315.0,
                }),
            }],
        );
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "M2".into(),
            crate::font_data::GlyphLayer {
                width: 720.0,
                contours: vec![crate::font_data::Contour {
                    points: vec![
                        crate::font_data::ContourPoint::on_curve(10.0, 20.0),
                        crate::font_data::ContourPoint::on_curve(110.0, 20.0),
                        crate::font_data::ContourPoint::on_curve(110.0, 120.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-multimaster-{}-{:?}.glyphs",
            std::process::id(),
            std::thread::current().id()
        ));
        save_glyphs(&project, &path).unwrap();
        let roundtripped = load_glyphs(&path).unwrap();
        assert_eq!(roundtripped.masters.len(), 2);
        assert_eq!(roundtripped.masters[1].id, "M2");
        assert_eq!(roundtripped.masters[1].axes.get("wght"), Some(&700.0));
        assert_eq!(roundtripped.glyphs["A"].layers["M2"].width, 720.0);
        assert_eq!(roundtripped.master_metrics_for("M2").ascender, 930.0);
        assert_eq!(roundtripped.master_metrics_for("M2").descender, -260.0);
        assert_eq!(roundtripped.master_metrics_for("M2").line_gap, 24.0);
        let color_layer = &roundtripped.color_layers["A"][0];
        let gradient = color_layer.gradient.as_ref().unwrap();
        assert_eq!(gradient.kind, crate::font_data::ColorGradientKind::Sweep);
        assert_eq!(gradient.start_angle, 45.0);
        assert_eq!(gradient.end_angle, 315.0);
        let transform = roundtripped.color_layer_transforms["A"][0]
            .as_ref()
            .unwrap();
        assert_eq!(transform.xx, 1.1);
        assert_eq!(transform.dy, -8.0);
        assert_eq!(
            roundtripped.glyphs["A"].layers["M2"].contours[0].points[2].y,
            120.0
        );
        assert_eq!(roundtripped.axis_names.get("wght"), Some(&"Weight".into()));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn glyphs_import_accepts_utf8_bom() {
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-bom-{}-{:?}.glyphs",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"{ glyphs = ({ glyphname = A; unicode = 65; }); }");
        std::fs::write(&path, bytes).unwrap();
        let project = load_glyphs(&path).unwrap();
        assert_eq!(project.glyphs["A"].unicode, Some(0x41));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_reads_non_default_unicode_variation_sequences() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.add_glyph("A.var".into(), None);
        project.unicode_variation_sequences = vec![crate::font_data::UnicodeVariationSequence {
            base: 0x41,
            selector: 0xFE00,
            glyph: "A.var".into(),
        }];
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-uvs-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert_eq!(
            loaded.unicode_variation_sequences,
            project.unicode_variation_sequences
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_preserves_custom_name_records_as_feature_source() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.opentype_features =
            "table name { nameid 256 \"Display Name\"; nameid 257 3 1 0x411 \"表示名\"; } name;"
                .into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-name-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("nameid 256"));
        assert!(loaded.opentype_features.contains("Display Name"));
        assert!(loaded.opentype_features.contains("nameid 257"));
        assert!(loaded.opentype_features.contains("表示名"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_simple_gsub_rules_as_feature_source() {
        let mut project = FontProject::new();
        for (name, unicode) in [
            ("A", Some(0x41)),
            ("A.alt", None),
            ("B", Some(0x42)),
            ("B.alt", None),
            ("f", Some(0x66)),
            ("i", Some(0x69)),
            ("f_i", None),
        ] {
            project.add_glyph(name.into(), unicode);
        }
        project.opentype_features = "feature salt { sub A by A.alt; } salt; feature cv01 { sub B by B.alt; } cv01; feature liga { sub f i by f_i; } liga;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-gsub-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("feature salt"));
        assert!(loaded.opentype_features.contains("sub A by A.alt"));
        assert!(loaded.opentype_features.contains("feature liga"));
        assert!(loaded.opentype_features.contains("sub f i by f_i"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_simple_gpos_rules_as_feature_source() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.add_glyph("V".into(), Some(0x56));
        project.opentype_features =
            "feature dist { lookupflag IgnoreMarks; pos A <20 3 10 0>; pos A V <0 0 -80 0>; } dist;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-gpos-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.opentype_features.contains("feature dist"));
        assert!(loaded.opentype_features.contains("lookupflag IgnoreMarks"));
        assert!(loaded.opentype_features.contains("pos A <20 3 10 0>"));
        assert!(loaded.opentype_features.contains("pos A V <0 0 -80 0>"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_mark_to_base_anchors() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.add_glyph("acute".into(), Some(0x301));
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .anchors
            .push(crate::font_data::GlyphAnchor {
                name: "top".into(),
                x: 300.0,
                y: 700.0,
            });
        project
            .glyphs
            .get_mut("acute")
            .unwrap()
            .anchors
            .push(crate::font_data::GlyphAnchor {
                name: "_top".into(),
                x: 20.0,
                y: 100.0,
            });
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mark-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.glyphs["A"]
            .anchors
            .iter()
            .any(|anchor| anchor.name == "class0" && anchor.x == 300.0 && anchor.y == 700.0));
        assert!(loaded.glyphs["acute"]
            .anchors
            .iter()
            .any(|anchor| anchor.name == "_class0" && anchor.x == 20.0 && anchor.y == 100.0));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_cursive_anchors() {
        let mut project = FontProject::new();
        project.add_glyph("alef".into(), Some(0x627));
        project.add_glyph("beh".into(), Some(0x628));
        project.glyphs.get_mut("alef").unwrap().anchors.extend([
            crate::font_data::GlyphAnchor {
                name: "entry".into(),
                x: 10.0,
                y: 500.0,
            },
            crate::font_data::GlyphAnchor {
                name: "exit".into(),
                x: 600.0,
                y: 500.0,
            },
        ]);
        project.glyphs.get_mut("beh").unwrap().anchors.extend([
            crate::font_data::GlyphAnchor {
                name: "entry".into(),
                x: 20.0,
                y: 450.0,
            },
            crate::font_data::GlyphAnchor {
                name: "exit".into(),
                x: 580.0,
                y: 450.0,
            },
        ]);
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-cursive-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.glyphs["alef"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "entry" && anchor.x == 10.0 && anchor.y == 500.0 }));
        assert!(loaded.glyphs["beh"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "exit" && anchor.x == 580.0 && anchor.y == 450.0 }));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_mark_to_mark_anchors() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), Some(0x301));
        project.add_glyph("acute2".into(), Some(0x302));
        project.glyphs.get_mut("acute").unwrap().anchors.extend([
            crate::font_data::GlyphAnchor {
                name: "_top".into(),
                x: 20.0,
                y: 100.0,
            },
            crate::font_data::GlyphAnchor {
                name: "top".into(),
                x: 25.0,
                y: 130.0,
            },
        ]);
        project
            .glyphs
            .get_mut("acute2")
            .unwrap()
            .anchors
            .push(crate::font_data::GlyphAnchor {
                name: "_top".into(),
                x: 30.0,
                y: 110.0,
            });
        project.opentype_features = "markClass acute <anchor 20 100> @TOP; markClass acute2 <anchor 30 110> @TOP; feature mkmk { pos mark @TOP mark @TOP; } mkmk;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mkmk-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.glyphs["acute"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "_class0" && anchor.x == 20.0 && anchor.y == 100.0 }));
        assert!(loaded.glyphs["acute"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "class0" && anchor.x == 20.0 && anchor.y == 100.0 }));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ttf_import_restores_mark_to_ligature_anchors() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), Some(0x301));
        project.add_glyph("f_i".into(), None);
        project
            .glyphs
            .get_mut("acute")
            .unwrap()
            .anchors
            .push(crate::font_data::GlyphAnchor {
                name: "_top".into(),
                x: 20.0,
                y: 100.0,
            });
        project.glyphs.get_mut("f_i").unwrap().anchors.extend([
            crate::font_data::GlyphAnchor {
                name: "top_1".into(),
                x: 250.0,
                y: 700.0,
            },
            crate::font_data::GlyphAnchor {
                name: "top_2".into(),
                x: 550.0,
                y: 700.0,
            },
        ]);
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mark-ligature-import-{}.ttf",
            std::process::id()
        ));
        crate::export::export_ttf(&project, &path).unwrap();
        let loaded = load_ttf(&path).unwrap();
        assert!(loaded.glyphs["f_i"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "class0_1" && anchor.x == 250.0 && anchor.y == 700.0 }));
        assert!(loaded.glyphs["f_i"]
            .anchors
            .iter()
            .any(|anchor| { anchor.name == "class0_2" && anchor.x == 550.0 && anchor.y == 700.0 }));
        std::fs::remove_file(path).unwrap();
    }
}
