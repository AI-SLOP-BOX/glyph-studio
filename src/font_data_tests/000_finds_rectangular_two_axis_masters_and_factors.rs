    #[test]
    fn finds_rectangular_two_axis_masters_and_factors() {
        let master = |id: &str, x: f64, y: f64| FontMaster {
            id: id.into(),
            name: id.into(),
            weight: x,
            width: y,
            is_bracket: false,
            axes: [("wght".into(), x), ("wdth".into(), y)]
                .into_iter()
                .collect(),
        };
        let masters = vec![
            master("bl", 100.0, 75.0),
            master("br", 900.0, 75.0),
            master("tl", 100.0, 125.0),
            master("tr", 900.0, 125.0),
        ];
        let (indices, factors) = find_bilinear_masters(&masters, "wght", "wdth", 500.0, 100.0)
            .expect("complete rectangle");
        assert_eq!(indices, [0, 1, 2, 3]);
        assert_eq!(factors, (0.5, 0.5));
    }
