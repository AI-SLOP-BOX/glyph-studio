    #[test]
    fn nested_feature_references_reach_a_transitive_parent() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "feature dlig { sub A by A.alt; } dlig; feature liga { feature dlig; } liga; feature calt { feature liga; } calt;";
        let bytes = build_simple_gsub(source, &glyph_ids)
            .expect("nested feature references should compile");
        assert!(bytes.windows(4).any(|window| window == b"dlig"));
        assert!(bytes.windows(4).any(|window| window == b"liga"));
        assert!(bytes.windows(4).any(|window| window == b"calt"));
    }
