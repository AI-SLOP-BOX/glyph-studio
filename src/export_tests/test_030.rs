    #[test]
    fn simple_gsub_accepts_contextual_marked_substitution_syntax() {
        let ids = [("A", 1_u16), ("B", 2_u16), ("A.alt", 3_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A' B by A.alt; } calt;", &ids)
            .expect("contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
