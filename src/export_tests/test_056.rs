    #[test]
    fn project_validation_rejects_invalid_guidelines() {
        let mut project = FontProject::new();
        project.guidelines.push(crate::font_data::Guideline {
            x: f64::NAN,
            y: 0.0,
            angle: 0.0,
            name: String::new(),
        });
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("ガイド")));
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.guidelines.push(crate::font_data::Guideline {
            x: 0.0,
            y: 0.0,
            angle: f64::INFINITY,
            name: String::new(),
        });
        project.glyphs.insert("A".into(), glyph);
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("グリフ 'A' のガイド")));
    }
