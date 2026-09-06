    #[test]
    fn feature_block_extraction_ignores_comment_text() {
        let blocks = extract_feature_blocks(
            "# feature bad { } bad;\nfeature liga { # } feature fake {\n sub f i by fi;\n} liga;",
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
        assert!(blocks[0].1.contains("sub f i by fi"));
    }
