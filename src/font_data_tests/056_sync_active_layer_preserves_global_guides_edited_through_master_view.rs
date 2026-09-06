    #[test]
    fn sync_active_layer_preserves_global_guides_edited_through_master_view() {
        let mut project = FontProject::new();
        project
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 0.0,
                y: 680.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        assert_eq!(project.guidelines.len(), 1);
        assert_eq!(project.guidelines_for_master("regular")[0].y, 680.0);
    }
