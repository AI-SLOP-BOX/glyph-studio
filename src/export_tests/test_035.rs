    #[test]
    fn feature_blocks_are_extracted_with_nested_brace_boundaries() {
        let blocks = extract_feature_blocks(
            "feature liga { lookup L { sub f i by f_i; } L; } liga;\nfeature salt { sub A by A.swash; } salt;",
        );
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
        assert!(blocks[0].1.contains("sub f i by f_i"));
        assert_eq!(blocks[1].0, Tag::new(b"salt"));
    }
