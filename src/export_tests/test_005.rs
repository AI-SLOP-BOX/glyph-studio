    #[test]
    fn feature_glyph_class_definition_overrides_inferred_gdef_class() {
        let glyph_ids = HashMap::from([("A", 1), ("f_i", 2), ("acute", 3), ("part", 4)]);
        let classes = parse_feature_glyph_classes(
            "table GDEF { GlyphClassDef [A], [f_i], [acute], [part]; } GDEF;",
            &glyph_ids,
        );
        assert_eq!(classes[&GlyphId16::new(1)], gdef::GlyphClassDef::Base);
        assert_eq!(classes[&GlyphId16::new(2)], gdef::GlyphClassDef::Ligature);
        assert_eq!(classes[&GlyphId16::new(3)], gdef::GlyphClassDef::Mark);
        assert_eq!(classes[&GlyphId16::new(4)], gdef::GlyphClassDef::Component);
    }
