    #[test]
    fn operation_is_appended_when_no_block_exists() {
        let mut source = String::new();
        insert_feature_operation(&mut source, "kern", "    pos A V <0 0 -80 0>;\n");
        assert_eq!(
            source,
            "feature kern {\n    pos A V <0 0 -80 0>;\n} kern;\n"
        );
    }
