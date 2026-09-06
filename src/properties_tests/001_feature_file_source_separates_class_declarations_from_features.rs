    #[test]
    fn feature_file_source_separates_class_declarations_from_features() {
        let source = "@Upper = [A B];\n\nfeature liga { sub f i by fi; } liga;\n";
        let (classes, features) = split_feature_file_source(source);
        assert_eq!(classes, "@Upper = [A B];");
        assert_eq!(features, "\n\nfeature liga { sub f i by fi; } liga;\n");
    }
