    #[test]
    fn variable_global_metrics_emit_mvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project
            .set_master_metrics(
                &project.masters[0].id.clone(),
                crate::font_data::MasterMetrics {
                    ascender: 800.0,
                    descender: -200.0,
                    line_gap: 0.0,
                },
            )
            .unwrap();
        project
            .set_master_metrics(
                &second.id,
                crate::font_data::MasterMetrics {
                    ascender: 900.0,
                    descender: -240.0,
                    line_gap: 20.0,
                },
            )
            .unwrap();
        let bytes = build_mvar(&project, &project.masters[0], &["wght".into()]);
        let bytes = bytes.expect("MVAR should be emitted");
        assert_eq!(&bytes[0..4], &[0, 1, 0, 0]);
        assert!(bytes.windows(4).any(|tag| tag == b"hasc"));
        assert!(bytes.windows(4).any(|tag| tag == b"hdsc"));
        assert!(bytes.windows(4).any(|tag| tag == b"hlgp"));
    }
