    #[test]
    fn center_glyphs_in_width_centers_each_master_independently() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 200.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(10.0, 0.0),
                ContourPoint::on_curve(110.0, 0.0),
                ContourPoint::on_curve(110.0, 100.0),
            ],
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 400.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(50.0, 0.0),
                        ContourPoint::on_curve(150.0, 0.0),
                        ContourPoint::on_curve(150.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);

        assert_eq!(project.center_glyphs_in_width(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].contours[0].points[0].x, 50.0);
        assert_eq!(
            project.glyphs["A"].layers["bold"].contours[0].points[0].x,
            150.0
        );
    }
