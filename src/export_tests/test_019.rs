    #[test]
    fn simple_gsub_synthesizes_aalt_from_feature_alternates() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub(
            "feature salt { sub A by A.alt; } salt; feature ss01 { sub A by A.swash; } ss01;",
            &ids,
        )
        .expect("automatic aalt should produce GSUB");
        assert!(bytes.windows(4).any(|window| window == b"aalt"));
    }
