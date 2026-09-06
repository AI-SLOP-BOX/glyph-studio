    #[test]
    fn languagesystems_survive_named_lookup_expansion() {
        let tags = vec![Tag::new(b"locl")];
        let source = "languagesystem latn dflt; lookup localizedI { sub i by i.loclTRK; } localizedI; feature locl { lookup localizedI; } locl;";
        let scripts = build_script_list(source, &tags);
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        assert!(scripts.script_records[0].script.default_lang_sys.is_some());
    }
