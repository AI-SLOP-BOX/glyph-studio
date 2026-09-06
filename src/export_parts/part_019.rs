
fn flatten_variation_components(project: &mut FontProject) -> Result<(), String> {
    let source = project.clone();
    let names = source.glyph_names_sorted();
    let master_ids: Vec<String> = source
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    for name in names {
        let mut base_contours = Vec::new();
        append_layer_contours(
            &source,
            name,
            None,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut base_contours,
        )?;
        let glyph = project
            .glyphs
            .get_mut(name)
            .ok_or_else(|| format!("グリフ '{}' がありません", name))?;
        glyph.contours = base_contours
            .into_iter()
            .map(|points| crate::font_data::Contour {
                points: points
                    .into_iter()
                    .map(|point| crate::font_data::ContourPoint {
                        x: f64::from(point.x),
                        y: f64::from(point.y),
                        point_type: if point.on_curve {
                            PointType::OnCurve
                        } else {
                            PointType::OffCurve
                        },
                        smooth: false,
                    })
                    .collect(),
            })
            .collect();
        glyph.components.clear();
        for master_id in &master_ids {
            let mut contours = Vec::new();
            append_layer_contours(
                &source,
                name,
                Some(master_id),
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut Vec::new(),
                &mut contours,
            )?;
            if let Some(layer) = glyph.layers.get_mut(master_id) {
                layer.contours = contours
                    .into_iter()
                    .map(|points| crate::font_data::Contour {
                        points: points
                            .into_iter()
                            .map(|point| crate::font_data::ContourPoint {
                                x: f64::from(point.x),
                                y: f64::from(point.y),
                                point_type: if point.on_curve {
                                    PointType::OnCurve
                                } else {
                                    PointType::OffCurve
                                },
                                smooth: false,
                            })
                            .collect(),
                    })
                    .collect();
                layer.components.clear();
            }
        }
    }
    Ok(())
}
