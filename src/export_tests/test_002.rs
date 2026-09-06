    #[test]
    fn exported_ttf_contains_horizontal_and_vertical_base_axes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let path =
            std::env::temp_dir().join(format!("glyph-studio-base-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        let base = font.base().unwrap();
        for axis in [base.horiz_axis(), base.vert_axis()] {
            let axis = axis.unwrap().unwrap();
            let tags = axis.base_tag_list().unwrap().unwrap().baseline_tags();
            assert_eq!(
                tags,
                &[
                    Tag::new(b"hang"),
                    Tag::new(b"ideo"),
                    Tag::new(b"math"),
                    Tag::new(b"romn"),
                ]
            );
            assert_eq!(
                axis.base_script_list().unwrap().base_script_records().len(),
                5
            );
        }
        std::fs::remove_file(path).unwrap();
    }
