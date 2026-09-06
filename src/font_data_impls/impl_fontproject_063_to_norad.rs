use super::*;

impl FontProject {
    pub fn to_norad(&self) -> Result<norad::Font, String> {
        if !self.metadata.units_per_em.is_finite()
            || !(16.0..=16384.0).contains(&self.metadata.units_per_em)
        {
            return Err("UPMは16〜16384の範囲で指定してください".to_string());
        }
        for glyph in self.glyphs.values() {
            if !glyph.width.is_finite() || glyph.width < 0.0 {
                return Err(format!("グリフ '{}' の幅が不正です", glyph.name));
            }
            for contour in &glyph.contours {
                if contour.points.len() < 3 {
                    return Err(format!("グリフ '{}' に不完全な輪郭があります", glyph.name));
                }
                if contour
                    .points
                    .iter()
                    .any(|p| !p.x.is_finite() || !p.y.is_finite())
                {
                    return Err(format!("グリフ '{}' に不正な座標があります", glyph.name));
                }
                if contour
                    .points
                    .windows(2)
                    .any(|pair| pair[0].x == pair[1].x && pair[0].y == pair[1].y)
                {
                    return Err(format!(
                        "グリフ '{}' に重複した連続ノードがあります",
                        glyph.name
                    ));
                }
            }
            for component in &glyph.components {
                if component.base == glyph.name {
                    return Err(format!(
                        "グリフ '{}' が自分自身を参照しています",
                        glyph.name
                    ));
                }
                if !self.glyphs.contains_key(&component.base) {
                    return Err(format!(
                        "グリフ '{}' が参照するコンポーネント '{}' がありません",
                        glyph.name, component.base
                    ));
                }
                let transform = [
                    component.x_scale,
                    component.xy_scale,
                    component.yx_scale,
                    component.y_scale,
                    component.x_offset,
                    component.y_offset,
                ];
                if transform.iter().any(|value| !value.is_finite()) {
                    return Err(format!(
                        "グリフ '{}' のコンポーネント変換が不正です",
                        glyph.name
                    ));
                }
            }
        }
        let mut font = norad::Font::new();

        font.font_info.family_name = Some(self.metadata.family_name.clone());
        font.font_info.style_name = Some(self.metadata.style_name.clone());
        font.font_info.copyright =
            (!self.metadata.copyright.trim().is_empty()).then(|| self.metadata.copyright.clone());
        font.font_info.open_type_name_designer =
            (!self.metadata.designer.trim().is_empty()).then(|| self.metadata.designer.clone());
        font.font_info.open_type_name_manufacturer =
            (!self.metadata.manufacturer.trim().is_empty())
                .then(|| self.metadata.manufacturer.clone());
        font.font_info.open_type_name_version =
            Some(format!("Version {:.3}", self.metadata.font_revision));
        let units_per_em = self
            .metadata
            .units_per_em
            .try_into()
            .map_err(|_| "UPMを整数に変換できません".to_string())?;
        font.font_info.units_per_em = Some(units_per_em);
        font.font_info.ascender = Some(self.metadata.ascender);
        font.font_info.descender = Some(self.metadata.descender);
        font.font_info.x_height = Some(self.metadata.x_height);
        font.font_info.cap_height = Some(self.metadata.cap_height);
        font.font_info.italic_angle = Some(self.metadata.italic_angle);
        font.font_info.postscript_underline_position = Some(self.metadata.underline_position);
        font.font_info.postscript_underline_thickness = Some(self.metadata.underline_thickness);
        font.font_info.postscript_is_fixed_pitch = Some(self.metadata.is_fixed_pitch);
        font.font_info.open_type_os2_typo_line_gap = Some(self.metadata.line_gap as i32);
        font.font_info.guidelines = Some(
            self.guidelines_for_master(&self.default_master_id)
                .iter()
                .map(|guide| {
                    let line = if guide.angle.abs() < f64::EPSILON {
                        norad::Line::Horizontal(guide.y)
                    } else if (guide.angle - 90.0).abs() < f64::EPSILON {
                        norad::Line::Vertical(guide.x)
                    } else {
                        norad::Line::Angle {
                            x: guide.x,
                            y: guide.y,
                            degrees: guide.angle,
                        }
                    };
                    let name = (!guide.name.is_empty())
                        .then(|| norad::Name::new(&guide.name))
                        .transpose()
                        .map_err(|error| format!("ガイド名が不正です: {error}"))?;
                    Ok(norad::Guideline::new(line, name, None, None, None))
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
        self.populate_norad_lib(&mut font)?;
        let layer = font.default_layer_mut();
        for glyph_data in self.glyphs.values() {
            let mut glyph = norad::Glyph::new(&glyph_data.name);
            glyph.width = glyph_data.width;
            if !glyph_data.left_kerning_group.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.leftKerningGroup".into(),
                    plist::Value::String(glyph_data.left_kerning_group.clone()),
                );
            }
            if !glyph_data.right_kerning_group.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.rightKerningGroup".into(),
                    plist::Value::String(glyph_data.right_kerning_group.clone()),
                );
            }
            if !glyph_data.left_metrics_key.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.leftMetricsKey".into(),
                    plist::Value::String(glyph_data.left_metrics_key.clone()),
                );
            }
            if !glyph_data.right_metrics_key.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.rightMetricsKey".into(),
                    plist::Value::String(glyph_data.right_metrics_key.clone()),
                );
            }
            for anchor in &glyph_data.anchors {
                glyph.anchors.push(norad::Anchor::new(
                    anchor.x,
                    anchor.y,
                    Some(
                        norad::Name::new(&anchor.name)
                            .map_err(|error| format!("アンカー名が不正です: {error}"))?,
                    ),
                    None,
                    None,
                    None,
                ));
            }
            for guide in &glyph_data.guidelines {
                let line = if guide.angle.abs() < f64::EPSILON {
                    norad::Line::Horizontal(guide.y)
                } else if (guide.angle - 90.0).abs() < f64::EPSILON {
                    norad::Line::Vertical(guide.x)
                } else {
                    norad::Line::Angle {
                        x: guide.x,
                        y: guide.y,
                        degrees: guide.angle,
                    }
                };
                let name = (!guide.name.is_empty())
                    .then(|| norad::Name::new(&guide.name))
                    .transpose()
                    .map_err(|error| format!("ガイド名が不正です: {error}"))?;
                glyph
                    .guidelines
                    .push(norad::Guideline::new(line, name, None, None, None));
            }
            if !glyph_data.master_guidelines.is_empty() {
                let master_guidelines = glyph_data
                    .master_guidelines
                    .iter()
                    .map(|(master_id, guides)| {
                        let values = guides
                            .iter()
                            .map(|guide| {
                                let mut value = plist::Dictionary::new();
                                value.insert("x".into(), plist::Value::Real(guide.x));
                                value.insert("y".into(), plist::Value::Real(guide.y));
                                value.insert("angle".into(), plist::Value::Real(guide.angle));
                                value.insert(
                                    "name".into(),
                                    plist::Value::String(guide.name.clone()),
                                );
                                plist::Value::Dictionary(value)
                            })
                            .collect();
                        (master_id.clone(), plist::Value::Array(values))
                    })
                    .collect();
                glyph.lib.insert(
                    "com.glyph-studio.masterGuidelines".into(),
                    plist::Value::Dictionary(master_guidelines),
                );
            }
            let mut codepoints = glyph_data.unicodes.clone();
            if let Some(unicode) = glyph_data.unicode {
                if !codepoints.contains(&unicode) {
                    codepoints.push(unicode);
                }
            }
            glyph.codepoints = codepoints.into_iter().filter_map(char::from_u32).collect();

            for contour_data in &glyph_data.contours {
                let mut norad_points = Vec::new();
                for point in &contour_data.points {
                    let point_type = match point.point_type {
                        PointType::OnCurve => norad::PointType::Line,
                        PointType::OffCurve => norad::PointType::OffCurve,
                    };
                    norad_points.push(norad::ContourPoint::new(
                        point.x,
                        point.y,
                        point_type,
                        point.smooth,
                        None,
                        None,
                        None,
                    ));
                }
                let contour = norad::Contour::new(norad_points, None, None);
                glyph.contours.push(contour);
            }
            for component in &glyph_data.components {
                glyph.components.push(norad::Component::new(
                    norad::Name::new(&component.base)
                        .map_err(|e| format!("コンポーネント名が不正です: {e}"))?,
                    norad::AffineTransform {
                        x_scale: component.x_scale,
                        xy_scale: component.xy_scale,
                        yx_scale: component.yx_scale,
                        y_scale: component.y_scale,
                        x_offset: component.x_offset,
                        y_offset: component.y_offset,
                    },
                    None,
                    None,
                ));
            }

            layer.insert_glyph(glyph);
        }

        for master in self
            .masters
            .iter()
            .filter(|master| master.id != self.default_master_id)
        {
            let layer_name = if master.name == "public.default"
                || font
                    .layers
                    .names()
                    .any(|name| name.to_string() == master.name)
            {
                &master.id
            } else {
                &master.name
            };
            let layer = font
                .layers
                .new_layer(layer_name)
                .map_err(|e| format!("UFOレイヤー作成エラー: {e}"))?;
            for glyph_data in self.glyphs.values() {
                let Some(layer_data) = glyph_data.layers.get(&master.id) else {
                    continue;
                };
                let mut glyph = norad::Glyph::new(&glyph_data.name);
                glyph.width = layer_data.width;
                if !glyph_data.left_kerning_group.is_empty() {
                    glyph.lib.insert(
                        "com.glyph-studio.leftKerningGroup".into(),
                        plist::Value::String(glyph_data.left_kerning_group.clone()),
                    );
                }
                if !glyph_data.right_kerning_group.is_empty() {
                    glyph.lib.insert(
                        "com.glyph-studio.rightKerningGroup".into(),
                        plist::Value::String(glyph_data.right_kerning_group.clone()),
                    );
                }
                for anchor in &layer_data.anchors {
                    glyph.anchors.push(norad::Anchor::new(
                        anchor.x,
                        anchor.y,
                        Some(
                            norad::Name::new(&anchor.name)
                                .map_err(|error| format!("アンカー名が不正です: {error}"))?,
                        ),
                        None,
                        None,
                        None,
                    ));
                }
                let mut codepoints = glyph_data.unicodes.clone();
                if let Some(unicode) = glyph_data.unicode {
                    if !codepoints.contains(&unicode) {
                        codepoints.push(unicode);
                    }
                }
                glyph.codepoints = codepoints.into_iter().filter_map(char::from_u32).collect();
                for contour_data in &layer_data.contours {
                    let points = contour_data
                        .points
                        .iter()
                        .map(|point| {
                            let point_type = match point.point_type {
                                PointType::OnCurve => norad::PointType::Line,
                                PointType::OffCurve => norad::PointType::OffCurve,
                            };
                            norad::ContourPoint::new(
                                point.x,
                                point.y,
                                point_type,
                                point.smooth,
                                None,
                                None,
                                None,
                            )
                        })
                        .collect();
                    glyph.contours.push(norad::Contour::new(points, None, None));
                }
                for component in &layer_data.components {
                    glyph.components.push(norad::Component::new(
                        norad::Name::new(&component.base)
                            .map_err(|e| format!("コンポーネント名が不正です: {e}"))?,
                        norad::AffineTransform {
                            x_scale: component.x_scale,
                            xy_scale: component.xy_scale,
                            yx_scale: component.yx_scale,
                            y_scale: component.y_scale,
                            x_offset: component.x_offset,
                            y_offset: component.y_offset,
                        },
                        None,
                        None,
                    ));
                }
                layer.insert_glyph(glyph);
            }
        }

        for ((left, right), value) in &self.kerning {
            let left = norad::Name::new(left)
                .map_err(|e| format!("カーニング左グリフ名が不正です: {e}"))?;
            let right = norad::Name::new(right)
                .map_err(|e| format!("カーニング右グリフ名が不正です: {e}"))?;
            font.kerning.entry(left).or_default().insert(right, *value);
        }
        if !self.kerning_by_master.is_empty() {
            let by_master = self
                .kerning_by_master
                .iter()
                .map(|(master_id, pairs)| {
                    let entries = pairs
                        .iter()
                        .map(|((left, right), value)| {
                            let mut entry = plist::Dictionary::new();
                            entry.insert("left".into(), plist::Value::String(left.clone()));
                            entry.insert("right".into(), plist::Value::String(right.clone()));
                            entry.insert("value".into(), plist::Value::Real(*value));
                            plist::Value::Dictionary(entry)
                        })
                        .collect();
                    (master_id.clone(), plist::Value::Array(entries))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.kerningByMaster".into(),
                plist::Value::Dictionary(by_master),
            );
        }

        Ok(font)
    }
}
