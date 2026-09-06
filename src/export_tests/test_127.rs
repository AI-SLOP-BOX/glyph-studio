    #[test]
    fn exports_valid_woff_header_and_table_directory() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.woff", std::process::id()));
        export_woff(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"wOFF");
        assert!(u16::from_be_bytes([bytes[12], bytes[13]]) > 0);
        let length = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        assert_eq!(length, bytes.len());
        assert!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()) > 0);
        std::fs::remove_file(path).unwrap();
    }
