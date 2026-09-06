    #[test]
    fn simple_gsub_expands_class_at_contextual_target() {
        let ids = [
            ("A", 1_u16),
            ("B", 2_u16),
            ("C", 3_u16),
            ("D", 4_u16),
            ("E", 5_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A [C D]' by E; } calt;", &ids)
            .expect("class target contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
