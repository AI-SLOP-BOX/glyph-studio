    #[test]
    fn exports_selected_master_as_static_ttf() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![contour.clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 700.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-master-{}.ttf", std::process::id()));
        export_ttf_for_master(&project, "bold", &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(!font.tables.contains_key(b"fvar"));
        std::fs::remove_file(path).unwrap();
    }
