    #[test]
    fn feature_source_merges_classes_before_features() {
        let mut project = FontProject::new();
        project.opentype_classes = "@Upper = [A B];".into();
        project.opentype_features = "feature ccmp { sub @Upper by A; } ccmp;".into();
        assert_eq!(
            project.feature_source(),
            "@Upper = [A B];\n\nfeature ccmp { sub @Upper by A; } ccmp;"
        );
    }
