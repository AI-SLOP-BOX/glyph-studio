
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
