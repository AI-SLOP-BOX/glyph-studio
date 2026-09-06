    #[test]
    fn feature_source_device_tables_compile_into_gpos() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let source = "feature kern { pos A V <-80 0 -160 0 <device 11 -1, 12 -1> <device NULL> <device 11 -2, 12 -2> <device NULL>>; } kern;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }
