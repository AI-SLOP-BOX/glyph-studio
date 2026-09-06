    #[test]
    fn simple_gsub_supports_one_to_one_class_substitution() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2), ("A.alt", 3), ("B.alt", 4)]);
        let bytes = build_simple_gsub("feature ss01 { sub [A B] by [A.alt B.alt]; } ss01;", &ids)
            .expect("class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
