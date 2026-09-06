    #[test]
    fn master_guidelines_follow_switch_and_remain_independent() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 100.0,
                y: 200.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].guidelines_for_master("bold").len(), 1);
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("bold")
            .push(Guideline {
                x: 120.0,
                y: 300.0,
                angle: 90.0,
                name: "bold-cap".into(),
            });
        project.sync_active_layer("bold");
        project.switch_master("bold", "regular");
        assert_eq!(project.glyphs["A"].guidelines.len(), 1);
        assert_eq!(project.glyphs["A"].guidelines[0].x, 100.0);
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].guidelines.len(), 2);
        assert_eq!(project.glyphs["A"].guidelines[1].x, 120.0);
    }
