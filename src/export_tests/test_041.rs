    #[test]
    fn avar_contains_normalized_axis_mapping_and_identity_axes() {
        let tags = vec!["wght".into(), "wdth".into()];
        let mappings = std::collections::HashMap::from([(
            "wght".into(),
            vec![AxisMappingPoint {
                input: 0.5,
                output: 0.25,
            }],
        )]);
        let bytes = build_avar(&tags, &mappings).expect("nonlinear mapping should emit avar");
        assert_eq!(&bytes[..8], &[0, 1, 0, 0, 0, 0, 0, 2]);
        assert_eq!(u16::from_be_bytes([bytes[8], bytes[9]]), 4);
        assert_eq!(u16::from_be_bytes([bytes[26], bytes[27]]), 3);
    }
