    #[test]
    fn simple_gsub_expands_contextual_class_sequences() {
        let ids = [("A", 1_u16), ("A.alt", 2_u16), ("B", 3_u16), ("C", 4_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub [A B] C' by A.alt; } calt;", &ids)
            .expect("class contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
