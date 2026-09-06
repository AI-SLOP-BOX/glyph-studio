    #[test]
    fn simple_gsub_compiles_ignore_substitution_rules() {
        let ids = HashMap::from([("A", 1_u16), ("acute", 2), ("Aacute", 3)]);
        let bytes = build_simple_gsub(
            "feature ccmp { ignore sub A acute; sub A acute by Aacute; } ccmp;",
            &ids,
        )
        .expect("ignore substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }
