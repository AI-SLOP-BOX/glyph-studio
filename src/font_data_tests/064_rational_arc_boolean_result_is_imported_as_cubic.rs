    #[test]
    fn rational_arc_boolean_result_is_imported_as_cubic() {
        let arc = i_curve::float::arc::EllipticArc {
            ellipse: i_curve::float::arc::Ellipse {
                center: [0.0, 0.0],
                radius_x: 100.0,
                radius_y: 100.0,
                rotation: 0.0,
            },
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        };
        let mut builder = i_curve::CurveBuilder::new();
        builder.move_to([100.0, 0.0]).unwrap();
        builder.arc_to(arc).unwrap();
        builder.line_to([100.0, 0.0]).unwrap();
        builder.close_contour().unwrap();
        let path = builder.build().unwrap().into_contours().remove(0);
        let contour = Contour::from_curve_path(path).unwrap();

        assert_eq!(contour.points.len(), 4);
        assert!(contour.points[0].is_on_curve());
        assert_eq!(contour.points[1].point_type, PointType::OffCurve);
        assert_eq!(contour.points[2].point_type, PointType::OffCurve);
        assert!(contour.points[3].is_on_curve());
        assert!((contour.points[3].x - 0.0).abs() < 1.0e-9);
        assert!((contour.points[3].y - 100.0).abs() < 1.0e-9);
    }
