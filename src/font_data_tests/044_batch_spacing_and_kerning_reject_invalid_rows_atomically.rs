    #[test]
    fn batch_spacing_and_kerning_reject_invalid_rows_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.add_glyph("B".into(), None);

        assert!(project
            .set_kerning_pairs([("A", "B", -40.0), ("A", "missing", f64::NAN)])
            .is_err());
        assert!(project.kerning.is_empty());

        assert!(project
            .set_side_bearings_batch([("A", 20.0, 20.0), ("missing", 20.0, 20.0)])
            .is_err());
        assert_eq!(project.glyphs["A"].width, 600.0);

        assert!(project
            .set_widths_batch([("A", 500.0), ("missing", 600.0)])
            .is_err());
        assert_eq!(project.glyphs["A"].width, 600.0);

        assert!(project
            .set_unicode_assignments_strict(&[("A".into(), 0x41), ("B".into(), 0x41)])
            .is_err());
        assert_eq!(project.glyphs["A"].unicode, None);
    }
