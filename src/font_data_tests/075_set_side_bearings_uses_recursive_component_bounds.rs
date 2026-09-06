    #[test]
    fn set_side_bearings_uses_recursive_component_bounds() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("acute".into(), Some('Á' as u32));
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
        project
            .glyphs
            .get_mut("acute")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 200.0,
                y_offset: 0.0,
            });
        assert_eq!(project.set_side_bearings(&["acute".into()], 20.0, 30.0), 1);
        let glyph = &project.glyphs["acute"];
        assert_eq!(glyph.width, 150.0);
        assert_eq!(glyph.components[0].x_offset, 20.0);
    }
