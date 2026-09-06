use super::*;

impl FontProject {
    fn norad_color_data(&self, font: &mut norad::Font) -> Result<(), String> {
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
        Ok(())
    }
}
