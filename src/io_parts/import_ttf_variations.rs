fn import_ttf_variations(face: &mut ttf_parser::Face<'_>, bytes: &[u8], project: &mut FontProject) {
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
                    master.axes.insert(default_axis.tag.to_string(), default_axis.def_value as f64);
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
        if let Ok(font_ref) = FontRef::new(bytes) {
            if let Ok(variable) = font_ref.fvar() {
                let names_table = font_ref.name().ok();
                let name_data = names_table.as_ref().map(|table| table.string_data());
                let instances = variable.instances().ok().into_iter().flat_map(|items| items.iter().filter_map(Result::ok));
                for (instance_index, instance) in instances.enumerate() {
                    let instance_name = names_table
                        .as_ref()
                        .and_then(|table| table.name_record().iter().find(|record| record.name_id() == instance.subfamily_name_id))
                        .and_then(|record| name_data.as_ref().and_then(|data| record.string(*data).ok()))
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
}
