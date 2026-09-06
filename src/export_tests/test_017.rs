    #[test]
    fn feature_classes_accept_optional_commas() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2), ("A.alt", 3), ("B.alt", 4)]);
        let bytes = build_simple_gsub("feature ss01 { sub [A, B] by [A.alt, B.alt]; } ss01;", &ids)
            .expect("comma-separated class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
