use super::*;

impl FontProject {
    fn norad_metadata(&self, font: &mut norad::Font) -> Result<(), String> {
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
        Ok(())
    }
}
