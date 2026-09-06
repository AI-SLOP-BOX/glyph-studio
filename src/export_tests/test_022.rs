    #[test]
    fn simple_gsub_supports_ligature_substitution() {
        let ids = HashMap::from([("f", 1_u16), ("i", 2), ("f_i", 3)]);
        let bytes = build_simple_gsub("feature liga { sub f i by f_i; } liga;", &ids)
            .expect("ligature substitution should produce GSUB");
        assert!(!bytes.is_empty());
        assert!(bytes.windows(4).any(|window| window == b"liga"));
    }
