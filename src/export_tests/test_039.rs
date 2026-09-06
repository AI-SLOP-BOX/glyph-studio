    #[test]
    fn postscript_name_is_ascii_safe() {
        assert_eq!(postscript_name("My Font!", "Regular"), "MyFont-Regular");
        assert_eq!(postscript_name("日本語", "標準"), "Font-Font");
        assert!(postscript_name(&"A".repeat(100), "Regular").len() <= 63);
        assert!(!postscript_name("---", "Regular").starts_with('-'));
    }
