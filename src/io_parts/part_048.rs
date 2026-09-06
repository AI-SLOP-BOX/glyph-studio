
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_data::{
        Contour, ContourPoint, GlyphComponent, GlyphData,
    };

    include!("../io_tests/test_000.rs");
    include!("../io_tests/test_001.rs");
}
