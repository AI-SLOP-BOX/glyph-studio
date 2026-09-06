    #[test]
    fn feature_references_share_gsub_lookups() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "feature dlig { sub A by A.alt; } dlig; feature liga { feature dlig; } liga;";
        let bytes =
            build_simple_gsub(source, &glyph_ids).expect("feature reference should compile");
        assert!(bytes.windows(4).any(|window| window == b"dlig"));
        assert!(bytes.windows(4).any(|window| window == b"liga"));
    }
