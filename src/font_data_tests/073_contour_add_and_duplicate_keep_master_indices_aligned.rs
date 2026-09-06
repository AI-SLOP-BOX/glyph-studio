    #[test]
    fn contour_add_and_duplicate_keep_master_indices_aligned() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(10.0, 20.0),
                ContourPoint::on_curve(110.0, 20.0),
                ContourPoint::on_curve(10.0, 120.0),
            ],
        };
        assert_eq!(
            project.add_contour_all_layers("A", contour.clone()),
            Some(0)
        );
        assert_eq!(project.duplicate_contour_all_layers("A", 0), Some(1));
        assert_eq!(project.glyphs["A"].contours.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].contours.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].contours[0], contour);
        assert!(project.duplicate_contour_all_layers("A", 99).is_none());
    }
