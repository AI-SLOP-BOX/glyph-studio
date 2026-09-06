    #[test]
    fn remove_master_cleans_layers_metrics_and_background_data() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..FontMaster::default()
        });
        project.axis_names.insert("oldx".into(), "Old Axis".into());
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project
            .vertical_metrics_by_master
            .insert("bold".into(), HashMap::new());
        project.background_images.insert(
            "A".into(),
            HashMap::from([("bold".into(), "/tmp/A.png".into())]),
        );
        project
            .background_opacities
            .insert("A".into(), HashMap::from([("bold".into(), 0.5)]));
        project.background_transforms.insert(
            "A".into(),
            HashMap::from([(
                "bold".into(),
                BackgroundImageTransform {
                    x: 10.0,
                    y: 20.0,
                    scale: 1.0,
                    rotation: 5.0,
                    flip_x: false,
                    flip_y: false,
                },
            )]),
        );
        assert!(project.remove_master("bold"));
        assert_eq!(project.masters.len(), 1);
        assert_eq!(project.default_master_id, "regular");
        assert!(!project.glyphs["A"].layers.contains_key("bold"));
        assert!(project.vertical_metrics_by_master.is_empty());
        assert!(project.background_images.is_empty());
        assert!(project.background_opacities.is_empty());
        assert!(project.background_transforms.is_empty());
        assert!(project.axis_names.is_empty());
        project.switch_master("bold", "regular");
        assert!(!project.kerning_by_master.contains_key("bold"));
        assert!(!project.guidelines_by_master.contains_key("bold"));
        assert!(!project.glyphs["A"].layers.contains_key("bold"));
        assert!(!project.remove_master("regular"));
    }
