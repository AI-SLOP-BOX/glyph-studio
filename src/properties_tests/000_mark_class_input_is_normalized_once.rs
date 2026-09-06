    #[test]
    fn mark_class_input_is_normalized_once() {
        assert_eq!(normalize_mark_class("top"), "@top");
        assert_eq!(normalize_mark_class(" @top "), "@top");
    }
