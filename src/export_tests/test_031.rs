    #[test]
    fn simple_gsub_accepts_context_on_both_sides_of_target() {
        let ids = [("A", 1_u16), ("B", 2_u16), ("C", 3_u16), ("B.alt", 4_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A B' C by B.alt; } calt;", &ids)
            .expect("two-sided contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
