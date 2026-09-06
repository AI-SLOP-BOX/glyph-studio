    #[test]
    fn move_master_changes_order_without_touching_layer_data() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..FontMaster::default()
        });
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 777.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );

        assert!(project.move_master("bold", -1));
        assert_eq!(project.masters[0].id, "bold");
        assert_eq!(project.glyphs["A"].layers["bold"].width, 777.0);
        assert!(!project.move_master("bold", -1));
        assert!(!project.move_master("missing", 1));
    }
