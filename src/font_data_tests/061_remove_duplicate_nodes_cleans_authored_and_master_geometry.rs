    #[test]
    fn remove_duplicate_nodes_cleans_authored_and_master_geometry() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
                ContourPoint::on_curve(0.0, 0.0),
            ],
        };
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.remove_duplicate_nodes(&["A".into()]), 4);
        assert_eq!(project.glyphs["A"].contours[0].points.len(), 3);
        assert_eq!(
            project.glyphs["A"].layers["regular"].contours[0]
                .points
                .len(),
            3
        );
        let mut fragile = GlyphData::new("fragile".into(), None);
        fragile.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(10.0, 10.0),
            ],
        });
        project.glyphs.insert("fragile".into(), fragile);
        assert_eq!(project.remove_duplicate_nodes(&["fragile".into()]), 0);
        assert_eq!(project.glyphs["fragile"].contours[0].points.len(), 3);
    }
