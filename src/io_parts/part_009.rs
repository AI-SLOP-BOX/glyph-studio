
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
