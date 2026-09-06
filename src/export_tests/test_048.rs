    #[test]
    fn exports_a_readable_ttf_with_outline_tables() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(500.0, 0.0),
                ContourPoint::on_curve(500.0, 700.0),
                ContourPoint::on_curve(0.0, 700.0),
            ],
        });
        let base_layer = GlyphLayer {
            width: glyph.width,
            contours: glyph.contours.clone(),
            components: glyph.components.clone(),
            anchors: glyph.anchors.clone(),
        };
        let mut target_layer = base_layer.clone();
        target_layer.width = 650.0;
        target_layer.contours[0].points[1].x = 550.0;
        glyph.layers.insert("regular".into(), base_layer);
        glyph.layers.insert("bold".into(), target_layer);
        project.glyphs.insert("A".into(), glyph);
        let mut b_glyph = GlyphData::new("B".into(), Some('B' as u32));
        b_glyph.width = 500.0;
        project.glyphs.insert("B".into(), b_glyph);
        project.opentype_features = "feature liga { sub A by B; } liga;".into();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 110.0,
            ..FontMaster::default()
        });
        project.instances.push(FontInstance {
            name: "Text Medium".into(),
            axes: HashMap::new(),
            weight: 550.0,
            width: 105.0,
        });
        project.kerning.insert(("A".into(), "A".into()), -50.0);
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(font.tables.contains_key(b"cmap"));
        assert!(font.tables.contains_key(b"hmtx"));
        assert!(font.tables.contains_key(b"kern"));
        assert!(font.tables.contains_key(b"GPOS"));
        assert!(font.tables.contains_key(b"GDEF"));
        assert!(font.tables.contains_key(b"OS/2"));
        assert!(font.tables.contains_key(b"fvar"));
        assert!(font.tables.contains_key(b"gvar"));
        let Some(fonttools::font::Table::Unknown(fvar_bytes)) = font.tables.get(b"fvar") else {
            panic!("fvar table was unexpectedly parsed");
        };
        assert_eq!(u16::from_be_bytes([fvar_bytes[12], fvar_bytes[13]]), 1);
        assert_eq!(u16::from_be_bytes([fvar_bytes[14], fvar_bytes[15]]), 12);
        let instance_offset = 16 + 40;
        assert_eq!(
            u16::from_be_bytes([fvar_bytes[instance_offset], fvar_bytes[instance_offset + 1]]),
            400
        );
        let weight = i32::from_be_bytes([
            fvar_bytes[instance_offset + 4],
            fvar_bytes[instance_offset + 5],
            fvar_bytes[instance_offset + 6],
            fvar_bytes[instance_offset + 7],
        ]) as f32
            / 65536.0;
        assert!((weight - 550.0).abs() < 0.01);
        let Some(fonttools::font::Table::Unknown(stat_bytes)) = font.tables.get(b"STAT") else {
            panic!("STAT table is missing");
        };
        assert!(stat_bytes.windows(2).any(|window| window == [1, 144]));
        assert!(font.tables.contains_key(b"GSUB"));
        assert!(font.tables.contains_key(b"name"));
        std::fs::remove_file(path).unwrap();
    }
