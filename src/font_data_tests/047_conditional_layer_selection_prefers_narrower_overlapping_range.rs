    #[test]
    fn conditional_layer_selection_prefers_narrower_overlapping_range() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        let layer = |id: &str, min: f64, max: f64| ConditionalLayer {
            id: id.into(),
            conditions: HashMap::from([(
                "wght".into(),
                AxisRange {
                    min: Some(min),
                    max: Some(max),
                },
            )]),
            layer: GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![layer("wide", 600.0, 900.0), layer("narrow", 700.0, 800.0)],
        );
        let coordinates = HashMap::from([("wght".into(), 750.0)]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "narrow"
        );
    }
