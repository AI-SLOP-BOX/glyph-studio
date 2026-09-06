    #[test]
    fn conditional_layer_selection_matches_axis_tags_case_insensitively() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.conditional_layers.insert(
            "A".into(),
            vec![ConditionalLayer {
                id: "uppercase-axis".into(),
                conditions: HashMap::from([(
                    "WGHT".into(),
                    AxisRange {
                        min: Some(700.0),
                        max: None,
                    },
                )]),
                layer: GlyphLayer {
                    width: 600.0,
                    contours: Vec::new(),
                    components: Vec::new(),
                    anchors: Vec::new(),
                },
            }],
        );
        let coordinates = HashMap::from([("wght".into(), 750.0)]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "uppercase-axis"
        );
    }
