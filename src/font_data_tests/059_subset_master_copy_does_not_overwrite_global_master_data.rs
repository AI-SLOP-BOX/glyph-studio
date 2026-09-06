    #[test]
    fn subset_master_copy_does_not_overwrite_global_master_data() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), Some(66));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project.kerning_by_master.insert(
            "regular".into(),
            HashMap::from([(("A".into(), "V".into()), -80.0)]),
        );
        project.kerning_by_master.insert(
            "bold".into(),
            HashMap::from([(("A".into(), "V".into()), -120.0)]),
        );
        project.guidelines_by_master.insert(
            "regular".into(),
            vec![Guideline {
                x: 0.0,
                y: 700.0,
                angle: 0.0,
                name: "regular".into(),
            }],
        );
        project.guidelines_by_master.insert(
            "bold".into(),
            vec![Guideline {
                x: 0.0,
                y: 720.0,
                angle: 0.0,
                name: "bold".into(),
            }],
        );
        project.copy_master_to_all_for_glyphs("regular", ["A"]);
        assert_eq!(
            project.kerning_by_master["bold"][&("A".into(), "V".into())],
            -120.0
        );
        assert_eq!(project.guidelines_by_master["bold"][0].y, 720.0);
    }
