    #[test]
    fn feature_source_expands_named_value_record_definitions() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let source =
            "valueRecordDef <0 0 -80 0> KERN_POS; feature kern { pos A V <KERN_POS>; } kern;";
        let bytes = build_kerning_gpos(&project, &ids, source)
            .expect("named value record should compile into GPOS");
        assert!(bytes.windows(4).any(|window| window == b"kern"));
        let named_single = build_kerning_gpos(
            &project,
            &ids,
            "valueRecordDef -80 KERN_POS; feature kern { pos A V <KERN_POS>; } kern;",
        )
        .expect("single-value named record should compile into GPOS");
        let shorthand = build_kerning_gpos(&project, &ids, "feature kern { pos A V -80; } kern;")
            .expect("short pair value should compile into GPOS");
        assert_eq!(named_single, shorthand);
    }
