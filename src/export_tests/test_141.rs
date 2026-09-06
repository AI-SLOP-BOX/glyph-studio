    #[test]
    fn nested_components_are_flattened_and_cycles_are_rejected() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut middle = GlyphData::new("middle".into(), None);
        middle.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 50.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("middle".into(), middle);
        let mut top = GlyphData::new("top".into(), None);
        top.components.push(GlyphComponent {
            base: "middle".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 75.0,
        });
        project.glyphs.insert("top".into(), top);
        let mut contours = Vec::new();
        append_contours(
            &project,
            "top",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut contours,
        )
        .unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0][0].x, 50);

        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "top".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        assert!(append_contours(
            &project,
            "top",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut Vec::new()
        )
        .is_err());
    }
