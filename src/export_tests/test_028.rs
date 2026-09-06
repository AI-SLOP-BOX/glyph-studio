    #[test]
    fn simple_gsub_expands_named_classes_in_context_replacements() {
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
            "@ctx = [C D]; @alts = [C.alt D.alt]; feature calt { sub A @ctx' by @alts; } calt;",
            &ids,
        )
        .expect("named contextual replacement classes should produce GSUB");
        assert!(!bytes.is_empty());
    }
