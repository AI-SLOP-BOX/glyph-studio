    #[test]
    fn feature_source_parses_explicit_ligature_carets() {
        let ids = [("f_i", 1), ("f_f", 2), ("f_l", 3), ("f_t", 4)]
            .into_iter()
            .collect();
        let carets = parse_feature_ligature_carets(
            "table GDEF { LigatureCaretByPos f_i 300 600; LigatureCaretByIndex f_f 1 2; LigatureCaretByPos [f_l f_t] 500; } GDEF;",
            &ids,
        );
        assert_eq!(carets.len(), 4);
        assert!(matches!(
            carets[&GlyphId16::new(1)][0],
            gdef::CaretValue::Format1(_)
        ));
        assert!(matches!(
            carets[&GlyphId16::new(2)][0],
            gdef::CaretValue::Format2(_)
        ));
        assert!(carets.contains_key(&GlyphId16::new(3)));
        assert!(carets.contains_key(&GlyphId16::new(4)));
    }
