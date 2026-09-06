use super::*;

impl FontProject {
    pub(crate) fn populate_norad_lib(&self, font: &mut norad::Font) -> Result<(), String> {
            font.features = self.feature_source();
            font.lib.insert(
                "com.glyph-studio.opentypeClasses".into(),
                plist::Value::String(self.opentype_classes.clone()),
            );
            let unicode_variation_sequences = serde_json::to_string(&self.unicode_variation_sequences)
                .map_err(|error| format!("IVSのUFO保存に失敗しました: {error}"))?;
            font.lib.insert(
                "com.glyph-studio.unicodeVariationSequences".into(),
                plist::Value::String(unicode_variation_sequences),
            );
            let axis_mappings = serde_json::to_string(&self.axis_mappings)
                .map_err(|error| format!("avar軸マッピングのUFO保存に失敗しました: {error}"))?;
            font.lib.insert(
                "com.glyph-studio.axisMappings".into(),
                plist::Value::String(axis_mappings),
            );
            let preserved_tables = serde_json::to_string(&self.preserved_tables)
                .map_err(|error| format!("OpenTypeテーブルのUFO保存に失敗しました: {error}"))?;
            font.lib.insert(
                "com.glyph-studio.preservedTables".into(),
                plist::Value::String(preserved_tables),
            );
            if let Some(source) = &self.preserved_layout_source {
                font.lib.insert(
                    "com.glyph-studio.preservedLayoutSource".into(),
                    plist::Value::String(source.clone()),
                );
            }
            if let Some(fingerprint) = self.preserved_layout_fingerprint {
                font.lib.insert(
                    "com.glyph-studio.preservedLayoutFingerprint".into(),
                    plist::Value::String(fingerprint.to_string()),
                );
            }
            let conditional_layers = serde_json::to_string(&self.conditional_layers)
                .map_err(|error| format!("条件レイヤーのUFO保存に失敗しました: {error}"))?;
            font.lib.insert(
                "com.glyph-studio.conditionalLayers".into(),
                plist::Value::String(conditional_layers),
            );
            let axis_names = self
                .axis_names
                .iter()
                .map(|(tag, name)| (tag.clone(), plist::Value::String(name.clone())))
                .collect();
            font.lib.insert(
                "com.glyph-studio.axisNames".into(),
                plist::Value::Dictionary(axis_names),
            );
            let mut master_info = plist::Dictionary::new();
            for master in &self.masters {
                let mut info = plist::Dictionary::new();
                info.insert("id".into(), plist::Value::String(master.id.clone()));
                info.insert("weight".into(), plist::Value::Real(master.weight));
                info.insert("width".into(), plist::Value::Real(master.width));
                info.insert("bracket".into(), plist::Value::Boolean(master.is_bracket));
                let axes = master
                    .axes
                    .iter()
                    .map(|(tag, value)| (tag.clone(), plist::Value::Real(*value)))
                    .collect();
                info.insert("axes".into(), plist::Value::Dictionary(axes));
                master_info.insert(master.name.clone(), plist::Value::Dictionary(info));
            }
            font.lib.insert(
                "com.glyph-studio.masters".into(),
                plist::Value::Dictionary(master_info),
            );
            let mut metadata = plist::Dictionary::new();
            metadata.insert(
                "fontRevision".into(),
                plist::Value::Real(self.metadata.font_revision),
            );
            font.lib.insert(
                "com.glyph-studio.metadata".into(),
                plist::Value::Dictionary(metadata),
            );
            let guidelines = self
                .guidelines_for_master(&self.default_master_id)
                .iter()
                .map(|guide| {
                    let mut value = plist::Dictionary::new();
                    value.insert("x".into(), plist::Value::Real(guide.x));
                    value.insert("y".into(), plist::Value::Real(guide.y));
                    value.insert("angle".into(), plist::Value::Real(guide.angle));
                    value.insert("name".into(), plist::Value::String(guide.name.clone()));
                    plist::Value::Dictionary(value)
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.guidelines".into(),
                plist::Value::Array(guidelines),
            );
            if !self.guidelines_by_master.is_empty() {
                let by_master = self
                    .guidelines_by_master
                    .iter()
                    .map(|(master_id, guides)| {
                        let values = guides
                            .iter()
                            .map(|guide| {
                                let mut value = plist::Dictionary::new();
                                value.insert("x".into(), plist::Value::Real(guide.x));
                                value.insert("y".into(), plist::Value::Real(guide.y));
                                value.insert("angle".into(), plist::Value::Real(guide.angle));
                                value.insert("name".into(), plist::Value::String(guide.name.clone()));
                                plist::Value::Dictionary(value)
                            })
                            .collect();
                        (master_id.clone(), plist::Value::Array(values))
                    })
                    .collect();
                font.lib.insert(
                    "com.glyph-studio.guidelinesByMaster".into(),
                    plist::Value::Dictionary(by_master),
                );
            }
            let vertical_metrics = self
                .vertical_metrics
                .iter()
                .map(|(name, metrics)| {
                    let mut value = plist::Dictionary::new();
                    value.insert(
                        "advanceHeight".into(),
                        plist::Value::Real(metrics.advance_height),
                    );
                    value.insert(
                        "topSideBearing".into(),
                        plist::Value::Real(metrics.top_side_bearing),
                    );
                    (name.clone(), plist::Value::Dictionary(value))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.verticalMetrics".into(),
                plist::Value::Dictionary(vertical_metrics),
            );
            let vertical_metrics_by_master = self
                .vertical_metrics_by_master
                .iter()
                .map(|(master_id, metrics)| {
                    let values = metrics
                        .iter()
                        .map(|(name, metric)| {
                            let mut value = plist::Dictionary::new();
                            value.insert(
                                "advanceHeight".into(),
                                plist::Value::Real(metric.advance_height),
                            );
                            value.insert(
                                "topSideBearing".into(),
                                plist::Value::Real(metric.top_side_bearing),
                            );
                            (name.clone(), plist::Value::Dictionary(value))
                        })
                        .collect();
                    (master_id.clone(), plist::Value::Dictionary(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.verticalMetricsByMaster".into(),
                plist::Value::Dictionary(vertical_metrics_by_master),
            );
            let metrics_by_master = self
                .metrics_by_master
                .iter()
                .map(|(master_id, metrics)| {
                    let mut value = plist::Dictionary::new();
                    value.insert("ascender".into(), plist::Value::Real(metrics.ascender));
                    value.insert("descender".into(), plist::Value::Real(metrics.descender));
                    value.insert("lineGap".into(), plist::Value::Real(metrics.line_gap));
                    (master_id.clone(), plist::Value::Dictionary(value))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.metricsByMaster".into(),
                plist::Value::Dictionary(metrics_by_master),
            );
            let background_images = self
                .background_images
                .iter()
                .map(|(glyph, masters)| {
                    let values = masters
                        .iter()
                        .map(|(master, path)| (master.clone(), plist::Value::String(path.clone())))
                        .collect();
                    (glyph.clone(), plist::Value::Dictionary(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.backgroundImages".into(),
                plist::Value::Dictionary(background_images),
            );
            let background_opacities = self
                .background_opacities
                .iter()
                .map(|(glyph, masters)| {
                    let values = masters
                        .iter()
                        .map(|(master, opacity)| {
                            (master.clone(), plist::Value::Real(f64::from(*opacity)))
                        })
                        .collect();
                    (glyph.clone(), plist::Value::Dictionary(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.backgroundOpacities".into(),
                plist::Value::Dictionary(background_opacities),
            );
            let background_transforms = self
                .background_transforms
                .iter()
                .map(|(glyph, masters)| {
                    let values = masters
                        .iter()
                        .map(|(master, transform)| {
                            let mut value = plist::Dictionary::new();
                            value.insert("x".into(), plist::Value::Real(f64::from(transform.x)));
                            value.insert("y".into(), plist::Value::Real(f64::from(transform.y)));
                            value.insert(
                                "scale".into(),
                                plist::Value::Real(f64::from(transform.scale)),
                            );
                            value.insert(
                                "rotation".into(),
                                plist::Value::Real(f64::from(transform.rotation)),
                            );
                            value.insert("flipX".into(), plist::Value::Boolean(transform.flip_x));
                            value.insert("flipY".into(), plist::Value::Boolean(transform.flip_y));
                            (master.clone(), plist::Value::Dictionary(value))
                        })
                        .collect();
                    (glyph.clone(), plist::Value::Dictionary(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.backgroundTransforms".into(),
                plist::Value::Dictionary(background_transforms),
            );
            let palettes = self
                .color_palettes
                .iter()
                .map(|palette| {
                    plist::Value::Array(
                        palette
                            .iter()
                            .map(|&[r, g, b, a]| {
                                plist::Value::Array(
                                    [r, g, b, a]
                                        .into_iter()
                                        .map(|value| plist::Value::Integer(i64::from(value).into()))
                                        .collect(),
                                )
                            })
                            .collect(),
                    )
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.colorPalettes".into(),
                plist::Value::Array(palettes),
            );
            font.lib.insert(
                "com.glyph-studio.colorPaletteNames".into(),
                plist::Value::Array(
                    self.color_palette_names
                        .iter()
                        .map(|name| plist::Value::String(name.clone()))
                        .collect(),
                ),
            );
            font.lib.insert(
                "com.glyph-studio.colorPaletteTypes".into(),
                plist::Value::Array(
                    self.color_palette_types
                        .iter()
                        .map(|value| plist::Value::Integer(i64::from(*value).into()))
                        .collect(),
                ),
            );
            font.lib.insert(
                "com.glyph-studio.colorPaletteEntryNames".into(),
                plist::Value::Array(
                    self.color_palette_entry_names
                        .iter()
                        .map(|name| plist::Value::String(name.clone()))
                        .collect(),
                ),
            );
            let color_layers = self
                .color_layers
                .iter()
                .map(|(name, layers)| {
                    let values = layers
                        .iter()
                        .map(|layer| {
                            let mut value = plist::Dictionary::new();
                            value.insert("glyph".into(), plist::Value::String(layer.glyph.clone()));
                            value.insert(
                                "paletteIndex".into(),
                                plist::Value::Integer(i64::from(layer.palette_index).into()),
                            );
                            if let Some(gradient) = &layer.gradient {
                                let mut info = plist::Dictionary::new();
                                info.insert(
                                    "startPaletteIndex".into(),
                                    plist::Value::Integer(
                                        i64::from(gradient.start_palette_index).into(),
                                    ),
                                );
                                info.insert(
                                    "endPaletteIndex".into(),
                                    plist::Value::Integer(i64::from(gradient.end_palette_index).into()),
                                );
                                let kind = match gradient.kind {
                                    ColorGradientKind::Linear => "linear",
                                    ColorGradientKind::Radial => "radial",
                                    ColorGradientKind::Sweep => "sweep",
                                };
                                info.insert("kind".into(), plist::Value::String(kind.into()));
                                let extend = match gradient.extend {
                                    ColorGradientExtend::Pad => "pad",
                                    ColorGradientExtend::Repeat => "repeat",
                                    ColorGradientExtend::Reflect => "reflect",
                                };
                                info.insert("extend".into(), plist::Value::String(extend.into()));
                                for (key, value) in [
                                    ("x0", gradient.x0),
                                    ("y0", gradient.y0),
                                    ("x1", gradient.x1),
                                    ("y1", gradient.y1),
                                    ("x2", gradient.x2),
                                    ("y2", gradient.y2),
                                    ("radius0", gradient.radius0),
                                    ("radius1", gradient.radius1),
                                    ("startAngle", gradient.start_angle),
                                    ("endAngle", gradient.end_angle),
                                ] {
                                    info.insert(key.into(), plist::Value::Real(value));
                                }
                                if !gradient.stops.is_empty() {
                                    info.insert(
                                        "stops".into(),
                                        plist::Value::Array(
                                            gradient
                                                .stops
                                                .iter()
                                                .map(|stop| {
                                                    let mut value = plist::Dictionary::new();
                                                    value.insert(
                                                        "offset".into(),
                                                        plist::Value::Real(stop.offset),
                                                    );
                                                    value.insert(
                                                        "paletteIndex".into(),
                                                        plist::Value::Integer(
                                                            i64::from(stop.palette_index).into(),
                                                        ),
                                                    );
                                                    value.insert(
                                                        "alpha".into(),
                                                        plist::Value::Real(stop.alpha),
                                                    );
                                                    plist::Value::Dictionary(value)
                                                })
                                                .collect(),
                                        ),
                                    );
                                }
                                value.insert("gradient".into(), plist::Value::Dictionary(info));
                            }
                            plist::Value::Dictionary(value)
                        })
                        .collect();
                    (name.clone(), plist::Value::Array(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.colorLayers".into(),
                plist::Value::Dictionary(color_layers),
            );
            let color_layer_transforms = self
                .color_layer_transforms
                .iter()
                .map(|(name, transforms)| {
                    let values = transforms
                        .iter()
                        .map(|transform| {
                            let Some(transform) = transform else {
                                return plist::Value::Dictionary(plist::Dictionary::new());
                            };
                            let mut value = plist::Dictionary::new();
                            for (key, number) in [
                                ("xx", transform.xx),
                                ("yx", transform.yx),
                                ("xy", transform.xy),
                                ("yy", transform.yy),
                                ("dx", transform.dx),
                                ("dy", transform.dy),
                            ] {
                                value.insert(key.into(), plist::Value::Real(number));
                            }
                            plist::Value::Dictionary(value)
                        })
                        .collect();
                    (name.clone(), plist::Value::Array(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.colorLayerTransforms".into(),
                plist::Value::Dictionary(color_layer_transforms),
            );
            let mut left_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
            let mut right_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
            for glyph in self.glyphs.values() {
                if !glyph.left_kerning_group.trim().is_empty() {
                    left_groups
                        .entry(format!("public.kern1.{}", glyph.left_kerning_group.trim()))
                        .or_default()
                        .push(glyph.name.clone());
                }
                if !glyph.right_kerning_group.trim().is_empty() {
                    right_groups
                        .entry(format!("public.kern2.{}", glyph.right_kerning_group.trim()))
                        .or_default()
                        .push(glyph.name.clone());
                }
            }
            for (group, members) in left_groups.into_iter().chain(right_groups) {
                let group_name = norad::Name::new(&group)
                    .map_err(|error| format!("カーニンググループ名が不正です: {error}"))?;
                let member_names = members
                    .into_iter()
                    .map(|member| {
                        norad::Name::new(&member)
                            .map_err(|error| format!("グループ所属グリフ名が不正です: {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                font.groups.insert(group_name, member_names);
            }
        Ok(())
    }
}
