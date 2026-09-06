    #[test]
    fn exports_static_otf_with_cff_table() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: None,
                alpha: 0.42,
            }],
        );
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.otf", std::process::id()));
        export_otf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert_eq!(&std::fs::read(&path).unwrap()[0..4], b"OTTO");
        assert!(font.tables.contains_key(b"CFF "));
        assert!(!font.tables.contains_key(b"glyf"));
        assert!(font.tables.contains_key(b"COLR"));
        assert!(font.tables.contains_key(b"CPAL"));
        assert!(font.tables.contains_key(b"SVG "));
        let loaded = crate::io::load_ttf(&path).unwrap();
        assert_eq!(loaded.color_layers["A"][0].glyph, "A.red");
        assert!((loaded.color_layers["A"][0].alpha - 0.42).abs() < 0.01);
        assert_eq!(loaded.color_palettes[0][0], [255, 0, 0, 255]);
        let bytes = std::fs::read(&path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, 0).is_ok());
        std::fs::remove_file(path).unwrap();
    }
