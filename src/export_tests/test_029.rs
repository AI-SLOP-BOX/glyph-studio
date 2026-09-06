    #[test]
    fn simple_gsub_accepts_marked_single_substitution_syntax() {
        let ids = [("A", 1_u16), ("A.alt", 2_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature salt { sub A' by A.alt; } salt;", &ids)
            .expect("marked single substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
