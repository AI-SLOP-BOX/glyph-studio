    #[test]
    fn legacy_metadata_defaults_are_restored() {
        let path =
            std::env::temp_dir().join(format!("glyph-studio-legacy-{}.json", std::process::id()));
        std::fs::write(&path, r#"{
            "metadata":{"family_name":"Legacy","style_name":"Regular","units_per_em":1000.0,"ascender":800.0,"descender":-200.0,"line_gap":0.0},
            "glyphs":{},"kerning":{}
        }"#).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.metadata.weight_class, 400);
        assert_eq!(loaded.metadata.width_class, 5);
        assert_eq!(loaded.metadata.x_height, 0.0);
        assert_eq!(loaded.metadata.cap_height, 0.0);
        std::fs::remove_file(path).unwrap();
