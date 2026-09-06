    #[test]
    fn color_tables_encode_cpal_v1_palette_labels() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.layer".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]], vec![[0, 0, 0, 255]]];
        project.color_palette_names = vec!["Light".into(), "Dark".into()];
        project.color_palette_entry_names = vec!["Primary".into()];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.layer".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let ids = [("A", 1), ("A.layer", 2)].into_iter().collect();
        let (_, cpal) = build_color_tables(&project, &ids).unwrap();
        assert_eq!(u16::from_be_bytes([cpal[0], cpal[1]]), 1);
        assert_eq!(
            u32::from_be_bytes([cpal[8], cpal[9], cpal[10], cpal[11]]),
            44
        );
        assert_eq!(
            u32::from_be_bytes([cpal[16], cpal[17], cpal[18], cpal[19]]),
            28
        );
        assert_eq!(
            u32::from_be_bytes([cpal[20], cpal[21], cpal[22], cpal[23]]),
            36
        );
        assert_eq!(
            u32::from_be_bytes([cpal[24], cpal[25], cpal[26], cpal[27]]),
            40
        );
        assert_eq!(u16::from_be_bytes([cpal[12], cpal[13]]), 0);
        assert_eq!(u16::from_be_bytes([cpal[14], cpal[15]]), 1);
        assert_eq!(u16::from_be_bytes([cpal[36], cpal[37]]), 1000);
        assert_eq!(u16::from_be_bytes([cpal[38], cpal[39]]), 1001);
        assert_eq!(u16::from_be_bytes([cpal[40], cpal[41]]), 2000);
        assert_eq!(&cpal[44..48], &[0, 0, 255, 255]);
        assert_eq!(&cpal[48..52], &[0, 0, 0, 255]);
    }
