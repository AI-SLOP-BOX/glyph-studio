    #[test]
    fn mark_attachment_classes_are_emitted_in_gdef() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.add_glyph("acute".into(), None);
        project.add_glyph("grave".into(), None);
        let ids = [("A", 1), ("acute", 2), ("grave", 3)].into_iter().collect();
        let source = "@Marks = [acute grave]; table GDEF { MarkAttachClassDef @Marks 3; } GDEF;";
        let bytes = build_gdef(&project, &ids, source).expect("GDEF should be emitted");
        let table = read_fonts::tables::gdef::Gdef::read(bytes.as_slice().into())
            .expect("generated GDEF should be readable");
        let class_def = table
            .mark_attach_class_def()
            .expect("mark attachment class definition should be present")
            .expect("mark attachment class definition should be valid");
        let read_fonts::tables::layout::ClassDef::Format2(class_def) = class_def else {
            panic!("mark attachment class definition should use format 2");
        };
        assert_eq!(class_def.class_range_count(), 2);
        assert_eq!(
            class_def.class_range_records()[0].start_glyph_id(),
            GlyphId16::new(2)
        );
        assert_eq!(class_def.class_range_records()[0].class(), 3);
    }
