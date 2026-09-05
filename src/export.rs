use crate::cff;
use crate::font_data::{
    AxisMappingPoint, FontInstance, FontMaster, FontMetadata, FontProject, GlyphData, PointType,
    UnicodeVariationSequence,
};
use flate2::{write::ZlibEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use write_fonts::tables::{base, gdef, gpos, gsub, layout};
use write_fonts::types::{GlyphId16, NameId, Tag, Uint24};

#[derive(Debug, Clone)]
struct ConditionalSubstitution {
    base: String,
    alternate: String,
    conditions: std::collections::HashMap<String, crate::font_data::AxisRange>,
}
type AxisBounds = std::collections::HashMap<String, (u16, f64, f64, f64)>;
type ReverseSubstitution = (
    Tag,
    Vec<GlyphId16>,
    Vec<Vec<GlyphId16>>,
    Vec<Vec<GlyphId16>>,
    Vec<GlyphId16>,
);

type Transform = (f64, f64, f64, f64, f64, f64);

#[allow(dead_code)]
pub fn export_svg(project: &FontProject, glyph_name: &str, path: &Path) -> Result<(), String> {
    export_svg_with_palette(project, glyph_name, 0, path)
}

fn append_svg_gradient_defs<'a>(
    svg: &mut String,
    gradients: impl Iterator<Item = (usize, &'a crate::font_data::ColorGradient)>,
    palette: &[[u8; 4]],
) {
    let gradients = gradients.collect::<Vec<_>>();
    if gradients.is_empty() {
        return;
    }
    svg.push_str("<defs>\n");
    for (index, gradient) in gradients {
        write_svg_gradient_def(
            svg,
            &format!("glyph-studio-gradient-{index}"),
            gradient,
            palette,
        );
    }
    svg.push_str("</defs>\n");
}

fn write_svg_gradient_def(
    svg: &mut String,
    id: &str,
    gradient: &crate::font_data::ColorGradient,
    palette: &[[u8; 4]],
) {
    let (tag, attributes) = match gradient.kind {
        crate::font_data::ColorGradientKind::Linear => (
            "linearGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                gradient.x0, gradient.y0, gradient.x1, gradient.y1
            ),
        ),
        crate::font_data::ColorGradientKind::Radial => (
            "radialGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\"",
                gradient.x1, gradient.y1, gradient.radius1, gradient.x0, gradient.y0
            ),
        ),
        crate::font_data::ColorGradientKind::Sweep => (
            "linearGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                gradient.x0, gradient.y0, gradient.x1, gradient.y1
            ),
        ),
    };
    let spread = match gradient.extend {
        crate::font_data::ColorGradientExtend::Pad => "pad",
        crate::font_data::ColorGradientExtend::Repeat => "repeat",
        crate::font_data::ColorGradientExtend::Reflect => "reflect",
    };
    writeln!(
        svg,
        "<{tag} id=\"{id}\" spreadMethod=\"{spread}\" {attributes}>"
    )
    .ok();
    for stop in gradient.effective_stops() {
        let color = palette
            .get(usize::from(stop.palette_index))
            .copied()
            .unwrap_or([0, 0, 0, 0]);
        writeln!(
            svg,
            "<stop offset=\"{}\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{}\" />",
            stop.offset,
            color[0],
            color[1],
            color[2],
            (f64::from(color[3]) / 255.0 * stop.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        )
        .ok();
    }
    writeln!(svg, "</{tag}>").ok();
}

fn svg_color_layer_transform(project: &FontProject, base_name: &str, index: usize) -> String {
    let Some(Some(transform)) = project
        .color_layer_transforms
        .get(base_name)
        .and_then(|transforms| transforms.get(index))
    else {
        return String::new();
    };
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        transform.xx, transform.yx, transform.xy, transform.yy, transform.dx, transform.dy
    )
}

fn nested_svg_gradient_id(path: &[usize]) -> String {
    let suffix = path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-");
    format!("glyph-studio-nested-gradient-{suffix}")
}

fn append_svg_nested_gradient_defs(
    project: &FontProject,
    base_name: &str,
    palette: &[[u8; 4]],
    path: &mut Vec<usize>,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<bool, String> {
    if stack.iter().any(|item| item == base_name) {
        return Err(format!(
            "カラーグリフ循環参照: {} -> {}",
            stack.join(" -> "),
            base_name
        ));
    }
    let layers = project
        .color_layers
        .get(base_name)
        .ok_or_else(|| format!("カラーグリフ '{}' がありません", base_name))?;
    let is_root = stack.is_empty();
    stack.push(base_name.to_string());
    let mut found = false;
    for (index, layer) in layers.iter().enumerate() {
        path.push(index);
        if !is_root {
            if let Some(gradient) = layer.gradient.as_ref() {
                write_svg_gradient_def(svg, &nested_svg_gradient_id(path), gradient, palette);
                found = true;
            }
        }
        if project.color_layers.contains_key(&layer.glyph) {
            found |=
                append_svg_nested_gradient_defs(project, &layer.glyph, palette, path, stack, svg)?;
        }
        path.pop();
    }
    stack.pop();
    Ok(found)
}

fn append_svg_nested_color_layers(
    project: &FontProject,
    base_name: &str,
    palette: &[[u8; 4]],
    path: &mut Vec<usize>,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<(), String> {
    if stack.iter().any(|item| item == base_name) {
        return Err(format!(
            "カラーグリフ循環参照: {} -> {}",
            stack.join(" -> "),
            base_name
        ));
    }
    let layers = project
        .color_layers
        .get(base_name)
        .ok_or_else(|| format!("カラーグリフ '{}' がありません", base_name))?;
    stack.push(base_name.to_string());
    for (index, layer) in layers.iter().enumerate() {
        path.push(index);
        let color = palette
            .get(usize::from(layer.palette_index))
            .copied()
            .unwrap_or([0, 0, 0, 255]);
        let is_nested = project.color_layers.contains_key(&layer.glyph);
        let opacity = if is_nested || layer.gradient.is_some() {
            layer.alpha.clamp(0.0, 1.0)
        } else {
            (f64::from(color[3]) / 255.0 * layer.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        };
        let transform = svg_color_layer_transform(project, base_name, index);
        let fill = layer.gradient.as_ref().map_or_else(
            || {
                if is_nested {
                    "none".to_string()
                } else {
                    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
                }
            },
            |_| format!("url(#{})", nested_svg_gradient_id(path)),
        );
        writeln!(
            svg,
            "<g fill=\"{fill}\" fill-opacity=\"{opacity:.6}\" fill-rule=\"nonzero\"{transform}>"
        )
        .map_err(|error| error.to_string())?;
        if project.color_layers.contains_key(&layer.glyph) {
            append_svg_nested_color_layers(project, &layer.glyph, palette, path, stack, svg)?;
        } else {
            append_svg_contours(
                project,
                &layer.glyph,
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                stack,
                svg,
            )?;
        }
        svg.push_str("</g>\n");
        path.pop();
    }
    stack.pop();
    Ok(())
}

pub fn export_svg_with_palette(
    project: &FontProject,
    glyph_name: &str,
    palette_index: usize,
    path: &Path,
) -> Result<(), String> {
    if !project.glyphs.contains_key(glyph_name) {
        return Err(format!("グリフ '{}' がありません", glyph_name));
    }
    let glyph_width = project.glyphs[glyph_name].width.max(1.0);
    let top = project.metadata.ascender.max(0.0);
    let bottom = project.metadata.descender.min(0.0);
    let mut svg = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 {} {} {}\">\n",
        -top,
        glyph_width,
        (top - bottom).max(1.0)
    );
    if let Some(layers) = project.color_layers.get(glyph_name) {
        let palette = project
            .color_palettes
            .get(palette_index)
            .or_else(|| project.color_palettes.first())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        append_svg_gradient_defs(
            &mut svg,
            layers.iter().enumerate().filter_map(|(index, layer)| {
                layer.gradient.as_ref().map(|gradient| (index, gradient))
            }),
            palette,
        );
        let mut nested_definitions = String::new();
        if append_svg_nested_gradient_defs(
            project,
            glyph_name,
            palette,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut nested_definitions,
        )? {
            svg.push_str("<defs>\n");
            svg.push_str(&nested_definitions);
            svg.push_str("</defs>\n");
        }
        for (index, layer) in layers.iter().enumerate() {
            let Some(color) = project
                .color_palettes
                .get(palette_index)
                .and_then(|palette| palette.get(usize::from(layer.palette_index)))
            else {
                continue;
            };
            let is_nested = project.color_layers.contains_key(&layer.glyph);
            let fill = layer.gradient.as_ref().map_or_else(
                || {
                    if is_nested {
                        "none".to_string()
                    } else {
                        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
                    }
                },
                |_| format!("url(#glyph-studio-gradient-{index})"),
            );
            let opacity = if is_nested || layer.gradient.is_some() {
                layer.alpha.clamp(0.0, 1.0)
            } else {
                (f64::from(color[3]) / 255.0 * layer.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
            };
            let transform = svg_color_layer_transform(project, glyph_name, index);
            writeln!(
                svg,
                "<g fill=\"{fill}\" fill-opacity=\"{opacity:.6}\" fill-rule=\"nonzero\"{transform}>"
            )
            .map_err(|e| e.to_string())?;
            if project.color_layers.contains_key(&layer.glyph) {
                append_svg_nested_color_layers(
                    project,
                    &layer.glyph,
                    palette,
                    &mut vec![index],
                    &mut vec![glyph_name.to_string()],
                    &mut svg,
                )?;
            } else {
                append_svg_contours(
                    project,
                    &layer.glyph,
                    (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                    &mut Vec::new(),
                    &mut svg,
                )?;
            }
            svg.push_str("</g>\n");
        }
    } else {
        svg.push_str("<g fill=\"black\" fill-rule=\"nonzero\">\n");
        append_svg_contours(
            project,
            glyph_name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut svg,
        )?;
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    std::fs::write(path, svg).map_err(|e| format!("SVG保存エラー: {e}"))
}

#[allow(dead_code)]
pub fn export_all_svg(project: &FontProject, directory: &Path) -> Result<usize, String> {
    export_all_svg_with_palette(project, 0, directory)
}

pub fn export_all_svg_with_palette(
    project: &FontProject,
    palette_index: usize,
    directory: &Path,
) -> Result<usize, String> {
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("SVG出力先を作成できません: {error}"))?;
    let mut exported = 0;
    let mut used_names = std::collections::HashSet::new();
    for glyph_name in project.glyph_names_sorted() {
        let base_name: String = glyph_name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        if base_name.is_empty() {
            continue;
        }
        let mut safe_name = base_name.clone();
        let mut suffix = 2;
        while !used_names.insert(safe_name.clone()) {
            safe_name = format!("{base_name}_{suffix}");
            suffix += 1;
        }
        export_svg_with_palette(
            project,
            glyph_name,
            palette_index,
            &directory.join(format!("{safe_name}.svg")),
        )?;
        exported += 1;
    }
    Ok(exported)
}

#[allow(dead_code)]
pub fn export_all_svg_for_master(
    project: &FontProject,
    master_id: &str,
    directory: &Path,
) -> Result<usize, String> {
    if !project.masters.iter().any(|master| master.id == master_id) {
        return Err(format!("マスター '{}' がありません", master_id));
    }
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    export_all_svg(&selected, directory)
}

pub fn export_all_svg_for_master_with_palette(
    project: &FontProject,
    master_id: &str,
    palette_index: usize,
    directory: &Path,
) -> Result<usize, String> {
    if !project.masters.iter().any(|master| master.id == master_id) {
        return Err(format!("マスター '{}' がありません", master_id));
    }
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    export_all_svg_with_palette(&selected, palette_index, directory)
}

fn append_svg_contours(
    project: &FontProject,
    name: &str,
    transform: Transform,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("コンポーネント循環参照: {}", stack.join(" -> ")));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("参照グリフ '{}' がありません", name))?;
    stack.push(name.to_string());
    let map = |p: kurbo::Point| {
        (
            transform.0 * p.x + transform.1 * p.y + transform.4,
            transform.2 * p.x + transform.3 * p.y + transform.5,
        )
    };
    for contour in &glyph.contours {
        write!(svg, "<path d=\"").map_err(|e| e.to_string())?;
        for element in contour.to_bezpath().segments() {
            match element {
                kurbo::PathSeg::Line(line) => {
                    let a = map(line.p0);
                    let b = map(line.p1);
                    write!(svg, "M {} {} L {} {} ", a.0, -a.1, b.0, -b.1)
                }
                kurbo::PathSeg::Quad(q) => {
                    let a = map(q.p0);
                    let b = map(q.p1);
                    let c = map(q.p2);
                    write!(
                        svg,
                        "M {} {} Q {} {} {} {} ",
                        a.0, -a.1, b.0, -b.1, c.0, -c.1
                    )
                }
                kurbo::PathSeg::Cubic(c) => {
                    let a = map(c.p0);
                    let b = map(c.p1);
                    let d = map(c.p2);
                    let e = map(c.p3);
                    write!(
                        svg,
                        "M {} {} C {} {} {} {} {} {} ",
                        a.0, -a.1, b.0, -b.1, d.0, -d.1, e.0, -e.1
                    )
                }
            }
            .map_err(|e| e.to_string())?;
        }
        svg.push_str("Z\"/>\n");
    }
    for component in &glyph.components {
        let t = (
            transform.0 * component.x_scale + transform.1 * component.yx_scale,
            transform.0 * component.xy_scale + transform.1 * component.y_scale,
            transform.2 * component.x_scale + transform.3 * component.yx_scale,
            transform.2 * component.xy_scale + transform.3 * component.y_scale,
            transform.0 * component.x_offset + transform.1 * component.y_offset + transform.4,
            transform.2 * component.x_offset + transform.3 * component.y_offset + transform.5,
        );
        append_svg_contours(project, &component.base, t, stack, svg)?;
    }
    stack.pop();
    Ok(())
}

fn append_contours(
    project: &FontProject,
    name: &str,
    transform: Transform,
    stack: &mut Vec<String>,
    output: &mut Vec<Vec<fonttools::glyf::Point>>,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("コンポーネント循環参照: {}", stack.join(" -> ")));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("参照グリフ '{}' がありません", name))?;
    stack.push(name.to_string());
    for contour in &glyph.contours {
        if contour.points.len() < 3 {
            return Err(format!("グリフ '{}' に不完全な輪郭があります", name));
        }
        output.push(
            contour
                .points
                .iter()
                .map(|point| {
                    let x = transform.0 * point.x + transform.1 * point.y + transform.4;
                    let y = transform.2 * point.x + transform.3 * point.y + transform.5;
                    Ok(fonttools::glyf::Point {
                        x: checked_i16(x, "コンポーネントX座標")?,
                        y: checked_i16(y, "コンポーネントY座標")?,
                        on_curve: point.point_type == PointType::OnCurve,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
    }
    for component in &glyph.components {
        let t = (
            transform.0 * component.x_scale + transform.1 * component.yx_scale,
            transform.0 * component.xy_scale + transform.1 * component.y_scale,
            transform.2 * component.x_scale + transform.3 * component.yx_scale,
            transform.2 * component.xy_scale + transform.3 * component.y_scale,
            transform.0 * component.x_offset + transform.1 * component.y_offset + transform.4,
            transform.2 * component.x_offset + transform.3 * component.y_offset + transform.5,
        );
        append_contours(project, &component.base, t, stack, output)?;
    }
    stack.pop();
    Ok(())
}

fn append_layer_contours(
    project: &FontProject,
    name: &str,
    master_id: Option<&str>,
    transform: Transform,
    stack: &mut Vec<String>,
    output: &mut Vec<Vec<fonttools::glyf::Point>>,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("コンポーネント循環参照: {}", stack.join(" -> ")));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("参照グリフ '{}' がありません", name))?;
    let (contours, components) = master_id
        .and_then(|id| glyph.layers.get(id))
        .map(|layer| (layer.contours.clone(), layer.components.clone()))
        .unwrap_or_else(|| (glyph.contours.clone(), glyph.components.clone()));
    stack.push(name.to_string());
    for contour in &contours {
        if contour.points.len() < 3 {
            return Err(format!("グリフ '{}' に不完全な輪郭があります", name));
        }
        output.push(
            contour
                .points
                .iter()
                .map(|point| {
                    Ok(fonttools::glyf::Point {
                        x: checked_i16(
                            transform.0 * point.x + transform.1 * point.y + transform.4,
                            "可変コンポーネントX座標",
                        )?,
                        y: checked_i16(
                            transform.2 * point.x + transform.3 * point.y + transform.5,
                            "可変コンポーネントY座標",
                        )?,
                        on_curve: point.point_type == PointType::OnCurve,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
    }
    for component in &components {
        let child_transform = (
            transform.0 * component.x_scale + transform.1 * component.yx_scale,
            transform.0 * component.xy_scale + transform.1 * component.y_scale,
            transform.2 * component.x_scale + transform.3 * component.yx_scale,
            transform.2 * component.xy_scale + transform.3 * component.y_scale,
            transform.0 * component.x_offset + transform.1 * component.y_offset + transform.4,
            transform.2 * component.x_offset + transform.3 * component.y_offset + transform.5,
        );
        append_layer_contours(
            project,
            &component.base,
            master_id,
            child_transform,
            stack,
            output,
        )?;
    }
    stack.pop();
    Ok(())
}

fn flatten_variation_components(project: &mut FontProject) -> Result<(), String> {
    let source = project.clone();
    let names = source.glyph_names_sorted();
    let master_ids: Vec<String> = source
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    for name in names {
        let mut base_contours = Vec::new();
        append_layer_contours(
            &source,
            name,
            None,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut base_contours,
        )?;
        let glyph = project
            .glyphs
            .get_mut(name)
            .ok_or_else(|| format!("グリフ '{}' がありません", name))?;
        glyph.contours = base_contours
            .into_iter()
            .map(|points| crate::font_data::Contour {
                points: points
                    .into_iter()
                    .map(|point| crate::font_data::ContourPoint {
                        x: f64::from(point.x),
                        y: f64::from(point.y),
                        point_type: if point.on_curve {
                            PointType::OnCurve
                        } else {
                            PointType::OffCurve
                        },
                        smooth: false,
                    })
                    .collect(),
            })
            .collect();
        glyph.components.clear();
        for master_id in &master_ids {
            let mut contours = Vec::new();
            append_layer_contours(
                &source,
                name,
                Some(master_id),
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut Vec::new(),
                &mut contours,
            )?;
            if let Some(layer) = glyph.layers.get_mut(master_id) {
                layer.contours = contours
                    .into_iter()
                    .map(|points| crate::font_data::Contour {
                        points: points
                            .into_iter()
                            .map(|point| crate::font_data::ContourPoint {
                                x: f64::from(point.x),
                                y: f64::from(point.y),
                                point_type: if point.on_curve {
                                    PointType::OnCurve
                                } else {
                                    PointType::OffCurve
                                },
                                smooth: false,
                            })
                            .collect(),
                    })
                    .collect();
                layer.components.clear();
            }
        }
    }
    Ok(())
}

/// Export project outlines, Unicode mappings, and horizontal metrics as TrueType.
fn materialize_conditional_substitutions(
    project: &mut FontProject,
) -> (Vec<ConditionalSubstitution>, AxisBounds) {
    let base = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .cloned()
        .unwrap_or_default();
    let default_master = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first());
    let mut axis_tags: Vec<String> = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|tag| {
            let first = default_master
                .and_then(|master| master.axes.get(tag))
                .copied()
                .unwrap_or(0.0);
            project.masters.iter().any(|master| {
                (master.axes.get(tag).copied().unwrap_or(0.0) - first).abs() > f64::EPSILON
            })
        })
        .collect();
    let has_width_axis = project
        .masters
        .iter()
        .any(|master| (master.width - base.width).abs() > f64::EPSILON);
    if axis_tags.is_empty() {
        axis_tags.push("wght".into());
    }
    if has_width_axis && !axis_tags.iter().any(|tag| tag == "wdth") {
        axis_tags.push("wdth".into());
    }
    let mut axis_bounds = AxisBounds::new();
    for (index, tag) in axis_tags.into_iter().enumerate() {
        let values: Vec<f64> = project
            .masters
            .iter()
            .map(|master| match tag.as_str() {
                "wght" => master.axes.get(&tag).copied().unwrap_or(master.weight),
                "wdth" => master.axes.get(&tag).copied().unwrap_or(master.width),
                _ => master.axes.get(&tag).copied().unwrap_or(0.0),
            })
            .collect();
        let Some(default) = default_master
            .map(|master| match tag.as_str() {
                "wght" => master.axes.get(&tag).copied().unwrap_or(master.weight),
                "wdth" => master.axes.get(&tag).copied().unwrap_or(master.width),
                _ => master.axes.get(&tag).copied().unwrap_or(0.0),
            })
            .or_else(|| values.first().copied())
        else {
            continue;
        };
        let min = values.iter().copied().fold(default, f64::min);
        let max = values.iter().copied().fold(default, f64::max);
        axis_bounds.insert(tag, (index as u16, min, default, max));
    }
    let mut substitutions = Vec::new();
    for (base_name, layers) in project.conditional_layers.clone() {
        let Some(base_glyph) = project.glyphs.get(&base_name).cloned() else {
            continue;
        };
        for (index, conditional_layer) in layers.into_iter().enumerate() {
            let suffix: String = conditional_layer
                .id
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        character
                    } else {
                        '_'
                    }
                })
                .collect();
            let name_stem = format!(".cond.{base_name}.{suffix}-{index}");
            let mut alternate_name = name_stem.clone();
            let mut collision = 1;
            while project.glyphs.contains_key(&alternate_name) {
                alternate_name = format!("{name_stem}-{collision}");
                collision += 1;
            }
            let mut alternate = base_glyph.clone();
            alternate.name = alternate_name.clone();
            alternate.unicode = None;
            alternate.unicodes.clear();
            alternate.width = conditional_layer.layer.width;
            alternate.contours = conditional_layer.layer.contours.clone();
            alternate.components = conditional_layer.layer.components.clone();
            alternate.anchors = conditional_layer.layer.anchors.clone();
            for master in &project.masters {
                alternate
                    .layers
                    .insert(master.id.clone(), conditional_layer.layer.clone());
            }
            project.glyphs.insert(alternate_name.clone(), alternate);
            substitutions.push(ConditionalSubstitution {
                base: base_name.clone(),
                alternate: alternate_name,
                conditions: conditional_layer.conditions,
            });
        }
    }
    substitutions.sort_by(|left, right| {
        let specificity = right.conditions.len().cmp(&left.conditions.len());
        if specificity != std::cmp::Ordering::Equal {
            return specificity;
        }
        let span = |substitution: &ConditionalSubstitution| {
            substitution.conditions.values().fold(0.0, |total, range| {
                total
                    + match (range.min, range.max) {
                        (Some(min), Some(max)) => (max - min).max(0.0),
                        _ => f64::INFINITY,
                    }
            })
        };
        span(left)
            .partial_cmp(&span(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (substitutions, axis_bounds)
}

pub fn export_ttf(project: &FontProject, path: &Path) -> Result<(), String> {
    use fonttools::cmap::{cmap, CmapSubtable};
    use fonttools::font::{Font, SfntVersion, Table};
    use fonttools::fvar::{fvar, InstanceRecord, VariationAxisRecord};
    use fonttools::glyf::{glyf, Glyph};
    use fonttools::gvar::gvar;
    use fonttools::head::head;
    use fonttools::hhea::hhea;
    use fonttools::hmtx::{hmtx, Metric};
    use fonttools::maxp::maxp;
    use fonttools::name::{name, NameRecord, NameRecordID};
    use fonttools::os2::{os2, Panose};
    use fonttools::post::post;

    let validation_issues = validate_project(project);
    if !validation_issues.is_empty() {
        return Err(format!(
            "フォント検証に失敗しました: {}",
            validation_issues.join("; ")
        ));
    }

    let mut project = project.clone();
    // UFO/JSON files from older versions may contain an axis on only some
    // masters. Fill those coordinates from the default master before building
    // fvar/gvar so every exported instance has the same axis count.
    project.normalize_masters();
    let (conditional_substitutions, axis_bounds) =
        materialize_conditional_substitutions(&mut project);
    let export_master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .map(|master| master.id.clone())
        .or_else(|| project.masters.first().map(|master| master.id.clone()));
    if let Some(master_id) = export_master_id {
        if let Some(kerning) = project.kerning_by_master.get(&master_id).cloned() {
            project.kerning = kerning;
        }
        for glyph in project.glyphs.values_mut() {
            if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                glyph.width = layer.width;
                glyph.contours = layer.contours;
                glyph.components = layer.components;
                glyph.anchors = layer.anchors;
            }
        }
    }
    if project.masters.len() >= 2 {
        flatten_variation_components(&mut project)?;
    }

    let upm = checked_u16(project.metadata.units_per_em, "UPM")?;
    let source_before_table_overrides = project.feature_source();
    apply_feature_table_overrides(&mut project, &source_before_table_overrides);
    let feature_source = project.feature_source();
    let preserve_imported_layout = project.preserved_layout_source.as_deref()
        == Some(feature_source.as_str())
        && project.preserved_layout_fingerprint == Some(layout_input_fingerprint(&project));
    let preserve_gsub = preserve_imported_layout && project.preserved_tables.contains_key("GSUB");
    let preserve_gpos = preserve_imported_layout && project.preserved_tables.contains_key("GPOS");
    let preserve_gdef = preserve_imported_layout && project.preserved_tables.contains_key("GDEF");
    let unicode_by_glyph = project
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            glyph
                .unicode
                .or_else(|| glyph.unicodes.first().copied())
                .map(|unicode| (name.clone(), unicode))
        })
        .collect::<BTreeMap<_, _>>();
    validate_feature_source(&feature_source)?;
    let base_master = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .ok_or_else(|| "可変フォントには基準マスターが必要です".to_string())?;
    validate_master_axes(&project)?;
    let has_width_axis = project.masters.len() >= 2
        && project
            .masters
            .iter()
            .any(|master| (master.width - base_master.width).abs() > f64::EPSILON);
    if !(1..=1000).contains(&project.metadata.weight_class) {
        return Err("Weight Classは1〜1000で指定してください".into());
    }
    if !(1..=9).contains(&project.metadata.width_class) {
        return Err("Width Classは1〜9で指定してください".into());
    }
    if project.metadata.vendor_id.len() != 4 || !project.metadata.vendor_id.is_ascii() {
        return Err("Vendor IDはASCII 4文字で指定してください".into());
    }
    let names = project.glyph_names_sorted();
    let glyph_ids: std::collections::HashMap<&str, u16> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, (index + 1) as u16))
        .collect();
    let mut glyph_ids = glyph_ids;
    glyph_ids.insert(".notdef", 0);
    if names.len() >= u16::MAX as usize {
        return Err("グリフ数が多すぎます".into());
    }
    let empty = || Glyph {
        contours: vec![],
        components: vec![],
        overlap: false,
        xMin: 0,
        yMin: 0,
        xMax: 0,
        yMax: 0,
        instructions: vec![],
    };
    let mut glyphs = vec![empty()];
    let mut metrics = vec![Metric {
        advanceWidth: upm,
        lsb: 0,
    }];
    let mut mapping = BTreeMap::new();

    for (index, name) in names.iter().enumerate() {
        let source = project.glyphs.get(*name).unwrap();
        let mut output = empty();
        let mut contours = Vec::new();
        if source.components.is_empty() || !source.contours.is_empty() {
            append_contours(
                &project,
                &source.name,
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut Vec::new(),
                &mut contours,
            )?;
        } else {
            for component in &source.components {
                let glyph_index = *glyph_ids.get(component.base.as_str()).ok_or_else(|| {
                    format!("コンポーネント '{}' が見つかりません", component.base)
                })?;
                output.components.push(fonttools::glyf::Component {
                    glyph_index,
                    transformation: kurbo08::Affine::new([
                        component.x_scale,
                        component.yx_scale,
                        component.xy_scale,
                        component.y_scale,
                        component.x_offset,
                        component.y_offset,
                    ]),
                    match_points: None,
                    flags: fonttools::glyf::ComponentFlags::empty(),
                });
            }
        }
        output.contours = contours;
        let all = output.contours.iter().flatten();
        output.xMin = all.clone().map(|p| p.x).min().unwrap_or(0);
        output.xMax = all.clone().map(|p| p.x).max().unwrap_or(0);
        output.yMin = all.clone().map(|p| p.y).min().unwrap_or(0);
        output.yMax = all.map(|p| p.y).max().unwrap_or(0);
        if !output.components.is_empty() {
            if let Some((min_x, min_y, max_x, max_y)) =
                project.outline_bounds_for_glyph(&source.name)
            {
                output.xMin = checked_i16(min_x.floor(), "複合グリフX最小値")?;
                output.yMin = checked_i16(min_y.floor(), "複合グリフY最小値")?;
                output.xMax = checked_i16(max_x.ceil(), "複合グリフX最大値")?;
                output.yMax = checked_i16(max_y.ceil(), "複合グリフY最大値")?;
            }
        }
        metrics.push(Metric {
            advanceWidth: checked_u16(source.width, "グリフ幅")?,
            lsb: output.xMin,
        });
        let mut codepoints = source.unicodes.clone();
        if let Some(codepoint) = source.unicode {
            if !codepoints.contains(&codepoint) {
                codepoints.push(codepoint);
            }
        }
        for codepoint in codepoints {
            if (0xD800..=0xDFFF).contains(&codepoint) || codepoint > 0x10FFFF {
                return Err(format!(
                    "グリフ '{}' のUnicode U+{codepoint:04X}は不正です",
                    source.name
                ));
            }
            if mapping.insert(codepoint, (index + 1) as u16).is_some() {
                return Err(format!("Unicode U+{codepoint:04X}が重複しています"));
            }
        }
        glyphs.push(output);
    }

    let all = glyphs.iter().flat_map(|g| g.contours.iter().flatten());
    let xmin = all.clone().map(|p| p.x).min().unwrap_or(0);
    let xmax = all.clone().map(|p| p.x).max().unwrap_or(0);
    let ymin = all.clone().map(|p| p.y).min().unwrap_or(0);
    let ymax = all.map(|p| p.y).max().unwrap_or(0);
    let outline = glyf { glyphs };
    if mapping.is_empty() {
        return Err("TTF出力にはUnicodeを持つグリフが1つ以上必要です".into());
    }
    let stats = outline.maxp_statistics();
    let min_right_side_bearing = metrics
        .iter()
        .zip(&outline.glyphs)
        .map(|(metric, glyph)| metric.advanceWidth as i32 - metric.lsb as i32 - glyph.xMax as i32)
        .min()
        .unwrap_or(0);
    let min_right_side_bearing =
        checked_i16(min_right_side_bearing as f64, "最小右サイドベアリング")?;
    let number_of_h_metrics = metrics
        .iter()
        .rposition(|metric| metric.advanceWidth != metrics.last().unwrap().advanceWidth)
        .map(|index| index + 2)
        .unwrap_or(1) as u16;
    let mut font = Font::new(SfntVersion::TrueType);
    let mut head_table = head::new(
        project.metadata.font_revision as f32,
        upm,
        xmin,
        ymin,
        xmax,
        ymax,
    );
    if project.metadata.head_flags != 0 {
        head_table.flags = project.metadata.head_flags;
    }
    if project.metadata.lowest_rec_ppem != 0 {
        head_table.lowestRecPPEM = project.metadata.lowest_rec_ppem;
    }
    head_table.fontDirectionHint = project.metadata.font_direction_hint;
    head_table.macStyle = if project.metadata.head_mac_style != 0 {
        project.metadata.head_mac_style
    } else {
        mac_style_flags(&project.metadata)
    };
    font.tables.insert(*b"head", Table::Head(head_table));
    let master_metrics = project.master_metrics_for(&base_master.id);
    let ascender = checked_i16(master_metrics.ascender, "Ascender")?;
    let descender = checked_i16(master_metrics.descender, "Descender")?;
    let line_gap = checked_i16(master_metrics.line_gap, "Line Gap")?;
    let win_ascent = u16::try_from(i32::from(ascender).max(i32::from(ymax)).max(0))
        .map_err(|_| "WinAscentが範囲外です".to_string())?;
    let win_descent = u16::try_from((-i32::from(descender)).max(-i32::from(ymin)).max(0))
        .map_err(|_| "WinDescentが範囲外です".to_string())?;
    let first_char = mapping
        .keys()
        .copied()
        .filter(|codepoint| *codepoint <= 0xFFFF)
        .min()
        .unwrap_or(0) as u16;
    let last_char = mapping
        .keys()
        .copied()
        .filter(|codepoint| *codepoint <= 0xFFFF)
        .max()
        .unwrap_or(0) as u16;
    let os2_scale = upm.min(i16::MAX as u16) as i16;
    let (unicode_range1, unicode_range2, unicode_range3, unicode_range4) =
        unicode_range_bits(&mapping);
    let (code_page_range1, code_page_range2) = code_page_range_bits(&mapping);
    let average_width = if metrics.len() > 1 {
        let total: i64 = metrics
            .iter()
            .skip(1)
            .map(|metric| i64::from(metric.advanceWidth))
            .sum();
        checked_i16(total as f64 / (metrics.len() - 1) as f64, "平均字幅")?
    } else {
        checked_i16(upm as f64, "平均字幅")?
    };
    font.tables.insert(
        *b"OS/2",
        Table::Os2(os2 {
            version: if project.metadata.x_height != 0.0
                || project.metadata.cap_height != 0.0
                || project.metadata.default_char != 0
                || project.metadata.break_char != 0
                || project.metadata.max_context != 0
            {
                if project.metadata.lower_optical_point_size != 0
                    || project.metadata.upper_optical_point_size != 0
                {
                    5
                } else {
                    2
                }
            } else if project.metadata.lower_optical_point_size != 0
                || project.metadata.upper_optical_point_size != 0
            {
                5
            } else {
                0
            },
            xAvgCharWidth: average_width,
            usWeightClass: project.metadata.weight_class,
            usWidthClass: project.metadata.width_class,
            fsType: project.metadata.fs_type,
            ySubscriptXSize: if project.metadata.subscript_x_size != 0 {
                project.metadata.subscript_x_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySubscriptYSize: if project.metadata.subscript_y_size != 0 {
                project.metadata.subscript_y_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySubscriptXOffset: project.metadata.subscript_x_offset,
            ySubscriptYOffset: project.metadata.subscript_y_offset,
            ySuperscriptXSize: if project.metadata.superscript_x_size != 0 {
                project.metadata.superscript_x_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySuperscriptYSize: if project.metadata.superscript_y_size != 0 {
                project.metadata.superscript_y_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySuperscriptXOffset: project.metadata.superscript_x_offset,
            ySuperscriptYOffset: if project.metadata.superscript_y_offset != 0 {
                project.metadata.superscript_y_offset
            } else {
                (os2_scale / 2).max(1)
            },
            yStrikeoutSize: if project.metadata.strikeout_size != 0 {
                project.metadata.strikeout_size
            } else {
                (os2_scale / 20).max(1)
            },
            yStrikeoutPosition: if project.metadata.strikeout_position != 0 {
                project.metadata.strikeout_position
            } else {
                (os2_scale / 3).max(1)
            },
            sFamilyClass: project.metadata.family_class,
            panose: Panose {
                panose0: project.metadata.panose[0],
                panose1: project.metadata.panose[1],
                panose2: project.metadata.panose[2],
                panose3: project.metadata.panose[3],
                panose4: project.metadata.panose[4],
                panose5: project.metadata.panose[5],
                panose6: project.metadata.panose[6],
                panose7: project.metadata.panose[7],
                panose8: project.metadata.panose[8],
                panose9: project.metadata.panose[9],
            },
            ulUnicodeRange1: unicode_range1,
            ulUnicodeRange2: unicode_range2,
            ulUnicodeRange3: unicode_range3,
            ulUnicodeRange4: unicode_range4,
            achVendID: font_vendor_id(&project.metadata.vendor_id),
            fsSelection: os2_selection_flags(&project.metadata),
            usFirstCharIndex: first_char,
            usLastCharIndex: last_char,
            sTypoAscender: ascender,
            sTypoDescender: descender,
            sTypoLineGap: line_gap,
            usWinAscent: if project.metadata.win_ascent != 0 {
                project.metadata.win_ascent
            } else {
                win_ascent
            },
            usWinDescent: if project.metadata.win_descent != 0 {
                project.metadata.win_descent
            } else {
                win_descent
            },
            ulCodePageRange1: Some(code_page_range1),
            ulCodePageRange2: Some(code_page_range2),
            sxHeight: (project.metadata.x_height != 0.0)
                .then(|| checked_i16(project.metadata.x_height, "x-height"))
                .transpose()?,
            sCapHeight: (project.metadata.cap_height != 0.0)
                .then(|| checked_i16(project.metadata.cap_height, "Cap height"))
                .transpose()?,
            usDefaultChar: Some(if project.metadata.default_char != 0 {
                project.metadata.default_char
            } else {
                0
            }),
            usBreakChar: Some(if project.metadata.break_char != 0 {
                project.metadata.break_char
            } else if mapping.contains_key(&0x20) {
                0x20
            } else {
                0
            }),
            usMaxContext: Some(if project.metadata.max_context != 0 {
                project.metadata.max_context
            } else {
                max_feature_context(&feature_source)
            }),
            usLowerOpticalPointSize: (project.metadata.lower_optical_point_size != 0)
                .then_some(project.metadata.lower_optical_point_size),
            usUpperOpticalPointSize: (project.metadata.upper_optical_point_size != 0)
                .then_some(project.metadata.upper_optical_point_size),
        }),
    );
    font.tables.insert(
        *b"hhea",
        Table::Hhea(hhea {
            majorVersion: 1,
            minorVersion: 0,
            ascender,
            descender,
            lineGap: line_gap,
            advanceWidthMax: metrics.iter().map(|m| m.advanceWidth).max().unwrap_or(upm),
            minLeftSideBearing: metrics.iter().map(|m| m.lsb).min().unwrap_or(0),
            minRightSideBearing: min_right_side_bearing,
            xMaxExtent: xmax,
            caretSlopeRise: if project.metadata.caret_slope_rise != 0 {
                project.metadata.caret_slope_rise
            } else {
                1
            },
            caretSlopeRun: project.metadata.caret_slope_run,
            caretOffset: project.metadata.caret_offset,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            metricDataFormat: 0,
            numberOfHMetrics: number_of_h_metrics,
        }),
    );
    font.tables.insert(*b"glyf", Table::Glyf(outline));
    if project.masters.len() >= 2 {
        let first_id = &base_master.id;
        validate_component_master_topology(&project, first_id)?;
        validate_component_master_transforms(&project, first_id)?;
        let mut variations = vec![None];
        let mut has_variation = false;
        for name in &names {
            let source = project.glyphs.get(*name).unwrap();
            let variation = build_gvar_variation(
                source,
                &project,
                first_id,
                has_width_axis,
                &mut has_variation,
            )?;
            variations.push(variation);
        }
        if has_variation {
            let glyph_table = font.tables.get(b"glyf").and_then(|table| match table {
                Table::Glyf(glyphs) => Some(glyphs),
                _ => None,
            });
            let bytes = gvar { variations }.to_bytes(glyph_table);
            font.tables.insert(*b"gvar", Table::Unknown(bytes));
        }
    }
    font.tables.insert(
        *b"maxp",
        Table::Maxp(maxp::new10(
            stats.0, stats.1, stats.2, stats.3, stats.4, stats.5, stats.6,
        )),
    );
    let hmtx_bytes = hmtx { metrics }.to_bytes().0;
    font.tables.insert(*b"hmtx", Table::Unknown(hmtx_bytes));
    let (vhea_bytes, vmtx_bytes) =
        build_vertical_metrics_tables(&project, &names, &project.default_master_id, upm)?;
    font.tables.insert(*b"vhea", Table::Unknown(vhea_bytes));
    font.tables.insert(*b"vmtx", Table::Unknown(vmtx_bytes));
    if let Some((colr, cpal)) = build_color_tables(&project, &glyph_ids) {
        font.tables.insert(*b"COLR", Table::Unknown(colr));
        font.tables.insert(*b"CPAL", Table::Unknown(cpal));
    }
    if let Some(svg_table) = build_svg_table(&project, &glyph_ids) {
        font.tables.insert(*b"SVG ", Table::Unknown(svg_table));
    }
    let has_non_bmp = mapping.keys().any(|codepoint| *codepoint > 0xFFFF);
    if has_non_bmp || !project.unicode_variation_sequences.is_empty() {
        font.tables.insert(
            *b"cmap",
            Table::Unknown(build_cmap_with_variations(
                &mapping,
                &project.unicode_variation_sequences,
                &glyph_ids,
            )),
        );
    } else {
        font.tables.insert(
            *b"cmap",
            Table::Cmap(cmap {
                subtables: vec![
                    CmapSubtable {
                        format: 4,
                        platformID: 0,
                        encodingID: 3,
                        languageID: 0,
                        mapping: mapping.clone(),
                    },
                    CmapSubtable {
                        format: 4,
                        platformID: 3,
                        encodingID: 1,
                        languageID: 0,
                        mapping,
                    },
                ],
            }),
        );
    }
    if !preserve_gsub {
        if let Some(gsub_bytes) = build_simple_gsub_with_variations_and_unicode(
            &feature_source,
            &glyph_ids,
            &conditional_substitutions,
            &axis_bounds,
            &unicode_by_glyph,
        ) {
            font.tables.insert(*b"GSUB", Table::Unknown(gsub_bytes));
        }
    }
    if !preserve_gpos {
        if let Some(gpos_bytes) = build_kerning_gpos_with_unicode(
            &project,
            &glyph_ids,
            &feature_source,
            &unicode_by_glyph,
        ) {
            font.tables.insert(*b"GPOS", Table::Unknown(gpos_bytes));
        }
    }
    if !preserve_gdef {
        if let Some(gdef_bytes) = build_gdef(&project, &glyph_ids, &feature_source) {
            font.tables.insert(*b"GDEF", Table::Unknown(gdef_bytes));
        }
    }
    if !project.preserved_tables.contains_key("BASE") {
        if let Some(base_bytes) = build_base_table() {
            font.tables.insert(*b"BASE", Table::Unknown(base_bytes));
        }
    }
    let mut pairs = project
        .kerning
        .iter()
        .filter_map(|((left, right), value)| {
            Some((
                *glyph_ids.get(left.as_str())?,
                *glyph_ids.get(right.as_str())?,
                checked_i16(*value, "カーニング値").ok()?,
            ))
        })
        .collect::<Vec<_>>();
    pairs.sort_unstable_by_key(|(left, right, _)| (*left, *right));
    if !pairs.is_empty() {
        let n_pairs =
            u16::try_from(pairs.len()).map_err(|_| "カーニングペアが多すぎます".to_string())?;
        let max_power = 1_u16 << (15 - n_pairs.leading_zeros());
        let search_range = max_power * 6;
        let entry_selector = (15 - max_power.leading_zeros()) as u16;
        let range_shift = n_pairs * 6 - search_range;
        let subtable_length = 14_u16 + n_pairs * 6;
        let mut bytes = Vec::with_capacity(18 + pairs.len() * 6);
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&subtable_length.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&n_pairs.to_be_bytes());
        bytes.extend_from_slice(&search_range.to_be_bytes());
        bytes.extend_from_slice(&entry_selector.to_be_bytes());
        bytes.extend_from_slice(&range_shift.to_be_bytes());
        for (left, right, value) in pairs {
            bytes.extend_from_slice(&left.to_be_bytes());
            bytes.extend_from_slice(&right.to_be_bytes());
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        font.tables.insert(*b"kern", Table::Unknown(bytes));
    }
    font.tables.insert(
        *b"name",
        Table::Name(name {
            records: vec![
                NameRecord::windows_unicode(
                    NameRecordID::FontFamilyName,
                    project.metadata.family_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::FullFontName,
                    format!(
                        "{} {}",
                        project.metadata.family_name, project.metadata.style_name
                    ),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::FontSubfamilyName,
                    project.metadata.style_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PreferredFamilyName,
                    project.metadata.family_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PreferredSubfamilyName,
                    project.metadata.style_name.clone(),
                ),
                NameRecord::windows_unicode(
                    3_u16,
                    format!(
                        "{};{:.3};{}",
                        project.metadata.family_name,
                        project.metadata.font_revision,
                        postscript_name(
                            &project.metadata.family_name,
                            &project.metadata.style_name
                        )
                    ),
                ),
                NameRecord::windows_unicode(16_u16, project.metadata.family_name.clone()),
                NameRecord::windows_unicode(17_u16, project.metadata.style_name.clone()),
                NameRecord::windows_unicode(21_u16, project.metadata.family_name.clone()),
                NameRecord::windows_unicode(22_u16, project.metadata.style_name.clone()),
                NameRecord::windows_unicode(
                    NameRecordID::Version,
                    format!("Version {:.3}", project.metadata.font_revision),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PostscriptName,
                    postscript_name(&project.metadata.family_name, &project.metadata.style_name),
                ),
            ],
        }),
    );
    if let Some(Table::Name(names_table)) = font.tables.get_mut(b"name") {
        for (name_id, value) in [
            (NameRecordID::Copyright, &project.metadata.copyright),
            (NameRecordID::Designer, &project.metadata.designer),
            (NameRecordID::Manufacturer, &project.metadata.manufacturer),
        ] {
            if !value.trim().is_empty() {
                names_table
                    .records
                    .push(NameRecord::windows_unicode(name_id, value.clone()));
            }
        }
        // CPAL v1 palette labels use name IDs outside the standardized range.
        // Keep the IDs deterministic so a round trip does not depend on UI order.
        for (palette_index, label) in project
            .color_palette_names
            .iter()
            .enumerate()
            .take(project.color_palettes.len())
        {
            if let Ok(name_id) = u16::try_from(1000usize.saturating_add(palette_index)) {
                if !label.trim().is_empty() {
                    names_table
                        .records
                        .push(NameRecord::windows_unicode(name_id, label.clone()));
                }
            }
        }
        for (entry_index, label) in project.color_palette_entry_names.iter().enumerate() {
            if let Ok(name_id) = u16::try_from(2000usize.saturating_add(entry_index)) {
                if !label.trim().is_empty() {
                    names_table
                        .records
                        .push(NameRecord::windows_unicode(name_id, label.clone()));
                }
            }
        }
        for number in 1_u16..=20 {
            let ss_tag = format!("ss{number:02}");
            if feature_source.contains(&format!("feature {ss_tag}")) {
                let name_id = 499 + number;
                let records = feature_name_records(&feature_source, &ss_tag, name_id);
                if records.is_empty() {
                    names_table.records.push(NameRecord::windows_unicode(
                        name_id,
                        format!("Stylistic Set {number}"),
                    ));
                } else {
                    names_table.records.extend(records);
                }
            }
            let cv_tag = format!("cv{number:02}");
            if feature_source.contains(&format!("feature {cv_tag}")) {
                let name_id = 519 + number;
                let records = feature_name_records(&feature_source, &cv_tag, name_id);
                if records.is_empty() {
                    names_table.records.push(NameRecord::windows_unicode(
                        name_id,
                        format!("Character Variant {number}"),
                    ));
                } else {
                    names_table.records.extend(records);
                }
            }
        }
        for override_record in parse_feature_name_records(&feature_source) {
            names_table.records.retain(|record| {
                (
                    record.platformID,
                    record.encodingID,
                    record.languageID,
                    record.nameID,
                ) != (
                    override_record.platformID,
                    override_record.encodingID,
                    override_record.languageID,
                    override_record.nameID,
                )
            });
            names_table.records.push(override_record);
        }
    }
    if project.masters.len() >= 2 {
        let mut custom_axis_tags: Vec<String> = project
            .masters
            .iter()
            .flat_map(|master| master.axes.keys())
            .filter(|tag| tag.len() == 4 && tag.is_ascii())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        custom_axis_tags.retain(|tag| {
            let mut values = project
                .masters
                .iter()
                .map(|master| master.axes.get(tag).copied().unwrap_or(0.0));
            values
                .next()
                .is_some_and(|first| values.any(|value| (value - first).abs() > f64::EPSILON))
        });
        let implicit_width_axis =
            has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth");
        if let Some(Table::Name(names_table)) = font.tables.get_mut(b"name") {
            if custom_axis_tags.is_empty() {
                names_table.records.push(NameRecord::windows_unicode(
                    256_u16,
                    project
                        .axis_names
                        .get("wght")
                        .cloned()
                        .unwrap_or_else(|| "Weight".into()),
                ));
            } else {
                for (index, tag) in custom_axis_tags.iter().enumerate() {
                    names_table.records.push(NameRecord::windows_unicode(
                        256 + index as u16,
                        project
                            .axis_names
                            .get(tag)
                            .cloned()
                            .unwrap_or_else(|| tag.clone()),
                    ));
                }
            }
            if has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth") {
                names_table.records.push(NameRecord::windows_unicode(
                    if custom_axis_tags.is_empty() {
                        257
                    } else {
                        256 + custom_axis_tags.len() as u16
                    },
                    project
                        .axis_names
                        .get("wdth")
                        .cloned()
                        .unwrap_or_else(|| "Width".into()),
                ));
            }
            for (index, master) in project.masters.iter().enumerate() {
                names_table.records.push(NameRecord::windows_unicode(
                    300_u16 + index as u16,
                    master.name.clone(),
                ));
            }
            for (index, instance) in project.instances.iter().enumerate() {
                names_table.records.push(NameRecord::windows_unicode(
                    400_u16 + index as u16,
                    instance.name.clone(),
                ));
            }
        }
        let axis_value =
            |master: &FontMaster, tag: &str| master.axes.get(tag).copied().unwrap_or(0.0);
        let instance_axis_value =
            |instance: &FontInstance, tag: &str| instance.axes.get(tag).copied().unwrap_or(0.0);
        let mut axes: Vec<VariationAxisRecord> = custom_axis_tags
            .iter()
            .enumerate()
            .map(|(index, tag)| VariationAxisRecord {
                axisTag: tag.as_bytes().try_into().unwrap(),
                flags: project.axis_flags.get(tag).copied().unwrap_or(0),
                minValue: project
                    .masters
                    .iter()
                    .map(|m| axis_value(m, tag))
                    .fold(f64::INFINITY, f64::min) as f32,
                defaultValue: axis_value(base_master, tag) as f32,
                maxValue: project
                    .masters
                    .iter()
                    .map(|m| axis_value(m, tag))
                    .fold(f64::NEG_INFINITY, f64::max) as f32,
                axisNameID: 256 + index as u16,
            })
            .collect();
        if axes.is_empty() {
            axes.push(VariationAxisRecord {
                axisTag: *b"wght",
                flags: project.axis_flags.get("wght").copied().unwrap_or(0),
                minValue: project
                    .masters
                    .iter()
                    .map(|m| m.weight)
                    .fold(f64::INFINITY, f64::min) as f32,
                defaultValue: base_master.weight as f32,
                maxValue: project
                    .masters
                    .iter()
                    .map(|m| m.weight)
                    .fold(f64::NEG_INFINITY, f64::max) as f32,
                axisNameID: 256,
            });
        }
        if has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth") {
            let min_width = project
                .masters
                .iter()
                .map(|master| master.width)
                .fold(f64::INFINITY, f64::min) as f32;
            let max_width = project
                .masters
                .iter()
                .map(|master| master.width)
                .fold(f64::NEG_INFINITY, f64::max) as f32;
            axes.push(VariationAxisRecord {
                axisTag: *b"wdth",
                flags: project.axis_flags.get("wdth").copied().unwrap_or(0),
                minValue: min_width,
                defaultValue: base_master.width as f32,
                maxValue: max_width,
                axisNameID: if custom_axis_tags.is_empty() {
                    257
                } else {
                    256 + custom_axis_tags.len() as u16
                },
            });
        }
        let hvar_axis_tags = axes
            .iter()
            .map(|axis| String::from_utf8_lossy(&axis.axisTag).to_string())
            .collect::<Vec<_>>();
        font.tables.insert(
            *b"fvar",
            Table::Fvar(fvar {
                axes,
                instances: if project.instances.is_empty() {
                    project
                        .masters
                        .iter()
                        .enumerate()
                        .map(|(index, master)| InstanceRecord {
                            subfamilyNameID: 300 + index as u16,
                            coordinates: custom_axis_tags
                                .iter()
                                .map(|tag| axis_value(master, tag) as f32)
                                .chain(if custom_axis_tags.is_empty() {
                                    Some(master.weight as f32)
                                } else {
                                    None
                                })
                                .chain(implicit_width_axis.then_some(master.width as f32))
                                .collect(),
                            postscriptNameID: None,
                        })
                        .collect()
                } else {
                    project
                        .instances
                        .iter()
                        .enumerate()
                        .map(|(index, instance)| InstanceRecord {
                            subfamilyNameID: 400 + index as u16,
                            coordinates: custom_axis_tags
                                .iter()
                                .map(|tag| instance_axis_value(instance, tag) as f32)
                                .chain(if custom_axis_tags.is_empty() {
                                    Some(instance.weight as f32)
                                } else {
                                    None
                                })
                                .chain(implicit_width_axis.then_some(instance.width as f32))
                                .collect(),
                            postscriptNameID: None,
                        })
                        .collect()
                },
            }),
        );
        if let Some(avar_bytes) = build_avar(&hvar_axis_tags, &project.axis_mappings) {
            font.tables.insert(*b"avar", Table::Unknown(avar_bytes));
        }
        if let Some(hvar_bytes) = build_hvar(&project, &names, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"HVAR", Table::Unknown(hvar_bytes));
        }
        if let Some(vvar_bytes) = build_vvar(&project, &names, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"VVAR", Table::Unknown(vvar_bytes));
        }
        if let Some(mvar_bytes) = build_mvar(&project, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"MVAR", Table::Unknown(mvar_bytes));
        }
        let mut stat_axes = custom_axis_tags
            .iter()
            .enumerate()
            .map(|(index, tag)| (tag.as_bytes().try_into().unwrap(), 256 + index as u16))
            .collect::<Vec<([u8; 4], u16)>>();
        if custom_axis_tags.is_empty() {
            stat_axes.push((*b"wght", 256));
        }
        if implicit_width_axis {
            stat_axes.push((
                *b"wdth",
                if custom_axis_tags.is_empty() {
                    257
                } else {
                    256 + custom_axis_tags.len() as u16
                },
            ));
        }
        let stat_values = if project.instances.is_empty() {
            project
                .masters
                .iter()
                .map(|master| {
                    custom_axis_tags
                        .iter()
                        .map(|tag| axis_value(master, tag) as f32)
                        .chain(if custom_axis_tags.is_empty() {
                            Some(master.weight as f32)
                        } else {
                            None
                        })
                        .chain(implicit_width_axis.then_some(master.width as f32))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            project
                .instances
                .iter()
                .map(|instance| {
                    custom_axis_tags
                        .iter()
                        .map(|tag| instance_axis_value(instance, tag) as f32)
                        .chain(if custom_axis_tags.is_empty() {
                            Some(instance.weight as f32)
                        } else {
                            None
                        })
                        .chain(implicit_width_axis.then_some(instance.width as f32))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let stat_name_ids = if project.instances.is_empty() {
            project
                .masters
                .iter()
                .enumerate()
                .map(|(index, _)| 300_u16 + index as u16)
                .collect::<Vec<_>>()
        } else {
            project
                .instances
                .iter()
                .enumerate()
                .map(|(index, _)| 400_u16 + index as u16)
                .collect::<Vec<_>>()
        };
        font.tables.insert(
            *b"STAT",
            Table::Unknown(build_stat_table_with_values(
                &stat_axes,
                &stat_values,
                &stat_name_ids,
            )),
        );
    }
    font.tables.insert(
        *b"post",
        Table::Post(post::new(
            2.0,
            project.metadata.italic_angle as f32,
            checked_i16(project.metadata.underline_position, "Underline position")?,
            checked_i16(project.metadata.underline_thickness, "Underline thickness")?,
            project.metadata.is_fixed_pitch,
            Some(
                std::iter::once(".notdef")
                    .chain(names.iter().copied())
                    .map(str::to_string)
                    .collect(),
            ),
        )),
    );
    // Advertise the standard bitmap behavior for all ppem sizes. This keeps
    // rasterizers from applying legacy embedded-bitmap rules to outline fonts.
    font.tables.insert(
        *b"gasp",
        Table::Unknown({
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(&0xFFFFu16.to_be_bytes());
            bytes.extend_from_slice(&0x000Fu16.to_be_bytes());
            bytes
        }),
    );
    // Preserve tables not yet modelled by Glyph Studio. Generated tables above
    // always win, so editing outlines/metrics/features cannot leave stale
    // copies of core tables in the output while specialised tables such as
    // MATH, JSTF, bitmap strikes, AAT, meta, and DSIG remain available.
    for (tag, bytes) in &project.preserved_tables {
        let Ok(tag_bytes) = <[u8; 4]>::try_from(tag.as_bytes()) else {
            continue;
        };
        font.tables
            .entry(tag_bytes)
            .or_insert_with(|| Table::Unknown(bytes.clone()));
    }
    let mut file = File::create(path).map_err(|e| format!("TTF作成エラー: {e}"))?;
    font.save(&mut file);
    Ok(())
}

fn build_color_tables(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut bases = Vec::new();
    let mut layers = Vec::new();
    let mut paint_layers = Vec::new();
    let mut color_glyphs: Vec<(&str, u16)> = project
        .color_layers
        .keys()
        .filter_map(|name| {
            glyph_ids
                .get(name.as_str())
                .copied()
                .map(|id| (name.as_str(), id))
        })
        .collect();
    color_glyphs.sort_unstable_by_key(|(_, base_id)| *base_id);
    for (name, base_id) in color_glyphs {
        let entries = project.color_layers.get(name)?;
        let first = u16::try_from(layers.len()).ok()?;
        for entry in entries {
            let &layer_id = glyph_ids.get(entry.glyph.as_str())?;
            layers.push((layer_id, entry.palette_index));
            paint_layers.push((
                layer_id,
                entry.palette_index,
                entry.gradient.clone(),
                entry.alpha,
                entry.gradient.is_none() && project.color_layers.contains_key(&entry.glyph),
                project
                    .color_layer_transforms
                    .get(name)
                    .and_then(|transforms| transforms.get(paint_layers.len()))
                    .copied()
                    .flatten(),
            ));
        }
        let count = u16::try_from(entries.len()).ok()?;
        if count > 0 {
            bases.push((base_id, first, count));
        }
    }
    if bases.is_empty() || project.color_palettes.is_empty() {
        return None;
    }
    if layers.len() > u16::MAX as usize {
        return None;
    }
    if layers.len() > u32::MAX as usize || bases.len() > u32::MAX as usize {
        return None;
    }
    let v0_base_offset = 34usize;
    let v0_layer_offset = v0_base_offset.checked_add(bases.len() * 6)?;
    let mut colr = Vec::with_capacity(v0_layer_offset + layers.len() * 4 + 256);
    put_u16(&mut colr, 1);
    put_u16(&mut colr, u16::try_from(bases.len()).ok()?);
    put_u32(&mut colr, u32::try_from(v0_base_offset).ok()?);
    put_u32(&mut colr, u32::try_from(v0_layer_offset).ok()?);
    put_u16(&mut colr, u16::try_from(layers.len()).ok()?);
    let base_glyph_list_offset_position = colr.len();
    colr.resize(34, 0);
    for (glyph, first, count) in &bases {
        put_u16(&mut colr, *glyph);
        put_u16(&mut colr, *first);
        put_u16(&mut colr, *count);
    }
    for (glyph, palette) in &layers {
        put_u16(&mut colr, *glyph);
        put_u16(&mut colr, *palette);
    }

    // COLR v1 keeps the v0 records above for older consumers and adds a
    // PaintColrLayers graph using PaintGlyph + PaintSolid for the same data.
    while colr.len() % 4 != 0 {
        colr.push(0);
    }
    let base_glyph_list_offset = colr.len();
    put_u32(&mut colr, u32::try_from(bases.len()).ok()?);
    for (glyph, _, _) in &bases {
        put_u16(&mut colr, *glyph);
        put_u32(&mut colr, 0);
    }
    let paint_colr_layers_offset = colr.len();
    for (_, first, count) in &bases {
        if *count > u8::MAX as u16 {
            return None;
        }
        put_u8(&mut colr, 1);
        put_u8(&mut colr, *count as u8);
        put_u32(&mut colr, u32::from(*first));
    }
    let layer_list_offset = colr.len();
    put_u32(&mut colr, u32::try_from(layers.len()).ok()?);
    let layer_offsets_start = colr.len();
    colr.resize(layer_offsets_start + layers.len() * 4, 0);
    for (index, (glyph, palette, gradient, alpha, nested, transform)) in
        paint_layers.iter().enumerate()
    {
        let paint_offset = colr.len().checked_sub(layer_list_offset)?;
        let offset_position = layer_offsets_start + index * 4;
        colr[offset_position..offset_position + 4]
            .copy_from_slice(&u32::try_from(paint_offset).ok()?.to_be_bytes());
        if let Some(transform) = transform {
            put_u8(&mut colr, 12); // PaintTransform
                                   // PaintTransform contains two Offset24 fields followed by the
                                   // 24-byte Affine2x3 record. The transform record starts after
                                   // the 7-byte PaintTransform header; the child paint starts
                                   // after that record.
            colr.extend_from_slice(&[0, 0, 31]); // child PaintGlyph Offset24
            colr.extend_from_slice(&[0, 0, 7]); // Affine2x3 Offset24
            for value in [
                transform.xx,
                transform.yx,
                transform.xy,
                transform.yy,
                transform.dx,
                transform.dy,
            ] {
                put_i32(&mut colr, checked_fixed_16_16(value, "COLR変形").ok()?);
            }
        }
        if *nested {
            put_u8(&mut colr, 11); // PaintColrGlyph
            put_u16(&mut colr, *glyph);
        } else {
            put_u8(&mut colr, 10); // PaintGlyph
            let child_offset = 6_u32;
            colr.extend_from_slice(&[0, 0, child_offset as u8]); // child Offset24
            put_u16(&mut colr, *glyph);
        }
        if !*nested {
            if let Some(gradient) = gradient {
                let (paint_format, color_line_offset) = match gradient.kind {
                    crate::font_data::ColorGradientKind::Linear => (4, 16),
                    crate::font_data::ColorGradientKind::Radial => (6, 16),
                    crate::font_data::ColorGradientKind::Sweep => (8, 12),
                };
                put_u8(&mut colr, paint_format);
                colr.extend_from_slice(&[0, 0, color_line_offset]); // ColorLine Offset24
                match gradient.kind {
                    crate::font_data::ColorGradientKind::Linear => {
                        for coordinate in [
                            gradient.x0,
                            gradient.y0,
                            gradient.x1,
                            gradient.y1,
                            gradient.x2,
                            gradient.y2,
                        ] {
                            put_i16(
                                &mut colr,
                                checked_i16(coordinate, "COLRグラデーション座標").ok()?,
                            );
                        }
                    }
                    crate::font_data::ColorGradientKind::Radial => {
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x0, "COLR円形グラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y0, "COLR円形グラデーションY").ok()?,
                        );
                        put_u16(
                            &mut colr,
                            checked_u16(gradient.radius0, "COLR円形グラデーション半径").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x1, "COLR円形グラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y1, "COLR円形グラデーションY").ok()?,
                        );
                        put_u16(
                            &mut colr,
                            checked_u16(gradient.radius1, "COLR円形グラデーション半径").ok()?,
                        );
                    }
                    crate::font_data::ColorGradientKind::Sweep => {
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x0, "COLRスイープグラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y0, "COLRスイープグラデーションY").ok()?,
                        );
                        put_u16(&mut colr, gradient_angle(gradient.start_angle));
                        put_u16(&mut colr, gradient_angle(gradient.end_angle));
                    }
                }
                put_u8(
                    &mut colr,
                    match gradient.extend {
                        crate::font_data::ColorGradientExtend::Pad => 0,
                        crate::font_data::ColorGradientExtend::Repeat => 1,
                        crate::font_data::ColorGradientExtend::Reflect => 2,
                    },
                );
                let mut stops = gradient.effective_stops();
                if stops.is_empty() || stops.len() > u16::MAX as usize {
                    return None;
                }
                stops.sort_by(|left, right| {
                    left.offset
                        .partial_cmp(&right.offset)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                put_u16(&mut colr, u16::try_from(stops.len()).ok()?);
                for stop in stops {
                    put_u16(&mut colr, gradient_stop_offset(stop.offset));
                    put_u16(&mut colr, stop.palette_index);
                    put_u16(&mut colr, gradient_alpha(stop.alpha));
                }
            } else {
                put_u8(&mut colr, 2); // PaintSolid
                put_u16(&mut colr, *palette);
                // Solid alpha is an F2Dot14 value in COLR v1.
                put_u16(&mut colr, gradient_alpha(*alpha));
            }
        }
    }
    for index in 0..bases.len() {
        let paint_offset = paint_colr_layers_offset
            .checked_sub(base_glyph_list_offset)?
            .checked_add(index.checked_mul(6)?)?;
        let offset_position = base_glyph_list_offset + 4 + index * 6 + 2;
        colr[offset_position..offset_position + 4]
            .copy_from_slice(&u32::try_from(paint_offset).ok()?.to_be_bytes());
    }
    colr[base_glyph_list_offset_position..base_glyph_list_offset_position + 4]
        .copy_from_slice(&u32::try_from(base_glyph_list_offset).ok()?.to_be_bytes());
    colr[base_glyph_list_offset_position + 4..base_glyph_list_offset_position + 8]
        .copy_from_slice(&u32::try_from(layer_list_offset).ok()?.to_be_bytes());

    let entries = project.color_palettes.first()?.len();
    let palettes = project.color_palettes.len();
    if entries == 0 || entries > u16::MAX as usize || palettes > u16::MAX as usize {
        return None;
    }
    let records = entries.checked_mul(palettes)?;
    let has_palette_labels = project
        .color_palette_names
        .iter()
        .take(palettes)
        .any(|label| !label.trim().is_empty());
    let has_palette_types = project
        .color_palette_types
        .iter()
        .take(palettes)
        .any(|palette_type| *palette_type != 0);
    let has_palette_entry_labels = project
        .color_palette_entry_names
        .iter()
        .take(entries)
        .any(|label| !label.trim().is_empty());
    let use_cpal_v1 = has_palette_labels || has_palette_types || has_palette_entry_labels;
    let mut cpal = Vec::new();
    let records_offset = if use_cpal_v1 {
        let color_record_indices_offset = 12usize;
        let version_one_header_offset =
            color_record_indices_offset.checked_add(palettes.checked_mul(2)?)?;
        let types_offset = version_one_header_offset.checked_add(12)?;
        let labels_offset = types_offset.checked_add(palettes.checked_mul(4)?)?;
        let color_labels_offset = labels_offset.checked_add(palettes.checked_mul(2)?)?;
        let records_offset = (color_labels_offset + entries.checked_mul(2)? + 3) & !3;
        put_u16(&mut cpal, 1);
        put_u16(&mut cpal, u16::try_from(entries).ok()?);
        put_u16(&mut cpal, u16::try_from(palettes).ok()?);
        put_u16(&mut cpal, u16::try_from(records).ok()?);
        put_u32(&mut cpal, u32::try_from(records_offset).ok()?);
        for palette_index in 0..palettes {
            put_u16(
                &mut cpal,
                u16::try_from(palette_index.checked_mul(entries)?).ok()?,
            );
        }
        put_u32(&mut cpal, u32::try_from(types_offset).ok()?);
        put_u32(&mut cpal, u32::try_from(labels_offset).ok()?);
        put_u32(&mut cpal, u32::try_from(color_labels_offset).ok()?);
        for palette_index in 0..palettes {
            put_u32(
                &mut cpal,
                project
                    .color_palette_types
                    .get(palette_index)
                    .copied()
                    .unwrap_or(0),
            );
        }
        for palette_index in 0..palettes {
            let name_id = project
                .color_palette_names
                .get(palette_index)
                .filter(|label| !label.trim().is_empty())
                .and_then(|_| u16::try_from(1000usize.saturating_add(palette_index)).ok())
                .unwrap_or(u16::MAX);
            put_u16(&mut cpal, name_id);
        }
        for entry_index in 0..entries {
            let name_id = project
                .color_palette_entry_names
                .get(entry_index)
                .filter(|label| !label.trim().is_empty())
                .and_then(|_| u16::try_from(2000usize.saturating_add(entry_index)).ok())
                .unwrap_or(u16::MAX);
            put_u16(&mut cpal, name_id);
        }
        while cpal.len() < records_offset {
            put_u8(&mut cpal, 0);
        }
        records_offset
    } else {
        put_u16(&mut cpal, 0);
        put_u16(&mut cpal, u16::try_from(entries).ok()?);
        put_u16(&mut cpal, u16::try_from(palettes).ok()?);
        put_u16(&mut cpal, u16::try_from(records).ok()?);
        let records_offset = 12usize.checked_add(palettes.checked_mul(4)?)?;
        put_u32(&mut cpal, u32::try_from(records_offset).ok()?);
        for index in 0..palettes {
            put_u32(
                &mut cpal,
                u32::try_from(records_offset + index * entries * 4).ok()?,
            );
        }
        records_offset
    };
    for palette in &project.color_palettes {
        if palette.len() != entries {
            return None;
        }
        for &[r, g, b, a] in palette {
            cpal.extend_from_slice(&[b, g, r, a]);
        }
    }
    debug_assert_eq!(cpal.len(), records_offset + records * 4);
    Some((colr, cpal))
}

/// Builds the OpenType SVG table from the same outline/component model used
/// by standalone SVG export. A separate document per glyph keeps the table
/// simple and allows color-layer glyphs to carry their palette colors.
fn build_svg_table(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    let mut documents = Vec::<(u16, Vec<u8>)>::new();
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        let has_outline = project
            .glyphs
            .get(name)
            .is_some_and(|glyph| !glyph.contours.is_empty() || !glyph.components.is_empty());
        if !has_outline && !project.color_layers.contains_key(name) {
            continue;
        }
        let document = build_svg_document(project, name)?;
        if document.len() > u32::MAX as usize {
            return None;
        }
        documents.push((glyph_id, document.into_bytes()));
    }
    documents.sort_by_key(|(glyph_id, _)| *glyph_id);
    if documents.is_empty() || documents.len() > u16::MAX as usize {
        return None;
    }
    let list_offset = 10usize;
    let entries_offset = list_offset + 2;
    let documents_offset = entries_offset + documents.len() * 12;
    let total_documents = documents.iter().try_fold(0usize, |total, (_, document)| {
        total.checked_add(document.len())
    })?;
    let total = documents_offset.checked_add(total_documents)?;
    let mut table = Vec::with_capacity(total);
    put_u16(&mut table, 0); // version
    put_u32(&mut table, u32::try_from(list_offset).ok()?);
    put_u32(&mut table, 0); // reserved
    put_u16(&mut table, u16::try_from(documents.len()).ok()?);
    let mut document_offset = documents_offset - list_offset;
    for (glyph_id, document) in &documents {
        put_u16(&mut table, *glyph_id);
        put_u16(&mut table, *glyph_id);
        put_u32(&mut table, u32::try_from(document_offset).ok()?);
        put_u32(&mut table, u32::try_from(document.len()).ok()?);
        document_offset = document_offset.checked_add(document.len())?;
    }
    for (_, document) in documents {
        table.extend_from_slice(&document);
    }
    Some(table)
}

fn build_svg_document(project: &FontProject, glyph_name: &str) -> Option<String> {
    let glyph = project.glyphs.get(glyph_name)?;
    let glyph_width = glyph.width.max(1.0);
    let top = project.metadata.ascender.max(0.0);
    let bottom = project.metadata.descender.min(0.0);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 {} {} {}\">\n",
        -top,
        glyph_width,
        (top - bottom).max(1.0)
    );
    if let Some(layers) = project.color_layers.get(glyph_name) {
        let palette = project
            .color_palettes
            .first()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        append_svg_gradient_defs(
            &mut svg,
            layers.iter().enumerate().filter_map(|(index, layer)| {
                layer.gradient.as_ref().map(|gradient| (index, gradient))
            }),
            palette,
        );
        let mut nested_definitions = String::new();
        if append_svg_nested_gradient_defs(
            project,
            glyph_name,
            palette,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut nested_definitions,
        )
        .ok()?
        {
            svg.push_str("<defs>\n");
            svg.push_str(&nested_definitions);
            svg.push_str("</defs>\n");
        }
        for (index, layer) in layers.iter().enumerate() {
            let color = project
                .color_palettes
                .first()
                .and_then(|palette| palette.get(usize::from(layer.palette_index)))
                .copied()
                .unwrap_or([0, 0, 0, 255]);
            let is_nested = project.color_layers.contains_key(&layer.glyph);
            let fill = layer.gradient.as_ref().map_or_else(
                || {
                    if is_nested {
                        "none".to_string()
                    } else {
                        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
                    }
                },
                |_| format!("url(#glyph-studio-gradient-{index})"),
            );
            let opacity = if is_nested || layer.gradient.is_some() {
                1.0
            } else {
                f64::from(color[3]) / 255.0
            };
            let transform = svg_color_layer_transform(project, glyph_name, index);
            writeln!(
                svg,
                "<g fill=\"{fill}\" fill-opacity=\"{opacity:.6}\" fill-rule=\"nonzero\"{transform}>"
            )
            .ok()?;
            if project.color_layers.contains_key(&layer.glyph) {
                append_svg_nested_color_layers(
                    project,
                    &layer.glyph,
                    palette,
                    &mut vec![index],
                    &mut vec![glyph_name.to_string()],
                    &mut svg,
                )
                .ok()?;
            } else {
                append_svg_contours(
                    project,
                    &layer.glyph,
                    (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                    &mut Vec::new(),
                    &mut svg,
                )
                .ok()?;
            }
            svg.push_str("</g>\n");
        }
    } else {
        svg.push_str("<g fill=\"black\" fill-rule=\"nonzero\">\n");
        append_svg_contours(
            project,
            glyph_name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut svg,
        )
        .ok()?;
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>");
    Some(svg)
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
fn put_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}
fn put_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
fn put_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
fn gradient_angle(degrees: f64) -> u16 {
    font_types::F2Dot14::from_f32((degrees / 180.0 - 1.0) as f32)
        .to_bits()
        .cast_unsigned()
}
fn gradient_stop_offset(offset: f64) -> u16 {
    font_types::F2Dot14::from_f32(offset as f32)
        .to_bits()
        .cast_unsigned()
}
fn gradient_alpha(alpha: f64) -> u16 {
    font_types::F2Dot14::from_f32(alpha.clamp(0.0, 1.0) as f32)
        .to_bits()
        .cast_unsigned()
}
fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

pub fn export_otf(project: &FontProject, path: &Path) -> Result<(), String> {
    let master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
        .ok_or_else(|| "OTFには基準マスターが必要です".to_string())?;
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(&master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    selected.default_master_id = master_id.clone();
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-otf-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    export_ttf_for_master(&selected, &master_id, &temp)?;
    let sfnt = std::fs::read(&temp).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&temp);
    let mut charstrings = vec![cff::encode_type2_with_width(
        selected.metadata.units_per_em,
        &[],
    )?]; // .notdef
    for name in selected.glyph_names_sorted() {
        charstrings.push(cff::encode_project_glyph(&selected, name)?);
    }
    let cff_table = cff::build_minimal_cff(&selected.metadata.family_name, &charstrings)?;
    let otf = cff::rebuild_sfnt_with_table(&sfnt, *b"OTTO", *b"CFF ", &cff_table)?;
    let vorg = build_vorg(&selected, &master_id)?;
    let otf = cff::rebuild_sfnt_with_table(&otf, *b"OTTO", *b"VORG", &vorg)?;
    std::fs::write(path, otf).map_err(|error| error.to_string())
}

/// Writes a static CFF2/OpenType font using the selected base master.
pub fn export_otf_cff2(project: &FontProject, path: &Path) -> Result<(), String> {
    let master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
        .ok_or_else(|| "CFF2には基準マスターが必要です".to_string())?;
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(&master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    selected.default_master_id = master_id.clone();
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-cff2-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    export_ttf_for_master(&selected, &master_id, &temp)?;
    let sfnt = std::fs::read(&temp).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&temp);
    let mut charstrings = vec![Vec::new()];
    for name in selected.glyph_names_sorted() {
        charstrings.push(cff::encode_project_glyph_cff2(&selected, name)?);
    }
    let cff2_table = cff::build_minimal_cff2(&charstrings)?;
    let otf = cff::rebuild_sfnt_with_table(&sfnt, *b"OTTO", *b"CFF2", &cff2_table)?;
    std::fs::write(path, otf).map_err(|error| error.to_string())
}

/// Builds the CFF vertical-origin table from the active outlines and vertical
/// side bearings. The default origin is used whenever a glyph agrees with it,
/// keeping the table compact while preserving per-glyph Japanese vertical
/// metrics where they differ.
fn build_vorg(project: &FontProject, master_id: &str) -> Result<Vec<u8>, String> {
    let default_origin = checked_i16(
        project.master_metrics_for(master_id).ascender,
        "VORGデフォルト原点",
    )?;
    let mut records = Vec::new();
    for (glyph_id, name) in project.glyph_names_sorted().iter().enumerate() {
        let Some((_, _, _, max_y)) = project.outline_bounds_for_glyph(name) else {
            continue;
        };
        let metric = project.vertical_metrics_for_glyph_in_master(name, master_id);
        let origin = checked_i16(max_y + metric.top_side_bearing, "VORG原点")?;
        if origin != default_origin {
            records.push((
                u16::try_from(glyph_id + 1).map_err(|_| "VORGのグリフ数が多すぎます")?,
                origin,
            ));
        }
    }
    let mut bytes = Vec::with_capacity(8 + records.len() * 4);
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&default_origin.to_be_bytes());
    bytes.extend_from_slice(
        &u16::try_from(records.len())
            .map_err(|_| "VORGレコード数が多すぎます")?
            .to_be_bytes(),
    );
    for (glyph_id, origin) in records {
        bytes.extend_from_slice(&glyph_id.to_be_bytes());
        bytes.extend_from_slice(&origin.to_be_bytes());
    }
    Ok(bytes)
}

pub fn export_otf_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    if !project.masters.iter().any(|master| master.id == master_id) {
        return Err(format!("マスター '{}' がありません", master_id));
    }
    let mut selected = project.clone();
    selected.default_master_id = master_id.to_string();
    export_otf(&selected, path)
}

/// Exports one static CFF/OpenType font per master into a directory.
pub fn export_all_otf_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("出力対象のマスターがありません".into());
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("OTF出力先を作成できません: {error}"))?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let base: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        let base = if base.is_empty() {
            format!("master-{}", index + 1)
        } else {
            base
        };
        let mut filename = base.clone();
        let mut suffix = 2;
        while !used.insert(filename.clone()) {
            filename = format!("{base}_{suffix}");
            suffix += 1;
        }
        export_otf_for_master(
            project,
            &master.id,
            &directory.join(format!("{filename}.otf")),
        )?;
    }
    Ok(project.masters.len())
}

/// Writes a WOFF 1.0 wrapper around the generated TrueType font.
pub fn export_woff(project: &FontProject, path: &Path) -> Result<(), String> {
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    let export_result = export_ttf(project, &temp);
    if let Err(error) = export_result {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    let sfnt = match std::fs::read(&temp) {
        Ok(sfnt) => sfnt,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error.to_string());
        }
    };
    let _ = std::fs::remove_file(&temp);
    if sfnt.len() < 12 {
        return Err("生成されたTTFが不正です".into());
    }
    let count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    if sfnt.len() < 12 + count * 16 {
        return Err("TTFテーブルディレクトリが不正です".into());
    }
    let mut records = Vec::new();
    let mut body = Vec::new();
    for index in 0..count {
        let base = 12 + index * 16;
        let offset = u32::from_be_bytes(sfnt[base + 8..base + 12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(sfnt[base + 12..base + 16].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(length)
            .ok_or("TTFテーブル範囲が不正です")?;
        if end > sfnt.len() {
            return Err("TTFテーブル範囲が不正です".into());
        }
        let original = &sfnt[offset..end];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(original)
            .map_err(|error| error.to_string())?;
        let compressed = encoder.finish().map_err(|error| error.to_string())?;
        let data = if compressed.len() < original.len() {
            compressed
        } else {
            original.to_vec()
        };
        while body.len() % 4 != 0 {
            body.push(0);
        }
        let body_offset = 44 + count * 20 + body.len();
        let checksum = u32::from_be_bytes(sfnt[base + 4..base + 8].try_into().unwrap());
        records.push((
            sfnt[base..base + 4].to_vec(),
            body_offset as u32,
            data.len() as u32,
            length as u32,
            checksum,
        ));
        body.extend(data);
    }
    let total = 44 + count * 20 + body.len();
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(b"wOFF");
    output.extend_from_slice(&sfnt[0..4]);
    output.extend_from_slice(&(total as u32).to_be_bytes());
    output.extend_from_slice(&(count as u16).to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&(sfnt.len() as u32).to_be_bytes());
    output.extend_from_slice(&1u16.to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&[0u8; 20]);
    for (tag, offset, compressed, original, checksum) in records {
        output.extend_from_slice(&tag);
        output.extend_from_slice(&offset.to_be_bytes());
        output.extend_from_slice(&compressed.to_be_bytes());
        output.extend_from_slice(&original.to_be_bytes());
        output.extend_from_slice(&checksum.to_be_bytes());
    }
    output.extend_from_slice(&body);
    std::fs::write(path, output).map_err(|error| error.to_string())
}

/// Writes a WOFF2 wrapper around the generated TrueType font.
///
/// Keeping the TrueType export path for multi-master projects preserves
/// variable-font tables such as `fvar`, `gvar`, `HVAR`, and `MVAR`. Static
/// callers retain the established CFF-based WOFF2 path.
pub fn export_woff2(project: &FontProject, path: &Path) -> Result<(), String> {
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff2-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    if project.masters.len() >= 2 {
        export_ttf(project, &temp)?;
    } else {
        export_otf(project, &temp)?;
    }
    let sfnt = match std::fs::read(&temp) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error.to_string());
        }
    };
    let _ = std::fs::remove_file(&temp);
    let woff = oxifont_webfont::encode_woff2(&sfnt)
        .map_err(|error| format!("WOFF2圧縮に失敗しました: {error}"))?;
    std::fs::write(path, woff).map_err(|error| error.to_string())
}

/// 出力先の拡張子に応じて、対応するフォント形式で書き出す。
pub fn export_by_extension(project: &FontProject, path: &Path) -> Result<(), String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("ttf") => export_ttf(project, path),
        Some("otf") => export_otf(project, path),
        Some("woff") => export_woff(project, path),
        Some("woff2") => export_woff2(project, path),
        _ => Err("出力形式は ttf / otf / woff / woff2 に対応しています".into()),
    }
}

pub fn export_woff_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    let master = project
        .masters
        .iter()
        .find(|master| master.id == master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", master_id))?;
    let mut single = project.clone();
    let mut axis_values = master.axes.clone();
    axis_values.insert("wght".into(), master.weight);
    axis_values.insert("wdth".into(), master.width);
    apply_conditional_layers(&mut single, &axis_values);
    for glyph in single.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
        glyph.layers.clear();
    }
    single.masters = vec![master.clone()];
    single.default_master_id = master.id;
    export_woff(&single, path)
}

/// Writes a WOFF2 font containing one static master.
pub fn export_woff2_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    let master = project
        .masters
        .iter()
        .find(|master| master.id == master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", master_id))?;
    let mut single = project.clone();
    for glyph in single.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
        glyph.layers.clear();
    }
    single.masters = vec![master.clone()];
    single.default_master_id = master.id;
    export_woff2(&single, path)
}

pub fn export_all_woff2_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("マスターがありません".to_string());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let stem: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let stem = if stem.trim_matches('-').is_empty() {
            format!("master-{}", index + 1)
        } else {
            stem.trim_matches('-').to_string()
        };
        let mut output = directory.join(format!("{stem}.woff2"));
        let mut suffix = 2;
        while output.exists() || !used.insert(output.clone()) {
            output = directory.join(format!("{stem}-{suffix}.woff2"));
            suffix += 1;
        }
        export_woff2_for_master(project, &master.id, &output)?;
    }
    Ok(project.masters.len())
}

pub fn export_all_woff_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("マスターがありません".to_string());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let stem: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let stem = if stem.trim_matches('-').is_empty() {
            format!("master-{}", index + 1)
        } else {
            stem.trim_matches('-').to_string()
        };
        let mut output = directory.join(format!("{stem}.woff"));
        let mut suffix = 2;
        while output.exists() || !used.insert(output.clone()) {
            output = directory.join(format!("{stem}-{suffix}.woff"));
            suffix += 1;
        }
        export_woff_for_master(project, &master.id, &output)?;
    }
    Ok(project.masters.len())
}

/// Collect issues that would make an exported font ambiguous or unusable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub message: String,
    pub glyph_name: Option<String>,
}

pub fn validate_project(project: &FontProject) -> Vec<String> {
    let mut issues = Vec::new();
    let is_noncharacter =
        |unicode: u32| (0xFDD0..=0xFDEF).contains(&unicode) || (unicode & 0xFFFF) >= 0xFFFE;
    let mut variation_keys = std::collections::HashSet::new();
    for variation in &project.unicode_variation_sequences {
        let valid_scalar = variation.base <= 0x10FFFF
            && !(0xD800..=0xDFFF).contains(&variation.base)
            && variation.selector <= 0xFFFFFF;
        let valid_selector = (0xFE00..=0xFE0F).contains(&variation.selector)
            || (0xE0100..=0xE01EF).contains(&variation.selector);
        if !valid_scalar || !valid_selector {
            issues.push(format!(
                "IVSのUnicodeまたはセレクタが不正です: U+{:X} U+{:X}",
                variation.base, variation.selector
            ));
        }
        if !project.glyphs.contains_key(&variation.glyph) {
            issues.push(format!(
                "IVSが存在しないグリフ '{}' を参照しています",
                variation.glyph
            ));
        }
        if !variation_keys.insert((variation.base, variation.selector)) {
            issues.push(format!(
                "IVSのUnicode／セレクタが重複しています: U+{:X} U+{:X}",
                variation.base, variation.selector
            ));
        }
    }
    for (tag, points) in &project.axis_mappings {
        if tag.len() != 4 || !tag.is_ascii() {
            issues.push(format!(
                "avar軸タグ '{}' はASCII 4文字で指定してください",
                tag
            ));
        }
        let mut inputs = std::collections::HashSet::new();
        for point in points {
            if !point.input.is_finite()
                || !point.output.is_finite()
                || !(-1.0..=1.0).contains(&point.input)
                || !(-1.0..=1.0).contains(&point.output)
            {
                issues.push(format!(
                    "avar軸 '{}' のマッピング値は-1.0〜1.0の有限値で指定してください",
                    tag
                ));
            }
            if !inputs.insert(point.input.to_bits()) {
                issues.push(format!("avar軸 '{}' の入力座標が重複しています", tag));
            }
        }
    }
    if project.metadata.family_name.trim().is_empty() {
        issues.push("ファミリー名が空です".into());
    }
    if project.metadata.style_name.trim().is_empty() {
        issues.push("スタイル名が空です".into());
    }
    if !project.metadata.font_revision.is_finite()
        || !(0.0..=65535.0).contains(&project.metadata.font_revision)
    {
        issues.push("フォントバージョンが0〜65535の範囲外です".into());
    }
    if !project.metadata.units_per_em.is_finite()
        || !(16.0..=16384.0).contains(&project.metadata.units_per_em)
    {
        issues.push("UPMが16〜16384の範囲外です".into());
    }
    if !project.metadata.ascender.is_finite()
        || !project.metadata.descender.is_finite()
        || !project.metadata.line_gap.is_finite()
    {
        issues.push("フォントメトリクスに不正な値があります".into());
    }
    for (master_id, metrics) in &project.metrics_by_master {
        if !project.masters.iter().any(|master| master.id == *master_id) {
            issues.push(format!(
                "マスターメトリクスが存在しないマスター '{}' を参照しています",
                master_id
            ));
        }
        if !metrics.ascender.is_finite()
            || !metrics.descender.is_finite()
            || !metrics.line_gap.is_finite()
            || metrics.ascender < i16::MIN as f64
            || metrics.ascender > i16::MAX as f64
            || metrics.descender < i16::MIN as f64
            || metrics.descender > i16::MAX as f64
            || metrics.line_gap < i16::MIN as f64
            || metrics.line_gap > i16::MAX as f64
            || metrics.ascender.fract() != 0.0
            || metrics.descender.fract() != 0.0
            || metrics.line_gap.fract() != 0.0
        {
            issues.push(format!(
                "マスター '{}' のメトリクスがTrueTypeの範囲外です",
                master_id
            ));
        }
    }
    for (glyph_name, masters) in &project.background_opacities {
        for (master_id, opacity) in masters {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "背景画像不透明度が存在しないグリフ '{}' を参照しています",
                    glyph_name
                ));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!(
                    "背景画像不透明度が存在しないマスター '{}' を参照しています",
                    master_id
                ));
            }
            if !opacity.is_finite() || !(0.0..=1.0).contains(opacity) {
                issues.push(format!(
                    "グリフ '{}' の背景画像不透明度（マスター '{}'）が0〜1の範囲外です",
                    glyph_name, master_id
                ));
            }
        }
    }
    for (glyph_name, masters) in &project.background_images {
        for master_id in masters.keys() {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "背景画像が存在しないグリフ '{}' を参照しています",
                    glyph_name
                ));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!(
                    "背景画像が存在しないマスター '{}' を参照しています",
                    master_id
                ));
            }
        }
    }
    for (glyph_name, masters) in &project.background_transforms {
        for (master_id, transform) in masters {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "背景画像変形が存在しないグリフ '{}' を参照しています",
                    glyph_name
                ));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!(
                    "背景画像変形が存在しないマスター '{}' を参照しています",
                    master_id
                ));
            }
            if !transform.x.is_finite()
                || !transform.y.is_finite()
                || !transform.scale.is_finite()
                || !transform.rotation.is_finite()
                || transform.scale <= 0.0
            {
                issues.push(format!(
                    "グリフ '{}' の背景画像変形（マスター '{}'）に不正な値があります",
                    glyph_name, master_id
                ));
            }
        }
    }
    for (glyph_name, layers) in &project.conditional_layers {
        if !project.glyphs.contains_key(glyph_name) {
            issues.push(format!(
                "条件レイヤーが存在しないグリフ '{}' を参照しています",
                glyph_name
            ));
        }
        let mut layer_ids = std::collections::HashSet::new();
        for layer in layers {
            if layer.id.trim().is_empty() || !layer_ids.insert(layer.id.clone()) {
                issues.push(format!(
                    "グリフ '{}' の条件レイヤーIDが空または重複しています",
                    glyph_name
                ));
            }
            for (tag, range) in &layer.conditions {
                if tag.len() != 4 || !tag.is_ascii() {
                    issues.push(format!(
                        "条件レイヤー '{}' の軸タグ '{}' が不正です",
                        layer.id, tag
                    ));
                } else if !tag.eq_ignore_ascii_case("wght")
                    && !tag.eq_ignore_ascii_case("wdth")
                    && !project.masters.iter().any(|master| {
                        master
                            .axes
                            .keys()
                            .any(|axis| axis.eq_ignore_ascii_case(tag))
                    })
                {
                    issues.push(format!(
                        "条件レイヤー '{}' が未定義の軸 '{}' を参照しています",
                        layer.id, tag
                    ));
                }
                if range.min.zip(range.max).is_some_and(|(min, max)| min > max)
                    || range.min.is_some_and(|value| !value.is_finite())
                    || range.max.is_some_and(|value| !value.is_finite())
                {
                    issues.push(format!("条件レイヤー '{}' の軸範囲が不正です", layer.id));
                }
            }
        }
    }
    let mut unicodes = std::collections::HashMap::<u32, String>::new();
    let mut master_ids = std::collections::HashSet::new();
    let mut master_names = std::collections::HashSet::new();
    for master in &project.masters {
        if master.id.trim().is_empty() {
            issues.push("マスターIDが空です".into());
        } else if !master_ids.insert(master.id.clone()) {
            issues.push(format!("マスターIDが重複しています: {}", master.id));
        }
        if master.name.trim().is_empty() {
            issues.push(format!("マスター '{}' の表示名が空です", master.id));
        } else if !master_names.insert(master.name.clone()) {
            issues.push(format!("マスター名が重複しています: {}", master.name));
        }
        if !master.weight.is_finite() || !master.width.is_finite() {
            issues.push(format!(
                "マスター '{}' のWeightまたはWidthが不正です",
                master.name
            ));
        }
        for (tag, value) in &master.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                issues.push(format!(
                    "マスター '{}' の軸タグ '{}' は4文字ASCIIで指定してください",
                    master.name, tag
                ));
            }
            if !value.is_finite() {
                issues.push(format!(
                    "マスター '{}' の軸 '{}' の値が不正です",
                    master.name, tag
                ));
            }
        }
    }
    let mut instance_names = std::collections::HashSet::new();
    for instance in &project.instances {
        if instance.name.trim().is_empty() {
            issues.push("名前付きインスタンスの表示名が空です".into());
        } else if !instance_names.insert(instance.name.trim().to_string()) {
            issues.push(format!(
                "名前付きインスタンス名が重複しています: {}",
                instance.name.trim()
            ));
        }
        if !instance.weight.is_finite() || !instance.width.is_finite() {
            issues.push(format!(
                "名前付きインスタンス '{}' のWeightまたはWidthが不正です",
                instance.name
            ));
        }
        for (tag, value) in &instance.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                issues.push(format!(
                    "名前付きインスタンス '{}' の軸タグ '{}' は4文字ASCIIで指定してください",
                    instance.name, tag
                ));
            }
            if !value.is_finite() {
                issues.push(format!(
                    "名前付きインスタンス '{}' の軸 '{}' の値が不正です",
                    instance.name, tag
                ));
            }
        }
    }
    if project.masters.is_empty() {
        issues.push("マスターが1つもありません".into());
    } else if !master_ids.contains(&project.default_master_id) {
        issues.push(format!(
            "デフォルトマスター '{}' が存在しません",
            project.default_master_id
        ));
    }
    let mut axis_display_names = std::collections::HashSet::new();
    for (tag, name) in &project.axis_names {
        if !master_ids.iter().any(|master_id| {
            project
                .masters
                .iter()
                .find(|master| &master.id == master_id)
                .is_some_and(|master| master.axes.contains_key(tag))
        }) {
            issues.push(format!("軸名 '{}' が存在しない軸タグを参照しています", tag));
        }
        if name.trim().is_empty() {
            issues.push(format!("軸タグ '{}' の表示名が空です", tag));
        } else if !axis_display_names.insert(name.trim().to_string()) {
            issues.push(format!("可変軸の表示名が重複しています: {}", name.trim()));
        }
    }
    let mut ordered_glyphs = std::collections::HashSet::new();
    for name in &project.glyph_order {
        if !project.glyphs.contains_key(name) {
            issues.push(format!("グリフ順序に未定義グリフがあります: {name}"));
        } else if !ordered_glyphs.insert(name) {
            issues.push(format!("グリフ順序に重複があります: {name}"));
        }
    }
    for (index, guide) in project.guidelines.iter().enumerate() {
        if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
            issues.push(format!("ガイド{}の座標または角度が不正です", index + 1));
        }
    }
    for (name, glyph) in &project.glyphs {
        if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
            issues.push(format!("グリフ名が不正です: '{name}'"));
        }
        if glyph.name != *name {
            issues.push(format!(
                "グリフ名の登録が不一致です: '{name}' / '{}'",
                glyph.name
            ));
        }
        if !glyph.width.is_finite() || glyph.width < 0.0 {
            issues.push(format!("グリフ '{}' の幅が不正です", name));
        }
        for (label, group) in [
            ("左カーニンググループ", glyph.left_kerning_group.trim()),
            ("右カーニンググループ", glyph.right_kerning_group.trim()),
        ] {
            if group.chars().any(char::is_whitespace) {
                issues.push(format!(
                    "グリフ '{}' の{}名に空白があります: '{}'",
                    name, label, group
                ));
            }
        }
        let mut anchor_names = std::collections::HashSet::new();
        for anchor in &glyph.anchors {
            if anchor.name.trim().is_empty() {
                issues.push(format!("グリフ '{}' に名前のないアンカーがあります", name));
            } else if !anchor_names.insert(anchor.name.trim().to_string()) {
                issues.push(format!(
                    "グリフ '{}' にアンカー名 '{}' が重複しています",
                    name,
                    anchor.name.trim()
                ));
            }
            if !anchor.x.is_finite() || !anchor.y.is_finite() {
                issues.push(format!(
                    "グリフ '{}' のアンカー '{}' の座標が不正です",
                    name, anchor.name
                ));
            }
        }
        for (index, guide) in glyph.guidelines.iter().enumerate() {
            if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
                issues.push(format!(
                    "グリフ '{}' のガイド{}の座標または角度が不正です",
                    name,
                    index + 1
                ));
            }
        }
        for (master_id, guides) in &glyph.master_guidelines {
            for (index, guide) in guides.iter().enumerate() {
                if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' のガイド{}の座標または角度が不正です",
                        name,
                        master_id,
                        index + 1
                    ));
                }
            }
        }
        for (contour_index, contour) in glyph.contours.iter().enumerate() {
            if contour.points.is_empty() {
                issues.push(format!(
                    "グリフ '{}' の輪郭{}が空です",
                    name,
                    contour_index + 1
                ));
            }
            validate_contour_topology(
                contour,
                &format!("グリフ '{}' の輪郭{}", name, contour_index + 1),
                &mut issues,
            );
            for point in &contour.points {
                if !point.x.is_finite() || !point.y.is_finite() {
                    issues.push(format!(
                        "グリフ '{}' の輪郭{}に不正な座標があります",
                        name,
                        contour_index + 1
                    ));
                    break;
                }
            }
        }
        let mut codepoints = glyph.unicodes.clone();
        if let Some(unicode) = glyph.unicode {
            if !codepoints.contains(&unicode) {
                codepoints.push(unicode);
            }
        }
        for unicode in codepoints {
            if unicode > 0x10FFFF || (0xD800..=0xDFFF).contains(&unicode) {
                issues.push(format!(
                    "グリフ '{}' のUnicode U+{unicode:04X}が不正です",
                    name
                ));
                continue;
            }
            if is_noncharacter(unicode) {
                issues.push(format!(
                    "グリフ '{}' のUnicode U+{unicode:04X}は非文字です",
                    name
                ));
            }
            if let Some(previous) = unicodes.insert(unicode, name.clone()) {
                issues.push(format!(
                    "Unicode U+{unicode:04X} が重複: {previous} / {name}"
                ));
            }
        }
        for component in &glyph.components {
            if !project.glyphs.contains_key(&component.base) {
                issues.push(format!(
                    "グリフ '{}' が未定義コンポーネント '{}' を参照",
                    name, component.base
                ));
            }
            let transform = [
                component.x_scale,
                component.xy_scale,
                component.yx_scale,
                component.y_scale,
                component.x_offset,
                component.y_offset,
            ];
            if transform.iter().any(|value| !value.is_finite()) {
                issues.push(format!("グリフ '{}' のコンポーネント変換が不正です", name));
            }
        }
        for (master_id, layer) in &glyph.layers {
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!(
                    "グリフ '{}' に未定義マスター '{}' のレイヤーがあります",
                    name, master_id
                ));
            }
            if !layer.width.is_finite() || layer.width < 0.0 {
                issues.push(format!(
                    "グリフ '{}' のマスター '{}' の幅が不正です",
                    name, master_id
                ));
            }
            let mut layer_anchor_names = std::collections::HashSet::new();
            for anchor in &layer.anchors {
                if anchor.name.trim().is_empty() {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' に名前のないアンカーがあります",
                        name, master_id
                    ));
                } else if !layer_anchor_names.insert(anchor.name.trim().to_string()) {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' にアンカー名 '{}' が重複しています",
                        name,
                        master_id,
                        anchor.name.trim()
                    ));
                }
                if !anchor.x.is_finite() || !anchor.y.is_finite() {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' のアンカー '{}' の座標が不正です",
                        name, master_id, anchor.name
                    ));
                }
            }
            for (contour_index, contour) in layer.contours.iter().enumerate() {
                if contour.points.is_empty() {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' の輪郭{}が空です",
                        name,
                        master_id,
                        contour_index + 1
                    ));
                }
                validate_contour_topology(
                    contour,
                    &format!(
                        "グリフ '{}' のマスター '{}' の輪郭{}",
                        name,
                        master_id,
                        contour_index + 1
                    ),
                    &mut issues,
                );
                if contour
                    .points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
                {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' に不正な座標があります",
                        name, master_id
                    ));
                }
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!(
                    "グリフ '{}' に未定義マスター '{}' のレイヤーがあります",
                    name, master_id
                ));
            }
            for component in &layer.components {
                if !project.glyphs.contains_key(&component.base) {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' が未定義コンポーネント '{}' を参照",
                        name, master_id, component.base
                    ));
                }
                let transform = [
                    component.x_scale,
                    component.xy_scale,
                    component.yx_scale,
                    component.y_scale,
                    component.x_offset,
                    component.y_offset,
                ];
                if transform.iter().any(|value| !value.is_finite()) {
                    issues.push(format!(
                        "グリフ '{}' のマスター '{}' のコンポーネント変換が不正です",
                        name, master_id
                    ));
                }
            }
        }
    }
    let palette_count = project
        .color_palettes
        .first()
        .map_or(0, |palette| palette.len());
    for (index, palette) in project.color_palettes.iter().enumerate() {
        if palette.is_empty() {
            issues.push(format!("カラー パレット{}が空です", index + 1));
        }
        if palette.len() != palette_count {
            issues.push(format!("カラー パレット{}の色数が一致しません", index + 1));
        }
    }
    for (base, layers) in &project.color_layers {
        if !project.glyphs.contains_key(base) {
            issues.push(format!("カラー基底グリフ '{}' が存在しません", base));
        }
        for (index, layer) in layers.iter().enumerate() {
            if !project.glyphs.contains_key(&layer.glyph) {
                issues.push(format!(
                    "カラーグリフ '{}' の層{}が未定義グリフ '{}' を参照しています",
                    base,
                    index + 1,
                    layer.glyph
                ));
            }
            if usize::from(layer.palette_index) >= palette_count {
                issues.push(format!(
                    "カラーグリフ '{}' の層{}のパレット番号が範囲外です",
                    base,
                    index + 1
                ));
            }
            if !layer.alpha.is_finite() || !(0.0..=1.0).contains(&layer.alpha) {
                issues.push(format!(
                    "カラーグリフ '{}' の層{}のアルファ値が範囲外です",
                    base,
                    index + 1
                ));
            }
            if let Some(gradient) = &layer.gradient {
                for (label, value) in [
                    ("始点X", gradient.x0),
                    ("始点Y", gradient.y0),
                    ("終点X", gradient.x1),
                    ("終点Y", gradient.y1),
                    ("回転点X", gradient.x2),
                    ("回転点Y", gradient.y2),
                ] {
                    if !value.is_finite()
                        || value < f64::from(i16::MIN)
                        || value > f64::from(i16::MAX)
                    {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}のグラデーション{}が不正です",
                            base,
                            index + 1,
                            label
                        ));
                    }
                }
                for (label, palette_index) in [
                    ("開始", gradient.start_palette_index),
                    ("終了", gradient.end_palette_index),
                ] {
                    if usize::from(palette_index) >= palette_count {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}のグラデーション{}色が範囲外です",
                            base,
                            index + 1,
                            label
                        ));
                    }
                }
                let mut previous_offset = f64::NEG_INFINITY;
                for (stop_index, stop) in gradient.stops.iter().enumerate() {
                    if !stop.offset.is_finite()
                        || stop.offset < -2.0
                        || stop.offset >= 2.0
                        || stop.offset < previous_offset
                    {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}の色ストップ{}の位置が不正です",
                            base,
                            index + 1,
                            stop_index + 1
                        ));
                    }
                    if !stop.alpha.is_finite() || !(0.0..=1.0).contains(&stop.alpha) {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}の色ストップ{}の不透明度が不正です",
                            base,
                            index + 1,
                            stop_index + 1
                        ));
                    }
                    if usize::from(stop.palette_index) >= palette_count {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}の色ストップ{}のパレット番号が範囲外です",
                            base,
                            index + 1,
                            stop_index + 1
                        ));
                    }
                    previous_offset = stop.offset;
                }
                if gradient.radius0 < 0.0 || gradient.radius1 < 0.0 {
                    issues.push(format!(
                        "カラーグリフ '{}' の層{}のグラデーション半径が負です",
                        base,
                        index + 1
                    ));
                }
                if matches!(gradient.kind, crate::font_data::ColorGradientKind::Linear) {
                    let p0p1 = (gradient.x1 - gradient.x0, gradient.y1 - gradient.y0);
                    let p0p2 = (gradient.x2 - gradient.x0, gradient.y2 - gradient.y0);
                    let determinant = p0p1.0 * p0p2.1 - p0p1.1 * p0p2.0;
                    if determinant.abs() <= f64::EPSILON {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}の線形グラデーションの回転点が退化しています",
                            base,
                            index + 1
                        ));
                    }
                }
                if matches!(gradient.kind, crate::font_data::ColorGradientKind::Sweep)
                    && (!(0.0..=360.0).contains(&gradient.start_angle)
                        || !(0.0..=360.0).contains(&gradient.end_angle))
                {
                    issues.push(format!(
                        "カラーグリフ '{}' の層{}のスイープ角度が0〜360度の範囲外です",
                        base,
                        index + 1
                    ));
                }
            }
            if let Some(Some(transform)) = project
                .color_layer_transforms
                .get(base)
                .and_then(|transforms| transforms.get(index))
            {
                for (label, value) in [
                    ("XX", transform.xx),
                    ("YX", transform.yx),
                    ("XY", transform.xy),
                    ("YY", transform.yy),
                    ("DX", transform.dx),
                    ("DY", transform.dy),
                ] {
                    if !value.is_finite()
                        || value * 65_536.0 < f64::from(i32::MIN)
                        || value * 65_536.0 > f64::from(i32::MAX)
                    {
                        issues.push(format!(
                            "カラーグリフ '{}' の層{}のCOLR変形{}が不正です",
                            base,
                            index + 1,
                            label
                        ));
                    }
                }
            }
        }
        if let Some(transforms) = project.color_layer_transforms.get(base) {
            if transforms.len() > layers.len() {
                issues.push(format!(
                    "カラー基底グリフ '{}' のCOLR変形数がカラー層数を超えています",
                    base
                ));
            }
        }
    }
    for base in project.color_layer_transforms.keys() {
        if !project.color_layers.contains_key(base) {
            issues.push(format!(
                "COLR変形がカラー層のない基底グリフ '{}' に設定されています",
                base
            ));
        }
    }
    fn visit_color_graph(
        project: &FontProject,
        name: &str,
        visiting: &mut Vec<String>,
        reported: &mut std::collections::HashSet<String>,
    ) {
        if let Some(index) = visiting.iter().position(|item| item == name) {
            reported.insert(visiting[index..].join(" -> ") + " -> " + name);
            return;
        }
        let Some(layers) = project.color_layers.get(name) else {
            return;
        };
        visiting.push(name.to_string());
        for layer in layers {
            if project.color_layers.contains_key(&layer.glyph) {
                visit_color_graph(project, &layer.glyph, visiting, reported);
            }
        }
        visiting.pop();
    }
    let mut color_cycles = std::collections::HashSet::new();
    for name in project.color_layers.keys() {
        visit_color_graph(project, name, &mut Vec::new(), &mut color_cycles);
    }
    issues.extend(
        color_cycles
            .into_iter()
            .map(|cycle| format!("COLRカラーグリフ循環参照: {cycle}")),
    );
    fn visit_component_graph(
        project: &FontProject,
        name: &str,
        visiting: &mut Vec<String>,
        reported: &mut std::collections::HashSet<String>,
    ) {
        if let Some(index) = visiting.iter().position(|item| item == name) {
            let cycle = visiting[index..].join(" -> ") + " -> " + name;
            reported.insert(cycle);
            return;
        }
        let Some(glyph) = project.glyphs.get(name) else {
            return;
        };
        visiting.push(name.to_string());
        for component in &glyph.components {
            visit_component_graph(project, &component.base, visiting, reported);
        }
        for layer in glyph.layers.values() {
            for component in &layer.components {
                visit_component_graph(project, &component.base, visiting, reported);
            }
        }
        visiting.pop();
    }
    let mut cycles = std::collections::HashSet::new();
    for name in project.glyphs.keys() {
        visit_component_graph(project, name, &mut Vec::new(), &mut cycles);
    }
    issues.extend(
        cycles
            .into_iter()
            .map(|cycle| format!("コンポーネント循環参照: {cycle}")),
    );
    for ((left, right), value) in &project.kerning {
        if !value.is_finite() {
            issues.push(format!("カーニング値が不正: {left} / {right}"));
        } else if *value < i16::MIN as f64 || *value > i16::MAX as f64 {
            issues.push(format!(
                "カーニング値が範囲外です: {left} / {right} ({value})"
            ));
        }
        if !project.glyphs.contains_key(left) || !project.glyphs.contains_key(right) {
            issues.push(format!("未定義グリフのカーニング: {left} / {right}"));
        }
    }
    for (master_id, pairs) in &project.kerning_by_master {
        for ((left, right), value) in pairs {
            if !value.is_finite() {
                issues.push(format!(
                    "マスター '{}' のカーニング値が不正: {left} / {right}",
                    master_id
                ));
            } else if *value < i16::MIN as f64 || *value > i16::MAX as f64 {
                issues.push(format!(
                    "マスター '{}' のカーニング値が範囲外です: {left} / {right} ({value})",
                    master_id
                ));
            }
            if !project.glyphs.contains_key(left) || !project.glyphs.contains_key(right) {
                issues.push(format!(
                    "マスター '{}' に未定義グリフのカーニング: {left} / {right}",
                    master_id
                ));
            }
        }
    }
    if project.masters.is_empty() {
        issues.push("マスターがありません".into());
    } else if !project
        .masters
        .iter()
        .any(|master| master.id == project.default_master_id)
    {
        issues.push("基準マスターが見つかりません".into());
    }
    for (name, glyph) in &project.glyphs {
        let Some(reference_id) = glyph
            .layers
            .contains_key(&project.default_master_id)
            .then_some(project.default_master_id.as_str())
            .or_else(|| glyph.layers.keys().next().map(String::as_str))
        else {
            continue;
        };
        let Some(reference) = glyph.layers.get(reference_id) else {
            continue;
        };
        for (master_id, layer) in &glyph.layers {
            if master_id == reference_id {
                continue;
            }
            if reference.interpolate(layer, 0.5).is_none() {
                issues.push(format!(
                    "グリフ '{}' のマスター '{}' は基準マスターと補間互換ではありません",
                    name, master_id
                ));
            }
        }
    }
    let feature_source = project.feature_source();
    if let Err(error) = validate_feature_source(&feature_source) {
        issues.push(error);
    } else {
        issues.extend(validate_feature_class_definitions(
            &feature_source,
            &project.glyphs,
        ));
        issues.extend(validate_feature_glyph_references(
            &feature_source,
            &project.glyphs,
        ));
    }
    issues
}

/// UI向けの検証結果。書き出し用の既存APIを保ったまま、対象グリフを構造化する。
pub fn validate_project_detailed(project: &FontProject) -> Vec<ValidationIssue> {
    validate_project(project)
        .into_iter()
        .map(|message| {
            let glyph_name = project.glyphs.keys().find(|name| {
                message.contains(&format!("'{}'", name))
                    || message.contains(&format!("{} の", name))
            });
            ValidationIssue {
                message,
                glyph_name: glyph_name.cloned(),
            }
        })
        .collect()
}

/// 指定した2マスター間で、全グリフを補間できるか確認する。
pub fn validate_interpolation(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if from_master_id == to_master_id {
        issues.push(ValidationIssue {
            message: "始点と終点に同じマスターは指定できません".into(),
            glyph_name: None,
        });
        return issues;
    }
    if !project
        .masters
        .iter()
        .any(|master| master.id == from_master_id)
    {
        issues.push(ValidationIssue {
            message: format!("マスター '{}' がありません", from_master_id),
            glyph_name: None,
        });
    }
    if !project
        .masters
        .iter()
        .any(|master| master.id == to_master_id)
    {
        issues.push(ValidationIssue {
            message: format!("マスター '{}' がありません", to_master_id),
            glyph_name: None,
        });
    }
    if !issues.is_empty() {
        return issues;
    }
    for glyph in project.glyphs.values() {
        match glyph
            .layers
            .get(from_master_id)
            .zip(glyph.layers.get(to_master_id))
        {
            None => issues.push(ValidationIssue {
                message: "対応する補間レイヤーがありません".into(),
                glyph_name: Some(glyph.name.clone()),
            }),
            Some((from_layer, to_layer)) => {
                if let Some(reason) = interpolation_mismatch_reason(from_layer, to_layer) {
                    issues.push(ValidationIssue {
                        message: reason,
                        glyph_name: Some(glyph.name.clone()),
                    });
                }
            }
        }
    }
    issues
}

fn interpolation_mismatch_reason(
    from: &crate::font_data::GlyphLayer,
    to: &crate::font_data::GlyphLayer,
) -> Option<String> {
    if from.contours.len() != to.contours.len() {
        return Some(format!(
            "輪郭数が不一致です（{} → {}）",
            from.contours.len(),
            to.contours.len()
        ));
    }
    for (index, (from_contour, to_contour)) in from.contours.iter().zip(&to.contours).enumerate() {
        if from_contour.points.len() != to_contour.points.len() {
            return Some(format!(
                "{}番目の輪郭のノード数が不一致です（{} → {}）",
                index + 1,
                from_contour.points.len(),
                to_contour.points.len()
            ));
        }
        if let Some(point_index) = from_contour
            .points
            .iter()
            .zip(&to_contour.points)
            .position(|(from_point, to_point)| from_point.point_type != to_point.point_type)
        {
            return Some(format!(
                "{}番目の輪郭の{}番目のノード種別が不一致です",
                index + 1,
                point_index + 1
            ));
        }
    }
    if from.components.len() != to.components.len() {
        return Some(format!(
            "コンポーネント数が不一致です（{} → {}）",
            from.components.len(),
            to.components.len()
        ));
    }
    if let Some(index) = from
        .components
        .iter()
        .zip(&to.components)
        .position(|(from_component, to_component)| from_component.base != to_component.base)
    {
        return Some(format!(
            "{}番目のコンポーネントの参照先が不一致です",
            index + 1
        ));
    }
    if from.anchors.len() != to.anchors.len() {
        return Some(format!(
            "アンカー数が不一致です（{} → {}）",
            from.anchors.len(),
            to.anchors.len()
        ));
    }
    if let Some(anchor) = from
        .anchors
        .iter()
        .find(|anchor| !to.anchors.iter().any(|other| other.name == anchor.name))
    {
        return Some(format!(
            "アンカー「{}」が終点マスターにありません",
            anchor.name
        ));
    }
    None
}

fn validate_contour_topology(
    contour: &crate::font_data::Contour,
    label: &str,
    issues: &mut Vec<String>,
) {
    for (index, pair) in contour
        .points
        .iter()
        .zip(contour.points.iter().cycle().skip(1))
        .take(contour.points.len())
        .enumerate()
    {
        if (pair.0.x - pair.1.x).abs() < 1e-9 && (pair.0.y - pair.1.y).abs() < 1e-9 {
            issues.push(format!(
                "{label}に重複した隣接点があります（{}番）",
                index + 1
            ));
            break;
        }
    }
    let on_curve_count = contour
        .points
        .iter()
        .filter(|point| point.is_on_curve())
        .count();
    if !contour.points.is_empty() && on_curve_count < 2 {
        issues.push(format!("{label}にオンカーブ点が2つ未満です"));
    }
    if contour.points.len() >= 2 {
        let mut consecutive_off = 0;
        for point in contour.points.iter().chain(contour.points.first()) {
            if point.is_on_curve() {
                consecutive_off = 0;
            } else {
                consecutive_off += 1;
                if consecutive_off > 2 {
                    issues.push(format!("{label}にオフカーブ点が3つ以上連続しています"));
                    break;
                }
            }
        }
    }
    if contour_self_intersects(contour) {
        issues.push(format!("{label}が自己交差しています"));
    }
    let on_curve: Vec<_> = contour
        .points
        .iter()
        .filter(|point| point.is_on_curve())
        .collect();
    if on_curve.len() >= 3 {
        let area = on_curve
            .iter()
            .zip(on_curve.iter().cycle().skip(1))
            .take(on_curve.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f64>()
            .abs()
            * 0.5;
        if area < 1e-9 {
            issues.push(format!("{label}の面積が0です（退化した輪郭）"));
        }
    }
}

fn contour_self_intersects(contour: &crate::font_data::Contour) -> bool {
    if contour.points.len() < 4 {
        return false;
    }
    let mut vertices = Vec::new();
    kurbo::flatten(contour.to_bezpath(), 0.5, |element| {
        if let kurbo::PathEl::MoveTo(point) | kurbo::PathEl::LineTo(point) = element {
            vertices.push(point);
        }
    });
    if vertices.len() < 4 {
        return false;
    }
    let intersects = |a: kurbo::Point, b: kurbo::Point, c: kurbo::Point, d: kurbo::Point| {
        let cross = |p: kurbo::Point, q: kurbo::Point, r: kurbo::Point| {
            (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
        };
        let ab_c = cross(a, b, c);
        let ab_d = cross(a, b, d);
        let cd_a = cross(c, d, a);
        let cd_b = cross(c, d, b);
        let eps = 1e-7;
        ((ab_c > eps && ab_d < -eps) || (ab_c < -eps && ab_d > eps))
            && ((cd_a > eps && cd_b < -eps) || (cd_a < -eps && cd_b > eps))
    };
    let segment_count = vertices.len();
    for first in 0..segment_count {
        let first_end = (first + 1) % segment_count;
        for second in (first + 1)..segment_count {
            let second_end = (second + 1) % segment_count;
            if first == second
                || first_end == second
                || second_end == first
                || (first == 0 && second_end == 0)
            {
                continue;
            }
            if intersects(
                vertices[first],
                vertices[first_end],
                vertices[second],
                vertices[second_end],
            ) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn validate_feature_glyph_references(
    source: &str,
    glyphs: &std::collections::HashMap<String, crate::font_data::GlyphData>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let mut defined_classes = std::collections::HashSet::new();
    for statement in source.split(';') {
        if let Some((name, _)) = statement.split_once('=') {
            let name = name.trim();
            if name.starts_with('@') {
                defined_classes.insert(name);
            }
        }
        if statement.trim_start().starts_with("markClass ") {
            if let Some(name) = statement
                .split_whitespace()
                .rev()
                .find(|token| token.starts_with('@'))
            {
                defined_classes.insert(name);
            }
        }
    }
    let keywords = [
        "sub",
        "substitute",
        "pos",
        "position",
        "by",
        "from",
        "ignore",
        "lookup",
        "enum",
        "mark",
        "NULL",
    ];
    let mut offset = 0;
    for statement_text in source.split(';') {
        let line_number = source[..offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let code = statement_text
            .lines()
            .map(|line| line.split('#').next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ");
        let mut statement = false;
        for raw in code.split_whitespace() {
            let token = raw.trim_matches(|c: char| ";{},()[]".contains(c));
            if token == "sub" || token == "substitute" || token == "pos" || token == "position" {
                statement = true;
                continue;
            }
            if !statement || token.is_empty() || token.contains('<') || token.contains('>') {
                continue;
            }
            if token.starts_with('@') {
                if !defined_classes.contains(token) {
                    issues.push(format!(
                        "OpenType feature {}行目の未定義クラス '{}'",
                        line_number, token
                    ));
                }
                continue;
            }
            if token == "by" || token == "from" {
                continue;
            }
            if keywords.contains(&token) || token.parse::<f64>().is_ok() {
                continue;
            }
            let glyph_name = token.trim_end_matches('\'');
            if glyph_name.is_empty()
                || glyph_name.starts_with('[')
                || glyph_name.starts_with('@')
                || glyph_name == "<"
                || glyph_name == ">"
            {
                continue;
            }
            if !glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "OpenType feature {}行目の未定義グリフ '{}': 出力時に無視される可能性があります",
                    line_number,
                    glyph_name
                ));
            }
        }
        offset = offset.saturating_add(statement_text.len() + 1);
    }
    issues.sort();
    issues.dedup();
    issues
}

pub(crate) fn validate_feature_class_definitions(
    source: &str,
    glyphs: &std::collections::HashMap<String, crate::font_data::GlyphData>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let mut names = std::collections::HashSet::new();
    for (index, statement) in source.split(';').enumerate() {
        let trimmed = statement.trim();
        let Some((raw_name, raw_values)) = trimmed.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        if !name.starts_with('@') {
            continue;
        }
        if name.len() < 2
            || !name[1..]
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            issues.push(format!("OpenType Class {}個目の名前が不正です", index + 1));
        }
        if !names.insert(name.to_string()) {
            issues.push(format!("OpenType Class '{}' が重複しています", name));
        }
        let values = raw_values.trim();
        if !values.starts_with('[') || !values.ends_with(']') {
            issues.push(format!("OpenType Class '{}' は [ ] で囲んでください", name));
            continue;
        }
        for glyph_name in values[1..values.len() - 1].split_whitespace() {
            let glyph_name = glyph_name.trim_matches(|c: char| ",[]".contains(c));
            if !glyph_name.is_empty() && !glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "OpenType Class '{}' の未定義グリフ '{}'",
                    name, glyph_name
                ));
            }
        }
    }
    issues.sort();
    issues.dedup();
    issues
}

fn parse_context_sequences(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Vec<(Vec<GlyphId16>, usize, usize)> {
    let mut groups: Vec<(Vec<String>, bool)> = Vec::new();
    let mut class = Vec::new();
    let mut in_class = false;
    let mut marked = false;
    for raw in parts {
        let mut token = (*raw).to_string();
        if token.starts_with('[') {
            in_class = true;
            token = token.trim_start_matches('[').to_string();
        }
        if token.ends_with(']') || token.ends_with("]'") {
            marked |= token.ends_with("]'");
            token = token
                .trim_end_matches('\'')
                .trim_end_matches(']')
                .to_string();
            if !token.is_empty() {
                class.push(token);
            }
            groups.push((std::mem::take(&mut class), marked));
            marked = false;
            in_class = false;
        } else if in_class {
            marked |= token.ends_with('\'');
            class.push(token.trim_end_matches('\'').to_string());
        } else {
            marked = token.ends_with('\'');
            groups.push((vec![token.trim_end_matches('\'').to_string()], marked));
            marked = false;
        }
    }
    if in_class || groups.iter().filter(|(_, marked)| *marked).count() != 1 {
        return Vec::new();
    }
    let target_index = groups.iter().position(|(_, marked)| *marked).unwrap();
    let alternatives: Option<Vec<Vec<GlyphId16>>> = groups
        .into_iter()
        .map(|(names, _)| {
            names
                .into_iter()
                .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                .collect::<Option<Vec<_>>>()
        })
        .collect();
    let Some(alternatives) = alternatives else {
        return Vec::new();
    };
    let mut output = vec![(Vec::new(), 0usize, 0usize)];
    for (group_index, choices) in alternatives.into_iter().enumerate() {
        let mut next = Vec::new();
        for (prefix, _, target_choice) in output {
            for (choice_index, choice) in choices.iter().enumerate() {
                let mut sequence = prefix.clone();
                sequence.push(*choice);
                next.push((
                    sequence,
                    target_index,
                    if group_index == target_index {
                        choice_index
                    } else {
                        target_choice
                    },
                ));
            }
        }
        output = next;
    }
    output
}

fn parse_feature_sequence(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<Vec<GlyphId16>>> {
    let mut groups = Vec::<Vec<String>>::new();
    let mut current = Vec::new();
    let mut in_class = false;
    for raw in parts {
        let mut token = (*raw).to_string();
        if token.starts_with('[') {
            in_class = true;
            token = token.trim_start_matches('[').to_string();
        }
        if token.ends_with(']') {
            token = token.trim_end_matches(']').to_string();
            if !token.is_empty() {
                current.push(token);
            }
            groups.push(std::mem::take(&mut current));
            in_class = false;
        } else if in_class {
            current.push(token);
        } else if !token.is_empty() {
            groups.push(vec![token]);
        }
    }
    if in_class || groups.is_empty() {
        return None;
    }
    groups
        .into_iter()
        .map(|group| {
            group
                .into_iter()
                .map(|name| {
                    let name = name.trim_matches(|character: char| ",[]".contains(character));
                    glyph_ids.get(name).copied().map(GlyphId16::new)
                })
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

fn parse_feature_groups(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<(Vec<Vec<GlyphId16>>, usize)> {
    let mut groups = Vec::<(Vec<String>, bool)>::new();
    let mut current = Vec::new();
    let mut in_class = false;
    let mut marked = false;
    for raw in parts {
        let mut token = (*raw).to_string();
        if token.starts_with('[') {
            in_class = true;
            token = token.trim_start_matches('[').to_string();
        }
        if token.ends_with(']') || token.ends_with("]'") {
            marked |= token.ends_with("]'");
            token = token
                .trim_end_matches('\'')
                .trim_end_matches(']')
                .to_string();
            if !token.is_empty() {
                current.push(token);
            }
            groups.push((std::mem::take(&mut current), marked));
            marked = false;
            in_class = false;
        } else if in_class {
            marked |= token.ends_with('\'');
            current.push(token.trim_end_matches('\'').to_string());
        } else {
            marked = token.ends_with('\'');
            groups.push((vec![token.trim_end_matches('\'').to_string()], marked));
            marked = false;
        }
    }
    if in_class || groups.iter().filter(|(_, marked)| *marked).count() != 1 {
        return None;
    }
    let target_index = groups.iter().position(|(_, marked)| *marked)?;
    let groups = groups
        .into_iter()
        .map(|(names, _)| {
            names
                .into_iter()
                .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()?;
    Some((groups, target_index))
}

fn clean_feature_class(parts: &[&str]) -> Vec<String> {
    parts
        .iter()
        .map(|part| part.trim_matches(|c: char| "[],'".contains(c)).to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_lookup_flags(source: &str) -> layout::LookupFlag {
    parse_lookup_options(source).0
}

fn parse_lookup_options(source: &str) -> (layout::LookupFlag, Option<String>) {
    let tokens = source
        .split(|character: char| character.is_whitespace() || character == ';')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut flags = layout::LookupFlag::empty();
    let mut mark_filtering_set = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.to_ascii_lowercase().as_str() {
            "righttoleft" => flags |= layout::LookupFlag::RIGHT_TO_LEFT,
            "ignorebaseglyphs" => flags |= layout::LookupFlag::IGNORE_BASE_GLYPHS,
            "ignoreligatures" => flags |= layout::LookupFlag::IGNORE_LIGATURES,
            "ignoremarks" => flags |= layout::LookupFlag::IGNORE_MARKS,
            "markattachmenttype" => {
                if let Some(value) = tokens.get(index + 1).and_then(|value| value.parse().ok()) {
                    flags.set_mark_attachment_class(value);
                }
            }
            "usemarkfilteringset" => {
                mark_filtering_set = tokens.get(index + 1).map(|value| (*value).to_string());
            }
            // MarkFilteringSet needs a GDEF MarkGlyphSets table. Until the
            // editor exposes named mark sets, do not emit the flag without
            // its required companion table.
            _ => {}
        }
    }
    (flags, mark_filtering_set)
}

fn parse_lookup_mark_filtering_set(source: &str) -> Option<String> {
    parse_lookup_options(source).1
}

/// Adobe Feature File also permits the long-form `substitute` and `position`
/// keywords. Internally the compiler uses the short forms so every lookup
/// parser accepts both spellings consistently.
fn normalize_feature_keywords(source: &str) -> String {
    let tokens = source.split_whitespace().collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        // `enum sub` and `enum pos` are Feature File's enumerated forms.
        // The compiler already expands class combinations independently, so
        // removing the marker gives both forms the same interoperable result.
        if matches!(token, "enum" | "enumerate")
            && matches!(tokens.get(index + 1), Some(&"sub") | Some(&"pos"))
        {
            index += 1;
            continue;
        }
        normalized.push(match token {
            "substitute" => "sub",
            "position" => "pos",
            "rsub" => "reversesub",
            _ => token,
        });
        index += 1;
    }
    normalized.join(" ")
}

fn feature_uses_extension_lookups(source: &str) -> bool {
    source
        .split(|character: char| character.is_whitespace() || character == ';')
        .any(|token| token.eq_ignore_ascii_case("useExtension"))
}

fn parse_feature_references(source: &str) -> Vec<(Tag, Tag)> {
    extract_feature_blocks(source)
        .into_iter()
        .flat_map(|(parent, body)| {
            body.split(';')
                .filter_map(move |statement| {
                    let tokens = statement.split_whitespace().collect::<Vec<_>>();
                    if tokens.first() != Some(&"feature") {
                        return None;
                    }
                    let child = tokens.get(1).and_then(|value| layout_tag(value))?;
                    Some((parent, child))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn wrap_gsub_extension_lookup(lookup: gsub::SubstitutionLookup) -> gsub::SubstitutionLookup {
    macro_rules! wrap_variant {
        ($lookup:expr, $extension_type:expr, $variant:ident) => {{
            let layout::Lookup {
                lookup_flag,
                subtables,
                mark_filtering_set,
            } = $lookup;
            let extension_subtables = subtables
                .iter()
                .map(|subtable| {
                    gsub::ExtensionSubstFormat1::new($extension_type, (**subtable).clone())
                })
                .map(gsub::ExtensionSubtable::$variant)
                .collect();
            let mut wrapped = layout::Lookup::new(lookup_flag, extension_subtables);
            wrapped.mark_filtering_set = mark_filtering_set;
            gsub::SubstitutionLookup::Extension(wrapped)
        }};
    }
    match lookup {
        gsub::SubstitutionLookup::Single(lookup) => wrap_variant!(lookup, 1, Single),
        gsub::SubstitutionLookup::Multiple(lookup) => wrap_variant!(lookup, 2, Multiple),
        gsub::SubstitutionLookup::Alternate(lookup) => wrap_variant!(lookup, 3, Alternate),
        gsub::SubstitutionLookup::Ligature(lookup) => wrap_variant!(lookup, 4, Ligature),
        gsub::SubstitutionLookup::Contextual(lookup) => wrap_variant!(lookup, 5, Contextual),
        gsub::SubstitutionLookup::ChainContextual(lookup) => {
            wrap_variant!(lookup, 6, ChainContextual)
        }
        gsub::SubstitutionLookup::Reverse(lookup) => wrap_variant!(lookup, 8, Reverse),
        gsub::SubstitutionLookup::Extension(lookup) => gsub::SubstitutionLookup::Extension(lookup),
    }
}

fn wrap_gpos_extension_lookup(lookup: gpos::PositionLookup) -> gpos::PositionLookup {
    macro_rules! wrap_variant {
        ($lookup:expr, $extension_type:expr, $variant:ident) => {{
            let layout::Lookup {
                lookup_flag,
                subtables,
                mark_filtering_set,
            } = $lookup;
            let extension_subtables = subtables
                .iter()
                .map(|subtable| {
                    gpos::ExtensionPosFormat1::new($extension_type, (**subtable).clone())
                })
                .map(gpos::ExtensionSubtable::$variant)
                .collect();
            let mut wrapped = layout::Lookup::new(lookup_flag, extension_subtables);
            wrapped.mark_filtering_set = mark_filtering_set;
            gpos::PositionLookup::Extension(wrapped)
        }};
    }
    match lookup {
        gpos::PositionLookup::Single(lookup) => wrap_variant!(lookup, 1, Single),
        gpos::PositionLookup::Pair(lookup) => wrap_variant!(lookup, 2, Pair),
        gpos::PositionLookup::Cursive(lookup) => wrap_variant!(lookup, 3, Cursive),
        gpos::PositionLookup::MarkToBase(lookup) => wrap_variant!(lookup, 4, MarkToBase),
        gpos::PositionLookup::MarkToLig(lookup) => wrap_variant!(lookup, 5, MarkToLig),
        gpos::PositionLookup::MarkToMark(lookup) => wrap_variant!(lookup, 6, MarkToMark),
        gpos::PositionLookup::Contextual(lookup) => wrap_variant!(lookup, 7, Contextual),
        gpos::PositionLookup::ChainContextual(lookup) => {
            wrap_variant!(lookup, 8, ChainContextual)
        }
        gpos::PositionLookup::Extension(lookup) => gpos::PositionLookup::Extension(lookup),
    }
}

fn is_aalt_source_feature(tag: Tag) -> bool {
    let bytes = tag.to_be_bytes();
    (bytes == *b"salt" || bytes == *b"swsh" || bytes == *b"titl" || bytes == *b"ornm")
        || ((bytes[..2] == *b"ss" || bytes[..2] == *b"cv")
            && bytes[2].is_ascii_digit()
            && bytes[3].is_ascii_digit())
}

fn parse_feature_anchor(tokens: &[&str], anchor_index: usize) -> Option<(i16, i16)> {
    let values = tokens
        .get(anchor_index + 1..)?
        .iter()
        .take_while(|token| !token.contains('>'))
        .chain(
            tokens
                .get(anchor_index + 1..)?
                .iter()
                .filter(|token| token.contains('>'))
                .take(1),
        )
        .map(|token| token.trim_matches(|character: char| "><".contains(character)))
        .filter_map(|token| token.parse::<i16>().ok())
        .collect::<Vec<_>>();
    (values.len() >= 2).then(|| (values[0], values[1]))
}

fn parse_mark_glyph_sets(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<String, (u16, layout::CoverageTable)> {
    let mut sets = BTreeMap::new();
    let mut next_index = 0_u16;
    for statement in source.split(';') {
        let Some((raw_name, raw_values)) = statement.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        let values = raw_values.trim();
        if !name.starts_with('@') || !values.starts_with('[') || !values.ends_with(']') {
            continue;
        }
        let glyphs = values[1..values.len() - 1]
            .split_whitespace()
            .filter_map(|value| glyph_ids.get(value.trim_matches(|c: char| ",[]".contains(c))))
            .copied()
            .map(GlyphId16::new)
            .collect::<Vec<_>>();
        if glyphs.is_empty() || sets.contains_key(name) {
            continue;
        }
        sets.insert(name.to_string(), (next_index, glyphs.into_iter().collect()));
        next_index = next_index.saturating_add(1);
    }
    sets
}

fn apply_lookup_mark_set<T>(
    mut lookup: layout::Lookup<T>,
    tag: Tag,
    lookup_mark_sets: &BTreeMap<Tag, String>,
    mark_sets: &BTreeMap<String, (u16, layout::CoverageTable)>,
) -> layout::Lookup<T> {
    if let Some(name) = lookup_mark_sets.get(&tag) {
        if let Some((index, _)) = mark_sets.get(name) {
            lookup.lookup_flag |= layout::LookupFlag::USE_MARK_FILTERING_SET;
            lookup.mark_filtering_set = Some(*index);
        }
    }
    lookup
}

pub(crate) fn extract_feature_blocks(source: &str) -> Vec<(Tag, String)> {
    // Comments may contain words such as `feature` or unmatched braces. Remove
    // them before scanning so they cannot distort the nesting depth.
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let source = uncommented.as_str();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("feature") {
        let start = cursor + relative;
        let before_is_identifier = source[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "feature".len();
        let after_is_identifier = source[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &source[start + "feature".len()..];
        let mut parts = tail.splitn(2, '{');
        let Some(header) = parts.next() else {
            break;
        };
        let Some(body_start) = parts.next() else {
            break;
        };
        let tag_name = header.split_whitespace().next().unwrap_or_default();
        if tag_name.len() != 4 || !tag_name.is_ascii() {
            cursor = start + "feature".len();
            continue;
        }
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in body_start.char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        let tag = Tag::new(tag_name.as_bytes().try_into().unwrap());
        blocks.push((tag, body_start[..end].to_string()));
        cursor = start + "feature".len() + end + 1;
    }
    blocks
}

fn extract_table_blocks(source: &str) -> Vec<(String, String)> {
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let lower = uncommented.to_ascii_lowercase();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("table") {
        let start = cursor + relative;
        let before_is_identifier = lower[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "table".len();
        let after_is_identifier = lower[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &uncommented[after..];
        let Some(open) = tail.find('{') else {
            break;
        };
        let tag = tail[..open].split_whitespace().next().unwrap_or_default();
        if tag.len() != 4 || !tag.is_ascii() {
            cursor = after + open + 1;
            continue;
        }
        let body_start = after + open + 1;
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in uncommented[body_start..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(body_start + index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        blocks.push((tag.to_string(), uncommented[body_start..end].to_string()));
        cursor = end + 1;
    }
    blocks
}

fn parse_feature_table_number(raw: &str) -> Option<f64> {
    let cleaned = raw.trim_matches(|character: char| "<>(),".contains(character));
    if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok().map(|value| value as f64);
    }
    cleaned.parse::<f64>().ok()
}

fn apply_feature_table_overrides(project: &mut FontProject, source: &str) {
    let mut hhea_values = BTreeMap::<String, f64>::new();
    let mut vertical_values = Vec::<(String, String, f64)>::new();
    let mut post_italic_angle = project.metadata.italic_angle;
    let mut post_underline_position = project.metadata.underline_position;
    let mut post_underline_thickness = project.metadata.underline_thickness;
    let mut post_is_fixed_pitch = project.metadata.is_fixed_pitch;
    for (tag, body) in extract_table_blocks(source) {
        for statement in body.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.len() < 2 {
                continue;
            }
            let key = tokens[0].to_ascii_lowercase();
            if tag.eq_ignore_ascii_case("OS/2") && key == "vendor" {
                let Some(start) = statement.find('"').map(|index| index + 1) else {
                    continue;
                };
                let Some(end) = statement[start..].find('"').map(|index| start + index) else {
                    continue;
                };
                let vendor = &statement[start..end];
                if vendor.len() == 4 && vendor.is_ascii() {
                    project.metadata.vendor_id = vendor.to_string();
                }
                continue;
            }
            if tag.eq_ignore_ascii_case("post") && key == "isfixedpitch" {
                if let Some(raw_value) = tokens.get(1) {
                    post_is_fixed_pitch = matches!(
                        raw_value.trim_matches(|character: char| "<>()".contains(character)),
                        "1" | "true" | "yes"
                    );
                }
                continue;
            }
            if tag.eq_ignore_ascii_case("OS/2") && key == "panose" {
                let values = tokens
                    .iter()
                    .skip(1)
                    .take(10)
                    .filter_map(|raw| {
                        parse_feature_table_number(raw)
                            .and_then(|value| u8::try_from(value as i64).ok())
                    })
                    .collect::<Vec<_>>();
                if values.len() == 10 {
                    project.metadata.panose.copy_from_slice(&values);
                }
                continue;
            }
            let value_index = if tag.eq_ignore_ascii_case("vmtx") {
                2
            } else {
                1
            };
            let Some(raw_value) = tokens.get(value_index) else {
                continue;
            };
            let Some(value) = parse_feature_table_number(raw_value) else {
                continue;
            };
            if tag.eq_ignore_ascii_case("head") && key == "fontrevision" {
                if value.is_finite() && (0.0..=65535.0).contains(&value) {
                    project.metadata.font_revision = value;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "flags" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.head_flags = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "macstyle" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.head_mac_style = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "lowestrecppem" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.lowest_rec_ppem = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "fontdirectionhint" {
                if value.is_finite() && (i16::MIN as f64..=i16::MAX as f64).contains(&value) {
                    project.metadata.font_direction_hint = value as i16;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "italicangle" {
                if value.is_finite() {
                    post_italic_angle = value;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "underlineposition" {
                if value.is_finite() {
                    post_underline_position = value;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "underlinethickness" {
                if value.is_finite() {
                    post_underline_thickness = value;
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "weightclass"
                && value.is_finite()
                && (1.0..=1000.0).contains(&value)
            {
                project.metadata.weight_class = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "widthclass"
                && value.is_finite()
                && (1.0..=9.0).contains(&value)
            {
                project.metadata.width_class = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "fstype"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.fs_type = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "fsselection"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.fs_selection = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "defaultchar"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.default_char = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "breakchar"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.break_char = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "maxcontext"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.max_context = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && matches!(
                    key.as_str(),
                    "ysubscriptxsize"
                        | "ysubcriptysize"
                        | "ysubscriptxoffset"
                        | "ysubscriptyoffset"
                )
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                match key.as_str() {
                    "ysubscriptxsize" => project.metadata.subscript_x_size = value as i16,
                    "ysubscriptysize" => project.metadata.subscript_y_size = value as i16,
                    "ysubscriptxoffset" => project.metadata.subscript_x_offset = value as i16,
                    _ => project.metadata.subscript_y_offset = value as i16,
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && matches!(
                    key.as_str(),
                    "ysuperscriptxsize"
                        | "ysuperscriptysize"
                        | "ysuperscriptxoffset"
                        | "ysuperscriptyoffset"
                )
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                match key.as_str() {
                    "ysuperscriptxsize" => project.metadata.superscript_x_size = value as i16,
                    "ysuperscriptysize" => project.metadata.superscript_y_size = value as i16,
                    "ysuperscriptxoffset" => project.metadata.superscript_x_offset = value as i16,
                    _ => project.metadata.superscript_y_offset = value as i16,
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "ystrikeoutsize"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.strikeout_size = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "ystrikeoutposition"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.strikeout_position = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "sfamilyclass"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.family_class = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "loweropticalpointsize"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.lower_optical_point_size = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "upperopticalpointsize"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.upper_optical_point_size = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "winascent"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.win_ascent = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "windescent"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.win_descent = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "xheight" && value.is_finite() {
                project.metadata.x_height = value;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "capheight" && value.is_finite() {
                project.metadata.cap_height = value;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "typoascender" && value.is_finite()
            {
                hhea_values.insert("ascender".into(), value);
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "typodescender"
                && value.is_finite()
            {
                hhea_values.insert("descender".into(), value);
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "typolinegap" && value.is_finite()
            {
                hhea_values.insert("linegap".into(), value);
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretsloperise"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_slope_rise = value as i16;
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretsloperun"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_slope_run = value as i16;
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretoffset"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_offset = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretsloperise"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_slope_rise = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretsloperun"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_slope_run = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretoffset"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_offset = value as i16;
            } else if tag.eq_ignore_ascii_case("vmtx")
                && matches!(key.as_str(), "vertoriginy" | "vertadvancey")
                && tokens.len() > 2
            {
                let glyphs = clean_feature_class(&tokens[1..value_index]);
                for glyph in glyphs {
                    vertical_values.push((key.clone(), glyph, value));
                }
            } else if tag.eq_ignore_ascii_case("hhea")
                && matches!(key.as_str(), "ascender" | "descender" | "linegap")
                && value.is_finite()
            {
                hhea_values.insert(key, value);
            }
        }
    }
    project.metadata.italic_angle = post_italic_angle;
    project.metadata.underline_position = post_underline_position;
    project.metadata.underline_thickness = post_underline_thickness;
    project.metadata.is_fixed_pitch = post_is_fixed_pitch;
    for (kind, glyph_name, value) in vertical_values {
        if !project.glyphs.contains_key(&glyph_name) {
            continue;
        }
        let max_y = project
            .outline_bounds_for_glyph(&glyph_name)
            .map(|(_, _, _, max_y)| max_y)
            .unwrap_or(0.0);
        let fallback = project.vertical_metrics_for_glyph(&glyph_name);
        let metric = project
            .vertical_metrics
            .entry(glyph_name.clone())
            .or_insert(fallback);
        if kind == "vertoriginy" {
            metric.top_side_bearing = value - max_y;
        } else {
            metric.advance_height = value;
        }
    }
    if hhea_values.is_empty() {
        return;
    }
    let default_metrics = crate::font_data::MasterMetrics {
        ascender: project.metadata.ascender,
        descender: project.metadata.descender,
        line_gap: project.metadata.line_gap,
    };
    if let Some(master_id) = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
    {
        let metrics = project
            .metrics_by_master
            .entry(master_id)
            .or_insert(default_metrics);
        if let Some(value) = hhea_values.get("ascender") {
            metrics.ascender = *value;
        }
        if let Some(value) = hhea_values.get("descender") {
            metrics.descender = *value;
        }
        if let Some(value) = hhea_values.get("linegap") {
            metrics.line_gap = *value;
        }
    } else {
        if let Some(value) = hhea_values.get("ascender") {
            project.metadata.ascender = *value;
        }
        if let Some(value) = hhea_values.get("descender") {
            project.metadata.descender = *value;
        }
        if let Some(value) = hhea_values.get("linegap") {
            project.metadata.line_gap = *value;
        }
    }
}

fn extract_lookup_blocks(source: &str) -> Vec<(String, String)> {
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let source = uncommented.as_str();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("lookup") {
        let start = cursor + relative;
        let before_is_identifier = source[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "lookup".len();
        let after_is_identifier = source[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &source[after..];
        let Some(open) = tail.find('{') else {
            break;
        };
        let header = tail[..open].trim();
        let Some(name) = header.split_whitespace().next() else {
            cursor = after + open + 1;
            continue;
        };
        if header != name {
            cursor = after + open + 1;
            continue;
        }
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
            })
        {
            cursor = after + open + 1;
            continue;
        }
        let body_start = after + open + 1;
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in source[body_start..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        blocks.push((
            name.to_string(),
            source[body_start..body_start + end].to_string(),
        ));
        cursor = body_start + end + 1;
    }
    blocks
}

fn expand_named_feature_lookups(source: &str) -> String {
    let lookup_blocks = extract_lookup_blocks(source)
        .into_iter()
        .collect::<BTreeMap<String, String>>();
    if lookup_blocks.is_empty() {
        return source.to_string();
    }
    let expand = |name: &str, visiting: &mut Vec<String>| -> String {
        fn expand_one(
            name: &str,
            definitions: &BTreeMap<String, String>,
            visiting: &mut Vec<String>,
        ) -> String {
            if visiting.iter().any(|current| current == name) {
                return String::new();
            }
            let Some(body) = definitions.get(name) else {
                return String::new();
            };
            visiting.push(name.to_string());
            let mut expanded = body.clone();
            for statement in body.split(';') {
                let tokens = statement.split_whitespace().collect::<Vec<_>>();
                if tokens.first() != Some(&"lookup") {
                    continue;
                }
                let Some(reference) = tokens.get(1) else {
                    continue;
                };
                expanded.push('\n');
                expanded.push_str(&expand_one(reference, definitions, visiting));
            }
            visiting.pop();
            expanded
        }
        expand_one(name, &lookup_blocks, visiting)
    };
    let mut expanded_blocks = Vec::new();
    for (tag, body) in extract_feature_blocks(source) {
        let mut merged = body.clone();
        for statement in body.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"lookup") {
                continue;
            }
            let Some(name) = tokens.get(1) else {
                continue;
            };
            merged.push('\n');
            merged.push_str(&expand(name, &mut Vec::new()));
        }
        expanded_blocks.push((tag, merged));
    }
    if expanded_blocks.is_empty() {
        return source.to_string();
    }
    // Replacing only the extracted feature bodies is unnecessary for the
    // compiler: return a synthetic source whose feature blocks contain both
    // their original statements and all referenced lookup bodies.
    expanded_blocks
        .into_iter()
        .map(|(tag, body)| format!("feature {} {{\n{}\n}} {};\n", tag, body, tag))
        .collect::<String>()
}

/// Build the standard FeatureParams payloads used by UI clients to identify
/// stylistic sets and character variants. The feature source remains the
/// source of truth for substitutions; these records only add the metadata
/// that makes `ss##`/`cv##` appear as named controls in font applications.
fn feature_params_for_tag(
    tag: Tag,
    source: &str,
    unicode_by_glyph: &BTreeMap<String, u32>,
) -> Option<layout::FeatureParams> {
    let source = normalize_feature_keywords(source);
    let bytes = tag.to_be_bytes();
    let prefix = &bytes[..2];
    if matches!(prefix, b"ss" | b"cv") && bytes[2].is_ascii_digit() && bytes[3].is_ascii_digit() {
        let number = u16::from(bytes[2] - b'0') * 10 + u16::from(bytes[3] - b'0');
        if !(1..=20).contains(&number) {
            return None;
        }
        let index = number - 1;
        if prefix == b"ss" {
            return Some(layout::FeatureParams::StylisticSet(
                layout::StylisticSetParams::new(NameId::new(500 + index)),
            ));
        }
        return Some(layout::FeatureParams::CharacterVariant(
            layout::CharacterVariantParams::new(
                NameId::new(520 + index),
                NameId::new(0),
                NameId::new(0),
                0,
                NameId::new(0),
                extract_feature_blocks(&source)
                    .into_iter()
                    .find(|(feature_tag, _)| *feature_tag == tag)
                    .into_iter()
                    .flat_map(|(_, body)| body.split(';').map(str::to_string).collect::<Vec<_>>())
                    .filter_map(|statement| {
                        let tokens = statement.split_whitespace().collect::<Vec<_>>();
                        let sub_index = tokens.iter().position(|token| *token == "sub")?;
                        let by_index = tokens[sub_index + 1..]
                            .iter()
                            .position(|token| *token == "by")?
                            + sub_index
                            + 1;
                        Some(
                            tokens[sub_index + 1..by_index]
                                .iter()
                                .filter_map(|name| {
                                    let name = name
                                        .trim_matches(|character: char| "[],'".contains(character));
                                    unicode_by_glyph.get(name).copied()
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .flatten()
                    .filter(|unicode| *unicode <= 0xFFFFFF)
                    .map(Uint24::new)
                    .collect(),
            ),
        ));
    }
    if bytes != *b"size" {
        return None;
    }
    let values = extract_feature_blocks(&source)
        .into_iter()
        .find(|(feature_tag, _)| *feature_tag == tag)
        .and_then(|(_, body)| {
            body.split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>())
                .find(|tokens| tokens.first() == Some(&"parameters"))
                .map(|tokens| {
                    tokens
                        .into_iter()
                        .skip(1)
                        .filter_map(|value| {
                            value
                                .trim_matches(|character: char| "<>;,".contains(character))
                                .parse()
                                .ok()
                        })
                        .collect::<Vec<u16>>()
                })
        })?;
    match values.as_slice() {
        [design_size] => Some(layout::FeatureParams::Size(layout::SizeParams::new(
            *design_size,
            0,
            0,
            0,
            0,
        ))),
        [design_size, identifier, range_start, range_end] => Some(layout::FeatureParams::Size(
            layout::SizeParams::new(*design_size, *identifier, 0, *range_start, *range_end),
        )),
        [design_size, identifier, range_start, range_end, name_entry] => {
            Some(layout::FeatureParams::Size(layout::SizeParams::new(
                *design_size,
                *identifier,
                *name_entry,
                *range_start,
                *range_end,
            )))
        }
        _ => None,
    }
}

/// Read all display names from a Feature File `featureNames` block. The
/// generated name ID is supplied by the caller because it is determined by
/// the registered `ss##`/`cv##` tag.
fn feature_name_records(source: &str, tag: &str, name_id: u16) -> Vec<fonttools::name::NameRecord> {
    let (_, body) = extract_feature_blocks(source)
        .into_iter()
        .find(|(feature_tag, _)| String::from_utf8_lossy(&feature_tag.to_be_bytes()) == tag)
        .unwrap_or((Tag::new(b"    "), String::new()));
    let Some(names_start) = body.find("featureNames") else {
        return Vec::new();
    };
    let names = &body[names_start..];
    let mut records = Vec::new();
    for statement in names.split(';') {
        let Some(name_start) = statement.find("name") else {
            continue;
        };
        let prefix = &statement[name_start + "name".len()..];
        let Some(quote_start) = prefix.find('"') else {
            continue;
        };
        let quote_start = quote_start + 1;
        let Some(quote_end) = prefix[quote_start..].find('"') else {
            continue;
        };
        let value = prefix[quote_start..quote_start + quote_end]
            .trim()
            .replace("\\\"", "\"");
        if value.is_empty() {
            continue;
        }
        let numeric = prefix[..quote_start - 1]
            .split_whitespace()
            .filter_map(parse_feature_number)
            .collect::<Vec<_>>();
        let record = match numeric.as_slice() {
            [platform, encoding, language] => fonttools::name::NameRecord {
                platformID: *platform,
                encodingID: *encoding,
                languageID: *language,
                nameID: name_id,
                string: value,
            },
            [] => fonttools::name::NameRecord::windows_unicode(name_id, value),
            _ => continue,
        };
        records.push(record);
    }
    records
}

fn parse_feature_name_records(source: &str) -> Vec<fonttools::name::NameRecord> {
    let body = extract_table_blocks(source)
        .into_iter()
        .find(|(tag, _)| tag.eq_ignore_ascii_case("name"))
        .map(|(_, body)| body)
        .unwrap_or_default();
    let mut records = Vec::new();
    for statement in body.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(nameid_index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("nameid"))
        else {
            continue;
        };
        let Some(quote_start) = statement.find('"') else {
            continue;
        };
        let Some(quote_end) = statement[quote_start + 1..].find('"') else {
            continue;
        };
        let value = statement[quote_start + 1..quote_start + 1 + quote_end].replace("\\\"", "\"");
        let numeric = tokens[nameid_index + 1..]
            .iter()
            .take_while(|token| !token.contains('"'))
            .filter_map(|value| parse_feature_number(value))
            .collect::<Vec<_>>();
        let Some(&name_id) = numeric.first() else {
            continue;
        };
        let record = match numeric.as_slice() {
            [_] => fonttools::name::NameRecord::windows_unicode(name_id, value),
            [_, platform] if *platform == 1 => fonttools::name::NameRecord {
                platformID: 1,
                encodingID: 0,
                languageID: 0,
                nameID: name_id,
                string: value,
            },
            [_, platform, encoding, language] => fonttools::name::NameRecord {
                platformID: *platform,
                encodingID: *encoding,
                languageID: *language,
                nameID: name_id,
                string: value,
            },
            _ => continue,
        };
        records.push(record);
    }
    records
}

fn parse_feature_number(value: &str) -> Option<u16> {
    let value = value.trim_matches(|character: char| "<>,".contains(character));
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).ok()
    } else {
        value.parse().ok()
    }
}

#[derive(Default)]
struct GsubRuleSet {
    substitutions: Vec<(Tag, GlyphId16, GlyphId16)>,
    multiples: Vec<(Tag, GlyphId16, Vec<GlyphId16>)>,
    alternates: Vec<(Tag, GlyphId16, Vec<GlyphId16>)>,
    ligatures: Vec<(Tag, GlyphId16, Vec<GlyphId16>, GlyphId16)>,
    contexts: Vec<(Tag, Vec<GlyphId16>, usize, GlyphId16)>,
    ignored_contexts: Vec<(Tag, Vec<Vec<GlyphId16>>)>,
    reverse_contexts: Vec<ReverseSubstitution>,
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_simple_gsub(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    build_simple_gsub_with_variations(source, glyph_ids, &[], &std::collections::HashMap::new())
}

fn build_simple_gsub_with_variations(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    conditional_substitutions: &[ConditionalSubstitution],
    axis_bounds: &AxisBounds,
) -> Option<Vec<u8>> {
    build_simple_gsub_with_variations_and_unicode(
        source,
        glyph_ids,
        conditional_substitutions,
        axis_bounds,
        &BTreeMap::new(),
    )
}

fn build_simple_gsub_with_variations_and_unicode(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    conditional_substitutions: &[ConditionalSubstitution],
    axis_bounds: &AxisBounds,
    unicode_by_glyph: &BTreeMap<String, u32>,
) -> Option<Vec<u8>> {
    let source = normalize_feature_keywords(source);
    let expanded_source = expand_named_feature_lookups(&expand_named_feature_classes(&source));
    let raw_feature_blocks = extract_feature_blocks(&expanded_source);
    let lookup_mark_sets = extract_feature_blocks(&source)
        .iter()
        .filter_map(|(tag, block)| parse_lookup_mark_filtering_set(block).map(|name| (*tag, name)))
        .collect::<BTreeMap<_, _>>();
    let mark_sets = parse_mark_glyph_sets(&source, glyph_ids);
    let feature_blocks = raw_feature_blocks.clone();
    let mut feature_tags = feature_blocks
        .iter()
        .map(|(tag, _)| *tag)
        .collect::<Vec<_>>();
    feature_tags.sort_by_key(|tag| tag.to_be_bytes());
    feature_tags.dedup();
    if !conditional_substitutions.is_empty() {
        feature_tags.push(Tag::new(b"rvrn"));
        feature_tags.sort_by_key(|tag| tag.to_be_bytes());
        feature_tags.dedup();
    }
    if feature_tags.is_empty() {
        feature_tags.push(Tag::new(b"liga"));
    }
    let rule_sources = if feature_blocks.is_empty() {
        vec![(Tag::new(b"liga"), expanded_source.clone())]
    } else {
        feature_blocks
    };
    let lookup_flags = rule_sources
        .iter()
        .map(|(tag, block)| (*tag, parse_lookup_flags(block)))
        .collect::<BTreeMap<_, _>>();
    let mut rules = GsubRuleSet::default();
    for substitution in conditional_substitutions {
        let (Some(&base), Some(&alternate)) = (
            glyph_ids.get(substitution.base.as_str()),
            glyph_ids.get(substitution.alternate.as_str()),
        ) else {
            continue;
        };
        rules.substitutions.push((
            Tag::new(b"rvrn"),
            GlyphId16::new(base),
            GlyphId16::new(alternate),
        ));
    }
    for (rule_tag, rule_source) in rule_sources {
        for statement in rule_source.split(';') {
            let tokens: Vec<_> = statement.split_whitespace().collect();
            if tokens.first() == Some(&"ignore") && tokens.get(1) == Some(&"sub") {
                if let Some(sequence) = parse_feature_sequence(&tokens[2..], glyph_ids) {
                    rules.ignored_contexts.push((rule_tag, sequence));
                }
                continue;
            }
            if let Some(reverse_index) = tokens.iter().position(|token| *token == "reversesub") {
                let reverse_tokens = &tokens[reverse_index + 1..];
                let Some(by_index) = reverse_tokens.iter().position(|token| *token == "by") else {
                    continue;
                };
                let Some((groups, target_index)) =
                    parse_feature_groups(&reverse_tokens[..by_index], glyph_ids)
                else {
                    continue;
                };
                let replacement = clean_feature_class(&reverse_tokens[by_index + 1..])
                    .into_iter()
                    .filter_map(|name| glyph_ids.get(name.as_str()).copied())
                    .map(GlyphId16::new)
                    .collect::<Vec<_>>();
                let Some(targets) = groups.get(target_index) else {
                    continue;
                };
                if replacement.len() != targets.len() {
                    continue;
                }
                let target = targets.clone();
                let backtrack = groups[..target_index].iter().rev().cloned().collect();
                let lookahead = groups[target_index + 1..].to_vec();
                rules
                    .reverse_contexts
                    .push((rule_tag, target, backtrack, lookahead, replacement));
                continue;
            }
            let alternate_tokens = tokens
                .iter()
                .position(|token| *token == "sub")
                .and_then(|index| tokens.get(index..))
                .filter(|tokens| tokens.len() >= 4 && tokens[2] == "from");
            if let Some(tokens) = alternate_tokens {
                let Some(&target_id) = glyph_ids.get(tokens[1]) else {
                    continue;
                };
                let names = tokens[3..].join(" ");
                let names = names.trim_start_matches('[').trim_end_matches(']');
                let Some(alts) = names
                    .split_whitespace()
                    .map(|name| glyph_ids.get(name).copied().map(GlyphId16::new))
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if !alts.is_empty() {
                    rules
                        .alternates
                        .push((rule_tag, GlyphId16::new(target_id), alts));
                }
                continue;
            }
            if let Some(sub_index) = tokens.iter().position(|token| *token == "sub") {
                let sub_tokens = &tokens[sub_index..];
                if let Some(by_index) = sub_tokens.iter().position(|token| *token == "by") {
                    let from = clean_feature_class(&sub_tokens[1..by_index]);
                    let to = clean_feature_class(&sub_tokens[by_index + 1..]);
                    if from.len() > 1 && from.len() == to.len() {
                        for (source, replacement) in from.into_iter().zip(to) {
                            if let (Some(&source_id), Some(&replacement_id)) = (
                                glyph_ids.get(source.as_str()),
                                glyph_ids.get(replacement.as_str()),
                            ) {
                                rules.substitutions.push((
                                    rule_tag,
                                    GlyphId16::new(source_id),
                                    GlyphId16::new(replacement_id),
                                ));
                            }
                        }
                        continue;
                    }
                }
            }
            if let Some(sub_index) = tokens.iter().position(|token| *token == "sub") {
                let sub_tokens = &tokens[sub_index..];
                if sub_tokens.len() < 4 {
                    continue;
                }
                let Some(by_index) = sub_tokens.iter().position(|token| *token == "by") else {
                    continue;
                };
                if by_index < 2 || by_index + 1 >= sub_tokens.len() {
                    continue;
                }
                if by_index > 2 {
                    let replacement_names = clean_feature_class(&sub_tokens[by_index + 1..]);
                    let replacement_ids = replacement_names
                        .iter()
                        .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                        .collect::<Option<Vec<_>>>();
                    let parsed = parse_context_sequences(&sub_tokens[1..by_index], glyph_ids);
                    if let Some(replacement_ids) = replacement_ids {
                        for (sequence, target_index, target_choice) in parsed.iter().cloned() {
                            let replacement = if replacement_ids.len() == 1 {
                                replacement_ids[0]
                            } else {
                                *replacement_ids
                                    .get(target_choice)
                                    .unwrap_or(&replacement_ids[0])
                            };
                            rules
                                .contexts
                                .push((rule_tag, sequence, target_index, replacement));
                        }
                        if !parsed.is_empty() {
                            continue;
                        }
                    }
                }
                let first_name = sub_tokens[1].trim_end_matches('\'');
                let Some(&first) = glyph_ids.get(first_name) else {
                    continue;
                };
                if by_index == 2
                    && sub_tokens[by_index + 1..].iter().all(|name| {
                        name.trim_matches(|character: char| "[]".contains(character)) == "NULL"
                    })
                {
                    rules
                        .multiples
                        .push((rule_tag, GlyphId16::new(first), Vec::new()));
                    continue;
                }
                let Some(replacements) = sub_tokens[by_index + 1..]
                    .iter()
                    .map(|name| {
                        let name = name.trim_matches(|character: char| "[]".contains(character));
                        glyph_ids.get(name).copied().map(GlyphId16::new)
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if by_index == 2 && replacements.len() > 1 {
                    rules
                        .multiples
                        .push((rule_tag, GlyphId16::new(first), replacements));
                    continue;
                }
                let Some(replacement) = replacements.first().copied() else {
                    continue;
                };
                let Some(components) = sub_tokens[2..by_index]
                    .iter()
                    .map(|name| {
                        let name = name.trim_matches(|character: char| "[]".contains(character));
                        glyph_ids.get(name).copied().map(GlyphId16::new)
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if components.is_empty() {
                    rules
                        .substitutions
                        .push((rule_tag, GlyphId16::new(first), replacement));
                    continue;
                }
                rules
                    .ligatures
                    .push((rule_tag, GlyphId16::new(first), components, replacement));
                continue;
            }
            for window in tokens.windows(4) {
                if window[0] == "sub" && window[2] == "by" {
                    let Some(&target_id) = glyph_ids.get(window[1]) else {
                        continue;
                    };
                    let Some(&replacement_id) = glyph_ids.get(window[3]) else {
                        continue;
                    };
                    rules.substitutions.push((
                        rule_tag,
                        GlyphId16::new(target_id),
                        GlyphId16::new(replacement_id),
                    ));
                }
            }
        }
    }
    // Glyphs and FontLab commonly expose an automatic `aalt` feature even
    // when the author only defined per-feature alternates. Synthesize it from
    // one-to-one substitutions while preserving an explicit `aalt` feature.
    let aalt_tag = Tag::new(b"aalt");
    if !feature_tags.contains(&aalt_tag) {
        let mut alternatives = BTreeMap::<GlyphId16, Vec<GlyphId16>>::new();
        for (tag, source, replacement) in &rules.substitutions {
            if !is_aalt_source_feature(*tag) {
                continue;
            }
            let entries = alternatives.entry(*source).or_default();
            if !entries.contains(replacement) {
                entries.push(*replacement);
            }
        }
        for (source, replacements) in alternatives {
            if !replacements.is_empty() {
                rules.alternates.push((aalt_tag, source, replacements));
            }
        }
        if rules.alternates.iter().any(|(tag, _, _)| *tag == aalt_tag) {
            feature_tags.push(aalt_tag);
            feature_tags.sort_by_key(|tag| tag.to_be_bytes());
            feature_tags.dedup();
        }
    }
    if rules.substitutions.is_empty()
        && rules.multiples.is_empty()
        && rules.alternates.is_empty()
        && rules.ligatures.is_empty()
        && rules.contexts.is_empty()
        && rules.ignored_contexts.is_empty()
        && rules.reverse_contexts.is_empty()
    {
        return None;
    }
    rules
        .substitutions
        .sort_by_key(|(_, target, _)| target.to_u16());
    rules
        .ligatures
        .sort_by_key(|(_, target, _, _)| target.to_u16());
    let mut lookups = Vec::new();
    let mut feature_indices_by_tag = BTreeMap::<Tag, Vec<u16>>::new();
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let substitutions = rules
            .substitutions
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if substitutions.is_empty() {
            continue;
        }
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::SingleSubst::format_2(
                rules
                    .substitutions
                    .iter()
                    .filter(|(rule_tag, _, _)| rule_tag == tag)
                    .map(|(_, target, _)| *target)
                    .collect(),
                rules
                    .substitutions
                    .iter()
                    .filter(|(rule_tag, _, _)| rule_tag == tag)
                    .map(|(_, _, replacement)| *replacement)
                    .collect(),
            )],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Single(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut multiples = rules
            .multiples
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if multiples.is_empty() {
            continue;
        }
        multiples.sort_by_key(|(_, target, _)| target.to_u16());
        let coverage: layout::CoverageTable =
            multiples.iter().map(|(_, target, _)| *target).collect();
        let sequences = multiples
            .iter()
            .map(|(_, _, replacements)| gsub::Sequence::new((*replacements).clone()))
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::MultipleSubstFormat1::new(coverage, sequences)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Multiple(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut alternates = rules
            .alternates
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if alternates.is_empty() {
            continue;
        }
        alternates.sort_by_key(|(_, target, _)| target.to_u16());
        let coverage: layout::CoverageTable =
            alternates.iter().map(|(_, target, _)| *target).collect();
        let sets = alternates
            .iter()
            .map(|(_, _, alternatives)| gsub::AlternateSet::new((*alternatives).clone()))
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::AlternateSubstFormat1::new(coverage, sets)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Alternate(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut grouped = std::collections::BTreeMap::<GlyphId16, Vec<_>>::new();
        for (rule_tag, first, components, replacement) in rules.ligatures.iter() {
            if rule_tag != tag {
                continue;
            }
            grouped
                .entry(*first)
                .or_default()
                .push((components.clone(), *replacement));
        }
        if grouped.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let sets = grouped
            .into_values()
            .map(|items| {
                gsub::LigatureSet::new(
                    items
                        .into_iter()
                        .map(|(components, replacement)| {
                            gsub::Ligature::new(replacement, components)
                        })
                        .collect(),
                )
            })
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::LigatureSubstFormat1::new(coverage, sets)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Ligature(lookup));
    }
    for (rule_tag, sequence, target_index, replacement) in rules.contexts {
        let Some(target) = sequence.get(target_index).copied() else {
            continue;
        };
        let single_lookup_index = lookups.len() as u16;
        let single = layout::Lookup::new(
            layout::LookupFlag::empty(),
            vec![gsub::SingleSubst::format_2(
                vec![target].into(),
                vec![replacement],
            )],
        );
        lookups.push(gsub::SubstitutionLookup::Single(single));
        let context = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::SubstitutionSequenceContext::from(
                layout::SequenceContext::format_3(
                    sequence
                        .into_iter()
                        .map(|glyph| std::iter::once(glyph).collect())
                        .collect(),
                    vec![layout::SequenceLookupRecord::new(
                        target_index as u16,
                        single_lookup_index,
                    )],
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Contextual(context));
    }
    for (rule_tag, sequence) in rules.ignored_contexts {
        let context = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::SubstitutionChainContext::from(
                layout::ChainedSequenceContext::format_3(
                    Vec::new(),
                    sequence.into_iter().map(Into::into).collect(),
                    Vec::new(),
                    Vec::new(),
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::ChainContextual(context));
    }
    for (rule_tag, target, backtrack, lookahead, replacement) in rules.reverse_contexts {
        let lookup = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::ReverseChainSingleSubstFormat1::new(
                target.clone().into(),
                backtrack.into_iter().map(Into::into).collect(),
                lookahead.into_iter().map(Into::into).collect(),
                replacement,
            )],
        );
        let lookup = apply_lookup_mark_set(lookup, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Reverse(lookup));
    }
    let feature_references = parse_feature_references(&source);
    loop {
        let mut changed = false;
        for (parent, child) in &feature_references {
            let child_indices = feature_indices_by_tag
                .get(child)
                .cloned()
                .unwrap_or_default();
            let parent_indices = feature_indices_by_tag.entry(*parent).or_default();
            for index in child_indices {
                if !parent_indices.contains(&index) {
                    parent_indices.push(index);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let lookups = if feature_uses_extension_lookups(&source) {
        lookups
            .into_iter()
            .map(wrap_gsub_extension_lookup)
            .collect()
    } else {
        lookups
    };
    let lookup_list = layout::LookupList::new(lookups);
    let rvrn_tag = Tag::new(b"rvrn");
    let rvrn_lookups = feature_indices_by_tag
        .get(&rvrn_tag)
        .cloned()
        .unwrap_or_default();
    let rvrn_feature_index = feature_tags.iter().position(|tag| *tag == rvrn_tag);
    let scripts = build_script_list(&source, &feature_tags);
    let feature_list = layout::FeatureList::new(
        feature_tags
            .into_iter()
            .map(|tag| {
                let indices = feature_indices_by_tag.remove(&tag).unwrap_or_default();
                let indices = if tag == rvrn_tag { Vec::new() } else { indices };
                layout::FeatureRecord::new(
                    tag,
                    layout::Feature::new(
                        feature_params_for_tag(tag, &source, unicode_by_glyph),
                        indices,
                    ),
                )
            })
            .collect(),
    );
    let mut table = gsub::Gsub::new(scripts, feature_list, lookup_list);
    if let Some(feature_index) = rvrn_feature_index {
        let records = conditional_substitutions
            .iter()
            .filter_map(|substitution| {
                let conditions = substitution
                    .conditions
                    .iter()
                    .filter_map(|(tag, range)| {
                        let (axis_index, min_value, default_value, max_value) =
                            *axis_bounds.get(tag).or_else(|| {
                                axis_bounds
                                    .iter()
                                    .find(|(axis, _)| axis.eq_ignore_ascii_case(tag))
                                    .map(|(_, bounds)| bounds)
                            })?;
                        let normalize = |value: f64| {
                            if value >= default_value {
                                (value - default_value) / (max_value - default_value).max(1e-9)
                            } else {
                                (value - default_value) / (default_value - min_value).max(1e-9)
                            }
                        };
                        let min = range.min.map(normalize).unwrap_or(-1.0).clamp(-1.0, 1.0);
                        let max = range.max.map(normalize).unwrap_or(1.0).clamp(-1.0, 1.0);
                        Some(layout::Condition::format_1_axis_range(
                            axis_index,
                            write_fonts::types::F2Dot14::from_f32(min as f32),
                            write_fonts::types::F2Dot14::from_f32(max as f32),
                        ))
                    })
                    .collect::<Vec<_>>();
                (!conditions.is_empty()).then(|| {
                    layout::FeatureVariationRecord::new(
                        Some(layout::ConditionSet::new(conditions)),
                        Some(layout::FeatureTableSubstitution::new(vec![
                            layout::FeatureTableSubstitutionRecord::new(
                                feature_index as u16,
                                layout::Feature::new(None, rvrn_lookups.clone()),
                            ),
                        ])),
                    )
                })
            })
            .collect::<Vec<_>>();
        if !records.is_empty() {
            table.feature_variations = Some(layout::FeatureVariations::new(records)).into();
        }
    }
    write_fonts::dump_table(&table).ok()
}

pub(crate) fn expand_named_feature_classes(source: &str) -> String {
    let mut expanded = source.to_string();
    let definitions: Vec<(String, String)> = source
        .split(';')
        .filter_map(|statement| {
            let (name, values) = statement.split_once('=')?;
            let name = name.trim();
            if !name.starts_with('@') {
                return None;
            }
            let values = values
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split_whitespace()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            (!values.is_empty()).then(|| (name.to_string(), format!("[{}]", values.join(" "))))
        })
        .collect();
    for (name, value) in definitions {
        expanded = expanded.replace(&name, &value);
    }
    expanded
}

/// Expand fixed `valueRecordDef` definitions before the GPOS parser reads
/// positioning statements. This covers the common reusable-value form; the
/// expanded values then use the same validation and ValueRecord machinery as
/// inline values.
fn expand_named_value_records(source: &str) -> String {
    let definitions = source
        .split(';')
        .filter_map(|statement| {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"valueRecordDef") || tokens.len() < 3 {
                return None;
            }
            let name = tokens
                .last()?
                .trim_matches(|character: char| ",;".contains(character));
            if name.is_empty()
                || !name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '.'
                })
            {
                return None;
            }
            let value_start = statement.find("valueRecordDef")? + "valueRecordDef".len();
            let value_end = statement.rfind(name)?;
            let value = statement[value_start..value_end].trim();
            (!value.is_empty()).then(|| {
                let replacement = if value.starts_with('<') && value.ends_with('>') {
                    value.to_string()
                } else {
                    // A one-number valueRecordDef is the AFM-style x/y
                    // advance shorthand, so keep it outside angle brackets.
                    value.to_string()
                };
                (name.to_string(), replacement)
            })
        })
        .collect::<Vec<_>>();
    let mut expanded = source.to_string();
    for (name, replacement) in definitions {
        expanded = expanded.replace(&format!("<{name}>"), &replacement);
    }
    expanded
}

/// Expand fixed-coordinate `anchorDef` references used by Feature File
/// mark/cursive rules. Named anchors are a source convenience; the generated
/// GPOS records still use the same concrete anchor parser as inline anchors.
fn expand_named_anchors(source: &str) -> String {
    let definitions = source
        .split(';')
        .filter_map(|statement| {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"anchorDef") || tokens.len() < 4 {
                return None;
            }
            let name = tokens
                .last()?
                .trim_matches(|character: char| ",;".contains(character));
            if name.is_empty()
                || !name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '.'
                })
            {
                return None;
            }
            let values = tokens[1..tokens.len() - 1]
                .iter()
                .flat_map(|token| token.split(|character: char| "<>,".contains(character)))
                .filter(|value| !value.is_empty())
                .filter_map(|value| value.parse::<i16>().ok())
                .collect::<Vec<_>>();
            (values.len() >= 2).then(|| (name.to_string(), (values[0], values[1])))
        })
        .collect::<Vec<_>>();
    let mut expanded = source.to_string();
    for (name, (x, y)) in definitions {
        expanded = expanded.replace(&format!("<anchor {name}>"), &format!("<anchor {x} {y}>"));
    }
    expanded
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_kerning_gpos(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    source: &str,
) -> Option<Vec<u8>> {
    build_kerning_gpos_with_unicode(project, glyph_ids, source, &BTreeMap::new())
}

fn build_kerning_gpos_with_unicode(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    source: &str,
    unicode_by_glyph: &BTreeMap<String, u32>,
) -> Option<Vec<u8>> {
    let source = expand_named_anchors(&expand_named_value_records(&normalize_feature_keywords(
        source,
    )));
    let mut grouped = std::collections::BTreeMap::<GlyphId16, Vec<(GlyphId16, i16, bool)>>::new();
    let mut class_pairs = std::collections::BTreeMap::<(String, String), i16>::new();
    let mut left_groups = std::collections::HashMap::<&str, Vec<&str>>::new();
    let mut right_groups = std::collections::HashMap::<&str, Vec<&str>>::new();
    for (name, glyph) in &project.glyphs {
        if !glyph.left_kerning_group.trim().is_empty() {
            left_groups
                .entry(glyph.left_kerning_group.trim())
                .or_default()
                .push(name.as_str());
        }
        if !glyph.right_kerning_group.trim().is_empty() {
            right_groups
                .entry(glyph.right_kerning_group.trim())
                .or_default()
                .push(name.as_str());
        }
    }
    let mut kerning_entries: Vec<_> = project.kerning.iter().collect();
    kerning_entries.sort_by(|((left_a, right_a), _), ((left_b, right_b), _)| {
        (left_a, right_a).cmp(&(left_b, right_b))
    });
    // Collect canonical group values first; differing glyph-level values
    // remain explicit exceptions in the PairPos format-1 lookup.
    for ((left, right), value) in &kerning_entries {
        let Ok(value) = checked_i16(**value, "GPOSカーニング値") else {
            continue;
        };
        let left_group = project
            .glyphs
            .get(left)
            .map(|g| g.left_kerning_group.trim())
            .filter(|g| !g.is_empty());
        let right_group = project
            .glyphs
            .get(right)
            .map(|g| g.right_kerning_group.trim())
            .filter(|g| !g.is_empty());
        if let (Some(left_group), Some(right_group)) = (left_group, right_group) {
            class_pairs
                .entry((left_group.to_string(), right_group.to_string()))
                .or_insert(value);
        }
    }
    for ((left, right), value) in &kerning_entries {
        let Ok(value) = checked_i16(**value, "GPOSカーニング値") else {
            continue;
        };
        let left_group = project
            .glyphs
            .get(left)
            .map(|glyph| glyph.left_kerning_group.trim())
            .filter(|group| !group.is_empty());
        let right_group = project
            .glyphs
            .get(right)
            .map(|glyph| glyph.right_kerning_group.trim())
            .filter(|group| !group.is_empty());
        if let (Some(left_group), Some(right_group)) = (left_group, right_group) {
            let pair = (left_group.to_string(), right_group.to_string());
            if class_pairs.get(&pair) == Some(&value) {
                continue;
            }
        }
        let left_names = project
            .glyphs
            .get(left)
            .and_then(|glyph| left_groups.get(glyph.left_kerning_group.trim()))
            .filter(|names| !names.is_empty())
            .cloned()
            .unwrap_or_else(|| vec![left.as_str()]);
        let right_names = project
            .glyphs
            .get(right)
            .and_then(|glyph| right_groups.get(glyph.right_kerning_group.trim()))
            .filter(|names| !names.is_empty())
            .cloned()
            .unwrap_or_else(|| vec![right.as_str()]);
        for expanded_left in left_names {
            let Some(&left_id) = glyph_ids.get(expanded_left) else {
                continue;
            };
            for expanded_right in &right_names {
                let Some(&right_id) = glyph_ids.get(*expanded_right) else {
                    continue;
                };
                grouped.entry(GlyphId16::new(left_id)).or_default().push((
                    GlyphId16::new(right_id),
                    value,
                    expanded_left == left.as_str() && *expanded_right == right.as_str(),
                ));
            }
        }
    }
    let mut lookups = Vec::new();
    let mut feature_indices = Vec::<(Tag, u16)>::new();

    // Compile the broadly useful subset of Adobe feature-file positioning
    // syntax in addition to the editor's native kerning/anchor data. Keeping
    // these in separate lookups means hand-authored features can coexist with
    // the generated `kern`, `mark`, and `mkmk` features.
    let expanded_source = expand_named_feature_lookups(&expand_named_feature_classes(&source));
    let raw_feature_blocks = extract_feature_blocks(&expanded_source);
    let lookup_mark_sets = extract_feature_blocks(&source)
        .iter()
        .filter_map(|(tag, block)| parse_lookup_mark_filtering_set(block).map(|name| (*tag, name)))
        .collect::<BTreeMap<_, _>>();
    let mark_sets = parse_mark_glyph_sets(&source, glyph_ids);
    let feature_blocks = raw_feature_blocks.clone();
    let lookup_flags = feature_blocks
        .iter()
        .map(|(tag, block)| (*tag, parse_lookup_flags(block)))
        .collect::<BTreeMap<_, _>>();
    let mut single_positions = Vec::<(Tag, GlyphId16, gpos::ValueRecord)>::new();
    let mut pair_positions = Vec::<(
        Tag,
        GlyphId16,
        GlyphId16,
        gpos::ValueRecord,
        gpos::ValueRecord,
    )>::new();
    let mut contextual_positions = Vec::<(Tag, Vec<GlyphId16>, usize, gpos::ValueRecord)>::new();
    let mut chained_positions = Vec::<(
        Tag,
        Vec<GlyphId16>,
        Vec<GlyphId16>,
        GlyphId16,
        Vec<GlyphId16>,
        gpos::ValueRecord,
    )>::new();
    let mut ignored_positions = Vec::<(Tag, Vec<Vec<GlyphId16>>)>::new();
    for (feature_tag, block) in feature_blocks {
        for statement in block.split(';') {
            let tokens: Vec<_> = statement.split_whitespace().collect();
            if tokens.first() == Some(&"ignore") && tokens.get(1) == Some(&"pos") {
                if let Some(sequence) = parse_feature_sequence(&tokens[2..], glyph_ids) {
                    ignored_positions.push((feature_tag, sequence));
                }
                continue;
            }
            let Some(pos_index) = tokens.iter().position(|token| *token == "pos") else {
                continue;
            };
            let tokens = &tokens[pos_index + 1..];
            let shorthand_value = tokens.last().and_then(|token| token.parse::<i16>().ok());
            let Some(value_start) = tokens
                .iter()
                .position(|token| token.starts_with('<'))
                .or_else(|| shorthand_value.map(|_| tokens.len().saturating_sub(1)))
            else {
                continue;
            };
            if value_start == 0 {
                continue;
            }
            let glyph_tokens = &tokens[..value_start];
            let mut operands = Vec::<Vec<&str>>::new();
            let mut operand = Vec::new();
            let mut bracket_depth = 0_i32;
            for token in glyph_tokens {
                bracket_depth += token.matches('[').count() as i32;
                bracket_depth -= token.matches(']').count() as i32;
                operand.push(*token);
                if bracket_depth == 0 {
                    operands.push(std::mem::take(&mut operand));
                }
            }
            if bracket_depth != 0 {
                continue;
            }
            let value_records = if tokens[value_start].starts_with('<') {
                let value_text = tokens[value_start..].join(" ");
                parse_feature_value_records(&value_text)
            } else {
                shorthand_value
                    .map(|value| ParsedGposValueRecord {
                        values: vec![0, 0, value, 0],
                        ..Default::default()
                    })
                    .into_iter()
                    .collect::<Vec<_>>()
            };
            let parse_value = |parsed: &ParsedGposValueRecord| {
                if !(1..=4).contains(&parsed.values.len()) {
                    return None;
                }
                let mut format = gpos::ValueFormat::empty();
                let mut record = gpos::ValueRecord::new();
                if let Some(&value) = parsed.values.first() {
                    format |= gpos::ValueFormat::X_PLACEMENT;
                    record = record.with_x_placement(value);
                }
                if let Some(&value) = parsed.values.get(1) {
                    format |= gpos::ValueFormat::Y_PLACEMENT;
                    record = record.with_y_placement(value);
                }
                if let Some(&value) = parsed.values.get(2) {
                    format |= gpos::ValueFormat::X_ADVANCE;
                    record = record.with_x_advance(value);
                }
                if let Some(&value) = parsed.values.get(3) {
                    format |= gpos::ValueFormat::Y_ADVANCE;
                    record = record.with_y_advance(value);
                }
                if let Some(device) = parsed.devices[0].clone() {
                    format |= gpos::ValueFormat::X_PLACEMENT_DEVICE;
                    record = record.with_x_placement_device(device);
                }
                if let Some(device) = parsed.devices[1].clone() {
                    format |= gpos::ValueFormat::Y_PLACEMENT_DEVICE;
                    record = record.with_y_placement_device(device);
                }
                if let Some(device) = parsed.devices[2].clone() {
                    format |= gpos::ValueFormat::X_ADVANCE_DEVICE;
                    record = record.with_x_advance_device(device);
                }
                if let Some(device) = parsed.devices[3].clone() {
                    format |= gpos::ValueFormat::Y_ADVANCE_DEVICE;
                    record = record.with_y_advance_device(device);
                }
                Some(record.with_explicit_value_format(format))
            };
            if glyph_tokens.iter().any(|token| token.ends_with('\'')) {
                let Some(value) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                for (sequence, target_index, _) in parse_context_sequences(glyph_tokens, glyph_ids)
                {
                    if target_index == 0 {
                        contextual_positions.push((
                            feature_tag,
                            sequence,
                            target_index,
                            value.clone(),
                        ));
                    } else if target_index < sequence.len() {
                        chained_positions.push((
                            feature_tag,
                            sequence[..target_index].iter().rev().copied().collect(),
                            Vec::new(),
                            sequence[target_index],
                            sequence[target_index + 1..].to_vec(),
                            value.clone(),
                        ));
                    }
                }
                continue;
            }
            let expand = |tokens: &[&str]| {
                clean_feature_class(tokens)
                    .into_iter()
                    .filter_map(|name| glyph_ids.get(name.as_str()).copied())
                    .map(GlyphId16::new)
                    .collect::<Vec<_>>()
            };
            if operands.len() == 1 {
                let glyphs = expand(&operands[0]);
                let Some(value) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                single_positions.extend(
                    glyphs
                        .into_iter()
                        .map(|glyph| (feature_tag, glyph, value.clone())),
                );
            } else if operands.len() == 2 {
                let left = expand(&operands[0]);
                let right = expand(&operands[1]);
                let Some(first) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                let second = value_records
                    .get(1)
                    .and_then(parse_value)
                    .unwrap_or_else(gpos::ValueRecord::new);
                for left_glyph in &left {
                    for right_glyph in &right {
                        pair_positions.push((
                            feature_tag,
                            *left_glyph,
                            *right_glyph,
                            first.clone(),
                            second.clone(),
                        ));
                    }
                }
            }
        }
    }
    single_positions.sort_by_key(|(_, glyph, _)| glyph.to_u16());
    for tag in feature_tags_from_positions(&single_positions) {
        let entries = single_positions
            .iter()
            .filter(|(entry_tag, _, _)| *entry_tag == tag)
            .collect::<Vec<_>>();
        if entries.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = entries.iter().map(|(_, glyph, _)| *glyph).collect();
        let values = entries
            .iter()
            .map(|(_, _, value)| (*value).clone())
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flags
                .get(&tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(
                coverage, values,
            ))],
        );
        let lookup = apply_lookup_mark_set(lookup, tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Single(lookup));
    }
    for tag in feature_tags_from_pair_positions(&pair_positions) {
        let entries = pair_positions
            .iter()
            .filter(|(entry_tag, _, _, _, _)| *entry_tag == tag)
            .collect::<Vec<_>>();
        let mut grouped = BTreeMap::<GlyphId16, Vec<_>>::new();
        for (_, left, right, first, second) in entries {
            grouped
                .entry(*left)
                .or_default()
                .push((*right, (*first).clone(), (*second).clone()));
        }
        if grouped.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let pair_sets = grouped
            .into_values()
            .map(|pairs| {
                gpos::PairSet::new(
                    pairs
                        .into_iter()
                        .map(|(right, first, second)| {
                            gpos::PairValueRecord::new(right, first, second)
                        })
                        .collect(),
                )
            })
            .collect();
        let pair_pos = gpos::PairPos::format_1(coverage, pair_sets);
        let lookup = layout::Lookup::new(
            lookup_flags
                .get(&tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![pair_pos],
        );
        let lookup = apply_lookup_mark_set(lookup, tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    for (feature_tag, sequence, target_index, value) in contextual_positions {
        if sequence.len() < 2 || target_index >= sequence.len() {
            continue;
        }
        let target = sequence[target_index];
        let single_lookup = layout::Lookup::new(
            lookup_flags
                .get(&feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(
                std::iter::once(target).collect(),
                vec![value],
            ))],
        );
        let single_lookup =
            apply_lookup_mark_set(single_lookup, feature_tag, &lookup_mark_sets, &mark_sets);
        let single_lookup_index = u16::try_from(lookups.len()).ok()?;
        lookups.push(gpos::PositionLookup::Single(single_lookup));
        let rule = layout::SequenceRule::new(
            sequence[1..].to_vec(),
            vec![layout::SequenceLookupRecord::new(
                u16::try_from(target_index).ok()?,
                single_lookup_index,
            )],
        );
        let context = layout::Lookup::new(
            lookup_flags
                .get(&feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::PositionSequenceContext::from(
                layout::SequenceContext::format_1(
                    std::iter::once(sequence[0]).collect(),
                    vec![Some(layout::SequenceRuleSet::new(vec![rule]))],
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Contextual(context));
    }
    for (feature_tag, backtrack, input, target, lookahead, value) in chained_positions {
        let single_lookup = layout::Lookup::new(
            lookup_flags
                .get(&feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(
                std::iter::once(target).collect(),
                vec![value],
            ))],
        );
        let single_lookup =
            apply_lookup_mark_set(single_lookup, feature_tag, &lookup_mark_sets, &mark_sets);
        let single_lookup_index = u16::try_from(lookups.len()).ok()?;
        lookups.push(gpos::PositionLookup::Single(single_lookup));
        let rule = layout::ChainedSequenceRule::new(
            backtrack,
            input,
            lookahead,
            vec![layout::SequenceLookupRecord::new(0, single_lookup_index)],
        );
        let context = layout::Lookup::new(
            lookup_flags
                .get(&feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::PositionChainContext::from(
                layout::ChainedSequenceContext::format_1(
                    std::iter::once(target).collect(),
                    vec![Some(layout::ChainedSequenceRuleSet::new(vec![rule]))],
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::ChainContextual(context));
    }
    for (feature_tag, sequence) in ignored_positions {
        let context = layout::Lookup::new(
            lookup_flags
                .get(&feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gpos::PositionChainContext::from(
                layout::ChainedSequenceContext::format_3(
                    Vec::new(),
                    sequence.into_iter().map(Into::into).collect(),
                    Vec::new(),
                    Vec::new(),
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::ChainContextual(context));
    }
    let mut cursive_anchors =
        BTreeMap::<GlyphId16, (Option<gpos::AnchorTable>, Option<gpos::AnchorTable>)>::new();
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let anchor_kind = anchor.name.trim_start_matches('_');
            if anchor_kind != "entry" && anchor_kind != "exit" {
                continue;
            }
            let (Ok(x), Ok(y)) = (
                checked_i16(anchor.x, "カ―シブアンカーX"),
                checked_i16(anchor.y, "カ―シブアンカーY"),
            ) else {
                continue;
            };
            let anchors = cursive_anchors.entry(GlyphId16::new(glyph_id)).or_default();
            let value = Some(gpos::AnchorTable::format_1(x, y));
            if anchor_kind == "entry" {
                anchors.0 = value;
            } else {
                anchors.1 = value;
            }
        }
    }
    let mut cursive_feature_tag = Tag::new(b"curs");
    for (feature_tag, block) in &raw_feature_blocks {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"pos") || tokens.get(1) != Some(&"cursive") {
                continue;
            }
            let operand_indices = tokens
                .iter()
                .enumerate()
                .filter_map(|(index, token)| {
                    (*token == "<anchor" || *token == "NULL").then_some(index)
                })
                .collect::<Vec<_>>();
            let Some(&entry_index) = operand_indices.first() else {
                continue;
            };
            let Some(&exit_index) = operand_indices.get(1) else {
                continue;
            };
            let parse_anchor = |index: usize| {
                if tokens.get(index) == Some(&"NULL") {
                    return Some(None);
                }
                let (x, y) = parse_feature_anchor(&tokens, index)?;
                Some(Some(gpos::AnchorTable::format_1(x, y)))
            };
            let (Some(entry), Some(exit)) = (parse_anchor(entry_index), parse_anchor(exit_index))
            else {
                continue;
            };
            for glyph_name in clean_feature_class(&tokens[2..entry_index]) {
                let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                    continue;
                };
                cursive_anchors
                    .entry(GlyphId16::new(glyph_id))
                    .or_default()
                    .clone_from(&(entry.clone(), exit.clone()));
            }
            cursive_feature_tag = *feature_tag;
        }
    }
    if !cursive_anchors.is_empty() {
        let coverage: layout::CoverageTable = cursive_anchors.keys().copied().collect();
        let records = cursive_anchors
            .into_values()
            .map(|(entry, exit)| gpos::EntryExitRecord::new(entry, exit))
            .collect();
        let cursive = gpos::CursivePosFormat1::new(coverage, records);
        let lookup = layout::Lookup::new(
            lookup_flags
                .get(&cursive_feature_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![cursive],
        );
        let lookup =
            apply_lookup_mark_set(lookup, cursive_feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((cursive_feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Cursive(lookup));
    }
    if !class_pairs.is_empty() {
        let mut left_class_ids = std::collections::BTreeMap::<String, u16>::new();
        let mut right_class_ids = std::collections::BTreeMap::<String, u16>::new();
        for (left_group, right_group) in class_pairs.keys() {
            let next_left = left_class_ids.len() as u16 + 1;
            left_class_ids
                .entry(left_group.clone())
                .or_insert(next_left);
            let next_right = right_class_ids.len() as u16 + 1;
            right_class_ids
                .entry(right_group.clone())
                .or_insert(next_right);
        }
        let class_def1 =
            layout::ClassDef::from_iter(left_groups.iter().flat_map(|(group, names)| {
                let class = left_class_ids[*group];
                names.iter().filter_map(move |name| {
                    glyph_ids
                        .get(*name)
                        .copied()
                        .map(|id| (GlyphId16::new(id), class))
                })
            }));
        let class_def2 =
            layout::ClassDef::from_iter(right_groups.iter().flat_map(|(group, names)| {
                let class = right_class_ids[*group];
                names.iter().filter_map(move |name| {
                    glyph_ids
                        .get(*name)
                        .copied()
                        .map(|id| (GlyphId16::new(id), class))
                })
            }));
        let mut rows = vec![
            gpos::Class1Record::new(vec![
                gpos::Class2Record::new(
                    gpos::ValueRecord::new()
                        .with_explicit_value_format(gpos::ValueFormat::X_ADVANCE),
                    gpos::ValueRecord::new(),
                );
                right_class_ids.len() + 1
            ]);
            left_class_ids.len() + 1
        ];
        for ((left_group, right_group), value) in class_pairs {
            let left_class = left_class_ids[&left_group] as usize;
            let right_class = right_class_ids[&right_group] as usize;
            rows[left_class].class2_records[right_class] = gpos::Class2Record::new(
                gpos::ValueRecord::new().with_x_advance(value),
                gpos::ValueRecord::new(),
            );
        }
        let coverage: layout::CoverageTable = class_def1.iter().map(|(glyph, _)| glyph).collect();
        let pair_pos = gpos::PairPos::format_2(coverage, class_def1, class_def2, rows);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![pair_pos]);
        feature_indices.push((Tag::new(b"kern"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    if !grouped.is_empty() {
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let pair_sets = grouped
            .into_values()
            .map(|pairs| {
                let mut pairs = pairs;
                pairs.sort_by_key(|(right, _, direct)| (right.to_u16(), !*direct));
                pairs.dedup_by_key(|(right, _, _)| *right);
                gpos::PairSet::new(
                    pairs
                        .into_iter()
                        .map(|(right, value, _)| {
                            gpos::PairValueRecord::new(
                                right,
                                gpos::ValueRecord::new().with_x_advance(value),
                                gpos::ValueRecord::new(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        let pair_pos = gpos::PairPos::format_1(coverage, pair_sets);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![pair_pos]);
        feature_indices.push((Tag::new(b"kern"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    let mut mark_names = BTreeMap::<GlyphId16, Vec<(String, gpos::AnchorTable)>>::new();
    let mut base_names = BTreeMap::<GlyphId16, Vec<(String, gpos::AnchorTable)>>::new();
    let mut source_mark_to_mark = false;
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let Ok(x) = checked_i16(anchor.x, "アンカーX") else {
                continue;
            };
            let Ok(y) = checked_i16(anchor.y, "アンカーY") else {
                continue;
            };
            if let Some(mark_name) = anchor.name.strip_prefix('_') {
                if !mark_name.is_empty() {
                    mark_names
                        .entry(GlyphId16::new(glyph_id))
                        .or_default()
                        .push((mark_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                }
            } else if !anchor.name.is_empty() {
                base_names
                    .entry(GlyphId16::new(glyph_id))
                    .or_default()
                    .push((anchor.name.clone(), gpos::AnchorTable::format_1(x, y)));
            }
        }
    }
    for block in std::iter::once(source.as_str())
        .chain(raw_feature_blocks.iter().map(|(_, block)| block.as_str()))
    {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() == Some(&"markClass") {
                let Some(anchor_index) = tokens.iter().position(|token| *token == "<anchor") else {
                    continue;
                };
                let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                    continue;
                };
                let Some(class_name) = tokens.last().map(|token| token.trim_start_matches('@'))
                else {
                    continue;
                };
                for glyph_name in clean_feature_class(&tokens[1..anchor_index]) {
                    let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                        continue;
                    };
                    mark_names
                        .entry(GlyphId16::new(glyph_id))
                        .or_default()
                        .push((class_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                }
            } else if tokens.first() == Some(&"pos") && tokens.get(1) == Some(&"base") {
                let anchor_indices = tokens
                    .iter()
                    .enumerate()
                    .filter_map(|(index, token)| (*token == "<anchor").then_some(index))
                    .collect::<Vec<_>>();
                let Some(&first_anchor) = anchor_indices.first() else {
                    continue;
                };
                let glyph_names = clean_feature_class(&tokens[2..first_anchor]);
                for (anchor_number, &anchor_index) in anchor_indices.iter().enumerate() {
                    let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                        continue;
                    };
                    let end = anchor_indices
                        .get(anchor_number + 1)
                        .copied()
                        .unwrap_or(tokens.len());
                    let Some(class_name) = tokens
                        .get(anchor_index + 3..end)
                        .and_then(|tokens| tokens.iter().find(|token| token.starts_with('@')))
                        .map(|token| token.trim_start_matches('@'))
                        .filter(|name| !name.is_empty())
                    else {
                        continue;
                    };
                    for glyph_name in &glyph_names {
                        let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                            continue;
                        };
                        base_names
                            .entry(GlyphId16::new(glyph_id))
                            .or_default()
                            .push((class_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                    }
                }
            } else if tokens.first() == Some(&"pos") && tokens.get(1) == Some(&"mark") {
                source_mark_to_mark = true;
            }
        }
    }
    let mark_names_for_mark = mark_names.clone();
    let base_names_for_mark = base_names.clone();
    if !mark_names.is_empty() && !base_names.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in base_names.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark_coverage: layout::CoverageTable = mark_names.keys().copied().collect();
        let base_coverage: layout::CoverageTable = base_names.keys().copied().collect();
        let mark_array = gpos::MarkArray::new(
            mark_names
                .into_values()
                .map(|anchors| {
                    let (name, anchor) = anchors.into_iter().next().unwrap();
                    gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor)
                })
                .collect(),
        );
        let base_array = gpos::BaseArray::new(
            base_names
                .into_values()
                .map(|anchors| {
                    let mut class_anchors = vec![None; classes.len()];
                    for (name, anchor) in anchors {
                        if let Some(&class) = classes.get(&name) {
                            class_anchors[class as usize] = Some(anchor);
                        }
                    }
                    gpos::BaseRecord::new(class_anchors)
                })
                .collect(),
        );
        let mark_base =
            gpos::MarkBasePosFormat1::new(mark_coverage, base_coverage, mark_array, base_array);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_base]);
        feature_indices.push((Tag::new(b"mark"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::MarkToBase(lookup));
    }
    let mut ligature_names = BTreeMap::<GlyphId16, Vec<(usize, String, gpos::AnchorTable)>>::new();
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let Some((anchor_name, suffix)) = anchor.name.rsplit_once('_') else {
                continue;
            };
            let Ok(component) = suffix.parse::<usize>() else {
                continue;
            };
            if component == 0 || anchor_name.is_empty() || anchor_name.starts_with('_') {
                continue;
            }
            let (Ok(x), Ok(y)) = (
                checked_i16(anchor.x, "合字アンカーX"),
                checked_i16(anchor.y, "合字アンカーY"),
            ) else {
                continue;
            };
            ligature_names
                .entry(GlyphId16::new(glyph_id))
                .or_default()
                .push((
                    component,
                    anchor_name.to_string(),
                    gpos::AnchorTable::format_1(x, y),
                ));
        }
    }
    for (_feature_tag, block) in &raw_feature_blocks {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"pos") || tokens.get(1) != Some(&"ligature") {
                continue;
            }
            let operand_indices = tokens
                .iter()
                .enumerate()
                .filter_map(|(index, token)| {
                    (*token == "<anchor" || *token == "NULL").then_some(index)
                })
                .collect::<Vec<_>>();
            let Some(&first_operand) = operand_indices.first() else {
                continue;
            };
            let glyph_names = clean_feature_class(&tokens[2..first_operand]);
            for (component_index, &operand_index) in operand_indices.iter().enumerate() {
                if tokens.get(operand_index) == Some(&"NULL") {
                    continue;
                }
                let anchor_index = operand_index;
                let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                    continue;
                };
                let end = operand_indices
                    .get(component_index + 1)
                    .copied()
                    .unwrap_or(tokens.len());
                let Some(class_name) = tokens
                    .get(anchor_index + 3..end)
                    .and_then(|tokens| tokens.iter().find(|token| token.starts_with('@')))
                    .map(|token| token.trim_start_matches('@'))
                    .filter(|name| !name.is_empty())
                else {
                    continue;
                };
                for glyph_name in &glyph_names {
                    let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                        continue;
                    };
                    ligature_names
                        .entry(GlyphId16::new(glyph_id))
                        .or_default()
                        .push((
                            component_index + 1,
                            class_name.to_string(),
                            gpos::AnchorTable::format_1(x, y),
                        ));
                }
            }
        }
    }
    if !mark_names_for_mark.is_empty() && !ligature_names.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in ligature_names.values() {
            for (_, name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark_coverage: layout::CoverageTable = mark_names_for_mark.keys().copied().collect();
        let ligature_coverage: layout::CoverageTable = ligature_names.keys().copied().collect();
        let mark_array = gpos::MarkArray::new(
            mark_names_for_mark
                .values()
                .map(|anchors| {
                    let (name, anchor) = anchors.first().unwrap();
                    gpos::MarkRecord::new(*classes.get(name).unwrap_or(&0), anchor.clone())
                })
                .collect(),
        );
        let ligature_array = gpos::LigatureArray::new(
            ligature_names
                .values()
                .map(|anchors| {
                    let component_count = anchors
                        .iter()
                        .map(|(component, _, _)| *component)
                        .max()
                        .unwrap_or(0);
                    let component_records = (1..=component_count)
                        .map(|component| {
                            let mut class_anchors = vec![None; classes.len()];
                            for (anchor_component, name, anchor) in anchors {
                                if *anchor_component == component {
                                    if let Some(&class) = classes.get(name) {
                                        class_anchors[class as usize] = Some(anchor.clone());
                                    }
                                }
                            }
                            gpos::ComponentRecord::new(class_anchors)
                        })
                        .collect();
                    gpos::LigatureAttach::new(component_records)
                })
                .collect(),
        );
        let mark_ligature = gpos::MarkLigPosFormat1::new(
            mark_coverage,
            ligature_coverage,
            mark_array,
            ligature_array,
        );
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_ligature]);
        feature_indices.push((Tag::new(b"mark"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::MarkToLig(lookup));
    }
    if !mark_names_for_mark.is_empty() && !base_names_for_mark.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in base_names_for_mark.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark1: Vec<_> = mark_names_for_mark
            .iter()
            .filter_map(|(glyph_id, anchors)| {
                let (name, anchor) = anchors.first()?.clone();
                Some((
                    *glyph_id,
                    gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor),
                ))
            })
            .collect();
        let mark2: Vec<_> = base_names_for_mark
            .iter()
            .filter(|(glyph_id, _)| mark_names_for_mark.contains_key(glyph_id))
            .map(|(_, anchors)| {
                let mut class_anchors = vec![None; classes.len()];
                for (name, anchor) in anchors {
                    if let Some(&class) = classes.get(name) {
                        class_anchors[class as usize] = Some(anchor.clone());
                    }
                }
                gpos::Mark2Record::new(class_anchors)
            })
            .collect();
        if !mark1.is_empty() && !mark2.is_empty() {
            let mark1_coverage: layout::CoverageTable = mark1.iter().map(|(id, _)| *id).collect();
            let mark2_coverage: layout::CoverageTable = base_names_for_mark
                .keys()
                .filter(|id| mark_names_for_mark.contains_key(id))
                .copied()
                .collect();
            let mark1_array =
                gpos::MarkArray::new(mark1.into_iter().map(|(_, record)| record).collect());
            let mark2_array = gpos::Mark2Array::new(mark2);
            let mark_mark = gpos::MarkMarkPosFormat1::new(
                mark1_coverage,
                mark2_coverage,
                mark1_array,
                mark2_array,
            );
            let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_mark]);
            feature_indices.push((Tag::new(b"mkmk"), lookups.len() as u16));
            lookups.push(gpos::PositionLookup::MarkToMark(lookup));
        }
    }
    if source_mark_to_mark && !mark_names_for_mark.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in mark_names_for_mark.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark1 = mark_names_for_mark
            .iter()
            .filter_map(|(glyph_id, anchors)| {
                let (name, anchor) = anchors.first()?.clone();
                Some((
                    *glyph_id,
                    gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor),
                ))
            })
            .collect::<Vec<_>>();
        let mark2 = mark_names_for_mark
            .values()
            .map(|anchors| {
                let mut class_anchors = vec![None; classes.len()];
                for (name, anchor) in anchors {
                    if let Some(&class) = classes.get(name) {
                        class_anchors[class as usize] = Some(anchor.clone());
                    }
                }
                gpos::Mark2Record::new(class_anchors)
            })
            .collect::<Vec<_>>();
        if !mark1.is_empty() && !mark2.is_empty() {
            let mark1_coverage: layout::CoverageTable = mark1.iter().map(|(id, _)| *id).collect();
            let mark2_coverage: layout::CoverageTable =
                mark_names_for_mark.keys().copied().collect();
            let mark1_array =
                gpos::MarkArray::new(mark1.into_iter().map(|(_, record)| record).collect());
            let mark2_array = gpos::Mark2Array::new(mark2);
            let mark_mark = gpos::MarkMarkPosFormat1::new(
                mark1_coverage,
                mark2_coverage,
                mark1_array,
                mark2_array,
            );
            let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_mark]);
            feature_indices.push((Tag::new(b"mkmk"), lookups.len() as u16));
            lookups.push(gpos::PositionLookup::MarkToMark(lookup));
        }
    }
    let mut kerning_variations = Vec::<(layout::ConditionSet, u16)>::new();
    if project.masters.len() >= 2 && project.kerning_by_master.len() >= 2 {
        let axis_values = variable_master_axis_values(project);
        let default_master_id = project.default_master_id.as_str();
        for master in &project.masters {
            if master.id == default_master_id {
                continue;
            }
            let Some(kerning) = project.kerning_by_master.get(&master.id) else {
                continue;
            };
            if kerning == &project.kerning {
                continue;
            }
            let Some(lookup) = build_direct_kerning_lookup(kerning, glyph_ids) else {
                continue;
            };
            let conditions = axis_values
                .iter()
                .enumerate()
                .filter_map(|(axis_index, (_, values))| {
                    let value = values.get(&master.id).copied()?;
                    let mut sorted = values.values().copied().collect::<Vec<_>>();
                    sorted.sort_by(f64::total_cmp);
                    sorted.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);
                    if sorted.len() < 2 {
                        return None;
                    }
                    let index = sorted
                        .iter()
                        .position(|candidate| (*candidate - value).abs() < f64::EPSILON)?;
                    let min = if index == 0 {
                        -1.0
                    } else {
                        (sorted[index - 1] + value) / 2.0
                    };
                    let max = if index + 1 == sorted.len() {
                        1.0
                    } else {
                        (value + sorted[index + 1]) / 2.0
                    };
                    let normalized = |coordinate: f64| {
                        let (axis_min, default, axis_max) =
                            project_axis_bounds(project, axis_index);
                        if coordinate <= default {
                            ((coordinate - default) / (default - axis_min).max(f64::EPSILON))
                                .clamp(-1.0, 1.0)
                        } else {
                            ((coordinate - default) / (axis_max - default).max(f64::EPSILON))
                                .clamp(-1.0, 1.0)
                        }
                    };
                    Some(layout::Condition::format_1_axis_range(
                        axis_index as u16,
                        font_types::F2Dot14::from_f32(normalized(min) as f32),
                        font_types::F2Dot14::from_f32(normalized(max) as f32),
                    ))
                })
                .collect::<Vec<_>>();
            if conditions.is_empty() {
                continue;
            }
            let lookup_index = lookups.len() as u16;
            lookups.push(lookup);
            kerning_variations.push((layout::ConditionSet::new(conditions), lookup_index));
        }
    }
    if lookups.is_empty() {
        return None;
    }
    let feature_references = parse_feature_references(&source);
    loop {
        let mut changed = false;
        for (parent, child) in &feature_references {
            let child_indices = feature_indices
                .iter()
                .filter(|(tag, _)| tag == child)
                .map(|(_, index)| *index)
                .collect::<Vec<_>>();
            for index in child_indices {
                if !feature_indices
                    .iter()
                    .any(|(tag, existing)| tag == parent && *existing == index)
                {
                    feature_indices.push((*parent, index));
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let feature_index_by_tag =
        feature_indices
            .iter()
            .fold(BTreeMap::<Tag, Vec<u16>>::new(), |mut map, (tag, index)| {
                map.entry(*tag).or_default().push(*index);
                map
            });
    let feature_tags = feature_index_by_tag.keys().copied().collect::<Vec<_>>();
    let features = layout::FeatureList::new(
        feature_index_by_tag
            .iter()
            .map(|(tag, indices)| {
                layout::FeatureRecord::new(
                    *tag,
                    layout::Feature::new(
                        feature_params_for_tag(*tag, &source, unicode_by_glyph),
                        indices.clone(),
                    ),
                )
            })
            .collect(),
    );
    let lookups = if feature_uses_extension_lookups(&source) {
        lookups
            .into_iter()
            .map(wrap_gpos_extension_lookup)
            .collect()
    } else {
        lookups
    };
    let lookups = layout::LookupList::new(lookups);
    let scripts = build_script_list(&source, &feature_tags);
    let mut table = gpos::Gpos::new(scripts, features, lookups);
    if let Some(kern_feature_index) = feature_tags
        .iter()
        .position(|tag| *tag == Tag::new(b"kern"))
    {
        let records = kerning_variations
            .into_iter()
            .map(|(condition_set, lookup_index)| {
                layout::FeatureVariationRecord::new(
                    Some(condition_set),
                    Some(layout::FeatureTableSubstitution::new(vec![
                        layout::FeatureTableSubstitutionRecord::new(
                            kern_feature_index as u16,
                            layout::Feature::new(None, vec![lookup_index]),
                        ),
                    ])),
                )
            })
            .collect::<Vec<_>>();
        if !records.is_empty() {
            table.feature_variations = layout::FeatureVariations::new(records).into();
        }
    }
    write_fonts::dump_table(&table).ok()
}

fn build_direct_kerning_lookup(
    kerning: &std::collections::HashMap<(String, String), f64>,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<gpos::PositionLookup> {
    let mut grouped = BTreeMap::<GlyphId16, Vec<(GlyphId16, i16)>>::new();
    for ((left, right), value) in kerning {
        let (Some(&left_id), Some(&right_id), Ok(value)) = (
            glyph_ids.get(left.as_str()),
            glyph_ids.get(right.as_str()),
            checked_i16(*value, "可変カーニング値"),
        ) else {
            continue;
        };
        grouped
            .entry(GlyphId16::new(left_id))
            .or_default()
            .push((GlyphId16::new(right_id), value));
    }
    if grouped.is_empty() {
        return None;
    }
    let coverage: layout::CoverageTable = grouped.keys().copied().collect();
    let pair_sets = grouped
        .into_values()
        .map(|mut pairs| {
            pairs.sort_by_key(|(right, _)| right.to_u16());
            pairs.dedup_by_key(|(right, _)| *right);
            gpos::PairSet::new(
                pairs
                    .into_iter()
                    .map(|(right, value)| {
                        gpos::PairValueRecord::new(
                            right,
                            gpos::ValueRecord::new().with_x_advance(value),
                            gpos::ValueRecord::new(),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    Some(gpos::PositionLookup::Pair(layout::Lookup::new(
        layout::LookupFlag::empty(),
        vec![gpos::PairPos::format_1(coverage, pair_sets)],
    )))
}

fn variable_master_axis_values(
    project: &FontProject,
) -> Vec<(String, std::collections::HashMap<String, f64>)> {
    let custom_tags = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut tags = custom_tags.iter().cloned().collect::<Vec<_>>();
    if custom_tags.is_empty() {
        tags.push("wght".into());
    }
    if project
        .masters
        .iter()
        .any(|master| (master.width - project.masters[0].width).abs() > f64::EPSILON)
    {
        tags.push("wdth".into());
    }
    tags.into_iter()
        .map(|tag| {
            let values = project
                .masters
                .iter()
                .map(|master| {
                    let value = match tag.as_str() {
                        "wght" if custom_tags.is_empty() => master.weight,
                        "wdth" => master.width,
                        _ => master.axes.get(&tag).copied().unwrap_or(0.0),
                    };
                    (master.id.clone(), value)
                })
                .collect();
            (tag, values)
        })
        .collect()
}

fn project_axis_bounds(project: &FontProject, axis_index: usize) -> (f64, f64, f64) {
    let axes = variable_master_axis_values(project);
    let Some((tag, values)) = axes.get(axis_index) else {
        return (-1.0, 0.0, 1.0);
    };
    let mut coordinates = values.values().copied().collect::<Vec<_>>();
    coordinates.sort_by(f64::total_cmp);
    let min = coordinates.first().copied().unwrap_or(0.0);
    let max = coordinates.last().copied().unwrap_or(0.0);
    let default_id = &project.default_master_id;
    let default = values.get(default_id).copied().unwrap_or_else(|| {
        if tag == "wght" {
            project
                .masters
                .first()
                .map(|master| master.weight)
                .unwrap_or(0.0)
        } else {
            0.0
        }
    });
    (min, default, max)
}

fn feature_tags_from_positions(positions: &[(Tag, GlyphId16, gpos::ValueRecord)]) -> Vec<Tag> {
    let mut tags = positions.iter().map(|(tag, _, _)| *tag).collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_be_bytes());
    tags.dedup();
    tags
}

fn build_script_list(source: &str, feature_tags: &[Tag]) -> layout::ScriptList {
    let default_script = Tag::new(b"DFLT");
    let mut assignments = BTreeMap::<(Tag, Option<Tag>), std::collections::BTreeSet<Tag>>::new();
    let mut required_assignments =
        BTreeMap::<(Tag, Option<Tag>), std::collections::BTreeSet<Tag>>::new();
    let mut global_defaults = std::collections::BTreeSet::<Tag>::new();
    let mut script_defaults = BTreeMap::<Tag, std::collections::BTreeSet<Tag>>::new();
    let mut excluded_defaults = std::collections::BTreeSet::<(Tag, Option<Tag>, Tag)>::new();
    let mut language_systems = Vec::<(Tag, Option<Tag>)>::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.first() != Some(&"languagesystem") {
            continue;
        }
        let (Some(script), Some(language)) = (
            tokens.get(1).and_then(|value| layout_tag(value)),
            tokens.get(2).and_then(|value| layout_language_tag(value)),
        ) else {
            continue;
        };
        let language = (!tokens
            .get(2)
            .is_some_and(|value| value.eq_ignore_ascii_case("dflt")))
        .then_some(language);
        if !language_systems.contains(&(script, language)) {
            language_systems.push((script, language));
        }
    }
    let mut has_explicit_scope = false;
    let source = expand_named_feature_lookups(source);
    for (feature_tag, block) in extract_feature_blocks(&source) {
        let mut script = default_script;
        let mut language = None;
        let mut required = false;
        let mut saw_script_or_language = false;
        let mut script_default_active = false;
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            match tokens.first().copied() {
                Some("script") => {
                    if let Some(tag) = tokens.get(1).and_then(|value| layout_tag(value)) {
                        script = tag;
                        language = None;
                        required = false;
                        saw_script_or_language = true;
                        script_default_active = true;
                        has_explicit_scope = true;
                    }
                }
                Some("language") => {
                    language = tokens.get(1).and_then(|value| {
                        (!value.eq_ignore_ascii_case("dflt"))
                            .then(|| layout_language_tag(value))
                            .flatten()
                    });
                    required = tokens
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case("required"));
                    saw_script_or_language = true;
                    has_explicit_scope = true;
                    if language.is_some()
                        && tokens.iter().any(|value| {
                            value.eq_ignore_ascii_case("exclude_dflt")
                                || value.eq_ignore_ascii_case("excludeDFLT")
                        })
                    {
                        excluded_defaults.insert((script, language, feature_tag));
                    }
                    if language.is_some() {
                        script_default_active = false;
                    }
                }
                Some("sub") | Some("reversesub") | Some("pos") | Some("ignore")
                | Some("lookup") => {
                    if required {
                        let key = (script, language);
                        required_assignments
                            .entry(key)
                            .or_default()
                            .insert(feature_tag);
                    }
                    if !saw_script_or_language {
                        global_defaults.insert(feature_tag);
                    } else if script_default_active {
                        script_defaults
                            .entry(script)
                            .or_default()
                            .insert(feature_tag);
                    } else {
                        let key = (script, language);
                        assignments.entry(key).or_default().insert(feature_tag);
                    }
                }
                _ => {}
            }
        }
    }
    let mut systems = language_systems.clone();
    systems.extend(assignments.keys().copied());
    systems.extend(script_defaults.keys().map(|script| (*script, None)));
    systems.sort_by_key(|(script, language)| {
        (script.to_be_bytes(), language.map(|tag| tag.to_be_bytes()))
    });
    systems.dedup();
    if systems.is_empty() {
        systems.push((default_script, None));
    }
    for (script, language) in systems {
        let key = (script, language);
        assignments.entry(key).or_default();
        let excluded = |tag: Tag| excluded_defaults.contains(&(script, language, tag));
        for tag in &global_defaults {
            if !excluded(*tag) {
                assignments.entry(key).or_default().insert(*tag);
            }
        }
        if let Some(defaults) = script_defaults.get(&script) {
            for tag in defaults {
                if !excluded(*tag) {
                    assignments.entry(key).or_default().insert(*tag);
                }
            }
        }
    }
    if !has_explicit_scope {
        let all = feature_tags
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for key in assignments.keys().copied().collect::<Vec<_>>() {
            assignments
                .entry(key)
                .or_default()
                .extend(all.iter().copied());
        }
    }
    let feature_index = |tag: &Tag| feature_tags.iter().position(|candidate| candidate == tag);
    let mut by_script = BTreeMap::<Tag, BTreeMap<Option<Tag>, Vec<u16>>>::new();
    for ((script, language), tags) in assignments {
        let indices = tags
            .iter()
            .filter_map(feature_index)
            .map(|index| index as u16)
            .collect::<Vec<_>>();
        by_script
            .entry(script)
            .or_default()
            .insert(language, indices);
    }
    let records = by_script
        .into_iter()
        .map(|(script_tag, languages)| {
            let make_lang_sys = |language: Option<Tag>, indices: Vec<u16>| {
                let mut lang_sys = layout::LangSys::new(indices);
                lang_sys.required_feature_index = required_assignments
                    .get(&(script_tag, language))
                    .and_then(|tags| tags.iter().next())
                    .and_then(feature_index)
                    .map(|index| index as u16)
                    .unwrap_or(0xFFFF);
                lang_sys
            };
            let default = languages
                .get(&None)
                .cloned()
                .map(|indices| make_lang_sys(None, indices));
            let language_records = languages
                .into_iter()
                .filter_map(|(language, indices)| {
                    language.map(|language| {
                        layout::LangSysRecord::new(language, make_lang_sys(Some(language), indices))
                    })
                })
                .collect();
            layout::ScriptRecord::new(script_tag, layout::Script::new(default, language_records))
        })
        .collect();
    layout::ScriptList::new(records)
}

fn layout_tag(value: &str) -> Option<Tag> {
    let value = value.trim_matches(|character: char| "{};".contains(character));
    if value.len() != 4 || !value.is_ascii() {
        return None;
    }
    let bytes: &[u8; 4] = value.as_bytes().try_into().ok()?;
    Some(Tag::new(bytes))
}

fn layout_language_tag(value: &str) -> Option<Tag> {
    let value = value.trim_matches(|character: char| "{};".contains(character));
    if value.len() == 4 && value.is_ascii() {
        return Some(Tag::new(value.as_bytes().try_into().ok()?));
    }
    if value.len() == 3 && value.is_ascii() {
        let mut bytes = [b' '; 4];
        bytes[..3].copy_from_slice(value.as_bytes());
        return Some(Tag::new(&bytes));
    }
    None
}

fn feature_tags_from_pair_positions(
    positions: &[(
        Tag,
        GlyphId16,
        GlyphId16,
        gpos::ValueRecord,
        gpos::ValueRecord,
    )],
) -> Vec<Tag> {
    let mut tags = positions
        .iter()
        .map(|(tag, _, _, _, _)| *tag)
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_be_bytes());
    tags.dedup();
    tags
}

#[derive(Default)]
struct ParsedGposValueRecord {
    values: Vec<i16>,
    devices: [Option<layout::DeviceOrVariationIndex>; 4],
}

fn top_level_angle_groups(text: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut depth = 0_i32;
    let mut start = None;
    for (index, character) in text.char_indices() {
        match character {
            '<' => {
                if depth == 0 {
                    start = Some(index + character.len_utf8());
                }
                depth += 1;
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start.take() {
                        groups.push(text[start..index].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    groups
}

fn parse_feature_device(value: &str) -> Option<Option<layout::DeviceOrVariationIndex>> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() == 1 && tokens.first()?.eq_ignore_ascii_case("NULL") {
        return Some(None);
    }
    if !tokens.first()?.eq_ignore_ascii_case("device") {
        return None;
    }
    if tokens
        .get(1)
        .is_some_and(|token| token.eq_ignore_ascii_case("NULL"))
    {
        return Some(None);
    }
    let numbers = tokens[1..]
        .iter()
        .flat_map(|token| token.split(','))
        .filter_map(|token| token.parse::<i16>().ok())
        .collect::<Vec<_>>();
    if numbers.len() < 2 || numbers.len() % 2 != 0 {
        return None;
    }
    let first_ppem = numbers[0];
    let last_ppem = numbers[numbers.len() - 2];
    if !(0..=255).contains(&first_ppem) || !(0..=255).contains(&last_ppem) || last_ppem < first_ppem
    {
        return None;
    }
    let mut values = vec![0_i8; (last_ppem - first_ppem + 1) as usize];
    for pair in numbers.chunks(2) {
        let ppem = pair[0];
        let delta = pair[1];
        if !(0..=255).contains(&ppem) || !(-128..=127).contains(&delta) {
            return None;
        }
        values[(ppem - first_ppem) as usize] = delta as i8;
    }
    Some(Some(layout::DeviceOrVariationIndex::device(
        first_ppem as u16,
        last_ppem as u16,
        &values,
    )))
}

fn parse_feature_value_records(text: &str) -> Vec<ParsedGposValueRecord> {
    top_level_angle_groups(text)
        .into_iter()
        .filter_map(|group| {
            let nested_devices = top_level_angle_groups(&group);
            let mut stripped = String::with_capacity(group.len());
            let mut depth = 0_i32;
            for character in group.chars() {
                match character {
                    '<' => depth += 1,
                    '>' => depth = (depth - 1).max(0),
                    _ if depth == 0 => stripped.push(character),
                    _ => {}
                }
            }
            let values = stripped
                .split_whitespace()
                .filter_map(|value| value.parse::<i16>().ok())
                .collect::<Vec<_>>();
            if values.is_empty() && nested_devices.is_empty() {
                return None;
            }
            let mut parsed = ParsedGposValueRecord {
                values,
                ..Default::default()
            };
            for (index, device) in nested_devices.into_iter().enumerate().take(4) {
                parsed.devices[index] = parse_feature_device(&device)?;
            }
            Some(parsed)
        })
        .collect()
}

fn parse_feature_glyph_classes(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<GlyphId16, gdef::GlyphClassDef> {
    let mut classes = BTreeMap::new();
    for statement in source.split(';') {
        let Some(definition) = statement.split_once("GlyphClassDef").map(|(_, rest)| rest) else {
            continue;
        };
        for (class_index, group) in definition.split(',').take(4).enumerate() {
            let Some(open) = group.find('[') else {
                continue;
            };
            let Some(close) = group[open + 1..].find(']') else {
                continue;
            };
            let names = &group[open + 1..open + 1 + close];
            let class = match class_index {
                0 => gdef::GlyphClassDef::Base,
                1 => gdef::GlyphClassDef::Ligature,
                2 => gdef::GlyphClassDef::Mark,
                3 => gdef::GlyphClassDef::Component,
                _ => continue,
            };
            for name in names.split_whitespace() {
                let name = name.trim_matches(|character: char| "[]".contains(character));
                if let Some(&glyph_id) = glyph_ids.get(name) {
                    classes.insert(GlyphId16::new(glyph_id), class);
                }
            }
        }
    }
    classes
}

fn build_gdef(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    feature_source: &str,
) -> Option<Vec<u8>> {
    let expanded_source = expand_named_feature_classes(feature_source);
    let explicit_classes = parse_feature_glyph_classes(&expanded_source, glyph_ids);
    let mark_attach_classes = parse_feature_mark_attach_classes(&expanded_source, glyph_ids);
    let mut records = Vec::new();
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        let anchors = project.anchors_for_glyph(name);
        let class = explicit_classes
            .get(&GlyphId16::new(glyph_id))
            .copied()
            .unwrap_or_else(|| {
                if anchors.iter().any(|anchor| anchor.name.starts_with('_')) {
                    gdef::GlyphClassDef::Mark
                } else if !glyph.components.is_empty() {
                    gdef::GlyphClassDef::Component
                } else {
                    gdef::GlyphClassDef::Base
                }
            });
        records.push(layout::ClassRangeRecord::new(
            GlyphId16::new(glyph_id),
            GlyphId16::new(glyph_id),
            class as u16,
        ));
    }
    if records.is_empty() {
        return None;
    }
    let class_def = layout::ClassDef::format_2(records);
    let mut ligature_carets = parse_feature_ligature_carets(&expanded_source, glyph_ids);
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        if ligature_carets.contains_key(&GlyphId16::new(glyph_id)) {
            continue;
        }
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        if glyph.components.len() < 2 {
            continue;
        }
        let mut position = 0.0;
        let mut carets = Vec::new();
        for component in glyph.components.iter().take(glyph.components.len() - 1) {
            let component_width = project
                .glyphs
                .get(&component.base)
                .map(|base| base.width * component.x_scale + component.x_offset)
                .unwrap_or(component.x_offset);
            position += component_width;
            if let Ok(coordinate) = checked_i16(position, "合字caret位置") {
                carets.push(gdef::CaretValue::format_1(coordinate));
            }
        }
        if !carets.is_empty() {
            ligature_carets.insert(GlyphId16::new(glyph_id), carets);
        }
    }
    let ligature_caret_list = (!ligature_carets.is_empty()).then(|| {
        gdef::LigCaretList::new(
            ligature_carets.keys().copied().collect(),
            ligature_carets
                .into_values()
                .map(gdef::LigGlyph::new)
                .collect(),
        )
    });
    // Keep the named class spelling here: class expansion is useful for
    // layout rules, but GDEF MarkGlyphSets needs the original set identity so
    // lookupflag UseMarkFilteringSet can resolve to its index.
    let mark_sets = parse_mark_glyph_sets(feature_source, glyph_ids);
    let mark_glyph_sets = (!mark_sets.is_empty()).then(|| {
        let mut sets = mark_sets.values().collect::<Vec<_>>();
        sets.sort_by_key(|(index, _)| *index);
        gdef::MarkGlyphSets::new(
            sets.into_iter()
                .map(|(_, coverage)| coverage.clone())
                .collect(),
        )
    });
    let mark_attach_class_def = (!mark_attach_classes.is_empty()).then(|| {
        layout::ClassDef::format_2(
            mark_attach_classes
                .into_iter()
                .map(|(glyph, class)| layout::ClassRangeRecord::new(glyph, glyph, class))
                .collect(),
        )
    });
    let attach_list = parse_feature_attach_points(&expanded_source, glyph_ids);
    let mut table = gdef::Gdef::new(
        Some(class_def),
        attach_list,
        ligature_caret_list,
        mark_attach_class_def,
    );
    table.mark_glyph_sets_def = mark_glyph_sets.into();
    write_fonts::dump_table(&table).ok()
}

/// Parse explicit GDEF ligature caret definitions. Feature File coordinates
/// are design-unit positions, while index definitions point to contour
/// positions; explicit records take precedence over the component-width
/// fallback generated by the editor.
fn parse_feature_ligature_carets(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<GlyphId16, Vec<gdef::CaretValue>> {
    let mut carets = BTreeMap::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(kind_index) = tokens.iter().position(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "ligaturecaretbypos" | "ligaturecaretbyindex"
            )
        }) else {
            continue;
        };
        let kind = tokens[kind_index].to_ascii_lowercase();
        let format = if kind == "ligaturecaretbypos" {
            1
        } else if kind == "ligaturecaretbyindex" {
            2
        } else {
            continue;
        };
        let Some(value_index) = tokens[kind_index + 1..]
            .iter()
            .position(|token| {
                token
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<i32>()
                    .is_ok()
            })
            .map(|index| kind_index + 1 + index)
        else {
            continue;
        };
        let glyph_names = clean_feature_class(&tokens[kind_index + 1..value_index]);
        if glyph_names.is_empty() {
            continue;
        }
        let values = tokens[value_index..]
            .iter()
            .filter_map(|value| {
                value
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<i32>()
                    .ok()
            })
            .filter_map(|value| {
                if format == 1 {
                    i16::try_from(value).ok()
                } else {
                    u16::try_from(value).ok().map(|value| value as i16)
                }
            })
            .map(|value| {
                if format == 1 {
                    gdef::CaretValue::format_1(value)
                } else {
                    gdef::CaretValue::format_2(value as u16)
                }
            })
            .collect::<Vec<_>>();
        if !values.is_empty() {
            for glyph_name in glyph_names {
                if let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) {
                    carets.insert(GlyphId16::new(glyph_id), values.clone());
                }
            }
        }
    }
    carets
}

/// Parse GDEF `Attach` records. Each record maps a glyph (or glyph class) to
/// contour point indices used by attachment-aware layout engines.
fn parse_feature_attach_points(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<gdef::AttachList> {
    let mut records = BTreeMap::<GlyphId16, Vec<u16>>::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(keyword_index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("Attach"))
        else {
            continue;
        };
        let Some(point_index) = tokens[keyword_index + 1..]
            .iter()
            .position(|token| {
                token
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<u16>()
                    .is_ok()
            })
            .map(|index| keyword_index + 1 + index)
        else {
            continue;
        };
        let names = clean_feature_class(&tokens[keyword_index + 1..point_index]);
        let points = tokens[point_index..]
            .iter()
            .filter_map(|value| {
                value
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<u16>()
                    .ok()
            })
            .collect::<Vec<_>>();
        if names.is_empty() || points.is_empty() {
            continue;
        }
        let mut points = points;
        points.sort_unstable();
        points.dedup();
        for name in names {
            if let Some(&glyph_id) = glyph_ids.get(name.as_str()) {
                records.insert(GlyphId16::new(glyph_id), points.clone());
            }
        }
    }
    if records.is_empty() {
        return None;
    }
    Some(gdef::AttachList::new(
        records.keys().copied().collect(),
        records.into_values().map(gdef::AttachPoint::new).collect(),
    ))
}

/// Parse the optional GDEF mark attachment classes used by
/// `lookupflag MarkAttachmentType`. AFDKO accepts both a glyph class and a
/// named class reference here; named references have already been expanded by
/// the caller when they originate in the project's class source.
fn parse_feature_mark_attach_classes(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<GlyphId16, u16> {
    let mut classes = BTreeMap::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("MarkAttachClassDef"))
        else {
            continue;
        };
        let Some(class_index) = tokens[index + 1..].iter().position(|value| {
            value
                .trim_matches(|character: char| ",;[]".contains(character))
                .parse::<u16>()
                .is_ok()
        }) else {
            continue;
        };
        let class = tokens[index + 1 + class_index]
            .trim_matches(|character: char| ",;[]".contains(character))
            .parse::<u16>()
            .unwrap_or_default();
        let glyphs = clean_feature_class(&tokens[index + 1..index + 1 + class_index]);
        for glyph in glyphs {
            if let Some(&glyph_id) = glyph_ids.get(glyph.as_str()) {
                classes.insert(GlyphId16::new(glyph_id), class);
            }
        }
    }
    classes
}

/// Emit a conservative BASE table with the standard horizontal and vertical
/// baseline tags. The project's baseline is the font origin (0), which is the
/// interoperable fallback when no script-specific baseline metrics are stored.
fn build_base_table() -> Option<Vec<u8>> {
    let baseline_tags = vec![
        Tag::new(b"hang"),
        Tag::new(b"ideo"),
        Tag::new(b"math"),
        Tag::new(b"romn"),
    ];
    let coordinate = base::BaseCoord::Format1(base::BaseCoordFormat1::new(0));
    let make_script = || {
        base::BaseScript::new(
            Some(base::BaseValues::new(
                3,
                vec![
                    coordinate.clone(),
                    coordinate.clone(),
                    coordinate.clone(),
                    coordinate.clone(),
                ],
            )),
            None,
            Vec::new(),
        )
    };
    let scripts = base::BaseScriptList::new(
        [b"DFLT", b"hang", b"hani", b"kana", b"latn"]
            .into_iter()
            .map(|tag| base::BaseScriptRecord::new(Tag::new(tag), make_script()))
            .collect(),
    );
    let axis = || {
        base::Axis::new(
            Some(base::BaseTagList::new(baseline_tags.clone())),
            scripts.clone(),
        )
    };
    write_fonts::dump_table(&base::Base::new(Some(axis()), Some(axis()))).ok()
}

fn validate_master_axes(project: &FontProject) -> Result<(), String> {
    for master in &project.masters {
        if !master.weight.is_finite() || !(1.0..=1000.0).contains(&master.weight) {
            return Err(format!("マスター '{}' のWeightが不正です", master.name));
        }
        if !master.width.is_finite() || !(1.0..=1000.0).contains(&master.width) {
            return Err(format!("マスター '{}' のWidthが不正です", master.name));
        }
        for (tag, value) in &master.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                return Err(format!(
                    "マスター '{}' の軸タグ '{}' が不正です",
                    master.name, tag
                ));
            }
            if tag == "wdth" {
                return Err("カスタム軸タグ 'wdth' はWidth属性と重複します".into());
            }
            if tag == "wght" {
                return Err("カスタム軸タグ 'wght' はWeight属性と重複します".into());
            }
            if !value.is_finite() || *value < f32::MIN as f64 || *value > f32::MAX as f64 {
                return Err(format!(
                    "マスター '{}' の軸 '{}' の値が不正です",
                    master.name, tag
                ));
            }
        }
    }
    for instance in &project.instances {
        if !instance.weight.is_finite() || !(1.0..=1000.0).contains(&instance.weight) {
            return Err(format!(
                "名前付きインスタンス '{}' のWeightが不正です",
                instance.name
            ));
        }
        if !instance.width.is_finite() || !(1.0..=1000.0).contains(&instance.width) {
            return Err(format!(
                "名前付きインスタンス '{}' のWidthが不正です",
                instance.name
            ));
        }
        for (tag, value) in &instance.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸タグ '{}' が不正です",
                    instance.name, tag
                ));
            }
            if tag.eq_ignore_ascii_case("wght") || tag.eq_ignore_ascii_case("wdth") {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸 '{}' はWeight/Width属性と重複します",
                    instance.name, tag
                ));
            }
            if !value.is_finite() || *value < f32::MIN as f64 || *value > f32::MAX as f64 {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸 '{}' の値が不正です",
                    instance.name, tag
                ));
            }
        }
    }
    Ok(())
}

pub fn export_ttf_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    let master = project
        .masters
        .iter()
        .find(|master| master.id == master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", master_id))?;
    let mut single = project.clone();
    for glyph in single.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
        glyph.layers.clear();
    }
    single.masters = vec![master.clone()];
    single.default_master_id = master.id;
    if let Some(kerning) = project.kerning_by_master.get(master_id) {
        single.kerning = kerning.clone();
    }
    export_ttf(&single, path)
}

fn apply_conditional_layers(
    project: &mut FontProject,
    axis_values: &std::collections::HashMap<String, f64>,
) {
    let names: Vec<String> = project.conditional_layers.keys().cloned().collect();
    for name in names {
        let Some(layer) = project
            .conditional_layer_for_glyph(&name, axis_values)
            .map(|layer| layer.layer.clone())
        else {
            continue;
        };
        if let Some(glyph) = project.glyphs.get_mut(&name) {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
}

/// Exports a static TTF generated by interpolating two master layers.
pub fn export_ttf_at_interpolation(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
    factor: f64,
    path: &Path,
) -> Result<(), String> {
    if !factor.is_finite() {
        return Err("補間率が不正です".to_string());
    }
    let from = project
        .masters
        .iter()
        .find(|master| master.id == from_master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", from_master_id))?;
    let to = project
        .masters
        .iter()
        .find(|master| master.id == to_master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", to_master_id))?;
    let mut instance = project.clone();
    let t = factor.clamp(0.0, 1.0);
    let from_kerning = project
        .kerning_by_master
        .get(from_master_id)
        .unwrap_or(&project.kerning);
    let to_kerning = project
        .kerning_by_master
        .get(to_master_id)
        .unwrap_or(&project.kerning);
    let keys: std::collections::HashSet<_> = from_kerning
        .keys()
        .chain(to_kerning.keys())
        .cloned()
        .collect();
    instance.kerning = keys
        .into_iter()
        .filter_map(|key| {
            let a = from_kerning.get(&key).copied().unwrap_or(0.0);
            let b = to_kerning.get(&key).copied().unwrap_or(0.0);
            let value = a + (b - a) * t;
            (value.abs() > f64::EPSILON).then_some((key, value))
        })
        .collect();
    let instance_master_id = format!("instance-{t:.3}");
    for glyph in instance.glyphs.values_mut() {
        let from_layer = glyph.layers.get(from_master_id).cloned();
        let to_layer = glyph.layers.get(to_master_id).cloned();
        let (Some(a), Some(b)) = (from_layer, to_layer) else {
            if glyph.layers.is_empty() {
                continue;
            }
            return Err(if !glyph.layers.contains_key(from_master_id) {
                format!(
                    "グリフ '{}' に補間元マスター '{}' の層がありません",
                    glyph.name, from.name
                )
            } else {
                format!(
                    "グリフ '{}' に補間先マスター '{}' の層がありません",
                    glyph.name, to.name
                )
            });
        };
        let Some(layer) = a.interpolate(&b, t) else {
            return Err(format!(
                "グリフ '{}' のマスター形状を補間できません",
                glyph.name
            ));
        };
        glyph.width = layer.width;
        glyph.contours = layer.contours;
        glyph.components = layer.components;
        glyph.anchors = layer.anchors;
        glyph.layers.clear();
    }
    let mut interpolated_vertical = std::collections::HashMap::new();
    for name in project.glyphs.keys() {
        let a = project.vertical_metrics_for_glyph_in_master(name, from_master_id);
        let b = project.vertical_metrics_for_glyph_in_master(name, to_master_id);
        interpolated_vertical.insert(
            name.clone(),
            crate::font_data::VerticalMetrics {
                advance_height: a.advance_height + (b.advance_height - a.advance_height) * t,
                top_side_bearing: a.top_side_bearing
                    + (b.top_side_bearing - a.top_side_bearing) * t,
            },
        );
    }
    instance.vertical_metrics = interpolated_vertical;
    instance.vertical_metrics_by_master.clear();
    let mut master = from;
    master.id = instance_master_id;
    master.name = format!("{}–{} ({:.0}%)", master.name, to.name, t * 100.0);
    master.weight += (to.weight - master.weight) * t;
    master.width += (to.width - master.width) * t;
    let axis_tags: std::collections::HashSet<String> =
        master.axes.keys().chain(to.axes.keys()).cloned().collect();
    for tag in axis_tags {
        let a = master.axes.get(&tag).copied().unwrap_or(0.0);
        let b = to.axes.get(&tag).copied().unwrap_or(a);
        master.axes.insert(tag, a + (b - a) * t);
    }
    let mut axis_values = master.axes.clone();
    axis_values.insert("wght".into(), master.weight);
    axis_values.insert("wdth".into(), master.width);
    apply_conditional_layers(&mut instance, &axis_values);
    instance.masters = vec![master.clone()];
    instance.default_master_id = master.id;
    export_ttf(&instance, path)
}

pub fn export_interpolation_set(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
    factors: &[f64],
    directory: &Path,
) -> Result<usize, String> {
    if factors.is_empty() {
        return Err("補間率を1つ以上指定してください".to_string());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut seen = std::collections::HashSet::new();
    let mut count = 0;
    for &factor in factors {
        if !factor.is_finite() || !(0.0..=1.0).contains(&factor) {
            return Err("補間率は0〜1の範囲で指定してください".to_string());
        }
        let key = (factor * 1000.0).round() as i64;
        if !seen.insert(key) {
            return Err(format!("補間率 {:.1}% が重複しています", factor * 100.0));
        }
        let filename = format!("instance-{:.0}.ttf", factor * 100.0);
        export_ttf_at_interpolation(
            project,
            from_master_id,
            to_master_id,
            factor,
            &directory.join(filename),
        )?;
        count += 1;
    }
    Ok(count)
}

/// Exports one static TTF per master into a directory.
pub fn export_all_ttf_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("出力対象のマスターがありません".into());
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("TTF出力先を作成できません: {error}"))?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let base: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        let base = if base.is_empty() {
            format!("master-{}", index + 1)
        } else {
            base
        };
        let mut filename = base.clone();
        let mut suffix = 2;
        while !used.insert(filename.clone()) {
            filename = format!("{base}_{suffix}");
            suffix += 1;
        }
        export_ttf_for_master(
            project,
            &master.id,
            &directory.join(format!("{filename}.ttf")),
        )?;
    }
    Ok(project.masters.len())
}

pub(crate) fn validate_feature_source(source: &str) -> Result<(), String> {
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(character) = chars.next() {
        if !in_string && character == '#' {
            for comment_char in chars.by_ref() {
                if comment_char == '\n' {
                    code.push('\n');
                    break;
                }
            }
            continue;
        }
        if character == '"' {
            in_string = !in_string;
            code.push(' ');
            continue;
        }
        if in_string {
            code.push(' ');
        } else {
            code.push(character);
        }
    }
    if in_string {
        return Err("OpenType featureの文字列が閉じていません".into());
    }
    let mut braces = 0_i32;
    for character in code.chars() {
        match character {
            '{' => braces += 1,
            '}' => braces -= 1,
            _ => {}
        }
        if braces < 0 {
            return Err("OpenType featureの閉じ括弧が不正です".into());
        }
    }
    if braces != 0 {
        return Err("OpenType featureの括弧が閉じていません".into());
    }
    let mut declared_tags = std::collections::HashSet::new();
    let tokens: Vec<_> = code.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        if *token != "feature" {
            continue;
        }
        let line_number = code[..code.find(token).unwrap_or(0)]
            .bytes()
            .filter(|&byte| byte == b'\n')
            .count()
            + 1;
        let tag = tokens
            .get(index + 1)
            .ok_or_else(|| format!("OpenType featureのタグがありません（{}行目）", line_number))?;
        if tag.len() != 4
            || !tag
                .bytes()
                .all(|byte| byte.is_ascii() && !byte.is_ascii_control())
        {
            return Err(format!(
                "OpenType featureタグはASCII 4文字で指定してください（{}行目）",
                line_number
            ));
        }
        if !declared_tags.insert((*tag).to_string()) {
            return Err(format!(
                "OpenType feature '{}' が重複しています（{}行目）",
                tag, line_number
            ));
        }
        if !tokens[index + 2..].contains(&"{") {
            return Err(format!(
                "OpenType feature宣言に '{{' がありません（{}行目）",
                line_number
            ));
        }
    }
    let mut lookup_names = std::collections::HashSet::new();
    for (name, _) in extract_lookup_blocks(source) {
        if !lookup_names.insert(name.clone()) {
            return Err(format!("OpenType lookup '{}' が重複しています", name));
        }
    }
    for statement in code.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.first() != Some(&"languagesystem") {
            continue;
        }
        if tokens.len() != 3 {
            return Err("languagesystemはScriptタグとLanguageタグを指定してください".into());
        }
        let script = tokens[1].trim_matches(|character: char| "{}".contains(character));
        let language = tokens[2].trim_matches(|character: char| "{}".contains(character));
        if script.len() != 4 || !script.is_ascii() {
            return Err(format!(
                "languagesystemのScriptタグはASCII 4文字で指定してください: {}",
                tokens[1]
            ));
        }
        if !(language.len() == 3 || language.len() == 4) || !language.is_ascii() {
            return Err(format!(
                "languagesystemのLanguageタグはASCII 3〜4文字で指定してください: {}",
                tokens[2]
            ));
        }
    }
    for (_, block) in extract_feature_blocks(source) {
        for statement in block.split(';') {
            let statement_tokens = statement.split_whitespace().collect::<Vec<_>>();
            if statement_tokens.first() != Some(&"lookup") {
                continue;
            }
            let Some(name) = statement_tokens.get(1) else {
                return Err("OpenType lookup参照に名前がありません".into());
            };
            if statement_tokens.contains(&"{") {
                continue;
            }
            if !lookup_names.contains(*name) {
                return Err(format!("OpenType lookup '{}' が未定義です", name));
            }
        }
    }
    Ok(())
}

fn validate_component_master_topology(project: &FontProject, base_id: &str) -> Result<(), String> {
    for glyph in project.glyphs.values() {
        let Some(base) = glyph.layers.get(base_id) else {
            continue;
        };
        for master in &project.masters {
            let Some(layer) = glyph.layers.get(&master.id) else {
                continue;
            };
            if base.components.len() != layer.components.len()
                || base
                    .components
                    .iter()
                    .zip(&layer.components)
                    .any(|(a, b)| a.base != b.base)
            {
                return Err(format!(
                    "グリフ '{}' のマスター '{}' 間でコンポーネント構造が一致しません",
                    glyph.name, master.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_component_master_transforms(
    project: &FontProject,
    base_id: &str,
) -> Result<(), String> {
    for glyph in project.glyphs.values() {
        let Some(base) = glyph.layers.get(base_id) else {
            continue;
        };
        for master in &project.masters {
            let Some(layer) = glyph.layers.get(&master.id) else {
                continue;
            };
            if base.components.iter().zip(&layer.components).any(|(a, b)| {
                a.base != b.base
                    || a.x_scale != b.x_scale
                    || a.xy_scale != b.xy_scale
                    || a.yx_scale != b.yx_scale
                    || a.y_scale != b.y_scale
            }) {
                return Err(format!(
                    "グリフ '{}' のマスター '{}' でコンポーネントの変形が一致しません",
                    glyph.name, master.name
                ));
            }
        }
    }
    Ok(())
}

fn checked_u16(value: f64, label: &str) -> Result<u16, String> {
    if !value.is_finite() || value < 0.0 || value > u16::MAX as f64 || value.fract() != 0.0 {
        return Err(format!("{label}は有効な整数範囲で指定してください"));
    }
    Ok(value as u16)
}

fn font_vendor_id(value: &str) -> [u8; 4] {
    let bytes = value.as_bytes();
    if bytes.len() == 4 && bytes.iter().all(|byte| byte.is_ascii()) {
        [bytes[0], bytes[1], bytes[2], bytes[3]]
    } else {
        *b"GLYP"
    }
}

fn build_vertical_metrics_tables(
    project: &FontProject,
    names: &[&str],
    master_id: &str,
    upm: u16,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let master_metrics = project.master_metrics_for(master_id);
    let mut metrics = vec![(upm, checked_i16(master_metrics.ascender, "縦TSB")?)];
    metrics.extend(
        names
            .iter()
            .map(|name| project.vertical_metrics_for_glyph_in_master(name, master_id))
            .map(|metric| {
                Ok((
                    checked_u16(metric.advance_height, "縦アドバンス")?,
                    checked_i16(metric.top_side_bearing, "縦TSB")?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?,
    );
    let max_advance = metrics
        .iter()
        .map(|(advance, _)| *advance)
        .max()
        .unwrap_or(upm);
    let mut vhea = Vec::with_capacity(36);
    vhea.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.ascender, "縦アセンダ")?.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.descender, "縦ディセンダ")?.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.line_gap, "縦Line Gap")?.to_be_bytes());
    vhea.extend_from_slice(&max_advance.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.ascender, "縦Y最大")?.to_be_bytes());
    vhea.extend_from_slice(
        &if project.metadata.vertical_caret_slope_rise != 0 {
            project.metadata.vertical_caret_slope_rise
        } else {
            1
        }
        .to_be_bytes(),
    );
    vhea.extend_from_slice(&project.metadata.vertical_caret_slope_run.to_be_bytes());
    vhea.extend_from_slice(&project.metadata.vertical_caret_offset.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(
        &(u16::try_from(metrics.len()).map_err(|_| "縦メトリクスが多すぎます")?).to_be_bytes(),
    );
    let mut vmtx = Vec::with_capacity(metrics.len() * 4);
    for (advance, bearing) in metrics {
        vmtx.extend_from_slice(&advance.to_be_bytes());
        vmtx.extend_from_slice(&bearing.to_be_bytes());
    }
    Ok((vhea, vmtx))
}

fn checked_i16(value: f64, label: &str) -> Result<i16, String> {
    if !value.is_finite()
        || value < i16::MIN as f64
        || value > i16::MAX as f64
        || value.fract() != 0.0
    {
        return Err(format!("{label}がTrueTypeの範囲外です"));
    }
    Ok(value as i16)
}

fn checked_fixed_16_16(value: f64, label: &str) -> Result<i32, String> {
    let scaled = value * 65_536.0;
    if !scaled.is_finite() || scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return Err(format!("{label}が16.16固定小数点の範囲外です"));
    }
    Ok(scaled.round() as i32)
}

fn build_gvar_variation(
    source: &GlyphData,
    project: &FontProject,
    base_id: &str,
    has_width_axis: bool,
    has_variation: &mut bool,
) -> Result<Option<fonttools::gvar::GlyphVariationData>, String> {
    let Some(base) = source.layers.get(base_id) else {
        return Ok(None);
    };
    // The static glyf table is built from GlyphData's active outline. Only
    // emit deltas when that outline is the same as the selected base layer;
    // otherwise the gvar point indices would describe a different shape.
    if source.width != base.width
        || source.contours != base.contours
        || source.components != base.components
    {
        return Ok(None);
    }
    let mut custom_axis_tags: Vec<String> = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    custom_axis_tags.retain(|tag| {
        let mut values = project
            .masters
            .iter()
            .map(|master| master.axes.get(tag).copied().unwrap_or(0.0));
        values
            .next()
            .is_some_and(|first| values.any(|value| (value - first).abs() > f64::EPSILON))
    });
    let axis_value = |master: &FontMaster, tag: &str| master.axes.get(tag).copied().unwrap_or(0.0);
    let base_master = project.masters.iter().find(|master| master.id == base_id);
    let widths: Vec<f64> = project.masters.iter().map(|master| master.width).collect();
    let min_width = widths.iter().copied().fold(f64::INFINITY, f64::min);
    let max_width = widths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let default_width = project
        .masters
        .iter()
        .find(|master| master.id == base_id)
        .map(|master| master.width)
        .unwrap_or_default();
    let mut deltasets = Vec::new();
    for master in project.masters.iter().filter(|master| master.id != base_id) {
        let Some(target) = source.layers.get(&master.id) else {
            continue;
        };
        let component_variation = !base.components.is_empty()
            && base.components.len() == target.components.len()
            && base
                .components
                .iter()
                .zip(&target.components)
                .all(|(a, b)| {
                    a.base == b.base
                        && a.x_scale == b.x_scale
                        && a.xy_scale == b.xy_scale
                        && a.yx_scale == b.yx_scale
                        && a.y_scale == b.y_scale
                });
        let contour_variation = base.components.is_empty()
            && target.components.is_empty()
            && base.contours.len() == target.contours.len()
            && base
                .contours
                .iter()
                .zip(&target.contours)
                .all(|(a, b)| a.points.len() == b.points.len());
        if !component_variation && !contour_variation {
            continue;
        }
        let mut deltas = Vec::new();
        if component_variation {
            for (a, b) in base.components.iter().zip(&target.components) {
                deltas.push((
                    checked_i16(b.x_offset, "gvar コンポーネントX")?
                        .checked_sub(checked_i16(a.x_offset, "gvar 基準コンポーネントX")?)
                        .ok_or_else(|| "gvar X差分が範囲外です".to_string())?,
                    checked_i16(b.y_offset, "gvar コンポーネントY")?
                        .checked_sub(checked_i16(a.y_offset, "gvar 基準コンポーネントY")?)
                        .ok_or_else(|| "gvar Y差分が範囲外です".to_string())?,
                ));
            }
        } else {
            for (a_contour, b_contour) in base.contours.iter().zip(&target.contours) {
                for (a, b) in a_contour.points.iter().zip(&b_contour.points) {
                    deltas.push((
                        checked_i16(b.x, "gvar ターゲットX")?
                            .checked_sub(checked_i16(a.x, "gvar 基準X")?)
                            .ok_or_else(|| "gvar X差分が範囲外です".to_string())?,
                        checked_i16(b.y, "gvar ターゲットY")?
                            .checked_sub(checked_i16(a.y, "gvar 基準Y")?)
                            .ok_or_else(|| "gvar Y差分が範囲外です".to_string())?,
                    ));
                }
            }
        }
        deltas.extend([
            (0, 0),
            (
                checked_i16(target.width, "gvar ターゲット幅")?
                    .checked_sub(checked_i16(base.width, "gvar 基準幅")?)
                    .ok_or_else(|| "gvar 幅差分が範囲外です".to_string())?,
                0,
            ),
            (0, 0),
            (0, 0),
        ]);
        if deltas.iter().any(|(x, y)| *x != 0 || *y != 0) {
            *has_variation = true;
            let custom_peaks = custom_axis_tags.iter().map(|tag| {
                let values: Vec<f64> = project.masters.iter().map(|m| axis_value(m, tag)).collect();
                normalize_axis(
                    axis_value(master, tag),
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    base_master.map(|m| axis_value(m, tag)).unwrap_or_default(),
                    values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            });
            let weight_peak = if custom_axis_tags.is_empty() {
                let values: Vec<f64> = project.masters.iter().map(|m| m.weight).collect();
                normalize_axis(
                    master.weight,
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    base_master.map(|m| m.weight).unwrap_or_default(),
                    values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            } else {
                0.0
            };
            let width_peak = normalize_axis(master.width, min_width, default_width, max_width);
            deltasets.push(fonttools::gvar::DeltaSet {
                peak: if has_width_axis {
                    if custom_axis_tags.is_empty() {
                        vec![weight_peak, width_peak]
                    } else {
                        custom_peaks.chain(std::iter::once(width_peak)).collect()
                    }
                } else {
                    if custom_axis_tags.is_empty() {
                        vec![weight_peak]
                    } else {
                        custom_peaks.collect()
                    }
                },
                start: if has_width_axis {
                    vec![
                        0.0;
                        custom_axis_tags.len()
                            + usize::from(has_width_axis || custom_axis_tags.is_empty())
                    ]
                } else {
                    vec![0.0; custom_axis_tags.len().max(1)]
                },
                end: if has_width_axis {
                    vec![
                        1.0;
                        custom_axis_tags.len()
                            + usize::from(has_width_axis || custom_axis_tags.is_empty())
                    ]
                } else {
                    vec![1.0; custom_axis_tags.len().max(1)]
                },
                deltas,
            });
        }
    }
    Ok((!deltasets.is_empty()).then_some(fonttools::gvar::GlyphVariationData { deltasets }))
}

fn normalize_axis(value: f64, min: f64, default: f64, max: f64) -> f32 {
    if value >= default {
        if (max - default).abs() < f64::EPSILON {
            0.0
        } else {
            ((value - default) / (max - default)).clamp(-1.0, 1.0) as f32
        }
    } else if (default - min).abs() < f64::EPSILON {
        0.0
    } else {
        ((value - default) / (default - min)).clamp(-1.0, 1.0) as f32
    }
}

fn build_hvar(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    if project.masters.len() < 2 || axis_tags.is_empty() {
        return None;
    }
    let axis_value = |master: &FontMaster, tag: &str| match tag {
        "wght" => master.weight,
        "wdth" => master.width,
        _ => master.axes.get(tag).copied().unwrap_or(0.0),
    };
    let axis_bounds = axis_tags
        .iter()
        .map(|tag| {
            let values = project.masters.iter().map(|master| axis_value(master, tag));
            (
                tag,
                values.clone().fold(f64::INFINITY, f64::min),
                axis_value(base_master, tag),
                values.fold(f64::NEG_INFINITY, f64::max),
            )
        })
        .collect::<Vec<_>>();
    let regions = project
        .masters
        .iter()
        .filter(|master| master.id != base_master.id)
        .map(|master| {
            let coords = axis_bounds
                .iter()
                .map(|(tag, min, default, max)| {
                    let peak = normalize_axis(axis_value(master, tag), *min, *default, *max);
                    let start = peak.min(0.0);
                    let end = peak.max(0.0);
                    write_fonts::tables::variations::RegionAxisCoordinates::new(
                        write_fonts::types::F2Dot14::from_f32(start),
                        write_fonts::types::F2Dot14::from_f32(peak),
                        write_fonts::types::F2Dot14::from_f32(end),
                    )
                })
                .collect::<Vec<_>>();
            (
                master.id.clone(),
                write_fonts::tables::variations::VariationRegion::new(coords),
            )
        })
        .collect::<Vec<_>>();
    if regions.is_empty() {
        return None;
    }
    let base_width = |name: &str| {
        project
            .glyphs
            .get(name)
            .and_then(|glyph| glyph.layers.get(&base_master.id).map(|layer| layer.width))
            .or_else(|| project.glyphs.get(name).map(|glyph| glyph.width))
            .unwrap_or(0.0)
    };
    let mut builder = write_fonts::tables::variations::ivs_builder::VariationStoreBuilder::new(
        axis_tags.len() as u16,
    );
    let mut temporary_indices = Vec::with_capacity(names.len() + 1);
    let mut has_delta = false;
    temporary_indices.push(builder.add_deltas::<i32>(Vec::new()));
    for name in names {
        let base = base_width(name);
        let deltas = regions
            .iter()
            .map(|(master_id, region)| {
                let target = project
                    .glyphs
                    .get(*name)
                    .and_then(|glyph| glyph.layers.get(master_id))
                    .map(|layer| layer.width)
                    .unwrap_or(base);
                let delta = (target - base).round() as i32;
                has_delta |= delta != 0;
                (region.clone(), delta)
            })
            .collect::<Vec<_>>();
        temporary_indices.push(builder.add_deltas(deltas));
    }
    if !has_delta {
        return None;
    }
    let (store, remapping) = builder.build();
    let mapping: write_fonts::tables::variations::DeltaSetIndexMap = temporary_indices
        .into_iter()
        .map(|index| remapping.get(index).unwrap())
        .collect();
    write_fonts::dump_table(&write_fonts::tables::hvar::Hvar::new(
        store,
        Some(mapping),
        None,
        None,
    ))
    .ok()
}

fn build_vvar(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    let (store, mapping) =
        build_metric_variation_store(project, names, base_master, axis_tags, |name, master_id| {
            project
                .vertical_metrics_for_glyph_in_master(name, master_id)
                .advance_height
        })?;
    let mapping: write_fonts::tables::variations::DeltaSetIndexMap = mapping.into_iter().collect();
    write_fonts::dump_table(&write_fonts::tables::vvar::Vvar::new(
        store,
        Some(mapping),
        None,
        None,
        None,
    ))
    .ok()
}

fn build_mvar(
    project: &FontProject,
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    let metric_names = ["ascender", "descender", "lineGap"];
    let (store, mapping) = build_metric_variation_store(
        project,
        &metric_names,
        base_master,
        axis_tags,
        |name, master_id| {
            let metrics = project.master_metrics_for(master_id);
            match name {
                "ascender" => metrics.ascender,
                "descender" => metrics.descender,
                _ => metrics.line_gap,
            }
        },
    )?;
    let store_bytes = write_fonts::dump_table(&store).ok()?;
    let record_size = 8u16;
    let record_count = u16::try_from(metric_names.len()).ok()?;
    let store_offset = 12usize + usize::from(record_size) * metric_names.len();
    let mut bytes = Vec::with_capacity(store_offset + store_bytes.len());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&record_size.to_be_bytes());
    bytes.extend_from_slice(&record_count.to_be_bytes());
    bytes.extend_from_slice(&u16::try_from(store_offset).ok()?.to_be_bytes());
    for (tag, index) in [(*b"hasc", 1usize), (*b"hdsc", 2), (*b"hlgp", 3)] {
        let variation_index = mapping.get(index)?;
        bytes.extend_from_slice(&tag);
        bytes.extend_from_slice(&variation_index.delta_set_outer_index.to_be_bytes());
        bytes.extend_from_slice(&variation_index.delta_set_inner_index.to_be_bytes());
    }
    bytes.extend_from_slice(&store_bytes);
    Some(bytes)
}

fn build_metric_variation_store<F>(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
    metric: F,
) -> Option<(
    write_fonts::tables::variations::ItemVariationStore,
    Vec<write_fonts::tables::layout::VariationIndex>,
)>
where
    F: Fn(&str, &str) -> f64,
{
    if project.masters.len() < 2 || axis_tags.is_empty() {
        return None;
    }
    let axis_value = |master: &FontMaster, tag: &str| match tag {
        "wght" => master.weight,
        "wdth" => master.width,
        _ => master.axes.get(tag).copied().unwrap_or(0.0),
    };
    let bounds = axis_tags
        .iter()
        .map(|tag| {
            let values = project.masters.iter().map(|master| axis_value(master, tag));
            (
                tag,
                values.clone().fold(f64::INFINITY, f64::min),
                axis_value(base_master, tag),
                values.fold(f64::NEG_INFINITY, f64::max),
            )
        })
        .collect::<Vec<_>>();
    let regions = project
        .masters
        .iter()
        .filter(|master| master.id != base_master.id)
        .map(|master| {
            let axes = bounds
                .iter()
                .map(|(tag, min, default, max)| {
                    let peak = normalize_axis(axis_value(master, tag), *min, *default, *max);
                    write_fonts::tables::variations::RegionAxisCoordinates::new(
                        write_fonts::types::F2Dot14::from_f32(peak.min(0.0)),
                        write_fonts::types::F2Dot14::from_f32(peak),
                        write_fonts::types::F2Dot14::from_f32(peak.max(0.0)),
                    )
                })
                .collect();
            (
                master.id.clone(),
                write_fonts::tables::variations::VariationRegion::new(axes),
            )
        })
        .collect::<Vec<_>>();
    if regions.is_empty() {
        return None;
    }
    let mut builder = write_fonts::tables::variations::ivs_builder::VariationStoreBuilder::new(
        axis_tags.len() as u16,
    );
    let mut temporary = vec![builder.add_deltas::<i32>(Vec::new())];
    let mut changed = false;
    for name in names {
        let base = metric(name, &base_master.id);
        let deltas = regions
            .iter()
            .map(|(master_id, region)| {
                let delta = (metric(name, master_id) - base).round() as i32;
                changed |= delta != 0;
                (region.clone(), delta)
            })
            .collect();
        temporary.push(builder.add_deltas(deltas));
    }
    if !changed {
        return None;
    }
    let (store, remapping) = builder.build();
    let mapping = temporary
        .into_iter()
        .map(|index| remapping.get(index).unwrap())
        .collect();
    Some((store, mapping))
}

fn postscript_name(family: &str, style: &str) -> String {
    let sanitize = |value: &str| {
        let mut result: String = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
            .collect();
        while result.starts_with('-') {
            result.remove(0);
        }
        result.truncate(30);
        if result.is_empty() {
            "Font".to_string()
        } else {
            result
        }
    };
    let mut result = format!("{}-{}", sanitize(family), sanitize(style));
    result.truncate(63);
    result
}

fn build_cmap_format12(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let mut groups = Vec::<(u32, u32, u32)>::new();
    for (&codepoint, &glyph_id) in mapping {
        let can_extend = groups.last().map(|(start, end, start_glyph)| {
            codepoint == *end + 1
                && u64::from(*start_glyph) + u64::from(codepoint - *start) == u64::from(glyph_id)
        }) == Some(true);
        if can_extend {
            groups.last_mut().unwrap().1 = codepoint;
        } else {
            groups.push((codepoint, codepoint, u32::from(glyph_id)));
        }
    }
    let subtable_length = 16 + groups.len() as u32 * 12;
    let mut subtable = Vec::with_capacity(subtable_length as usize);
    subtable.extend_from_slice(&12_u16.to_be_bytes());
    subtable.extend_from_slice(&0_u16.to_be_bytes());
    subtable.extend_from_slice(&subtable_length.to_be_bytes());
    subtable.extend_from_slice(&0_u32.to_be_bytes());
    subtable.extend_from_slice(&(groups.len() as u32).to_be_bytes());
    for (start, end, start_glyph) in groups {
        subtable.extend_from_slice(&start.to_be_bytes());
        subtable.extend_from_slice(&end.to_be_bytes());
        subtable.extend_from_slice(&start_glyph.to_be_bytes());
    }
    let mut output = Vec::with_capacity(12 + subtable.len());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&3_u16.to_be_bytes());
    output.extend_from_slice(&10_u16.to_be_bytes());
    output.extend_from_slice(&12_u32.to_be_bytes());
    output.extend_from_slice(&subtable);
    output
}

fn style_is_italic(metadata: &FontMetadata) -> bool {
    metadata.italic_angle.abs() > f64::EPSILON
        || metadata.style_name.to_ascii_lowercase().contains("italic")
        || metadata.style_name.to_ascii_lowercase().contains("oblique")
}

fn normalized_f2dot14(value: f64) -> i16 {
    (value.clamp(-1.0, 1.0) * 16384.0).round() as i16
}

fn build_avar(
    axis_tags: &[String],
    mappings: &std::collections::HashMap<String, Vec<AxisMappingPoint>>,
) -> Option<Vec<u8>> {
    let mut axis_maps = Vec::new();
    let mut has_non_identity = false;
    for tag in axis_tags {
        let mut points = mappings
            .get(tag)
            .into_iter()
            .flatten()
            .copied()
            .filter(|point| point.input.is_finite() && point.output.is_finite())
            .map(|point| (point.input.clamp(-1.0, 1.0), point.output.clamp(-1.0, 1.0)))
            .collect::<Vec<_>>();
        points.sort_by(|left, right| left.0.total_cmp(&right.0));
        points.dedup_by(|left, right| (left.0 - right.0).abs() < f64::EPSILON);
        if points.is_empty() {
            points = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
        }
        if !points.iter().any(|(input, _)| input.abs() < f64::EPSILON) {
            points.push((0.0, 0.0));
            points.sort_by(|left, right| left.0.total_cmp(&right.0));
        }
        if !points
            .iter()
            .any(|(input, _)| (*input + 1.0).abs() < f64::EPSILON)
        {
            points.insert(0, (-1.0, -1.0));
        }
        if !points
            .iter()
            .any(|(input, _)| (*input - 1.0).abs() < f64::EPSILON)
        {
            points.push((1.0, 1.0));
        }
        if points
            .iter()
            .all(|(input, output)| (input - output).abs() < f64::EPSILON)
        {
            axis_maps.push(points);
        } else {
            has_non_identity = true;
            axis_maps.push(points);
        }
    }
    if axis_maps.len() != axis_tags.len() || !has_non_identity {
        return None;
    }
    let mut output = Vec::new();
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(axis_tags.len() as u16).to_be_bytes());
    for points in axis_maps {
        output.extend_from_slice(&(points.len() as u16).to_be_bytes());
        for (input, output_value) in points {
            output.extend_from_slice(&normalized_f2dot14(input).to_be_bytes());
            output.extend_from_slice(&normalized_f2dot14(output_value).to_be_bytes());
        }
    }
    Some(output)
}

fn style_is_bold(metadata: &FontMetadata) -> bool {
    metadata.weight_class >= 700 || metadata.style_name.to_ascii_lowercase().contains("bold")
}

fn mac_style_flags(metadata: &FontMetadata) -> u16 {
    (style_is_bold(metadata) as u16) | ((style_is_italic(metadata) as u16) << 1)
}

/// Fingerprint the project inputs that can change generated GDEF/GPOS or
/// invalidate glyph IDs. Outline coordinates are intentionally excluded: a
/// contour edit does not alter the layout tables.
pub(crate) fn layout_input_fingerprint(project: &FontProject) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project.glyph_names_sorted().hash(&mut hasher);
    for name in project.glyph_names_sorted() {
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        glyph.unicode.hash(&mut hasher);
        glyph.unicodes.hash(&mut hasher);
        glyph.width.to_bits().hash(&mut hasher);
        glyph.left_kerning_group.hash(&mut hasher);
        glyph.right_kerning_group.hash(&mut hasher);
        glyph.anchors.len().hash(&mut hasher);
        for anchor in &glyph.anchors {
            anchor.name.hash(&mut hasher);
            anchor.x.to_bits().hash(&mut hasher);
            anchor.y.to_bits().hash(&mut hasher);
        }
    }
    let mut kerning = project.kerning.iter().collect::<Vec<_>>();
    kerning.sort_by(|left, right| left.0.cmp(right.0));
    for ((left, right), value) in kerning {
        left.hash(&mut hasher);
        right.hash(&mut hasher);
        value.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn os2_selection_flags(metadata: &FontMetadata) -> u16 {
    if metadata.fs_selection != 0 {
        return metadata.fs_selection;
    }
    let italic = style_is_italic(metadata);
    let bold = style_is_bold(metadata);
    let regular = !italic && !bold && metadata.weight_class == 400;
    // USE_TYPO_METRICS and WWS make modern consumers prefer the same
    // typographic metrics and family/style grouping used by the editor.
    (italic as u16) | ((bold as u16) << 5) | ((regular as u16) << 6) | (1 << 7) | (1 << 8)
}

fn max_feature_context(source: &str) -> u16 {
    let mut maximum = 1_usize;
    for statement in normalize_feature_keywords(source).split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(index) = tokens
            .iter()
            .position(|token| matches!(*token, "sub" | "pos" | "reversesub"))
        else {
            continue;
        };
        let end = tokens[index..]
            .iter()
            .position(|token| matches!(*token, "by" | "from"))
            .map(|offset| index + offset)
            .unwrap_or(tokens.len());
        maximum = maximum.max(end.saturating_sub(index + 1));
    }
    maximum.min(u16::MAX as usize) as u16
}

/// Returns the four OS/2 Unicode-range bitfields. The ranges are deliberately
/// block-oriented: a bit is advertised when at least one mapped code point is
/// in that Unicode block, which is the convention used by font consumers.
fn unicode_range_bits(mapping: &BTreeMap<u32, u16>) -> (u32, u32, u32, u32) {
    const RANGES: &[(u32, u32, u8)] = &[
        (0x0000, 0x007F, 0),  // Basic Latin
        (0x0080, 0x00FF, 1),  // Latin-1 Supplement
        (0x0100, 0x017F, 2),  // Latin Extended-A
        (0x0180, 0x024F, 3),  // Latin Extended-B
        (0x0250, 0x02AF, 4),  // IPA Extensions
        (0x02B0, 0x02FF, 5),  // Spacing Modifier Letters
        (0x0300, 0x036F, 6),  // Combining Diacritical Marks
        (0x0370, 0x03FF, 7),  // Greek and Coptic
        (0x0400, 0x04FF, 8),  // Cyrillic
        (0x0530, 0x058F, 9),  // Armenian
        (0x0590, 0x05FF, 10), // Hebrew
        (0x0600, 0x06FF, 11), // Arabic
        (0x0900, 0x097F, 12), // Devanagari
        (0x0980, 0x09FF, 13), // Bengali
        (0x0A00, 0x0A7F, 14), // Gurmukhi
        (0x0A80, 0x0AFF, 15), // Gujarati
        (0x0B00, 0x0B7F, 16), // Oriya
        (0x0B80, 0x0BFF, 17), // Tamil
        (0x0C00, 0x0C7F, 18), // Telugu
        (0x0C80, 0x0CFF, 19), // Kannada
        (0x0D00, 0x0D7F, 20), // Malayalam
        (0x0E00, 0x0E7F, 21), // Thai
        (0x0E80, 0x0EFF, 22), // Lao
        (0x10A0, 0x10FF, 23), // Georgian
        (0x1100, 0x11FF, 24), // Hangul Jamo
        (0x1E00, 0x1EFF, 25), // Latin Extended Additional
        (0x1F00, 0x1FFF, 26), // Greek Extended
        (0x2000, 0x206F, 27), // General Punctuation
        (0x2070, 0x209F, 28), // Superscripts and Subscripts
        (0x20A0, 0x20CF, 29), // Currency Symbols
        (0x20D0, 0x20FF, 30), // Combining Diacritical Marks for Symbols
        (0x2100, 0x214F, 31), // Letterlike Symbols
        (0x2150, 0x218F, 32), // Number Forms
        (0x2190, 0x21FF, 33), // Arrows
        (0x2200, 0x22FF, 34), // Mathematical Operators
        (0x2300, 0x23FF, 35), // Miscellaneous Technical
        (0x2500, 0x257F, 36), // Box Drawing
        (0x2580, 0x259F, 37), // Block Elements
        (0x25A0, 0x25FF, 38), // Geometric Shapes
        (0x2600, 0x26FF, 39), // Miscellaneous Symbols
        (0x2700, 0x27BF, 40), // Dingbats
        (0x3000, 0x303F, 48), // CJK Symbols and Punctuation
        (0x3040, 0x309F, 49), // Hiragana
        (0x30A0, 0x30FF, 50), // Katakana
        (0x3100, 0x312F, 51), // Bopomofo
        (0x3130, 0x318F, 52), // Hangul Compatibility Jamo
        (0x31A0, 0x31BF, 53), // Bopomofo Extended
        (0x31F0, 0x31FF, 54), // Katakana Phonetic Extensions
        (0x4E00, 0x9FFF, 59), // CJK Unified Ideographs
        (0xAC00, 0xD7AF, 56), // Hangul Syllables
        (0xF900, 0xFAFF, 60), // CJK Compatibility Ideographs
        (0xFE30, 0xFE4F, 61), // CJK Compatibility Forms
        (0xFF00, 0xFFEF, 62), // Halfwidth and Fullwidth Forms
    ];
    let mut bits = [0u32; 4];
    for &codepoint in mapping.keys() {
        for &(start, end, bit) in RANGES {
            if (start..=end).contains(&codepoint) && bit < 128 {
                bits[(bit / 32) as usize] |= 1u32 << (bit % 32);
            }
        }
    }
    (bits[0], bits[1], bits[2], bits[3])
}

fn code_page_range_bits(mapping: &BTreeMap<u32, u16>) -> (u32, u32) {
    let mut range1 = 0u32;
    let mut range2 = 0u32;
    let has = |ranges: &[(u32, u32)]| {
        mapping.keys().any(|codepoint| {
            ranges
                .iter()
                .any(|(start, end)| (*start..=*end).contains(codepoint))
        })
    };
    for (bit, ranges) in [
        (0, &[(0x0000, 0x007F), (0x00A0, 0x00FF)][..]), // Latin 1 / 1252
        (1, &[(0x0100, 0x024F), (0x1E00, 0x1EFF)][..]), // Latin 2 / 1250
        (2, &[(0x0400, 0x04FF)][..]),                   // Cyrillic / 1251
        (3, &[(0x0370, 0x03FF)][..]),                   // Greek / 1253
        (4, &[(0x0100, 0x017F)][..]),                   // Turkish / 1254
        (5, &[(0x0590, 0x05FF)][..]),                   // Hebrew / 1255
        (6, &[(0x0600, 0x06FF)][..]),                   // Arabic / 1256
        (16, &[(0x0E00, 0x0E7F)][..]),                  // Thai / 874
        (17, &[(0x3040, 0x30FF), (0x4E00, 0x9FFF)][..]), // Japanese / 932
        (19, &[(0xAC00, 0xD7AF)][..]),                  // Korean / 949
        (20, &[(0xFF00, 0xFFEF)][..]),                  // Traditional CJK / 950
    ] {
        if has(ranges) {
            if bit < 32 {
                range1 |= 1u32 << bit;
            } else {
                range2 |= 1u32 << (bit - 32);
            }
        }
    }
    (range1, range2)
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_cmap_with_bmp_and_full_unicode(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let format4 = build_cmap_format4(mapping);
    let format12 = build_cmap_format12(mapping);
    let format12_subtable = &format12[12..];
    let records = [
        (0_u16, 3_u16, format4.as_slice()),
        (3_u16, 1_u16, format4.as_slice()),
        (0_u16, 4_u16, format12_subtable),
        (3_u16, 10_u16, format12_subtable),
    ];
    let header_length = 4 + records.len() * 8;
    let mut output =
        Vec::with_capacity(header_length + format4.len() * 2 + format12_subtable.len() * 2);
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(records.len() as u16).to_be_bytes());
    let mut offset = header_length;
    for (platform, encoding, data) in records {
        output.extend_from_slice(&platform.to_be_bytes());
        output.extend_from_slice(&encoding.to_be_bytes());
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, _, data) in records {
        output.extend_from_slice(data);
    }
    output
}

fn build_cmap_with_variations(
    mapping: &BTreeMap<u32, u16>,
    variations: &[UnicodeVariationSequence],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Vec<u8> {
    let format4 = build_cmap_format4(mapping);
    let format12 = build_cmap_format12(mapping);
    let format12_subtable = &format12[12..];
    let format14 = build_cmap_format14(variations, glyph_ids);
    let subtable_count = 4 + 2 * usize::from(format14.is_some());
    let header_length = 4 + subtable_count * 8;
    let mut records = vec![
        (0_u16, 3_u16, format4.as_slice()),
        (3_u16, 1_u16, format4.as_slice()),
        (0_u16, 4_u16, format12_subtable),
        (3_u16, 10_u16, format12_subtable),
    ];
    if let Some(format14) = format14.as_ref() {
        records.push((0_u16, 5_u16, format14.as_slice()));
        records.push((3_u16, 10_u16, format14.as_slice()));
    }
    let mut output = Vec::with_capacity(
        header_length + records.iter().map(|(_, _, data)| data.len()).sum::<usize>(),
    );
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(subtable_count as u16).to_be_bytes());
    let mut offset = header_length;
    for (platform, encoding, data) in &records {
        output.extend_from_slice(&platform.to_be_bytes());
        output.extend_from_slice(&encoding.to_be_bytes());
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, _, data) in records {
        output.extend_from_slice(data);
    }
    output
}

fn build_cmap_format14(
    variations: &[UnicodeVariationSequence],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    let mut by_selector = BTreeMap::<u32, BTreeMap<u32, u16>>::new();
    for variation in variations {
        if variation.base > 0x10FFFF || variation.selector > 0xFFFFFF {
            continue;
        }
        let Some(&glyph_id) = glyph_ids.get(variation.glyph.as_str()) else {
            continue;
        };
        by_selector
            .entry(variation.selector)
            .or_default()
            .insert(variation.base, glyph_id);
    }
    if by_selector.is_empty() {
        return None;
    }
    let records_length = by_selector.len() * 11;
    let header_length = 10 + records_length;
    let mut records = Vec::with_capacity(records_length);
    let mut payload = Vec::new();
    for (selector, mappings) in by_selector {
        let offset = header_length + payload.len();
        records.extend_from_slice(&selector.to_be_bytes()[1..]);
        records.extend_from_slice(&0_u32.to_be_bytes());
        records.extend_from_slice(&(offset as u32).to_be_bytes());
        payload.extend_from_slice(&(mappings.len() as u32).to_be_bytes());
        for (base, glyph_id) in mappings {
            payload.extend_from_slice(&base.to_be_bytes()[1..]);
            payload.extend_from_slice(&glyph_id.to_be_bytes());
        }
    }
    let length = header_length + payload.len();
    let mut output = Vec::with_capacity(length);
    output.extend_from_slice(&14_u16.to_be_bytes());
    output.extend_from_slice(&(length as u32).to_be_bytes());
    output.extend_from_slice(&(by_selector_count(&records) as u32).to_be_bytes());
    output.extend_from_slice(&records);
    output.extend_from_slice(&payload);
    Some(output)
}

fn by_selector_count(records: &[u8]) -> usize {
    records.len() / 11
}

fn build_cmap_format4(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let bmp = mapping
        .iter()
        .filter_map(|(&codepoint, &glyph_id)| {
            (codepoint <= 0xFFFF).then_some((codepoint as u16, glyph_id))
        })
        .collect::<Vec<_>>();
    let segment_count = bmp.len() + 1;
    let search_power = 1_u16 << (15 - (segment_count as u16).leading_zeros());
    let search_range = search_power * 2;
    let entry_selector = (15 - search_power.leading_zeros()) as u16;
    let range_shift = (segment_count as u16) * 2 - search_range;
    let length = 16 + segment_count * 8 + bmp.len() * 2;
    let mut output = Vec::with_capacity(length);
    output.extend_from_slice(&4_u16.to_be_bytes());
    output.extend_from_slice(&(length as u16).to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&((segment_count * 2) as u16).to_be_bytes());
    output.extend_from_slice(&search_range.to_be_bytes());
    output.extend_from_slice(&entry_selector.to_be_bytes());
    output.extend_from_slice(&range_shift.to_be_bytes());
    for &(codepoint, _) in &bmp {
        output.extend_from_slice(&codepoint.to_be_bytes());
    }
    output.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    for &(codepoint, _) in &bmp {
        output.extend_from_slice(&codepoint.to_be_bytes());
    }
    output.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    for _ in &bmp {
        output.extend_from_slice(&0_u16.to_be_bytes());
    }
    output.extend_from_slice(&1_u16.to_be_bytes());
    let glyph_array_start = 16 + segment_count * 8;
    for (index, _) in bmp.iter().enumerate() {
        let id_range_word =
            14 + segment_count * 2 + 2 + segment_count * 2 + segment_count * 2 + index * 2;
        let offset = glyph_array_start + index * 2 - id_range_word;
        output.extend_from_slice(&(offset as u16).to_be_bytes());
    }
    output.extend_from_slice(&0_u16.to_be_bytes());
    for (_, glyph_id) in bmp {
        output.extend_from_slice(&glyph_id.to_be_bytes());
    }
    output
}

fn build_stat_table_with_values(
    axes: &[([u8; 4], u16)],
    values: &[Vec<f32>],
    value_name_ids: &[u16],
) -> Vec<u8> {
    let axis_value_count = values.len().min(value_name_ids.len());
    let offsets_start = 20 + axes.len() * 8;
    let values_start = offsets_start + axis_value_count * 2;
    let mut axis_value_tables = Vec::new();
    let mut offsets = Vec::with_capacity(axis_value_count);
    for (coordinates, value_name_id) in values.iter().zip(value_name_ids).take(axis_value_count) {
        let mut record = Vec::with_capacity(8 + axes.len() * 4);
        record.extend_from_slice(&4_u16.to_be_bytes());
        record.extend_from_slice(&(axes.len() as u16).to_be_bytes());
        record.extend_from_slice(&0_u16.to_be_bytes());
        record.extend_from_slice(&value_name_id.to_be_bytes());
        for (axis_index, coordinate) in coordinates.iter().enumerate().take(axes.len()) {
            record.extend_from_slice(&(axis_index as u16).to_be_bytes());
            let fixed = (*coordinate * 65536.0).round() as i32;
            record.extend_from_slice(&fixed.to_be_bytes());
        }
        offsets.push((values_start + axis_value_tables.len()) as u16);
        axis_value_tables.extend(record);
    }
    let mut table = Vec::with_capacity(values_start + axis_value_tables.len());
    table.extend_from_slice(&0x0001_0002_u32.to_be_bytes());
    table.extend_from_slice(&8_u16.to_be_bytes());
    table.extend_from_slice(&(axes.len() as u16).to_be_bytes());
    table.extend_from_slice(&20_u32.to_be_bytes());
    table.extend_from_slice(&(axis_value_count as u16).to_be_bytes());
    table.extend_from_slice(&(offsets_start as u32).to_be_bytes());
    table.extend_from_slice(&2_u16.to_be_bytes());
    for (tag, name_id) in axes {
        table.extend_from_slice(tag);
        table.extend_from_slice(&name_id.to_be_bytes());
        table.extend_from_slice(&0_u16.to_be_bytes());
    }
    for offset in offsets {
        table.extend_from_slice(&offset.to_be_bytes());
    }
    table.extend_from_slice(&axis_value_tables);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_data::{
        Contour, ContourPoint, FontInstance, FontMaster, FontMetadata, GlyphAnchor, GlyphComponent,
        GlyphData, GlyphLayer,
    };
    use read_fonts::{FontRead, TableProvider};
    use std::collections::HashMap;

    #[test]
    fn feature_parameters_cover_stylistic_set_character_variant_and_size() {
        let stylistic = feature_params_for_tag(Tag::new(b"ss03"), "", &BTreeMap::new());
        assert!(matches!(
            stylistic,
            Some(layout::FeatureParams::StylisticSet(_))
        ));
        let character = feature_params_for_tag(Tag::new(b"cv07"), "", &BTreeMap::new());
        assert!(matches!(
            character,
            Some(layout::FeatureParams::CharacterVariant(_))
        ));
        let character = feature_params_for_tag(
            Tag::new(b"cv07"),
            "feature cv07 { sub A by A.cv07; } cv07;",
            &BTreeMap::from([("A".to_string(), 0x41)]),
        );
        let Some(layout::FeatureParams::CharacterVariant(character)) = character else {
            panic!("character variant parameters should be parsed");
        };
        assert_eq!(character.character, vec![Uint24::new(0x41)]);
        let size = feature_params_for_tag(
            Tag::new(b"size"),
            "feature size { parameters 12 2 8 72; } size;",
            &BTreeMap::new(),
        );
        let Some(layout::FeatureParams::Size(size)) = size else {
            panic!("size feature parameters should be parsed");
        };
        assert_eq!(size.design_size, 12);
        assert_eq!(size.identifier, 2);
        assert_eq!(size.range_start, 8);
        assert_eq!(size.range_end, 72);
    }

    #[test]
    fn feature_names_override_default_stylistic_set_label() {
        let source = r#"
            feature ss01 {
                featureNames {
                    name "Handwritten Alternates";
                    name 3 1 0x409 "Localized Alternates";
                };
                sub A by A.alt;
            } ss01;
        "#;
        let records = feature_name_records(source, "ss01", 500);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].nameID, 500);
        assert_eq!(records[0].string, "Handwritten Alternates");
        assert_eq!(records[1].platformID, 3);
        assert_eq!(records[1].languageID, 0x409);
        assert_eq!(records[1].string, "Localized Alternates");
    }

    #[test]
    fn exported_ttf_contains_horizontal_and_vertical_base_axes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let path =
            std::env::temp_dir().join(format!("glyph-studio-base-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        let base = font.base().unwrap();
        for axis in [base.horiz_axis(), base.vert_axis()] {
            let axis = axis.unwrap().unwrap();
            let tags = axis.base_tag_list().unwrap().unwrap().baseline_tags();
            assert_eq!(
                tags,
                &[
                    Tag::new(b"hang"),
                    Tag::new(b"ideo"),
                    Tag::new(b"math"),
                    Tag::new(b"romn"),
                ]
            );
            assert_eq!(
                axis.base_script_list().unwrap().base_script_records().len(),
                5
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exported_ttf_round_trips_unmodelled_opentype_tables() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let payload = vec![0, 1, 2, 3, 4, 5];
        project
            .preserved_tables
            .insert("MATH".into(), payload.clone());
        let base_payload = vec![0, 1, 2, 3];
        project
            .preserved_tables
            .insert("BASE".into(), base_payload.clone());
        let colr_payload = vec![0, 1, 0, 0, 0, 0];
        project
            .preserved_tables
            .insert("COLR".into(), colr_payload.clone());
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-preserved-table-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        assert_eq!(
            font.table_data(Tag::new(b"MATH")).unwrap().as_bytes(),
            payload
        );
        assert_eq!(
            font.table_data(Tag::new(b"BASE")).unwrap().as_bytes(),
            base_payload
        );
        assert_eq!(
            font.table_data(Tag::new(b"COLR")).unwrap().as_bytes(),
            colr_payload
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn preserved_layout_table_is_used_when_no_source_replacement_exists() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let payload = vec![0, 1, 0, 0, 0, 0, 0, 0];
        project
            .preserved_tables
            .insert("GSUB".into(), payload.clone());
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-preserved-gsub-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        assert_eq!(
            font.table_data(Tag::new(b"GSUB")).unwrap().as_bytes(),
            payload
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn feature_glyph_class_definition_overrides_inferred_gdef_class() {
        let glyph_ids = HashMap::from([("A", 1), ("f_i", 2), ("acute", 3), ("part", 4)]);
        let classes = parse_feature_glyph_classes(
            "table GDEF { GlyphClassDef [A], [f_i], [acute], [part]; } GDEF;",
            &glyph_ids,
        );
        assert_eq!(classes[&GlyphId16::new(1)], gdef::GlyphClassDef::Base);
        assert_eq!(classes[&GlyphId16::new(2)], gdef::GlyphClassDef::Ligature);
        assert_eq!(classes[&GlyphId16::new(3)], gdef::GlyphClassDef::Mark);
        assert_eq!(classes[&GlyphId16::new(4)], gdef::GlyphClassDef::Component);
    }

    #[test]
    fn simple_gsub_supports_alternate_substitution() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub(
            "feature salt { sub A from [A.alt A.swash]; } salt;",
            &glyph_ids,
        );
        assert!(bytes.is_some());
        let bytes = bytes.unwrap();
        assert!(bytes.len() > 20);
        assert!(bytes.windows(4).any(|window| window == b"salt"));
    }

    #[test]
    fn use_extension_wraps_gsub_and_gpos_lookups() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub(
            "feature salt { lookupflag useExtension; sub A by A.alt; } salt;",
            &glyph_ids,
        )
        .expect("useExtension GSUB should compile");
        assert!(gsub.windows(2).any(|window| window == [0, 7]));

        let project = FontProject::new();
        let gpos = build_kerning_gpos(
            &project,
            &glyph_ids,
            "feature kern { useExtension; pos A V -80; } kern;",
        )
        .expect("useExtension GPOS should compile");
        assert!(gpos.windows(2).any(|window| window == [0, 9]));
    }

    #[test]
    fn feature_source_accepts_enumerated_substitution_and_positioning() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub("feature salt { enum sub A by A.alt; } salt;", &glyph_ids)
            .expect("enum sub should compile as an enumerated substitution");
        assert!(gsub.windows(4).any(|window| window == b"salt"));
        let enumerate = build_simple_gsub(
            "feature salt { enumerate sub A by A.alt; } salt;",
            &glyph_ids,
        )
        .expect("enumerate sub should compile as an enumerated substitution");
        assert!(enumerate.windows(4).any(|window| window == b"salt"));

        let project = FontProject::new();
        let gpos = build_kerning_gpos(
            &project,
            &glyph_ids,
            "feature kern { enum pos A V <0 0 -80 0>; } kern;",
        )
        .expect("enum pos should compile as an enumerated positioning rule");
        assert!(gpos.windows(4).any(|window| window == b"kern"));
    }

    #[test]
    fn named_lookup_references_are_expanded_into_the_feature() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "lookup stylisticA { sub A by A.alt; } stylisticA;\nfeature salt { lookup stylisticA; } salt;";
        let expanded = expand_named_feature_lookups(source);
        assert!(expanded.contains("sub A by A.alt;"));
        let bytes = build_simple_gsub(source, &glyph_ids).expect("named lookup should compile");
        assert!(bytes.windows(4).any(|window| window == b"salt"));
    }

    #[test]
    fn feature_references_share_gsub_lookups() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "feature dlig { sub A by A.alt; } dlig; feature liga { feature dlig; } liga;";
        let bytes =
            build_simple_gsub(source, &glyph_ids).expect("feature reference should compile");
        assert!(bytes.windows(4).any(|window| window == b"dlig"));
        assert!(bytes.windows(4).any(|window| window == b"liga"));
    }

    #[test]
    fn nested_feature_references_reach_a_transitive_parent() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "feature dlig { sub A by A.alt; } dlig; feature liga { feature dlig; } liga; feature calt { feature liga; } calt;";
        let bytes = build_simple_gsub(source, &glyph_ids)
            .expect("nested feature references should compile");
        assert!(bytes.windows(4).any(|window| window == b"dlig"));
        assert!(bytes.windows(4).any(|window| window == b"liga"));
        assert!(bytes.windows(4).any(|window| window == b"calt"));
    }

    #[test]
    fn named_lookup_references_are_expanded_into_gpos() {
        let project = FontProject::new();
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("V", 2)]);
        let source = "lookup pairAdjust { pos A V <0 0 -80 0>; } pairAdjust;\nfeature kern { lookup pairAdjust; } kern;";
        let bytes = build_kerning_gpos(&project, &glyph_ids, source)
            .expect("named GPOS lookup should compile");
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_references_share_gpos_lookups() {
        let project = FontProject::new();
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("V", 2)]);
        let source = "feature krn2 { pos A V -80; } krn2; feature kern { feature krn2; } kern;";
        let bytes = build_kerning_gpos(&project, &glyph_ids, source)
            .expect("GPOS feature reference should compile");
        assert!(bytes.windows(4).any(|window| window == b"krn2"));
        assert!(bytes.windows(4).any(|window| window == b"kern"));
    }

    #[test]
    fn gsub_feature_variations_emit_version_11_for_conditional_substitution() {
        let glyph_ids = HashMap::from([("A", 1_u16), ("A.cond", 2)]);
        let substitutions = vec![ConditionalSubstitution {
            base: "A".into(),
            alternate: "A.cond".into(),
            conditions: HashMap::from([(
                "WGHT".into(),
                crate::font_data::AxisRange {
                    min: Some(700.0),
                    max: None,
                },
            )]),
        }];
        let bounds = HashMap::from([(String::from("wght"), (0, 400.0, 400.0, 700.0))]);
        let bytes = build_simple_gsub_with_variations("", &glyph_ids, &substitutions, &bounds)
            .expect("conditional substitution should produce GSUB");
        assert_eq!(&bytes[..4], &[0, 1, 0, 1]);
        assert!(bytes.windows(4).any(|window| window == b"rvrn"));
    }

    #[test]
    fn conditional_axis_bounds_follow_fvar_axis_order() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut bold = FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 110.0,
            ..FontMaster::default()
        };
        bold.axes.insert("opsz".into(), 14.0);
        project.masters[0].axes.insert("opsz".into(), 10.0);
        project.masters.push(bold);
        project.default_master_id = "bold".into();
        let layer = GlyphLayer {
            width: 600.0,
            contours: Vec::new(),
            components: Vec::new(),
            anchors: Vec::new(),
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![
                crate::font_data::ConditionalLayer {
                    id: "wide".into(),
                    conditions: HashMap::from([(
                        "wght".into(),
                        crate::font_data::AxisRange {
                            min: Some(600.0),
                            max: Some(900.0),
                        },
                    )]),
                    layer: layer.clone(),
                },
                crate::font_data::ConditionalLayer {
                    id: "narrow".into(),
                    conditions: HashMap::from([(
                        "wght".into(),
                        crate::font_data::AxisRange {
                            min: Some(700.0),
                            max: Some(800.0),
                        },
                    )]),
                    layer,
                },
            ],
        );
        project.add_glyph(".cond.A.narrow-1".into(), None);
        let (substitutions, bounds) = materialize_conditional_substitutions(&mut project);
        assert_eq!(bounds["opsz"].0, 0);
        assert_eq!(bounds["wdth"].0, 1);
        assert_eq!(bounds["opsz"].2, 14.0);
        assert!(substitutions[0].alternate.contains("narrow"));
        assert_ne!(substitutions[0].alternate, ".cond.A.narrow-1");
        let alternate = project.glyphs.get(&substitutions[0].alternate).unwrap();
        assert!(alternate.unicode.is_none());
        assert!(alternate.unicodes.is_empty());
    }

    #[test]
    fn simple_gsub_supports_one_to_one_class_substitution() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2), ("A.alt", 3), ("B.alt", 4)]);
        let bytes = build_simple_gsub("feature ss01 { sub [A B] by [A.alt B.alt]; } ss01;", &ids)
            .expect("class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn feature_classes_accept_optional_commas() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2), ("A.alt", 3), ("B.alt", 4)]);
        let bytes = build_simple_gsub("feature ss01 { sub [A, B] by [A.alt, B.alt]; } ss01;", &ids)
            .expect("comma-separated class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn feature_file_accepts_long_form_substitute_and_position_keywords() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub("feature salt { substitute A by A.alt; } salt;", &ids)
            .expect("long-form substitute should produce GSUB");
        assert!(!gsub.is_empty());
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.add_glyph("V".into(), Some(0x56));
        project.kerning.insert(("A".into(), "V".into()), -50.0);
        let gpos = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { position A V < -50 0 0 0 >; } kern;",
        )
        .expect("long-form position should produce GPOS");
        assert!(!gpos.is_empty());
    }

    #[test]
    fn simple_gsub_synthesizes_aalt_from_feature_alternates() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub(
            "feature salt { sub A by A.alt; } salt; feature ss01 { sub A by A.swash; } ss01;",
            &ids,
        )
        .expect("automatic aalt should produce GSUB");
        assert!(bytes.windows(4).any(|window| window == b"aalt"));
    }

    #[test]
    fn simple_gsub_compiles_ignore_substitution_rules() {
        let ids = HashMap::from([("A", 1_u16), ("acute", 2), ("Aacute", 3)]);
        let bytes = build_simple_gsub(
            "feature ccmp { ignore sub A acute; sub A acute by Aacute; } ccmp;",
            &ids,
        )
        .expect("ignore substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_compiles_null_substitution_as_deletion() {
        let ids = HashMap::from([("A", 1_u16)]);
        let bytes = build_simple_gsub("feature ccmp { sub A by NULL; } ccmp;", &ids)
            .expect("NULL substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_supports_ligature_substitution() {
        let ids = HashMap::from([("f", 1_u16), ("i", 2), ("f_i", 3)]);
        let bytes = build_simple_gsub("feature liga { sub f i by f_i; } liga;", &ids)
            .expect("ligature substitution should produce GSUB");
        assert!(!bytes.is_empty());
        assert!(bytes.windows(4).any(|window| window == b"liga"));
    }

    #[test]
    fn simple_gsub_supports_reverse_chain_substitution() {
        let ids = [("A", 1), ("A.alt", 2), ("B", 3), ("C", 4)]
            .into_iter()
            .collect();
        let bytes =
            build_simple_gsub("feature rvrn { reversesub B A' C by A.alt; } rvrn;", &ids).unwrap();
        assert!(bytes.len() > 40);
        let shorthand =
            build_simple_gsub("feature rvrn { rsub B A' C by A.alt; } rvrn;", &ids).unwrap();
        assert_eq!(bytes, shorthand);
    }

    #[test]
    fn simple_gsub_supports_bracketed_multiple_replacements() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub("feature salt { sub A by [A.alt A.swash]; } salt;", &ids)
            .expect("bracketed multiple substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_ignores_unknown_rules_without_dropping_valid_rules() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2)]);
        let bytes = build_simple_gsub("feature liga { sub missing by B; sub A by B; } liga;", &ids)
            .expect("valid rules should still produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_preserves_multiple_feature_tags() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2)]);
        let bytes = build_simple_gsub(
            "feature liga { sub A by B; } liga; feature calt { sub B by A; } calt;",
            &ids,
        )
        .expect("multiple feature tags should produce GSUB");
        assert!(bytes.windows(4).any(|window| window == b"liga"));
        assert!(bytes.windows(4).any(|window| window == b"calt"));
    }

    #[test]
    fn simple_gsub_expands_named_feature_classes() {
        let ids = [
            ("A", 1_u16),
            ("B", 2_u16),
            ("A.alt", 3_u16),
            ("B.alt", 4_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub(
            "@caps = [A B]; feature salt { sub @caps by [A.alt B.alt]; } salt;",
            &ids,
        )
        .expect("named class substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_expands_named_classes_in_context_replacements() {
        let ids = [
            ("A", 1_u16),
            ("C", 2_u16),
            ("D", 3_u16),
            ("C.alt", 4_u16),
            ("D.alt", 5_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub(
            "@ctx = [C D]; @alts = [C.alt D.alt]; feature calt { sub A @ctx' by @alts; } calt;",
            &ids,
        )
        .expect("named contextual replacement classes should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_accepts_marked_single_substitution_syntax() {
        let ids = [("A", 1_u16), ("A.alt", 2_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature salt { sub A' by A.alt; } salt;", &ids)
            .expect("marked single substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_accepts_contextual_marked_substitution_syntax() {
        let ids = [("A", 1_u16), ("B", 2_u16), ("A.alt", 3_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A' B by A.alt; } calt;", &ids)
            .expect("contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_accepts_context_on_both_sides_of_target() {
        let ids = [("A", 1_u16), ("B", 2_u16), ("C", 3_u16), ("B.alt", 4_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A B' C by B.alt; } calt;", &ids)
            .expect("two-sided contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_expands_contextual_class_sequences() {
        let ids = [("A", 1_u16), ("A.alt", 2_u16), ("B", 3_u16), ("C", 4_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub [A B] C' by A.alt; } calt;", &ids)
            .expect("class contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_expands_class_at_contextual_target() {
        let ids = [
            ("A", 1_u16),
            ("B", 2_u16),
            ("C", 3_u16),
            ("D", 4_u16),
            ("E", 5_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub("feature calt { sub A [C D]' by E; } calt;", &ids)
            .expect("class target contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn simple_gsub_pairs_contextual_target_and_replacement_classes() {
        let ids = [
            ("A", 1_u16),
            ("C", 2_u16),
            ("D", 3_u16),
            ("C.alt", 4_u16),
            ("D.alt", 5_u16),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_simple_gsub(
            "feature calt { sub A [C D]' by [C.alt D.alt]; } calt;",
            &ids,
        )
        .expect("class replacement contextual substitution should produce GSUB");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn feature_blocks_are_extracted_with_nested_brace_boundaries() {
        let blocks = extract_feature_blocks(
            "feature liga { lookup L { sub f i by f_i; } L; } liga;\nfeature salt { sub A by A.swash; } salt;",
        );
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
        assert!(blocks[0].1.contains("sub f i by f_i"));
        assert_eq!(blocks[1].0, Tag::new(b"salt"));
    }

    #[test]
    fn feature_block_extraction_ignores_comment_text() {
        let blocks = extract_feature_blocks(
            "# feature bad { } bad;\nfeature liga { # } feature fake {\n sub f i by fi;\n} liga;",
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
        assert!(blocks[0].1.contains("sub f i by fi"));
    }

    #[test]
    fn feature_block_extraction_requires_identifier_boundaries() {
        let blocks = extract_feature_blocks(
            "myfeature liga { sub A by B; } liga;\nfeature liga { sub A by B; } liga;",
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, Tag::new(b"liga"));
    }

    #[test]
    fn invalid_multibyte_feature_tag_does_not_panic() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("B", 2)]);
        let result = std::panic::catch_unwind(|| {
            build_simple_gsub("feature あいうえ { sub A by B; } あいうえ;", &glyph_ids)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn postscript_name_is_ascii_safe() {
        assert_eq!(postscript_name("My Font!", "Regular"), "MyFont-Regular");
        assert_eq!(postscript_name("日本語", "標準"), "Font-Font");
        assert!(postscript_name(&"A".repeat(100), "Regular").len() <= 63);
        assert!(!postscript_name("---", "Regular").starts_with('-'));
    }

    #[test]
    fn style_flags_follow_weight_and_italic_metadata() {
        let mut metadata = FontMetadata::default();
        assert_eq!(mac_style_flags(&metadata), 0);
        assert_eq!(os2_selection_flags(&metadata), 0x1C0);
        metadata.weight_class = 700;
        metadata.style_name = "Bold Italic".into();
        metadata.italic_angle = -12.0;
        assert_eq!(mac_style_flags(&metadata), 3);
        assert_eq!(os2_selection_flags(&metadata), 0x1A1);
        assert_eq!(
            max_feature_context("feature liga { sub f i j by f_i_j; } liga;"),
            3
        );
    }

    #[test]
    fn avar_contains_normalized_axis_mapping_and_identity_axes() {
        let tags = vec!["wght".into(), "wdth".into()];
        let mappings = std::collections::HashMap::from([(
            "wght".into(),
            vec![AxisMappingPoint {
                input: 0.5,
                output: 0.25,
            }],
        )]);
        let bytes = build_avar(&tags, &mappings).expect("nonlinear mapping should emit avar");
        assert_eq!(&bytes[..8], &[0, 1, 0, 0, 0, 0, 0, 2]);
        assert_eq!(u16::from_be_bytes([bytes[8], bytes[9]]), 4);
        assert_eq!(u16::from_be_bytes([bytes[26], bytes[27]]), 3);
    }

    #[test]
    fn cmap_format12_preserves_supplementary_codepoints() {
        let mapping = BTreeMap::from([(0x1F600, 4_u16), (0x1F601, 5_u16)]);
        let bytes = build_cmap_format12(&mapping);
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 3);
        assert_eq!(u16::from_be_bytes([bytes[6], bytes[7]]), 10);
        assert_eq!(u16::from_be_bytes([bytes[0], bytes[1]]), 0);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 1);
        assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 12);
        assert_eq!(
            u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            0x1F600
        );
    }

    #[test]
    fn os2_unicode_ranges_follow_mapped_blocks() {
        let mapping = BTreeMap::from([(65, 1_u16), (0x3042, 2_u16), (0x1F600, 3_u16)]);
        let (range1, range2, range3, range4) = unicode_range_bits(&mapping);
        assert_ne!(range1 & (1 << 0), 0); // Basic Latin
        assert_ne!(range2 & (1 << (49 - 32)), 0); // Hiragana
        assert_eq!(range3 | range4, 0);
        let (code_pages1, _) = code_page_range_bits(&mapping);
        assert_ne!(code_pages1 & (1 << 0), 0); // Windows Latin 1
        assert_ne!(code_pages1 & (1 << 17), 0); // Japanese
    }

    #[test]
    fn vorg_contains_non_default_vertical_origins() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours = vec![Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 900.0),
            ],
        }];
        let vorg = build_vorg(&project, "regular").unwrap();
        assert_eq!(&vorg[0..4], &[0, 1, 0, 0]);
        assert_eq!(u16::from_be_bytes([vorg[6], vorg[7]]), 1);
        assert_eq!(u16::from_be_bytes([vorg[8], vorg[9]]), 1);
    }

    #[test]
    fn combined_cmap_keeps_bmp_and_supplementary_subtables() {
        let mapping = BTreeMap::from([(65, 2_u16), (0x1F600, 4_u16)]);
        let bytes = build_cmap_with_bmp_and_full_unicode(&mapping);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 4);
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 0);
        let format4_offset =
            u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let format12_offset =
            u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;
        assert_eq!(
            u16::from_be_bytes([bytes[format4_offset], bytes[format4_offset + 1]]),
            4
        );
        assert_eq!(
            u16::from_be_bytes([bytes[format12_offset], bytes[format12_offset + 1]]),
            12
        );
    }

    #[test]
    fn cmap_format14_contains_unicode_variation_sequence_mappings() {
        let mapping = BTreeMap::from([(0x4E00, 1_u16)]);
        let variations = vec![UnicodeVariationSequence {
            base: 0x4E00,
            selector: 0xE0100,
            glyph: "A.ivs".into(),
        }];
        let glyph_ids = HashMap::from([("A.ivs", 2_u16)]);
        let bytes = build_cmap_with_variations(&mapping, &variations, &glyph_ids);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 6);
        let format14_offset =
            u32::from_be_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
        assert_eq!(
            u16::from_be_bytes([bytes[format14_offset], bytes[format14_offset + 1]]),
            14
        );
        assert_eq!(
            u32::from_be_bytes([
                0,
                bytes[format14_offset + 10],
                bytes[format14_offset + 11],
                bytes[format14_offset + 12],
            ]),
            0xE0100
        );
    }

    #[test]
    fn exported_ttf_resolves_bmp_and_supplementary_unicode_together() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("grinning".into(), Some('😀' as u32));
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mixed-unicode-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
        assert!(face.glyph_index('A').is_some());
        assert!(face.glyph_index('😀').is_some());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exports_a_readable_ttf_with_outline_tables() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(500.0, 0.0),
                ContourPoint::on_curve(500.0, 700.0),
                ContourPoint::on_curve(0.0, 700.0),
            ],
        });
        let base_layer = GlyphLayer {
            width: glyph.width,
            contours: glyph.contours.clone(),
            components: glyph.components.clone(),
            anchors: glyph.anchors.clone(),
        };
        let mut target_layer = base_layer.clone();
        target_layer.width = 650.0;
        target_layer.contours[0].points[1].x = 550.0;
        glyph.layers.insert("regular".into(), base_layer);
        glyph.layers.insert("bold".into(), target_layer);
        project.glyphs.insert("A".into(), glyph);
        let mut b_glyph = GlyphData::new("B".into(), Some('B' as u32));
        b_glyph.width = 500.0;
        project.glyphs.insert("B".into(), b_glyph);
        project.opentype_features = "feature liga { sub A by B; } liga;".into();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 110.0,
            ..FontMaster::default()
        });
        project.instances.push(FontInstance {
            name: "Text Medium".into(),
            axes: HashMap::new(),
            weight: 550.0,
            width: 105.0,
        });
        project.kerning.insert(("A".into(), "A".into()), -50.0);
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(font.tables.contains_key(b"cmap"));
        assert!(font.tables.contains_key(b"hmtx"));
        assert!(font.tables.contains_key(b"kern"));
        assert!(font.tables.contains_key(b"GPOS"));
        assert!(font.tables.contains_key(b"GDEF"));
        assert!(font.tables.contains_key(b"OS/2"));
        assert!(font.tables.contains_key(b"fvar"));
        assert!(font.tables.contains_key(b"gvar"));
        let Some(fonttools::font::Table::Unknown(fvar_bytes)) = font.tables.get(b"fvar") else {
            panic!("fvar table was unexpectedly parsed");
        };
        assert_eq!(u16::from_be_bytes([fvar_bytes[12], fvar_bytes[13]]), 1);
        assert_eq!(u16::from_be_bytes([fvar_bytes[14], fvar_bytes[15]]), 12);
        let instance_offset = 16 + 40;
        assert_eq!(
            u16::from_be_bytes([fvar_bytes[instance_offset], fvar_bytes[instance_offset + 1]]),
            400
        );
        let weight = i32::from_be_bytes([
            fvar_bytes[instance_offset + 4],
            fvar_bytes[instance_offset + 5],
            fvar_bytes[instance_offset + 6],
            fvar_bytes[instance_offset + 7],
        ]) as f32
            / 65536.0;
        assert!((weight - 550.0).abs() < 0.01);
        let Some(fonttools::font::Table::Unknown(stat_bytes)) = font.tables.get(b"STAT") else {
            panic!("STAT table is missing");
        };
        assert!(stat_bytes.windows(2).any(|window| window == [1, 144]));
        assert!(font.tables.contains_key(b"GSUB"));
        assert!(font.tables.contains_key(b"name"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exported_variable_ttf_contains_conditional_gsub_variations() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let layer = GlyphLayer {
            width: 600.0,
            contours: vec![Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        glyph.layers.insert("regular".into(), layer.clone());
        glyph.layers.insert("bold".into(), layer.clone());
        project.glyphs.insert("A".into(), glyph);
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        project.conditional_layers.insert(
            "A".into(),
            vec![crate::font_data::ConditionalLayer {
                id: "bold".into(),
                conditions: HashMap::from([(
                    "wght".into(),
                    crate::font_data::AxisRange {
                        min: Some(700.0),
                        max: None,
                    },
                )]),
                layer,
            }],
        );
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-conditional-gsub-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        let fonttools::font::Table::Unknown(gsub) = font.tables.get(b"GSUB").unwrap() else {
            panic!("GSUB should be serialized as raw bytes");
        };
        assert_eq!(&gsub[..4], &[0, 1, 0, 1]);
        assert!(gsub.windows(4).any(|window| window == b"rvrn"));
        let shape = |variation: &str| {
            std::process::Command::new("hb-shape")
                .arg(&path)
                .arg("A")
                .arg(format!("--variations={variation}"))
                .output()
                .expect("HarfBuzz should be available for variable-font verification")
        };
        let regular = shape("wght=400");
        let bold = shape("wght=700");
        assert!(regular.status.success());
        assert!(bold.status.success());
        assert_ne!(regular.stdout, bold.stdout);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn project_validation_finds_component_cycles_and_invalid_geometry() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.components.push(GlyphComponent {
            base: "B".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        let mut b = GlyphData::new("B".into(), None);
        b.components.push(GlyphComponent {
            base: "A".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        a.width = f64::NAN;
        a.anchors.push(GlyphAnchor {
            name: String::new(),
            x: f64::NAN,
            y: 0.0,
        });
        project.glyphs.insert("A".into(), a);
        project.glyphs.insert("B".into(), b);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("循環参照")));
        assert!(issues.iter().any(|issue| issue.contains("幅が不正")));
        assert!(issues.iter().any(|issue| issue.contains("アンカー")));
    }

    #[test]
    fn project_validation_rejects_invalid_background_opacity() {
        let mut project = FontProject::new();
        project
            .background_opacities
            .entry("A".into())
            .or_default()
            .insert("regular".into(), 1.5);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("背景画像不透明度")));
    }

    #[test]
    fn project_validation_rejects_invalid_background_transform() {
        let mut project = FontProject::new();
        project
            .background_transforms
            .entry("A".into())
            .or_default()
            .insert(
                "regular".into(),
                crate::font_data::BackgroundImageTransform {
                    x: 0.0,
                    y: 0.0,
                    scale: 0.0,
                    rotation: f32::NAN,
                    flip_x: false,
                    flip_y: false,
                },
            );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("背景画像変形")));
    }

    #[test]
    fn project_validation_reports_orphaned_background_references() {
        let mut project = FontProject::new();
        project
            .background_images
            .entry("Missing".into())
            .or_default()
            .insert("unknown-master".into(), "/tmp/ref.png".into());
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないグリフ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないマスター")));
    }

    #[test]
    fn project_validation_reports_invalid_axis_display_names() {
        let mut project = FontProject::new();
        project.masters[0].axes.insert("wght".into(), 400.0);
        project.axis_names.insert("wght".into(), "".into());
        project.axis_names.insert("wdth".into(), "Weight".into());
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("表示名が空")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しない軸タグ")));
    }

    #[test]
    fn project_validation_reports_invalid_conditional_layers() {
        let mut project = FontProject::new();
        project.conditional_layers.insert(
            "Missing".into(),
            vec![crate::font_data::ConditionalLayer {
                id: "alt".into(),
                conditions: std::collections::HashMap::from([(
                    "wght".into(),
                    crate::font_data::AxisRange {
                        min: Some(700.0),
                        max: Some(400.0),
                    },
                )]),
                layer: GlyphLayer {
                    width: 600.0,
                    contours: Vec::new(),
                    components: Vec::new(),
                    anchors: Vec::new(),
                },
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("条件レイヤーが存在しない")));
        assert!(issues.iter().any(|issue| issue.contains("軸範囲が不正")));
    }

    #[test]
    fn project_validation_rejects_invalid_guidelines() {
        let mut project = FontProject::new();
        project.guidelines.push(crate::font_data::Guideline {
            x: f64::NAN,
            y: 0.0,
            angle: 0.0,
            name: String::new(),
        });
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("ガイド")));
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.guidelines.push(crate::font_data::Guideline {
            x: 0.0,
            y: 0.0,
            angle: f64::INFINITY,
            name: String::new(),
        });
        project.glyphs.insert("A".into(), glyph);
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("グリフ 'A' のガイド")));
    }

    #[test]
    fn project_validation_rejects_invalid_font_metadata() {
        let mut project = FontProject::new();
        project.metadata.family_name.clear();
        project.metadata.style_name = "   ".into();
        project.metadata.units_per_em = 0.0;
        project.metadata.ascender = f64::NAN;
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("ファミリー名が空")));
        assert!(issues.iter().any(|issue| issue.contains("スタイル名が空")));
        assert!(issues.iter().any(|issue| issue.contains("UPMが")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("フォントメトリクス")));
    }

    #[test]
    fn project_validation_rejects_duplicate_anchor_names() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.anchors = vec![
            GlyphAnchor {
                name: "top".into(),
                x: 0.0,
                y: 700.0,
            },
            GlyphAnchor {
                name: " top ".into(),
                x: 10.0,
                y: 700.0,
            },
        ];
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("アンカー名 'top' が重複")));
    }

    #[test]
    fn project_validation_rejects_invalid_names_master_ids_and_layer_transforms() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: project.masters[0].id.clone(),
            ..FontMaster::default()
        });
        let mut glyph = GlyphData::new("different".into(), None);
        glyph.components.push(GlyphComponent {
            base: "different".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: f64::INFINITY,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: Vec::new(),
                components: vec![GlyphComponent {
                    base: "different".into(),
                    x_scale: f64::NAN,
                    xy_scale: 0.0,
                    yx_scale: 0.0,
                    y_scale: 1.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                }],
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("bad name".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("グリフ名が不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ 'bad name' のコンポーネント変換")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("マスターIDが重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ名の登録が不一致")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("コンポーネント変換が不正")));
    }

    #[test]
    fn project_validation_rejects_duplicate_and_unknown_glyph_order_entries() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyph_order = vec!["A".into(), "A".into(), "missing".into()];
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ順序に重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ順序に未定義")));
    }

    #[test]
    fn variable_master_kerning_emits_gpos_feature_variations() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("V".into(), Some('V' as u32));
        let mut bold = project.masters[0].clone();
        bold.id = "bold".into();
        bold.name = "Bold".into();
        bold.weight = 700.0;
        project.masters.push(bold);
        project.kerning.insert(("A".into(), "V".into()), -50.0);
        project.kerning_by_master.insert(
            "regular".into(),
            [(("A".into(), "V".into()), -50.0)].into_iter().collect(),
        );
        project.kerning_by_master.insert(
            "bold".into(),
            [(("A".into(), "V".into()), -100.0)].into_iter().collect(),
        );
        let glyph_ids = [("A", 1_u16), ("V", 2_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_kerning_gpos(&project, &glyph_ids, "").unwrap();
        assert_eq!(&bytes[..4], &[0, 1, 0, 1]);
    }

    #[test]
    fn kerning_groups_expand_into_gpos_pairs() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.left_kerning_group = "left".into();
        let mut a_alt = GlyphData::new("A.alt".into(), None);
        a_alt.left_kerning_group = "left".into();
        let mut v = GlyphData::new("V".into(), None);
        v.right_kerning_group = "right".into();
        let mut v_alt = GlyphData::new("V.alt".into(), None);
        v_alt.right_kerning_group = "right".into();
        project.glyphs.extend([
            ("A".into(), a),
            ("A.alt".into(), a_alt),
            ("V".into(), v),
            ("V.alt".into(), v_alt),
        ]);
        project.kerning.insert(("A".into(), "V".into()), -80.0);
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), -120.0);
        let ids = [("A", 1), ("A.alt", 2), ("V", 3), ("V.alt", 4)]
            .into_iter()
            .collect();
        let first = build_kerning_gpos(&project, &ids, "").unwrap();
        let second = build_kerning_gpos(&project, &ids, "").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn feature_source_positioning_compiles_single_pair_and_class_rules() {
        let project = FontProject::new();
        let ids = [("A", 1), ("A.alt", 2), ("V", 3), ("V.alt", 4)]
            .into_iter()
            .collect();
        let source = r#"
            feature kern { pos A V <0 0 -80 0>; } kern;
            feature mark { pos [A A.alt] <10 20 0 0>; } mark;
            feature calt { pos [A A.alt] [V V.alt] <0 0 -40 0>; } calt;
            feature ccmp { pos A' V <0 0 -20 0>; } ccmp;
            feature dist { pos A V' A <0 0 -30 0>; } dist;
            feature ss01 { pos A.alt <50>; } ss01;
            feature kern2 { pos A.alt V.alt <10 20> <-5 0>; } kern2;
        "#;
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
        assert!(bytes.windows(2).any(|window| window == [0, 7]));
        assert!(bytes.windows(2).any(|window| window == [0, 8]));
    }

    #[test]
    fn feature_source_accepts_short_pair_positioning_syntax() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let short = build_kerning_gpos(&project, &ids, "feature kern { pos A V -80; } kern;")
            .expect("short pair positioning should compile");
        let long = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { pos A V <0 0 -80 0>; } kern;",
        )
        .expect("ValueRecord pair positioning should compile");
        assert_eq!(short, long);
    }

    #[test]
    fn feature_source_expands_named_value_record_definitions() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let source =
            "valueRecordDef <0 0 -80 0> KERN_POS; feature kern { pos A V <KERN_POS>; } kern;";
        let bytes = build_kerning_gpos(&project, &ids, source)
            .expect("named value record should compile into GPOS");
        assert!(bytes.windows(4).any(|window| window == b"kern"));
        let named_single = build_kerning_gpos(
            &project,
            &ids,
            "valueRecordDef -80 KERN_POS; feature kern { pos A V <KERN_POS>; } kern;",
        )
        .expect("single-value named record should compile into GPOS");
        let shorthand = build_kerning_gpos(&project, &ids, "feature kern { pos A V -80; } kern;")
            .expect("short pair value should compile into GPOS");
        assert_eq!(named_single, shorthand);
    }

    #[test]
    fn feature_source_compiles_ignore_positioning_rules() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { ignore pos A V; pos A V <0 0 -80 0>; } kern;",
        )
        .expect("ignore positioning should compile");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn contextual_positioning_is_applied_by_harfbuzz_when_available() {
        if std::process::Command::new("hb-shape")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("V".into(), Some('V' as u32));
        project.opentype_features = "feature ccmp { pos A' V <0 0 -100 0>; } ccmp;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-context-pos-{}-{:?}.ttf",
            std::process::id(),
            std::thread::current().id()
        ));
        export_ttf(&project, &path).unwrap();
        let result = std::process::Command::new("hb-shape")
            .arg(&path)
            .arg("AV")
            .arg("--features=ccmp")
            .arg("--output-format=json")
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            result.status.success(),
            "HarfBuzz could not shape contextual GPOS: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let shaped = String::from_utf8_lossy(&result.stdout);
        assert!(
            shaped.contains("\"ax\":500"),
            "unexpected shaping: {shaped}"
        );
    }

    #[test]
    fn chained_contextual_positioning_is_applied_by_harfbuzz_when_available() {
        if std::process::Command::new("hb-shape")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("V".into(), Some('V' as u32));
        project.opentype_features = "feature dist { pos A V' A <0 0 -100 0>; } dist;".into();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-chain-pos-{}-{:?}.ttf",
            std::process::id(),
            std::thread::current().id()
        ));
        export_ttf(&project, &path).unwrap();
        let result = std::process::Command::new("hb-shape")
            .arg(&path)
            .arg("AVA")
            .arg("--features=dist")
            .arg("--output-format=json")
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            result.status.success(),
            "HarfBuzz could not shape chained GPOS: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let shaped = String::from_utf8_lossy(&result.stdout);
        assert!(
            shaped.contains("\"g\":\"V\",\"cl\":1,\"dx\":0,\"dy\":0,\"ax\":500"),
            "unexpected shaping: {shaped}"
        );
    }

    #[test]
    fn mark_to_ligature_anchors_compile_into_gpos() {
        let mut project = FontProject::new();
        let mut mark = GlyphData::new("acute".into(), None);
        mark.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 0.0,
            y: 0.0,
        });
        let mut ligature = GlyphData::new("f_i".into(), None);
        ligature.anchors.extend([
            GlyphAnchor {
                name: "top_1".into(),
                x: 250.0,
                y: 700.0,
            },
            GlyphAnchor {
                name: "top_2".into(),
                x: 550.0,
                y: 700.0,
            },
        ]);
        project.glyphs.insert("acute".into(), mark);
        project.glyphs.insert("f_i".into(), ligature);
        let ids = [("acute", 1), ("f_i", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(&project, &ids, "").unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn cursive_anchors_compile_into_gpos() {
        let mut project = FontProject::new();
        for (name, entry, exit) in [
            ("alef", (0.0, 500.0), (500.0, 500.0)),
            ("beh", (0.0, 500.0), (500.0, 500.0)),
        ] {
            let mut glyph = GlyphData::new(name.into(), None);
            glyph.anchors.extend([
                GlyphAnchor {
                    name: "entry".into(),
                    x: entry.0,
                    y: entry.1,
                },
                GlyphAnchor {
                    name: "exit".into(),
                    x: exit.0,
                    y: exit.1,
                },
            ]);
            project.glyphs.insert(name.into(), glyph);
        }
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(&project, &ids, "").unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_source_cursive_anchors_compile_into_gpos() {
        let project = FontProject::new();
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let source = "feature curs { pos cursive alef <anchor 0 500> <anchor 500 500>; pos cursive beh <anchor 0 500> <anchor 500 500>; } curs;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_source_cursive_allows_null_anchors() {
        let project = FontProject::new();
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let source = "feature curs { pos cursive alef NULL <anchor 500 500>; pos cursive beh <anchor 0 500> NULL; } curs;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_value_records_parse_device_tables() {
        let records = parse_feature_value_records(
            "<-80 0 -160 0 <device 11 -1, 12 -1> <device NULL> <device 11 -2, 12 -2> <device NULL>>",
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].values, [-80, 0, -160, 0]);
        assert!(records[0].devices[0].is_some());
        assert!(records[0].devices[1].is_none());
        assert!(records[0].devices[2].is_some());
        assert!(records[0].devices[3].is_none());
    }

    #[test]
    fn feature_source_device_tables_compile_into_gpos() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let source = "feature kern { pos A V <-80 0 -160 0 <device 11 -1, 12 -1> <device NULL> <device 11 -2, 12 -2> <device NULL>>; } kern;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_source_mark_to_ligature_compiles_into_gpos() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("f_i".into(), None);
        let ids = [("acute", 1), ("f_i", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @top; feature mark { pos ligature f_i <anchor 250 700> mark @top <anchor 550 700> mark @top; } mark;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_source_mark_to_ligature_preserves_null_component_slots() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("f_i".into(), None);
        let ids = [("acute", 1), ("f_i", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @top; feature mark { pos ligature f_i NULL <anchor 550 700> mark @top; } mark;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn lookup_flags_parse_open_type_feature_qualifiers() {
        let flags = parse_lookup_flags(
            "lookupflag IgnoreMarks; lookupflag IgnoreLigatures; lookupflag MarkAttachmentType 3;",
        );
        assert!(flags.contains(layout::LookupFlag::IGNORE_MARKS));
        assert!(flags.contains(layout::LookupFlag::IGNORE_LIGATURES));
        assert_eq!(flags.mark_attachment_class(), Some(3));
    }

    #[test]
    fn mark_filtering_set_is_emitted_from_named_class() {
        let ids = [("acute", 1), ("grave", 2)].into_iter().collect();
        let source = "@Marks = [acute grave]; feature mark { lookupflag UseMarkFilteringSet @Marks; pos acute <0 0 0 0>; } mark;";
        let sets = parse_mark_glyph_sets(source, &ids);
        assert_eq!(sets.get("@Marks").map(|(index, _)| *index), Some(0));
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("grave".into(), None);
        let bytes = build_gdef(&project, &ids, source);
        assert!(bytes.is_some());
    }

    #[test]
    fn feature_source_parses_explicit_ligature_carets() {
        let ids = [("f_i", 1), ("f_f", 2), ("f_l", 3), ("f_t", 4)]
            .into_iter()
            .collect();
        let carets = parse_feature_ligature_carets(
            "table GDEF { LigatureCaretByPos f_i 300 600; LigatureCaretByIndex f_f 1 2; LigatureCaretByPos [f_l f_t] 500; } GDEF;",
            &ids,
        );
        assert_eq!(carets.len(), 4);
        assert!(matches!(
            carets[&GlyphId16::new(1)][0],
            gdef::CaretValue::Format1(_)
        ));
        assert!(matches!(
            carets[&GlyphId16::new(2)][0],
            gdef::CaretValue::Format2(_)
        ));
        assert!(carets.contains_key(&GlyphId16::new(3)));
        assert!(carets.contains_key(&GlyphId16::new(4)));
    }

    #[test]
    fn feature_source_parses_gdef_attach_points() {
        let ids = [("A", 1), ("B", 2), ("C", 3)].into_iter().collect();
        let attach = parse_feature_attach_points(
            "table GDEF { Attach [A B] 7 2 7; Attach C 4; } GDEF;",
            &ids,
        )
        .expect("attach list should be emitted");
        assert_eq!(attach.attach_points.len(), 3);
        assert_eq!(attach.attach_points[0].point_indices, vec![2, 7]);
        assert_eq!(attach.attach_points[2].point_indices, vec![4]);
    }

    #[test]
    fn feature_table_overrides_apply_to_head_and_hhea() {
        let mut project = FontProject::new();
        let source = "table head { FontRevision 2.75; Flags 0x5; MacStyle 0x3; LowestRecPPEM 9; FontDirectionHint -1; } head; table hhea { Ascender 900; Descender -250; LineGap 40; CaretSlopeRise 2; CaretSlopeRun -1; CaretOffset 3; } hhea; table post { ItalicAngle -12.5; UnderlinePosition -110; UnderlineThickness 55; IsFixedPitch 1; } post; table OS/2 { TypoAscender 920; TypoDescender -260; TypoLineGap 42; XHeight 500; CapHeight 700; FSType 8; FsSelection 0x140; DefaultChar 0x25A1; BreakChar 0x20; MaxContext 7; YSubscriptXSize 300; YSubscriptYSize 280; YSubscriptXOffset 12; YSubscriptYOffset -18; YSuperscriptXSize 310; YSuperscriptYSize 290; YSuperscriptXOffset 14; YSuperscriptYOffset 420; YStrikeoutSize 35; YStrikeoutPosition 310; SFamilyClass 4660; LowerOpticalPointSize 9; UpperOpticalPointSize 72; WinAscent 1200; WinDescent 350; Panose 2 11 6 3 5 4 2 2 2 4; } OS/2;";
        apply_feature_table_overrides(&mut project, source);
        assert!((project.metadata.font_revision - 2.75).abs() < f64::EPSILON);
        assert!((project.metadata.italic_angle + 12.5).abs() < f64::EPSILON);
        assert_eq!(project.metadata.underline_position, -110.0);
        assert_eq!(project.metadata.underline_thickness, 55.0);
        assert!(project.metadata.is_fixed_pitch);
        assert_eq!(project.metadata.x_height, 500.0);
        assert_eq!(project.metadata.cap_height, 700.0);
        assert_eq!(project.metadata.fs_type, 8);
        assert_eq!(project.metadata.fs_selection, 0x140);
        assert_eq!(project.metadata.default_char, 0x25A1);
        assert_eq!(project.metadata.break_char, 0x20);
        assert_eq!(project.metadata.max_context, 7);
        assert_eq!(os2_selection_flags(&project.metadata), 0x140);
        assert_eq!(project.metadata.head_flags, 5);
        assert_eq!(project.metadata.head_mac_style, 3);
        assert_eq!(project.metadata.lowest_rec_ppem, 9);
        assert_eq!(project.metadata.font_direction_hint, -1);
        assert_eq!(project.metadata.caret_slope_rise, 2);
        assert_eq!(project.metadata.caret_slope_run, -1);
        assert_eq!(project.metadata.caret_offset, 3);
        assert_eq!(project.metadata.panose, [2, 11, 6, 3, 5, 4, 2, 2, 2, 4]);
        assert_eq!(project.metadata.subscript_x_size, 300);
        assert_eq!(project.metadata.superscript_y_offset, 420);
        assert_eq!(project.metadata.strikeout_position, 310);
        assert_eq!(project.metadata.family_class, 4660);
        assert_eq!(project.metadata.lower_optical_point_size, 9);
        assert_eq!(project.metadata.upper_optical_point_size, 72);
        assert_eq!(project.metadata.win_ascent, 1200);
        assert_eq!(project.metadata.win_descent, 350);
        let metrics = project.master_metrics_for(&project.default_master_id);
        assert_eq!(metrics.ascender, 920.0);
        assert_eq!(metrics.descender, -260.0);
        assert_eq!(metrics.line_gap, 42.0);
    }

    #[test]
    fn layout_fingerprint_ignores_outlines_but_tracks_layout_inputs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        let original = layout_input_fingerprint(&project);
        project.glyphs.get_mut("A").unwrap().contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        });
        assert_eq!(layout_input_fingerprint(&project), original);
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .anchors
            .push(GlyphAnchor {
                name: "top".into(),
                x: 50.0,
                y: 700.0,
            });
        assert_ne!(layout_input_fingerprint(&project), original);
    }

    #[test]
    fn feature_table_overrides_apply_to_vmtx_glyph_metrics() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some(65));
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        project.glyphs.insert("A".into(), glyph);
        apply_feature_table_overrides(
            &mut project,
            "table vmtx { VertOriginY A 800; VertAdvanceY A 1200; } vmtx;",
        );
        let metric = project.vertical_metrics["A"];
        assert_eq!(metric.top_side_bearing, 700.0);
        assert_eq!(metric.advance_height, 1200.0);
    }

    #[test]
    fn feature_table_name_records_parse_custom_and_localized_names() {
        let records = parse_feature_name_records(
            "table name { nameid 256 \"Display Name\"; nameid 257 3 1 0x411 \"表示名\"; } name;",
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].nameID, 256);
        assert_eq!(records[0].string, "Display Name");
        assert_eq!(records[1].platformID, 3);
        assert_eq!(records[1].encodingID, 1);
        assert_eq!(records[1].languageID, 0x411);
    }

    #[test]
    fn feature_table_name_records_are_written_to_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.opentype_features = "table name { nameid 256 \"Display Name\"; } name;".to_string();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-name-table-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        let names = font.name().unwrap();
        assert!(names
            .name_record()
            .iter()
            .any(|record| record.name_id().to_u16() == 256));
        std::fs::remove_file(path).unwrap();
    }

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

    #[test]
    fn feature_source_mark_class_and_anchor_positioning_compile() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("A".into(), None);
        let ids = [("acute", 1), ("A", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @TOP; feature mark { pos base A <anchor 300 700> mark @TOP; } mark;";
        assert_eq!(
            parse_feature_anchor(&["markClass", "acute", "<anchor", "0", "0>", "@TOP"], 2),
            Some((0, 0))
        );
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }

    #[test]
    fn feature_source_expands_named_anchor_definitions() {
        let project = FontProject::new();
        let ids = [("acute", 1), ("A", 2)].into_iter().collect();
        let source = "anchorDef <300 700> TOP_ANCHOR; markClass acute <anchor TOP_ANCHOR> @TOP; feature mark { pos base A <anchor 300 700> mark @TOP; } mark;";
        let expanded = expand_named_anchors(source);
        assert!(expanded.contains("<anchor 300 700>"));
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }

    #[test]
    fn feature_source_base_positioning_accepts_multiple_mark_anchors() {
        let project = FontProject::new();
        let ids = [("A", 1), ("acute", 2), ("grave", 3)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @top; markClass grave <anchor 0 0> @bottom; feature mark { pos base A <anchor 300 700> mark @top <anchor 300 0> mark @bottom; } mark;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }

    #[test]
    fn feature_source_mark_to_mark_compiles_from_mark_class() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("grave".into(), None);
        let ids = [("acute", 1), ("grave", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @TOP; markClass grave <anchor 10 0> @TOP; feature mkmk { pos mark @TOP mark @TOP; } mkmk;";
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }

    #[test]
    fn component_ligatures_emit_gdef_caret_list() {
        let mut project = FontProject::new();
        project.add_glyph("f".into(), None);
        project.add_glyph("i".into(), None);
        let mut ligature = GlyphData::new("f_i".into(), None);
        ligature.components = vec![
            GlyphComponent {
                base: "f".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
            GlyphComponent {
                base: "i".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        ];
        project.glyphs.insert("f_i".into(), ligature);
        let ids = [("f", 1), ("i", 2), ("f_i", 3)].into_iter().collect();
        assert!(build_gdef(&project, &ids, "").is_some());
    }

    #[test]
    fn script_and_language_statements_build_layout_script_list() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"kern")];
        let scripts = build_script_list(
            "feature liga { script latn; language TRK; sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        let script = &scripts.script_records[0].script;
        assert_eq!(script.lang_sys_records.len(), 1);
        assert_eq!(script.lang_sys_records[0].lang_sys_tag, Tag::new(b"TRK "));
    }

    #[test]
    fn languagesystem_declarations_populate_default_and_language_systems() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"locl")];
        let scripts = build_script_list(
            "languagesystem latn dflt; languagesystem latn TRK;\nfeature liga { sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        let script = &scripts.script_records[0].script;
        assert!(script.default_lang_sys.is_some());
        assert_eq!(script.lang_sys_records.len(), 1);
        assert_eq!(script.lang_sys_records[0].lang_sys_tag, Tag::new(b"TRK "));
    }

    #[test]
    fn language_dflt_and_required_feature_are_encoded_in_langsys() {
        let tags = vec![Tag::new(b"liga")];
        let scripts = build_script_list(
            "feature liga { script DFLT; language dflt required; sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        let default = scripts.script_records[0]
            .script
            .default_lang_sys
            .as_ref()
            .expect("language dflt should use the default LangSys");
        assert_eq!(default.required_feature_index, 0);
    }

    #[test]
    fn exclude_dflt_omits_global_default_feature_from_language() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"locl")];
        let source = "languagesystem DFLT dflt; languagesystem latn dflt; languagesystem latn DEU; feature liga { sub A by A.alt; script latn; language DEU excludeDFLT; } liga;";
        let scripts = build_script_list(source, &tags);
        let script = scripts
            .script_records
            .iter()
            .find(|record| record.script_tag == Tag::new(b"latn"))
            .expect("latn script should be emitted");
        assert!(script.script.default_lang_sys.is_some());
        let deu = script
            .script
            .lang_sys_records
            .iter()
            .find(|record| record.lang_sys_tag == Tag::new(b"DEU "))
            .expect("DEU language should be emitted");
        assert!(deu.lang_sys.feature_indices.is_empty());
    }

    #[test]
    fn languagesystems_survive_named_lookup_expansion() {
        let tags = vec![Tag::new(b"locl")];
        let source = "languagesystem latn dflt; lookup localizedI { sub i by i.loclTRK; } localizedI; feature locl { lookup localizedI; } locl;";
        let scripts = build_script_list(source, &tags);
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        assert!(scripts.script_records[0].script.default_lang_sys.is_some());
    }

    #[test]
    fn variable_widths_emit_hvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project.glyphs.get_mut("A").unwrap().width = 500.0;
        project.glyphs.get_mut("A").unwrap().layers.insert(
            second.id.clone(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let bytes = build_hvar(&project, &["A"], &project.masters[0], &["wght".into()]);
        assert!(bytes.is_some());
    }

    #[test]
    fn variable_vertical_metrics_emit_vvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project
            .set_vertical_metrics_for_master("A", &project.masters[0].id.clone(), 1000.0, 800.0)
            .unwrap();
        project
            .set_vertical_metrics_for_master("A", &second.id, 1200.0, 900.0)
            .unwrap();
        let bytes = build_vvar(&project, &["A"], &project.masters[0], &["wght".into()]);
        assert!(bytes.is_some());
    }

    #[test]
    fn variable_global_metrics_emit_mvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project
            .set_master_metrics(
                &project.masters[0].id.clone(),
                crate::font_data::MasterMetrics {
                    ascender: 800.0,
                    descender: -200.0,
                    line_gap: 0.0,
                },
            )
            .unwrap();
        project
            .set_master_metrics(
                &second.id,
                crate::font_data::MasterMetrics {
                    ascender: 900.0,
                    descender: -240.0,
                    line_gap: 20.0,
                },
            )
            .unwrap();
        let bytes = build_mvar(&project, &project.masters[0], &["wght".into()]);
        let bytes = bytes.expect("MVAR should be emitted");
        assert_eq!(&bytes[0..4], &[0, 1, 0, 0]);
        assert!(bytes.windows(4).any(|tag| tag == b"hasc"));
        assert!(bytes.windows(4).any(|tag| tag == b"hdsc"));
        assert!(bytes.windows(4).any(|tag| tag == b"hlgp"));
    }

    #[test]
    fn cff2_export_is_readable_by_harfbuzz_when_available() {
        if std::process::Command::new("hb-shape")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(500.0, 0.0),
                ContourPoint::on_curve(250.0, 700.0),
            ],
        });
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-cff2-hb-{}-{:?}.otf",
            std::process::id(),
            std::thread::current().id()
        ));
        export_otf_cff2(&project, &path).unwrap();
        let result = std::process::Command::new("hb-shape")
            .arg(&path)
            .arg("A")
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            result.status.success(),
            "Harfbuzz could not read generated CFF2: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn project_validation_rejects_out_of_range_kerning_values() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.kerning.insert(("A".into(), "A".into()), 40000.0);
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("カーニング値が範囲外")));
    }

    #[test]
    fn project_validation_reports_invalid_variation_sequences() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.unicode_variation_sequences = vec![
            UnicodeVariationSequence {
                base: 0xD800,
                selector: 0xFE00,
                glyph: "missing".into(),
            },
            UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "A".into(),
            },
            UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "A".into(),
            },
        ];
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("IVSのUnicodeまたはセレクタ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないグリフ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("IVSのUnicode／セレクタが重複")));
    }

    #[test]
    fn variation_axis_coordinates_are_normalized_around_default() {
        assert_eq!(normalize_axis(400.0, 400.0, 400.0, 700.0), 0.0);
        assert_eq!(normalize_axis(700.0, 400.0, 400.0, 700.0), 1.0);
        assert_eq!(normalize_axis(300.0, 300.0, 500.0, 700.0), -1.0);
        assert_eq!(normalize_axis(600.0, 300.0, 500.0, 700.0), 0.5);
    }

    #[test]
    fn project_validation_reports_unicode_noncharacters() {
        let mut project = FontProject::new();
        project.add_glyph("noncharacter".into(), Some(0xFDD0));
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("非文字")));
    }

    #[test]
    fn project_validation_reports_orphaned_master_layers() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "deleted-master".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("未定義マスター")));
    }

    #[test]
    fn project_validation_reports_missing_default_master() {
        let mut project = FontProject::new();
        project.default_master_id = "missing".into();
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("デフォルトマスター")));
    }

    #[test]
    fn feature_validation_ignores_comments_and_accepts_multiline_declarations() {
        let source = "feature liga {\n  # } ; this is a comment\n  sub f i by fi;\n} liga;";
        assert!(validate_feature_source(source).is_ok());
        assert!(validate_feature_source("feature liga {\n  sub f i by fi;\n").is_err());
        assert!(validate_feature_source(
            "feature liga {\n  sub f i by fi;\n} liga;\n\"unterminated"
        )
        .is_err());
    }

    #[test]
    fn feature_validation_reports_unknown_glyph_references() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.opentype_features = "feature liga { sub A by missing; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("missing")));
    }

    #[test]
    fn feature_validation_reports_undefined_named_classes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature calt { sub @missing A' by A; } calt;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("未定義クラス")));
    }

    #[test]
    fn feature_validation_checks_glyphs_inside_classes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga { sub [A absent] by A; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("absent")));
    }

    #[test]
    fn feature_validation_handles_multiline_statements() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga {\n  sub A\n    by missing;\n} liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("missing")));
    }

    #[test]
    fn feature_validation_reports_statement_line_without_an_offset() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga { sub missing by A; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("OpenType feature 1行目") && issue.contains("missing")));
    }

    #[test]
    fn project_validation_reports_invalid_contour_topology() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "broken".into(),
            GlyphData {
                name: "broken".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::off_curve(0.0, 0.0),
                        ContourPoint::off_curve(1.0, 1.0),
                        ContourPoint::off_curve(2.0, 2.0),
                    ],
                }],
                ..GlyphData::new("broken".into(), None)
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("オンカーブ点")));
        assert!(issues.iter().any(|issue| issue.contains("オフカーブ点")));
    }

    #[test]
    fn project_validation_reports_duplicate_and_degenerate_contours() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "broken".into(),
            GlyphData {
                name: "broken".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(10.0, 0.0),
                        ContourPoint::on_curve(20.0, 0.0),
                    ],
                }],
                ..GlyphData::new("broken".into(), Some('A' as u32))
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("重複した隣接点")));
        assert!(issues.iter().any(|issue| issue.contains("退化した輪郭")));
    }

    #[test]
    fn project_validation_reports_self_intersecting_contours() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "cross".into(),
            GlyphData {
                name: "cross".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 100.0),
                        ContourPoint::on_curve(0.0, 100.0),
                        ContourPoint::on_curve(100.0, 0.0),
                    ],
                }],
                ..GlyphData::new("cross".into(), Some('A' as u32))
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("自己交差")));
    }

    #[test]
    fn project_validation_reports_incompatible_master_layers() {
        let mut project = FontProject::new();
        project.masters.push(crate::font_data::FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 0.0),
                        ContourPoint::on_curve(0.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 0.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("補間互換ではありません")));
        let interpolation_issues = validate_interpolation(&project, "regular", "bold");
        assert_eq!(interpolation_issues.len(), 1);
        assert!(interpolation_issues[0].message.contains("ノード数が不一致"));
    }

    #[test]
    fn exports_selected_master_as_static_ttf() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![contour.clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 700.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-master-{}.ttf", std::process::id()));
        export_ttf_for_master(&project, "bold", &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(!font.tables.contains_key(b"fvar"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exports_static_otf_with_cff_table() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: None,
                alpha: 0.42,
            }],
        );
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.otf", std::process::id()));
        export_otf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert_eq!(&std::fs::read(&path).unwrap()[0..4], b"OTTO");
        assert!(font.tables.contains_key(b"CFF "));
        assert!(!font.tables.contains_key(b"glyf"));
        assert!(font.tables.contains_key(b"COLR"));
        assert!(font.tables.contains_key(b"CPAL"));
        assert!(font.tables.contains_key(b"SVG "));
        let loaded = crate::io::load_ttf(&path).unwrap();
        assert_eq!(loaded.color_layers["A"][0].glyph, "A.red");
        assert!((loaded.color_layers["A"][0].alpha - 0.42).abs() < 0.01);
        assert_eq!(loaded.color_palettes[0][0], [255, 0, 0, 255]);
        let bytes = std::fs::read(&path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, 0).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn colr_v1_gradient_and_transform_round_trip_through_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.color".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_palette_names = vec!["Brand Light".into()];
        project.color_palette_types = vec![0x0000_0001];
        project.color_palette_entry_names = vec!["Fill".into(), "Outline".into()];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.color".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Reflect,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 500.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 500.0,
                    stops: vec![
                        crate::font_data::ColorGradientStop {
                            offset: 0.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 1.0,
                            palette_index: 1,
                            alpha: 0.75,
                        },
                    ],
                    radius0: 0.0,
                    radius1: 0.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
                alpha: 1.0,
            }],
        );
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 1.1,
                yx: 0.0,
                xy: 0.0,
                yy: 0.9,
                dx: 12.0,
                dy: -8.0,
            })],
        );
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-colr-v1-roundtrip-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let exported_bytes = std::fs::read(&path).unwrap();
        let exported_font = read_fonts::FontRef::new(&exported_bytes).unwrap();
        let exported_cpal = exported_font.cpal().unwrap();
        assert_eq!(exported_cpal.version(), 1);
        let exported_labels = exported_cpal.palette_labels_array().unwrap().unwrap();
        assert_eq!(exported_labels[0].get().to_u16(), 1000);
        let exported_name = exported_font.name().unwrap();
        let exported_name_data = exported_name.string_data();
        assert!(exported_name
            .name_record()
            .iter()
            .any(|record| record.name_id().to_u16() == 1000
                && record.string(exported_name_data).is_ok()));
        assert_eq!(
            exported_name
                .name_record()
                .iter()
                .find(|record| record.name_id().to_u16() == 1000)
                .unwrap()
                .string(exported_name_data)
                .unwrap()
                .chars()
                .collect::<String>(),
            "Brand Light"
        );
        let loaded = crate::io::load_ttf(&path).unwrap();
        let layer = &loaded.color_layers["A"][0];
        let gradient = layer.gradient.as_ref().unwrap();
        assert_eq!(layer.glyph, "A.color");
        assert_eq!(gradient.kind, crate::font_data::ColorGradientKind::Linear);
        assert_eq!(
            gradient.extend,
            crate::font_data::ColorGradientExtend::Reflect
        );
        assert_eq!(gradient.stops.len(), 2);
        assert!((gradient.stops[1].alpha - 0.75).abs() < 0.01);
        let transform = loaded.color_layer_transforms["A"][0].unwrap();
        assert!((transform.xx - 1.1).abs() < 0.001);
        assert!((transform.dy + 8.0).abs() < 0.001);
        assert_eq!(loaded.color_palette_names, vec!["Brand Light"]);
        assert_eq!(loaded.color_palette_types, vec![0x0000_0001]);
        assert_eq!(loaded.color_palette_entry_names, vec!["Fill", "Outline"]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exports_otf_with_component_glyphs() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), Some('A' as u32));
        project.add_glyph("composite".into(), Some('B' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        project
            .glyphs
            .get_mut("composite")
            .unwrap()
            .components
            .push(crate::font_data::GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 20.0,
                y_offset: 30.0,
            });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-component-{}.otf", std::process::id()));
        export_otf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, 0).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exports_ttf_with_mixed_contours_and_components() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), Some('A' as u32));
        project.add_glyph("mixed".into(), Some('B' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        project
            .glyphs
            .get_mut("mixed")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(200.0, 0.0),
                    ContourPoint::on_curve(300.0, 0.0),
                    ContourPoint::on_curve(300.0, 100.0),
                ],
            });
        project.glyphs.get_mut("mixed").unwrap().components.push(
            crate::font_data::GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        );
        let path =
            std::env::temp_dir().join(format!("glyph-studio-mixed-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, 0).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn variable_ttf_contains_component_transform_variation() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), Some('A' as u32));
        project.add_glyph("accented".into(), Some('B' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        let component = |x_scale, xy_scale, yx_scale, y_scale, x_offset| GlyphComponent {
            base: "base".into(),
            x_scale,
            xy_scale,
            yx_scale,
            y_scale,
            x_offset,
            y_offset: 0.0,
        };
        project
            .glyphs
            .get_mut("accented")
            .unwrap()
            .components
            .push(component(1.0, 0.0, 0.0, 1.0, 0.0));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        for (name, glyph) in &mut project.glyphs {
            let layer = GlyphLayer {
                width: glyph.width,
                contours: glyph.contours.clone(),
                components: glyph.components.clone(),
                anchors: glyph.anchors.clone(),
            };
            glyph.layers.insert("regular".into(), layer.clone());
            let mut bold = layer;
            if name == "accented" {
                bold.components = vec![component(1.1, 0.2, -0.2, 0.9, 25.0)];
            }
            glyph.layers.insert("bold".into(), bold);
        }
        let mut flattened = project.clone();
        flatten_variation_components(&mut flattened).unwrap();
        let regular = &flattened.glyphs["accented"].layers["regular"];
        let bold = &flattened.glyphs["accented"].layers["bold"];
        assert!(regular.components.is_empty() && bold.components.is_empty());
        assert_ne!(regular.contours[0].points, bold.contours[0].points);
        project.masters.swap(0, 1);
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-component-variation-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"gvar"));
        assert!(font.tables.contains_key(b"STAT"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stat_table_has_a_valid_axis_directory_header() {
        let table = build_stat_table_with_values(&[(*b"wght", 256), (*b"wdth", 257)], &[], &[]);
        assert_eq!(&table[0..4], &0x0001_0002_u32.to_be_bytes());
        assert_eq!(u16::from_be_bytes([table[4], table[5]]), 8);
        assert_eq!(u16::from_be_bytes([table[6], table[7]]), 2);
        assert_eq!(
            u32::from_be_bytes([table[8], table[9], table[10], table[11]]),
            20
        );
        assert_eq!(&table[20..24], b"wght");
        assert_eq!(&table[28..32], b"wdth");
    }

    #[test]
    fn stat_table_encodes_named_multi_axis_values() {
        let table = build_stat_table_with_values(
            &[(*b"wght", 256), (*b"wdth", 257)],
            &[vec![700.0, 110.0]],
            &[300],
        );
        assert_eq!(u16::from_be_bytes([table[12], table[13]]), 1);
        assert_eq!(
            u32::from_be_bytes([table[14], table[15], table[16], table[17]]),
            36
        );
        assert_eq!(u16::from_be_bytes([table[36], table[37]]), 38);
        assert_eq!(u16::from_be_bytes([table[38], table[39]]), 4);
        assert_eq!(u16::from_be_bytes([table[40], table[41]]), 2);
        assert_eq!(u16::from_be_bytes([table[44], table[45]]), 300);
    }

    #[test]
    fn exports_interpolated_static_ttf() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let regular = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let bold = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(200.0, 0.0),
                ContourPoint::on_curve(200.0, 200.0),
            ],
        };
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![regular],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 700.0,
                contours: vec![bold],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project
            .set_vertical_metrics_for_master("A", "regular", 1000.0, 800.0)
            .unwrap();
        project
            .set_vertical_metrics_for_master("A", "bold", 1200.0, 600.0)
            .unwrap();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-instance-{}.ttf", std::process::id()));
        export_ttf_at_interpolation(&project, "regular", "bold", 0.5, &path).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let font = fonttools::font::load(&mut file).unwrap();
        assert!(font.tables.contains_key(b"glyf"));
        assert!(!font.tables.contains_key(b"fvar"));
        let imported = crate::io::load_ttf(&path).unwrap();
        assert_eq!(imported.vertical_metrics["A"].advance_height, 1100.0);
        assert_eq!(imported.vertical_metrics["A"].top_side_bearing, 700.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn interpolation_rejects_glyphs_missing_a_master_layer() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-missing-layer-{}.ttf",
            std::process::id()
        ));
        let error = export_ttf_at_interpolation(&project, "regular", "bold", 0.5, &path)
            .expect_err("missing master layer must be rejected");
        assert!(error.contains("補間元マスター") || error.contains("補間先マスター"));
        assert!(!path.exists());
    }

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

    #[test]
    fn exports_requested_interpolation_set() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-set-{}", std::process::id()));
        let count =
            export_interpolation_set(&project, "regular", "bold", &[0.1, 0.5, 0.9], &directory)
                .unwrap();
        assert_eq!(count, 3);
        assert!(directory.join("instance-10.ttf").exists());
        assert!(directory.join("instance-50.ttf").exists());
        assert!(directory.join("instance-90.ttf").exists());
        assert!(export_interpolation_set(&project, "regular", "bold", &[], &directory).is_err());
        assert!(
            export_interpolation_set(&project, "regular", "bold", &[0.5, 0.5], &directory).is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn exports_each_master_as_a_separate_static_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-masters-{}", std::process::id()));
        let count = export_all_ttf_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn exports_each_master_as_a_separate_static_otf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "太字".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-otf-masters-{}", std::process::id()));
        let count = export_all_otf_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        for entry in std::fs::read_dir(&directory).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert_eq!(&bytes[..4], b"OTTO");
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn exports_each_master_as_a_separate_woff() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-woff-masters-{}", std::process::id()));
        let count = export_all_woff_for_masters(&project, &directory).unwrap();
        assert_eq!(count, 2);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn exports_each_master_as_a_separate_woff2() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-woff2-masters-{}", std::process::id()));
        let count = export_all_woff2_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("woff2"));
            assert_eq!(&std::fs::read(&path).unwrap()[..4], b"wOF2");
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn feature_source_validation_rejects_unbalanced_or_malformed_source() {
        assert!(validate_feature_source("feature liga { sub f i by fi; } liga;").is_ok());
        assert!(validate_feature_source("feature liga { sub f i by fi;").is_err());
        assert!(validate_feature_source("feature liga { sub f i by fi; };").is_ok());
        assert!(validate_feature_source("feature liga { sub f i by fi; }").is_ok());
    }

    #[test]
    fn feature_source_validation_rejects_invalid_or_duplicate_tags() {
        assert!(validate_feature_source("feature lig { sub f i by fi; } lig;").is_err());
        assert!(validate_feature_source("feature ligä { sub f i by fi; } ligä;").is_err());
        assert!(validate_feature_source(
            "feature liga { sub f i by fi; } liga; feature liga { sub f by f.alt; } liga;"
        )
        .is_err());
    }

    #[test]
    fn feature_source_validation_checks_languagesystem_tags() {
        assert!(validate_feature_source(
            "languagesystem latn dflt; feature liga { sub f i by fi; } liga;"
        )
        .is_ok());
        assert!(validate_feature_source(
            "languagesystem latin dflt; feature liga { sub f i by fi; } liga;"
        )
        .is_err());
        assert!(validate_feature_source(
            "languagesystem latn Japanese; feature liga { sub f i by fi; } liga;"
        )
        .is_err());
        assert!(validate_feature_source(
            "# languagesystem bad dflt;\nfeature liga { sub f i by fi; } liga;"
        )
        .is_ok());
    }

    #[test]
    fn feature_source_validation_checks_named_lookup_references() {
        let valid = "lookup L { sub f i by fi; } L; feature liga { lookup L; } liga;";
        assert!(validate_feature_source(valid).is_ok());
        assert!(validate_feature_source("feature liga { lookup Missing; } liga;").is_err());
        assert!(validate_feature_source(
            "lookup L { sub f i by fi; } L; lookup L { sub f by f.alt; } L;"
        )
        .is_err());
    }

    #[test]
    fn feature_class_validation_reports_duplicates_and_missing_glyphs() {
        let mut glyphs = std::collections::HashMap::new();
        glyphs.insert("A".into(), GlyphData::new("A".into(), Some('A' as u32)));
        let issues =
            validate_feature_class_definitions("@Upper = [A Missing]; @Upper = [A];", &glyphs);
        assert!(issues.iter().any(|issue| issue.contains("重複")));
        assert!(issues.iter().any(|issue| issue.contains("Missing")));
    }

    #[test]
    fn master_axis_validation_rejects_non_finite_or_out_of_range_values() {
        let mut project = FontProject::new();
        project.masters[0].weight = f64::NAN;
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].weight = 400.0;
        project.masters[0].width = 0.0;
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].width = 100.0;
        project.masters[0].axes.insert("too".into(), 10.0);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("opsz".into(), f64::NAN);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("wdth".into(), 100.0);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("wght".into(), 400.0);
        assert!(validate_master_axes(&project).is_err());
    }

    #[test]
    fn instance_axis_validation_rejects_reserved_or_invalid_values() {
        let mut project = FontProject::new();
        project.instances.push(FontInstance {
            name: "Bad".into(),
            axes: HashMap::from([("wght".into(), 500.0)]),
            weight: 500.0,
            width: 100.0,
        });
        assert!(validate_master_axes(&project).is_err());
        project.instances[0].axes.clear();
        project.instances[0].weight = f64::NAN;
        assert!(validate_master_axes(&project).is_err());
    }

    #[test]
    fn project_validation_reports_invalid_master_metadata() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: project.masters[0].name.clone(),
            weight: f64::INFINITY,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::from([("weight".into(), f64::NAN)]),
        });
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("マスター名が重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("WeightまたはWidthが不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("軸 'weight' の値が不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("軸タグ 'weight' は4文字ASCII")));
    }

    #[test]
    fn nested_components_are_flattened_and_cycles_are_rejected() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut middle = GlyphData::new("middle".into(), None);
        middle.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 50.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("middle".into(), middle);
        let mut top = GlyphData::new("top".into(), None);
        top.components.push(GlyphComponent {
            base: "middle".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 75.0,
        });
        project.glyphs.insert("top".into(), top);
        let mut contours = Vec::new();
        append_contours(
            &project,
            "top",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut contours,
        )
        .unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0][0].x, 50);

        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "top".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        assert!(append_contours(
            &project,
            "top",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut Vec::new()
        )
        .is_err());
    }

    #[test]
    fn svg_export_preserves_bezier_commands() {
        let mut project = FontProject::new();
        project.metadata.ascender = 900.0;
        project.metadata.descender = -250.0;
        let mut glyph = GlyphData::new("curve".into(), None);
        glyph.width = 720.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, -100.0),
                ContourPoint::on_curve(0.0, -100.0),
            ],
        });
        project.glyphs.insert("curve".into(), glyph);
        let path =
            std::env::temp_dir().join(format!("glyph-studio-curve-{}.svg", std::process::id()));
        export_svg(&project, "curve", &path).unwrap();
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.contains("Q "));
        assert!(svg.contains("fill-rule"));
        assert!(svg.contains("viewBox=\"0 -900 720 1150\""));
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(40.0, 0.0),
                ContourPoint::on_curve(40.0, 40.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 100.0,
            y_offset: 25.0,
        });
        project.glyphs.insert("composite".into(), composite);
        export_svg(&project, "composite", &path).unwrap();
        let composite_svg = std::fs::read_to_string(&path).unwrap();
        assert_eq!(composite_svg.matches("<path").count(), 1);
        assert!(composite_svg.contains("100"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exports_all_glyphs_to_safe_svg_filenames() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("あ".into(), Some('あ' as u32));
        project.add_glyph("い".into(), Some('い' as u32));
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-{}", std::process::id()));
        let count = export_all_svg(&project, &directory).unwrap();
        assert_eq!(count, 3);
        assert!(directory.join("A.svg").is_file());
        assert!(directory.join("_.svg").is_file());
        assert!(directory.join("__2.svg").is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn master_svg_export_rejects_unknown_master_without_mutating_project() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let original = project.clone();
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-missing-{}", std::process::id()));
        let result = export_all_svg_for_master(&project, "missing", &directory);
        assert!(result.is_err());
        assert_eq!(project, original);
        assert!(!directory.exists());
    }

    #[test]
    fn master_svg_export_uses_the_requested_layer_geometry() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 500.0;
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 800.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(700.0, 0.0),
                        ContourPoint::on_curve(700.0, 700.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-master-{}", std::process::id()));
        export_all_svg_for_master(&project, "bold", &directory).unwrap();
        let svg = std::fs::read_to_string(directory.join("A.svg")).unwrap();
        assert!(svg.contains("viewBox=\"0 -800 800 1000\""));
        assert!(svg.contains("700 -0"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_validation_reports_invalid_color_layers() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.color_palettes = vec![vec![[0, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "missing".into(),
                palette_index: 4,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("未定義グリフ 'missing'")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("パレット番号が範囲外")));
    }

    #[test]
    fn color_tables_encode_colr_layers_and_cpal_bgra_records() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.add_glyph("A.green".into(), None);
        project.add_glyph("A.blue".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 128], [0, 32, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![
                crate::font_data::ColorLayer {
                    glyph: "A.red".into(),
                    palette_index: 1,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Linear,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 0.0,
                        y0: 0.0,
                        x1: 1000.0,
                        y1: 0.0,
                        x2: 0.0,
                        y2: 1000.0,
                        stops: vec![
                            crate::font_data::ColorGradientStop {
                                offset: 0.0,
                                palette_index: 0,
                                alpha: 1.0,
                            },
                            crate::font_data::ColorGradientStop {
                                offset: 0.5,
                                palette_index: 1,
                                alpha: 0.5,
                            },
                            crate::font_data::ColorGradientStop {
                                offset: 1.0,
                                palette_index: 0,
                                alpha: 1.0,
                            },
                        ],
                        radius0: 0.0,
                        radius1: 500.0,
                        start_angle: 0.0,
                        end_angle: 360.0,
                    }),
                    alpha: 1.0,
                },
                crate::font_data::ColorLayer {
                    glyph: "A.green".into(),
                    palette_index: 0,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Radial,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 100.0,
                        y0: 200.0,
                        x1: 300.0,
                        y1: 400.0,
                        x2: 300.0,
                        y2: 200.0,
                        stops: Vec::new(),
                        radius0: 10.0,
                        radius1: 500.0,
                        start_angle: 0.0,
                        end_angle: 360.0,
                    }),
                    alpha: 1.0,
                },
                crate::font_data::ColorLayer {
                    glyph: "A.blue".into(),
                    palette_index: 0,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Sweep,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 500.0,
                        y0: 500.0,
                        x1: 0.0,
                        y1: 0.0,
                        x2: 1000.0,
                        y2: 500.0,
                        stops: Vec::new(),
                        radius0: 0.0,
                        radius1: 500.0,
                        start_angle: 30.0,
                        end_angle: 270.0,
                    }),
                    alpha: 1.0,
                },
            ],
        );
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 0.9,
                yx: 0.1,
                xy: -0.1,
                yy: 1.1,
                dx: 24.0,
                dy: -12.0,
            })],
        );
        let ids = [("A", 1), ("A.red", 2), ("A.green", 3), ("A.blue", 4)]
            .into_iter()
            .collect();
        let (colr, cpal) = build_color_tables(&project, &ids).unwrap();
        assert_eq!(&colr[0..2], &[0, 1]);
        assert_eq!(u16::from_be_bytes([colr[2], colr[3]]), 1);
        assert_eq!(u16::from_be_bytes([colr[12], colr[13]]), 3);
        assert_ne!(
            u32::from_be_bytes([colr[14], colr[15], colr[16], colr[17]]),
            0
        );
        assert_eq!(&cpal[0..2], &[0, 0]);
        assert_eq!(u16::from_be_bytes([cpal[2], cpal[3]]), 2);
        assert_eq!(&cpal[20..24], &[255, 32, 0, 255]);
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 2, 4]));
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 3, 6]));
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 4, 8]));
        assert!(colr
            .windows(7)
            .any(|window| window == [12, 0, 0, 31, 0, 0, 7]));
    }

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

    #[test]
    fn color_tables_keep_base_and_layer_order_stable_for_hash_maps() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), Some(66));
        project.add_glyph("A.layer".into(), None);
        project.add_glyph("B.layer".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "B".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "B.layer".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.layer".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let ids = [("A", 1), ("B", 2), ("A.layer", 3), ("B.layer", 4)]
            .into_iter()
            .collect();
        let (colr, _) = build_color_tables(&project, &ids).unwrap();
        assert_eq!(&colr[34..40], &[0, 1, 0, 0, 0, 1]);
        assert_eq!(&colr[40..46], &[0, 2, 0, 1, 0, 1]);
        assert_eq!(&colr[46..50], &[0, 3, 0, 0]);
        assert_eq!(&colr[50..54], &[0, 4, 0, 0]);
    }

    #[test]
    fn color_tables_can_reuse_nested_color_glyphs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.inner".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let ids = [("A", 1), ("A.inner", 2), ("A.leaf", 3)]
            .into_iter()
            .collect();
        let (colr, _) = build_color_tables(&project, &ids).unwrap();
        assert!(colr.windows(3).any(|window| window == [11, 0, 2]));
    }

    #[test]
    fn svg_export_expands_nested_color_glyphs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 1,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.inner".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .glyphs
            .get_mut("A.leaf")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        let svg = build_svg_document(&project, "A").unwrap();
        assert!(svg.contains("fill=\"none\" fill-opacity=\"1.000000\""));
        assert!(svg.contains("fill=\"#0000ff\""));
        assert_eq!(svg.matches("<path").count(), 1);
    }

    #[test]
    fn svg_export_preserves_nested_color_gradients() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Pad,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: Vec::new(),
                    radius0: 0.0,
                    radius1: 100.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.inner".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .glyphs
            .get_mut("A.leaf")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        let svg = build_svg_document(&project, "A").unwrap();
        assert!(svg.contains("id=\"glyph-studio-nested-gradient-0-0\""));
        assert!(svg.contains("fill=\"url(#glyph-studio-nested-gradient-0-0)\""));
        assert!(svg
            .contains("fill=\"url(#glyph-studio-nested-gradient-0-0)\" fill-opacity=\"1.000000\""));
        assert_eq!(svg.matches("<stop ").count(), 2);
    }

    #[test]
    fn project_validation_reports_nested_color_cycles() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("COLRカラーグリフ循環参照")));
    }

    #[test]
    fn svg_export_encodes_color_layers_with_palette_alpha() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 128]], vec![[0, 255, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: None,
                alpha: 0.5,
            }],
        );
        project
            .glyphs
            .get_mut("A.red")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        let path =
            std::env::temp_dir().join(format!("glyph-studio-color-{}.svg", std::process::id()));
        export_svg_with_palette(&project, "A", 1, &path).unwrap();
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.contains("fill=\"#00ff00\""));
        assert!(svg.contains("fill-opacity=\"0.500000\""));
        assert_eq!(svg.matches("<path").count(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn svg_export_encodes_color_gradients_and_spread_method() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 255, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Reflect,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: vec![
                        crate::font_data::ColorGradientStop {
                            offset: 0.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 0.5,
                            palette_index: 1,
                            alpha: 0.75,
                        },
                    ],
                    radius0: 0.0,
                    radius1: 100.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
                alpha: 1.0,
            }],
        );
        project
            .glyphs
            .get_mut("A.red")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 1.0,
                yx: 0.0,
                xy: 0.0,
                yy: 1.0,
                dx: 12.0,
                dy: -6.0,
            })],
        );
        let svg = build_svg_document(&project, "A").unwrap();
        assert!(svg.contains("<linearGradient"));
        assert!(svg.contains("spreadMethod=\"reflect\""));
        assert!(svg.contains("matrix(1 0 0 1 12 -6)"));
        assert_eq!(svg.matches("<stop ").count(), 2);
    }
}
