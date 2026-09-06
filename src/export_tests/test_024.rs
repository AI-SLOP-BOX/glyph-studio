    #[test]
    fn simple_gsub_supports_bracketed_multiple_replacements() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub("feature salt { sub A by [A.alt A.swash]; } salt;", &ids)
            .expect("bracketed multiple substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
