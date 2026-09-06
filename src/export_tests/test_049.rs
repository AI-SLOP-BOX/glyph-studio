    #[test]
    fn exported_variable_ttf_contains_conditional_gsub_variations() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let layer = GlyphLayer {
            width: 600.0,
            contours: vec![Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        glyph.layers.insert("regular".into(), layer.clone());
        glyph.layers.insert("bold".into(), layer.clone());
        project.glyphs.insert("A".into(), glyph);
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        project.conditional_layers.insert(
            "A".into(),
            vec![crate::font_data::ConditionalLayer {
                id: "bold".into(),
                conditions: HashMap::from([(
                    "wght".into(),
                    crate::font_data::AxisRange {
                        min: Some(700.0),
                        max: None,
                    },
                )]),
                layer,
            }],
        );
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-conditional-gsub-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        let fonttools::font::Table::Unknown(gsub) = font.tables.get(b"GSUB").unwrap() else {
            panic!("GSUB should be serialized as raw bytes");
        };
        assert_eq!(&gsub[..4], &[0, 1, 0, 1]);
        assert!(gsub.windows(4).any(|window| window == b"rvrn"));
        let shape = |variation: &str| {
            std::process::Command::new("hb-shape")
                .arg(&path)
                .arg("A")
                .arg(format!("--variations={variation}"))
                .output()
                .expect("HarfBuzz should be available for variable-font verification")
        };
        let regular = shape("wght=400");
        let bold = shape("wght=700");
        assert!(regular.status.success());
        assert!(bold.status.success());
        assert_ne!(regular.stdout, bold.stdout);
        std::fs::remove_file(path).unwrap();
    }
