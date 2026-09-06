    #[test]
    fn master_guidelines_translate_with_geometry() {
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.guidelines.push(Guideline {
            x: 10.0,
            y: 20.0,
            angle: 0.0,
            name: String::new(),
        });
        glyph.master_guidelines.insert(
            "bold".into(),
            vec![Guideline {
                x: 30.0,
                y: 40.0,
                angle: 90.0,
                name: String::new(),
            }],
        );
        glyph.translate_geometry(5.0, -7.0);
        assert_eq!(glyph.guidelines[0].x, 15.0);
        assert_eq!(glyph.guidelines[0].y, 13.0);
        assert_eq!(glyph.guidelines_for_master("bold")[0].x, 35.0);
        assert_eq!(glyph.guidelines_for_master("bold")[0].y, 33.0);
    }
