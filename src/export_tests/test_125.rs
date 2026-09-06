    #[test]
    fn exports_interpolated_static_ttf() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let regular = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let bold = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(200.0, 0.0),
                ContourPoint::on_curve(200.0, 200.0),
            ],
        };
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![regular],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 700.0,
                contours: vec![bold],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project
            .set_vertical_metrics_for_master("A", "regular", 1000.0, 800.0)
            .unwrap();
        project
            .set_vertical_metrics_for_master("A", "bold", 1200.0, 600.0)
            .unwrap();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-instance-{}.ttf", std::process::id()));
        export_ttf_at_interpolation(&project, "regular", "bold", 0.5, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(!font.tables.contains_key(b"fvar"));
        let imported = crate::io::load_ttf(&path).unwrap();
        assert_eq!(imported.vertical_metrics["A"].advance_height, 1100.0);
        assert_eq!(imported.vertical_metrics["A"].top_side_bearing, 700.0);
        std::fs::remove_file(path).unwrap();
    }
