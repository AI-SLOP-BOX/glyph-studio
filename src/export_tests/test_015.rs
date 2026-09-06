    #[test]
    fn conditional_axis_bounds_follow_fvar_axis_order() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut bold = FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 110.0,
            ..FontMaster::default()
        };
        bold.axes.insert("opsz".into(), 14.0);
        project.masters[0].axes.insert("opsz".into(), 10.0);
        project.masters.push(bold);
        project.default_master_id = "bold".into();
        let layer = GlyphLayer {
            width: 600.0,
            contours: Vec::new(),
            components: Vec::new(),
            anchors: Vec::new(),
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![
                crate::font_data::ConditionalLayer {
                    id: "wide".into(),
                    conditions: HashMap::from([(
                        "wght".into(),
                        crate::font_data::AxisRange {
                            min: Some(600.0),
                            max: Some(900.0),
                        },
                    )]),
                    layer: layer.clone(),
                },
                crate::font_data::ConditionalLayer {
                    id: "narrow".into(),
                    conditions: HashMap::from([(
                        "wght".into(),
                        crate::font_data::AxisRange {
                            min: Some(700.0),
                            max: Some(800.0),
                        },
                    )]),
                    layer,
                },
            ],
        );
        project.add_glyph(".cond.A.narrow-1".into(), None);
        let (substitutions, bounds) = materialize_conditional_substitutions(&mut project);
        assert_eq!(bounds["opsz"].0, 0);
        assert_eq!(bounds["wdth"].0, 1);
        assert_eq!(bounds["opsz"].2, 14.0);
        assert!(substitutions[0].alternate.contains("narrow"));
        assert_ne!(substitutions[0].alternate, ".cond.A.narrow-1");
        let alternate = project.glyphs.get(&substitutions[0].alternate).unwrap();
        assert!(alternate.unicode.is_none());
        assert!(alternate.unicodes.is_empty());
    }
