    #[test]
    fn variation_axis_coordinates_are_normalized_around_default() {
        assert_eq!(normalize_axis(400.0, 400.0, 400.0, 700.0), 0.0);
        assert_eq!(normalize_axis(700.0, 400.0, 400.0, 700.0), 1.0);
        assert_eq!(normalize_axis(300.0, 300.0, 500.0, 700.0), -1.0);
        assert_eq!(normalize_axis(600.0, 300.0, 500.0, 700.0), 0.5);
    }
