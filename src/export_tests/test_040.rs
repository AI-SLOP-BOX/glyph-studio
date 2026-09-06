    #[test]
    fn style_flags_follow_weight_and_italic_metadata() {
        let mut metadata = FontMetadata::default();
        assert_eq!(mac_style_flags(&metadata), 0);
        assert_eq!(os2_selection_flags(&metadata), 0x1C0);
        metadata.weight_class = 700;
        metadata.style_name = "Bold Italic".into();
        metadata.italic_angle = -12.0;
        assert_eq!(mac_style_flags(&metadata), 3);
        assert_eq!(os2_selection_flags(&metadata), 0x1A1);
        assert_eq!(
            max_feature_context("feature liga { sub f i j by f_i_j; } liga;"),
            3
        );
    }
