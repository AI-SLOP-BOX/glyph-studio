    #[test]
    fn feature_parameters_cover_stylistic_set_character_variant_and_size() {
        let stylistic = feature_params_for_tag(Tag::new(b"ss03"), "", &BTreeMap::new());
        assert!(matches!(
            stylistic,
            Some(layout::FeatureParams::StylisticSet(_))
        ));
        let character = feature_params_for_tag(Tag::new(b"cv07"), "", &BTreeMap::new());
        assert!(matches!(
            character,
            Some(layout::FeatureParams::CharacterVariant(_))
        ));
        let character = feature_params_for_tag(
            Tag::new(b"cv07"),
            "feature cv07 { sub A by A.cv07; } cv07;",
            &BTreeMap::from([("A".to_string(), 0x41)]),
        );
        let Some(layout::FeatureParams::CharacterVariant(character)) = character else {
            panic!("character variant parameters should be parsed");
        };
        assert_eq!(character.character, vec![Uint24::new(0x41)]);
        let size = feature_params_for_tag(
            Tag::new(b"size"),
            "feature size { parameters 12 2 8 72; } size;",
            &BTreeMap::new(),
        );
        let Some(layout::FeatureParams::Size(size)) = size else {
            panic!("size feature parameters should be parsed");
        };
        assert_eq!(size.design_size, 12);
        assert_eq!(size.identifier, 2);
        assert_eq!(size.range_start, 8);
        assert_eq!(size.range_end, 72);
    }
