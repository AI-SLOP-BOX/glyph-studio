    #[test]
    fn duplicate_master_copies_metadata_and_every_glyph_layer() {
        let mut project = FontProject::new();
        project.masters[0].name = "Regular".into();
        project.masters[0].axes.insert("wght".into(), 400.0);
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.kerning_by_master.insert(
            "regular".into(),
            HashMap::from([(("A".into(), "A".into()), -70.0)]),
        );
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 612.0,
                contours: vec![Contour {
                    points: vec![ContourPoint::on_curve(10.0, 20.0)],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );

        let new_id = project.duplicate_master("regular").unwrap();
        assert_eq!(new_id, "regular.copy1");
        assert_eq!(project.masters[1].id, new_id);
        assert_eq!(project.masters[1].name, "Regular Copy");
        assert_eq!(project.masters[1].axes["wght"], 400.0);
        assert_eq!(project.glyphs["A"].layers[&new_id].width, 612.0);
        assert_eq!(
            project.glyphs["A"].layers[&new_id].contours[0].points[0].x,
            10.0
        );
        assert_eq!(
            project.kerning_by_master[&new_id][&("A".into(), "A".into())],
            -70.0
        );
        assert_eq!(
            project.duplicate_master("regular").unwrap(),
            "regular.copy2"
        );
    }
