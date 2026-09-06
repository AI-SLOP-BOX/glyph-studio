#[rustfmt::skip]
pub fn load_ufo(path: &Path) -> Result<FontProject, String> {
    let font = norad::Font::load(path).map_err(|e| format!("UFO読み込みエラー: {}", e))?;
    let mut project = FontProject::new();
    project.opentype_features = font.features.clone();
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.unicodeVariationSequences") {
        if let Ok(sequences) = serde_json::from_str(serialized) {
            project.unicode_variation_sequences = sequences;
        }
    }
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.axisMappings") {
        if let Ok(mappings) = serde_json::from_str(serialized) {
            project.axis_mappings = mappings;
        }
    }
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.preservedTables") {
        if let Ok(tables) = serde_json::from_str(serialized) {
            project.preserved_tables = tables;
        }
    }
    if let Some(plist::Value::String(source)) = font.lib.get("com.glyph-studio.preservedLayoutSource") {
        project.preserved_layout_source = Some(source.clone());
    }
    if let Some(plist::Value::String(fingerprint)) = font.lib.get("com.glyph-studio.preservedLayoutFingerprint") {
        project.preserved_layout_fingerprint = fingerprint.parse().ok();
    }
    if let Some(plist::Value::String(classes)) = font.lib.get("com.glyph-studio.opentypeClasses") {
        project.opentype_classes = classes.clone();
        if !project.opentype_classes.trim().is_empty() {
            let class_source = project.opentype_classes.clone();
            project.opentype_features = project.opentype_features.strip_prefix(&class_source).unwrap_or(&project.opentype_features).trim().to_string();
        }
    }
    if let Some(plist::Value::String(serialized)) = font.lib.get("com.glyph-studio.conditionalLayers") {
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
    project.metadata.family_name = font.font_info.family_name.clone().unwrap_or(project.metadata.family_name);
    project.metadata.style_name = font.font_info.style_name.clone().unwrap_or(project.metadata.style_name);
    project.metadata.copyright = font.font_info.copyright.clone().unwrap_or_default();
    project.metadata.designer = font.font_info.open_type_name_designer.clone().unwrap_or_default();
    project.metadata.manufacturer = font.font_info.open_type_name_manufacturer.clone().unwrap_or_default();
    project.metadata.x_height = font.font_info.x_height.unwrap_or(0.0);
    project.metadata.cap_height = font.font_info.cap_height.unwrap_or(0.0);
    project.metadata.italic_angle = font.font_info.italic_angle.unwrap_or(0.0);
    project.metadata.underline_position = font.font_info.postscript_underline_position.unwrap_or(-100.0);
    project.metadata.underline_thickness = font.font_info.postscript_underline_thickness.unwrap_or(50.0);
    project.metadata.is_fixed_pitch = font.font_info.postscript_is_fixed_pitch.unwrap_or(false);
    if let Some(plist::Value::Dictionary(metadata)) = font.lib.get("com.glyph-studio.metadata") {
        if let Some(revision) = metadata.get("fontRevision").and_then(plist::Value::as_real) {
            project.metadata.font_revision = revision;
        }
    }
    if let Some(version) = font.font_info.open_type_name_version.as_deref() {
        if let Some(revision) = version.strip_prefix("Version ").and_then(|value| value.trim().parse::<f64>().ok()) {
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
                    angle: info.get("angle").and_then(plist::Value::as_real).unwrap_or(0.0),
                    name: info.get("name").and_then(plist::Value::as_string).unwrap_or_default().to_string(),
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
                    name: guide.name.as_ref().map(ToString::to_string).unwrap_or_default(),
                }
            })
            .collect();
    }
    if let Some(plist::Value::Dictionary(by_master)) = font.lib.get("com.glyph-studio.guidelinesByMaster") {
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
                        angle: info.get("angle").and_then(plist::Value::as_real).unwrap_or(0.0),
                        name: info.get("name").and_then(plist::Value::as_string).unwrap_or_default().to_string(),
                    })
                })
                .collect();
            project.guidelines_by_master.insert(master_id.clone(), guides);
        }
    }
    if let Some(plist::Value::Dictionary(master_info)) = font.lib.get("com.glyph-studio.masters") {
        project.masters.clear();
        for (name, value) in master_info {
            let plist::Value::Dictionary(info) = value else {
                continue;
            };
            let id = info.get("id").and_then(plist::Value::as_string).unwrap_or(name).to_string();
            let weight = info.get("weight").and_then(plist::Value::as_real).unwrap_or(400.0);
            let width = info.get("width").and_then(plist::Value::as_real).unwrap_or(100.0);
            let is_bracket = info.get("bracket").and_then(plist::Value::as_boolean).unwrap_or(false);
            let axes = info
                .get("axes")
                .and_then(plist::Value::as_dictionary)
                .map(|values| values.iter().filter_map(|(tag, value)| Some((tag.clone(), value.as_real()?))).collect())
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
    if let Some(plist::Value::Dictionary(vertical_metrics)) = font.lib.get("com.glyph-studio.verticalMetrics") {
        for (name, value) in vertical_metrics {
            let plist::Value::Dictionary(info) = value else {
                continue;
            };
            let Some(advance_height) = info.get("advanceHeight").and_then(plist::Value::as_real) else {
                continue;
            };
            let top_side_bearing = info.get("topSideBearing").and_then(plist::Value::as_real).unwrap_or(0.0);
            project.vertical_metrics.insert(name.clone(), crate::font_data::VerticalMetrics { advance_height, top_side_bearing });
        }
    }
    if let Some(plist::Value::Dictionary(by_master)) = font.lib.get("com.glyph-studio.verticalMetricsByMaster") {
        for (master_id, values) in by_master {
            let plist::Value::Dictionary(values) = values else {
                continue;
            };
            let metrics = project.vertical_metrics_by_master.entry(master_id.clone()).or_default();
            for (name, value) in values {
                let plist::Value::Dictionary(info) = value else {
                    continue;
                };
                let Some(advance_height) = info.get("advanceHeight").and_then(plist::Value::as_real) else {
                    continue;
                };
                metrics.insert(
                    name.clone(),
                    crate::font_data::VerticalMetrics {
                        advance_height,
                        top_side_bearing: info.get("topSideBearing").and_then(plist::Value::as_real).unwrap_or(0.0),
                    },
                );
            }
        }
    }
    if let Some(plist::Value::Dictionary(by_master)) = font.lib.get("com.glyph-studio.metricsByMaster") {
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
            let line_gap = info.get("lineGap").and_then(plist::Value::as_real).unwrap_or(0.0);
            project.metrics_by_master.insert(master_id.clone(), crate::font_data::MasterMetrics { ascender, descender, line_gap });
        }
    }
    if let Some(plist::Value::Dictionary(images)) = font.lib.get("com.glyph-studio.backgroundImages") {
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
    if let Some(plist::Value::Dictionary(opacities)) = font.lib.get("com.glyph-studio.backgroundOpacities") {
        for (glyph, masters) in opacities {
            let plist::Value::Dictionary(masters) = masters else {
                continue;
            };
            let target = project.background_opacities.entry(glyph.clone()).or_default();
            for (master, opacity) in masters {
                if let Some(opacity) = opacity.as_real() {
                    target.insert(master.clone(), opacity as f32);
                }
            }
        }
    }
    if let Some(plist::Value::Dictionary(transforms)) = font.lib.get("com.glyph-studio.backgroundTransforms") {
        for (glyph, masters) in transforms {
            let plist::Value::Dictionary(masters) = masters else {
                continue;
            };
            let target = project.background_transforms.entry(glyph.clone()).or_default();
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
                let scale = value.get("scale").and_then(plist::Value::as_real).unwrap_or(1.0);
                target.insert(
                    master.clone(),
                    crate::font_data::BackgroundImageTransform {
                        x: x as f32,
                        y: y as f32,
                        scale: scale as f32,
                        rotation: value.get("rotation").and_then(plist::Value::as_real).unwrap_or(0.0) as f32,
                        flip_x: value.get("flipX").and_then(plist::Value::as_boolean).unwrap_or(false),
                        flip_y: value.get("flipY").and_then(plist::Value::as_boolean).unwrap_or(false),
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
                            .or_else(|| value.as_signed_integer().and_then(|v| u64::try_from(v).ok()))
                            .and_then(|v| u8::try_from(v).ok())
                    })
                    .collect();
                channels.as_chunks::<4>().0.iter().map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]]).collect()
            })
            .collect();
    }
    if let Some(plist::Value::Array(names)) = font.lib.get("com.glyph-studio.colorPaletteNames") {
        project.color_palette_names = names.iter().map(|value| value.as_string().unwrap_or_default().to_string()).collect();
    }
    if let Some(plist::Value::Array(types)) = font.lib.get("com.glyph-studio.colorPaletteTypes") {
        project.color_palette_types = types
            .iter()
            .filter_map(|value| {
                value
                    .as_unsigned_integer()
                    .or_else(|| value.as_signed_integer().and_then(|v| u64::try_from(v).ok()))
                    .and_then(|value| u32::try_from(value).ok())
            })
            .collect();
    }
    if let Some(plist::Value::Array(names)) = font.lib.get("com.glyph-studio.colorPaletteEntryNames") {
        project.color_palette_entry_names = names.iter().map(|value| value.as_string().unwrap_or_default().to_string()).collect();
    }
    if let Some(plist::Value::Dictionary(color_layers)) = font.lib.get("com.glyph-studio.colorLayers") {
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
                            .and_then(|value| value.as_unsigned_integer().or_else(|| value.as_signed_integer().and_then(|v| u64::try_from(v).ok())))
                            .and_then(|value| u16::try_from(value).ok())?,
                        gradient: parse_color_gradient(info),
                        alpha: info.get("alpha").and_then(plist::Value::as_real).unwrap_or(1.0),
                    })
                })
                .collect();
            project.color_layers.insert(name.clone(), layers);
        }
    }

    if let Some(plist::Value::Dictionary(color_layer_transforms)) = font.lib.get("com.glyph-studio.colorLayerTransforms") {
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
            project.color_layer_transforms.insert(name.clone(), transforms);
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
        if !project.masters.iter().any(|master| master.id == layer.name().to_string() || master.name == layer.name().to_string()) {
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
            glyph_data.left_kerning_group = glyph.lib.get("com.glyph-studio.leftKerningGroup").and_then(plist::Value::as_string).unwrap_or_default().to_string();
            glyph_data.right_kerning_group = glyph.lib.get("com.glyph-studio.rightKerningGroup").and_then(plist::Value::as_string).unwrap_or_default().to_string();
            glyph_data.left_metrics_key = glyph.lib.get("com.glyph-studio.leftMetricsKey").and_then(plist::Value::as_string).unwrap_or_default().to_string();
            glyph_data.right_metrics_key = glyph.lib.get("com.glyph-studio.rightMetricsKey").and_then(plist::Value::as_string).unwrap_or_default().to_string();
            if let Some(plist::Value::Dictionary(master_guidelines)) = glyph.lib.get("com.glyph-studio.masterGuidelines") {
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
                                    angle: info.get("angle").and_then(plist::Value::as_real).unwrap_or(0.0),
                                    name: info.get("name").and_then(plist::Value::as_string).unwrap_or_default().to_string(),
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
                            name: guide.name.as_ref().map(ToString::to_string).unwrap_or_default(),
                        }
                    })
                    .collect();
            }

            for contour in &glyph.contours {
                let mut contour_data = crate::font_data::Contour::new();
                for point in &contour.points {
                    let point_type = match point.typ {
                        norad::PointType::Move | norad::PointType::Line | norad::PointType::Curve | norad::PointType::QCurve => crate::font_data::PointType::OnCurve,
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
                glyph_data.components.push(crate::font_data::GlyphComponent {
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
                let existing = project.glyphs.entry(name.clone()).or_insert_with(|| crate::font_data::GlyphData::new(name, unicode));
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
                    if project.glyphs.contains_key(expanded_left) && project.glyphs.contains_key(expanded_right) {
                        project.kerning.insert((expanded_left.clone(), expanded_right.clone()), *value);
                    }
                }
            }
        }
    }

    if let Some(plist::Value::Dictionary(by_master)) = font.lib.get("com.glyph-studio.kerningByMaster") {
        for (master_id, value) in by_master {
            let Some(entries) = value.as_array() else {
                continue;
            };
            let pairs = entries
                .iter()
                .filter_map(|entry| {
                    let entry = entry.as_dictionary()?;
                    Some((
                        (entry.get("left").and_then(plist_string)?, entry.get("right").and_then(plist_string)?),
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
