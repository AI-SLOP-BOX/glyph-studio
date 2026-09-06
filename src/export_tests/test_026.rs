    #[test]
    fn simple_gsub_preserves_multiple_feature_tags() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2)]);
        let bytes = build_simple_gsub(
            "feature liga { sub A by B; } liga; feature calt { sub B by A; } calt;",
            &ids,
        )
        .expect("multiple feature tags should produce GSUB");
        assert!(bytes.windows(4).any(|window| window == b"liga"));
        assert!(bytes.windows(4).any(|window| window == b"calt"));
    }
