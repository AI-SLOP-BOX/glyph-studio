    #[test]
    fn simple_gsub_compiles_null_substitution_as_deletion() {
        let ids = HashMap::from([("A", 1_u16)]);
        let bytes = build_simple_gsub("feature ccmp { sub A by NULL; } ccmp;", &ids)
            .expect("NULL substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
