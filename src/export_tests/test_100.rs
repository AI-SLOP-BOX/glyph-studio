    #[test]
    fn cff2_export_is_readable_by_harfbuzz_when_available() {
        if std::process::Command::new("hb-shape")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(500.0, 0.0),
                ContourPoint::on_curve(250.0, 700.0),
            ],
        });
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-cff2-hb-{}-{:?}.otf",
            std::process::id(),
            std::thread::current().id()
        ));
        export_otf_cff2(&project, &path).unwrap();
        let result = std::process::Command::new("hb-shape")
            .arg(&path)
            .arg("A")
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            result.status.success(),
            "Harfbuzz could not read generated CFF2: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
