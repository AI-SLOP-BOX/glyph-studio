    #[test]
    fn variable_ttf_contains_component_transform_variation() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), Some('A' as u32));
        project.add_glyph("accented".into(), Some('B' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        let component = |x_scale, xy_scale, yx_scale, y_scale, x_offset| GlyphComponent {
            base: "base".into(),
            x_scale,
            xy_scale,
            yx_scale,
            y_scale,
            x_offset,
            y_offset: 0.0,
        };
        project
            .glyphs
            .get_mut("accented")
            .unwrap()
            .components
            .push(component(1.0, 0.0, 0.0, 1.0, 0.0));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        for (name, glyph) in &mut project.glyphs {
            let layer = GlyphLayer {
                width: glyph.width,
                contours: glyph.contours.clone(),
                components: glyph.components.clone(),
                anchors: glyph.anchors.clone(),
            };
            glyph.layers.insert("regular".into(), layer.clone());
            let mut bold = layer;
            if name == "accented" {
                bold.components = vec![component(1.1, 0.2, -0.2, 0.9, 25.0)];
            }
            glyph.layers.insert("bold".into(), bold);
        }
        let mut flattened = project.clone();
        flatten_variation_components(&mut flattened).unwrap();
        let regular = &flattened.glyphs["accented"].layers["regular"];
        let bold = &flattened.glyphs["accented"].layers["bold"];
        assert!(regular.components.is_empty() && bold.components.is_empty());
        assert_ne!(regular.contours[0].points, bold.contours[0].points);
        project.masters.swap(0, 1);
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-component-variation-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"gvar"));
        assert!(font.tables.contains_key(b"STAT"));
        std::fs::remove_file(path).unwrap();
    }
