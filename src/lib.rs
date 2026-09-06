pub mod app;
pub mod canvas;
#[allow(dead_code)]
pub mod cff;
pub mod export;
pub mod font_data;
pub mod generator;
pub mod glyph_list;
pub mod history;
pub mod io;
pub mod properties;
pub mod tools;

/// 外部ツールから利用する、フォント制作Coreの主要API。
pub mod core {
    pub use crate::export::{
        expand_named_feature_classes, export_all_otf_for_masters, export_all_svg,
        export_all_svg_for_master_with_palette, export_all_svg_with_palette,
        export_all_ttf_for_masters, export_all_woff2_for_masters, export_all_woff_for_masters,
        export_by_extension, export_interpolation_set, export_otf, export_otf_cff2,
        export_otf_for_master, export_svg, export_svg_with_palette, export_ttf,
        export_ttf_at_interpolation, export_ttf_for_master, export_woff, export_woff2,
        export_woff2_for_master, export_woff_for_master, extract_feature_blocks,
        validate_feature_class_definitions, validate_feature_glyph_references,
        validate_feature_source, validate_interpolation, validate_project,
        validate_project_detailed, ValidationIssue,
    };
    pub use crate::font_data::{
        AxisMappingPoint, FontInstance, FontProject, GlyphData, GlyphLayer,
    };
    pub use crate::generator::generate_all_japanese;
    pub use crate::io::{
        load_glyphs, load_project, load_svg, load_ttf, load_ufo, load_woff, load_woff2,
        save_glyphs, save_project, save_ufo,
    };

    use std::path::Path;

    /// プロジェクトを検証してから、出力先の拡張子に応じてフォントを生成する。
    pub fn build(project: &FontProject, output: &Path) -> Result<(), String> {
        if output
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("glyphs"))
        {
            return crate::io::save_glyphs(project, output);
        }
        let issues = validate_project(project);
        if !issues.is_empty() {
            return Err(format!(
                "書き出し前の検証に失敗しました: {}",
                issues.join("; ")
            ));
        }
        export_by_extension(project, output)
    }

    /// Updates the source-level OpenType classes and features after validating
    /// their combined Feature File syntax.
    pub fn set_opentype_source(
        project: &mut FontProject,
        classes: String,
        features: String,
    ) -> Result<(), String> {
        let previous_classes = std::mem::replace(&mut project.opentype_classes, classes);
        let previous_features = std::mem::replace(&mut project.opentype_features, features);
        if let Err(error) = crate::export::validate_feature_source(&project.feature_source()) {
            project.opentype_classes = previous_classes;
            project.opentype_features = previous_features;
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::core::set_opentype_source;
    use super::font_data::FontProject;

    #[test]
    fn opentype_source_api_rolls_back_on_invalid_syntax() {
        let mut project = FontProject::new();
        project.opentype_classes = "@Upper = [A];".into();
        project.opentype_features = "feature liga { sub A by A.alt; } liga;".into();
        let previous = project.clone();
        assert!(set_opentype_source(
            &mut project,
            "@Lower = [a];".into(),
            "feature liga {".into(),
        )
        .is_err());
        assert_eq!(project, previous);
    }
}
