    #[test]
    fn outline_bounds_include_transformed_component_geometry() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 200.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 3.0,
            x_offset: 50.0,
            y_offset: -20.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.outline_bounds_for_glyph("composite"),
            Some((50.0, -20.0, 250.0, 580.0))
        );
    }
