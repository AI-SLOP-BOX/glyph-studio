    #[test]
    fn master_svg_export_uses_the_requested_layer_geometry() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 500.0;
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 800.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(700.0, 0.0),
                        ContourPoint::on_curve(700.0, 700.0),
                    ],
                }],
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
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-master-{}", std::process::id()));
        export_all_svg_for_master(&project, "bold", &directory).unwrap();
        let svg = std::fs::read_to_string(directory.join("A.svg")).unwrap();
        assert!(svg.contains("viewBox=\"0 -800 800 1000\""));
        assert!(svg.contains("700 -0"));
        std::fs::remove_dir_all(directory).unwrap();
    }
