    #[test]
    fn rejects_incomplete_two_axis_master_rectangle() {
        let masters = vec![
            FontMaster {
                id: "bl".into(),
                name: "bl".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 100.0), ("wdth".into(), 75.0)]
                    .into_iter()
                    .collect(),
            },
            FontMaster {
                id: "br".into(),
                name: "br".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 900.0), ("wdth".into(), 75.0)]
                    .into_iter()
                    .collect(),
            },
            FontMaster {
                id: "tl".into(),
                name: "tl".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 100.0), ("wdth".into(), 125.0)]
                    .into_iter()
                    .collect(),
            },
        ];
        assert!(find_bilinear_masters(&masters, "wght", "wdth", 500.0, 100.0).is_none());
    }
