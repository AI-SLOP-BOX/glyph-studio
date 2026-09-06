    #[test]
    fn simple_gsub_pairs_contextual_target_and_replacement_classes() {
        let ids = [
            ("A", 1_u16),
            ("C", 2_u16),
            ("D", 3_u16),
            ("C.alt", 4_u16),
            ("D.alt", 5_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub(
            "feature calt { sub A [C D]' by [C.alt D.alt]; } calt;",
            &ids,
        )
        .expect("class replacement contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
