    #[test]
    fn feature_source_parses_gdef_attach_points() {
        let ids = [("A", 1), ("B", 2), ("C", 3)].into_iter().collect();
        let attach = parse_feature_attach_points(
            "table GDEF { Attach [A B] 7 2 7; Attach C 4; } GDEF;",
            &ids,
        )
        .expect("attach list should be emitted");
        assert_eq!(attach.attach_points.len(), 3);
        assert_eq!(attach.attach_points[0].point_indices, vec![2, 7]);
        assert_eq!(attach.attach_points[2].point_indices, vec![4]);
    }
