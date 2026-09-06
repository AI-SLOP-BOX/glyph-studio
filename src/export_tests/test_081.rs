    #[test]
    fn feature_table_overrides_apply_to_head_and_hhea() {
        let mut project = FontProject::new();
        let source = "table head { FontRevision 2.75; Flags 0x5; MacStyle 0x3; LowestRecPPEM 9; FontDirectionHint -1; } head; table hhea { Ascender 900; Descender -250; LineGap 40; CaretSlopeRise 2; CaretSlopeRun -1; CaretOffset 3; } hhea; table post { ItalicAngle -12.5; UnderlinePosition -110; UnderlineThickness 55; IsFixedPitch 1; } post; table OS/2 { TypoAscender 920; TypoDescender -260; TypoLineGap 42; XHeight 500; CapHeight 700; FSType 8; FsSelection 0x140; DefaultChar 0x25A1; BreakChar 0x20; MaxContext 7; YSubscriptXSize 300; YSubscriptYSize 280; YSubscriptXOffset 12; YSubscriptYOffset -18; YSuperscriptXSize 310; YSuperscriptYSize 290; YSuperscriptXOffset 14; YSuperscriptYOffset 420; YStrikeoutSize 35; YStrikeoutPosition 310; SFamilyClass 4660; LowerOpticalPointSize 9; UpperOpticalPointSize 72; WinAscent 1200; WinDescent 350; Panose 2 11 6 3 5 4 2 2 2 4; } OS/2;";
        apply_feature_table_overrides(&mut project, source);
        assert!((project.metadata.font_revision - 2.75).abs() < f64::EPSILON);
        assert!((project.metadata.italic_angle + 12.5).abs() < f64::EPSILON);
        assert_eq!(project.metadata.underline_position, -110.0);
        assert_eq!(project.metadata.underline_thickness, 55.0);
        assert!(project.metadata.is_fixed_pitch);
        assert_eq!(project.metadata.x_height, 500.0);
        assert_eq!(project.metadata.cap_height, 700.0);
        assert_eq!(project.metadata.fs_type, 8);
        assert_eq!(project.metadata.fs_selection, 0x140);
        assert_eq!(project.metadata.default_char, 0x25A1);
        assert_eq!(project.metadata.break_char, 0x20);
        assert_eq!(project.metadata.max_context, 7);
        assert_eq!(os2_selection_flags(&project.metadata), 0x140);
        assert_eq!(project.metadata.head_flags, 5);
        assert_eq!(project.metadata.head_mac_style, 3);
        assert_eq!(project.metadata.lowest_rec_ppem, 9);
        assert_eq!(project.metadata.font_direction_hint, -1);
        assert_eq!(project.metadata.caret_slope_rise, 2);
        assert_eq!(project.metadata.caret_slope_run, -1);
        assert_eq!(project.metadata.caret_offset, 3);
        assert_eq!(project.metadata.panose, [2, 11, 6, 3, 5, 4, 2, 2, 2, 4]);
        assert_eq!(project.metadata.subscript_x_size, 300);
        assert_eq!(project.metadata.superscript_y_offset, 420);
        assert_eq!(project.metadata.strikeout_position, 310);
        assert_eq!(project.metadata.family_class, 4660);
        assert_eq!(project.metadata.lower_optical_point_size, 9);
        assert_eq!(project.metadata.upper_optical_point_size, 72);
        assert_eq!(project.metadata.win_ascent, 1200);
        assert_eq!(project.metadata.win_descent, 350);
        let metrics = project.master_metrics_for(&project.default_master_id);
        assert_eq!(metrics.ascender, 920.0);
        assert_eq!(metrics.descender, -260.0);
        assert_eq!(metrics.line_gap, 42.0);
    }
