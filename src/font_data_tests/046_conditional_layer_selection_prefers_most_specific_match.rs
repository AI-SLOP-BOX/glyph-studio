    #[test]
    fn conditional_layer_selection_prefers_most_specific_match() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        let layer = |id: &str, conditions| ConditionalLayer {
            id: id.into(),
            conditions,
            layer: GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![
                layer(
                    "weight",
                    HashMap::from([(
                        "wght".into(),
                        AxisRange {
                            min: Some(700.0),
                            max: None,
                        },
                    )]),
                ),
                layer(
                    "weight-width",
                    HashMap::from([
                        (
                            "wght".into(),
                            AxisRange {
                                min: Some(700.0),
                                max: None,
                            },
                        ),
                        (
                            "wdth".into(),
                            AxisRange {
                                min: Some(90.0),
                                max: Some(110.0),
                            },
                        ),
                        (
                            "opsz".into(),
                            AxisRange {
                                min: Some(12.0),
                                max: Some(18.0),
                            },
                        ),
                        (
                            "GRAD".into(),
                            AxisRange {
                                min: Some(0.0),
                                max: Some(100.0),
                            },
                        ),
                        (
                            "slnt".into(),
                            AxisRange {
                                min: Some(-15.0),
                                max: Some(0.0),
                            },
                        ),
                    ]),
                ),
            ],
        );
        let coordinates = HashMap::from([
            ("wght".into(), 750.0),
            ("wdth".into(), 100.0),
            ("opsz".into(), 14.0),
            ("GRAD".into(), 50.0),
            ("slnt".into(), -10.0),
        ]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "weight-width"
        );
    }
