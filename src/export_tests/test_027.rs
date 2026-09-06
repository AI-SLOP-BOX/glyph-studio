    #[test]
    fn simple_gsub_expands_named_feature_classes() {
        let ids = [
            ("A", 1_u16),
            ("B", 2_u16),
            ("A.alt", 3_u16),
            ("B.alt", 4_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub(
            "@caps = [A B]; feature salt { sub @caps by [A.alt B.alt]; } salt;",
            &ids,
        )
        .expect("named class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
