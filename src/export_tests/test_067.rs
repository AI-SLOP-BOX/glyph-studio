    #[test]
    fn contextual_positioning_is_applied_by_harfbuzz_when_available() {
        if std::process::Command::new("hb-shape")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("V".into(), Some('V' as u32));
        project.opentype_features = "feature ccmp { pos A' V <0 0 -100 0>; } ccmp;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-context-pos-{}-{:?}.ttf",
            std::process::id(),
            std::thread::current().id()
        ));
        export_ttf(&project, &path).unwrap();
        let result = std::process::Command::new("hb-shape")
            .arg(&path)
            .arg("AV")
            .arg("--features=ccmp")
            .arg("--output-format=json")
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            result.status.success(),
            "HarfBuzz could not shape contextual GPOS: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let shaped = String::from_utf8_lossy(&result.stdout);
        assert!(
            shaped.contains("\"ax\":500"),
            "unexpected shaping: {shaped}"
        );
    }
