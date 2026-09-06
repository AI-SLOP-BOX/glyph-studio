
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_data::{
        Contour, ContourPoint, FontMaster, GlyphComponent, GlyphData, GlyphLayer,
    };

    include!("../io_tests/test_000.rs");
    include!("../io_tests/test_001.rs");
}
