    #[test]
    fn feature_block_extraction_requires_identifier_boundaries() {
        let blocks = extract_feature_blocks(
            "myfeature liga { sub A by B; } liga;\nfeature liga { sub A by B; } liga;",
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
    }
