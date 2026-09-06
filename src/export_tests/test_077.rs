    #[test]
    fn lookup_flags_parse_open_type_feature_qualifiers() {
        let flags = parse_lookup_flags(
            "lookupflag IgnoreMarks; lookupflag IgnoreLigatures; lookupflag MarkAttachmentType 3;",
        );
        assert!(flags.contains(layout::LookupFlag::IGNORE_MARKS));
        assert!(flags.contains(layout::LookupFlag::IGNORE_LIGATURES));
        assert_eq!(flags.mark_attachment_class(), Some(3));
    }
