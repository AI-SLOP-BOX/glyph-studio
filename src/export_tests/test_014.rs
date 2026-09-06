    #[test]
    fn gsub_feature_variations_emit_version_11_for_conditional_substitution() {
        let glyph_ids = HashMap::from([("A", 1_u16), ("A.cond", 2)]);
        let substitutions = vec![ConditionalSubstitution {
            base: "A".into(),
            alternate: "A.cond".into(),
            conditions: HashMap::from([(
                "WGHT".into(),
                crate::font_data::AxisRange {
                    min: Some(700.0),
                    max: None,
                },
            )]),
        }];
        let bounds = HashMap::from([(String::from("wght"), (0, 400.0, 400.0, 700.0))]);
        let bytes = build_simple_gsub_with_variations("", &glyph_ids, &substitutions, &bounds)
            .expect("conditional substitution should produce GSUB");
        assert_eq!(&bytes[..4], &[0, 1, 0, 1]);
        assert!(bytes.windows(4).any(|window| window == b"rvrn"));
    }
