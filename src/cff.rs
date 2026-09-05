use crate::font_data::{Contour, FontProject, GlyphComponent};
use std::collections::HashSet;

/// Encodes a quadratic-node outline as a Type 2 charstring.
///
/// This is the small, deterministic core used by the future CFF table writer;
pub fn encode_type2_contours(contours: &[Contour]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut current = (0i32, 0i32);
    for contour in contours {
        if contour.points.is_empty() {
            continue;
        }
        let start_idx = contour
            .points
            .iter()
            .position(|point| point.is_on_curve())
            .ok_or_else(|| "CFF輪郭にオンカーブ点がありません".to_string())?;
        let first = contour.points[start_idx];
        let first_xy = checked_coordinate(first.x, first.y)?;
        push_number(&mut output, first_xy.0 - current.0);
        push_number(&mut output, first_xy.1 - current.1);
        output.push(21); // rmoveto
        current = first_xy;
        let mut index = (start_idx + 1) % contour.points.len();
        while index != start_idx {
            let point = contour.points[index];
            if point.is_on_curve() {
                let xy = checked_coordinate(point.x, point.y)?;
                push_number(&mut output, xy.0 - current.0);
                push_number(&mut output, xy.1 - current.1);
                output.push(5); // rlineto
                current = xy;
                index = (index + 1) % contour.points.len();
            } else {
                let next_index = (index + 1) % contour.points.len();
                let next = contour.points[next_index];
                if next.is_on_curve() {
                    return Err("CFF曲線には2つのオフカーブ点が必要です".to_string());
                }
                let end_index = (next_index + 1) % contour.points.len();
                let end = contour.points[end_index];
                if !end.is_on_curve() {
                    return Err("CFF曲線の終点が不正です".to_string());
                }
                let control_1 = checked_coordinate(point.x, point.y)?;
                let control_2 = checked_coordinate(next.x, next.y)?;
                let end = checked_coordinate(end.x, end.y)?;
                for (x, y) in [control_1, control_2, end] {
                    push_number(&mut output, x - current.0);
                    push_number(&mut output, y - current.1);
                    current = (x, y);
                }
                output.push(8); // rrcurveto
                index = (end_index + 1) % contour.points.len();
            }
        }
    }
    // endchar is required even for an intentionally empty glyph.
    output.push(14);
    Ok(output)
}

/// Adds an explicit advance width to a Type 2 charstring. CFF's hmtx table is
/// not consulted by consumers for CFF glyph widths, so every non-default width
/// must be encoded in the charstring itself.
pub fn encode_type2_with_width(width: f64, contours: &[Contour]) -> Result<Vec<u8>, String> {
    let width = checked_coordinate(width, 0.0)?.0;
    let mut output = Vec::new();
    push_number(&mut output, width);
    output.extend(encode_type2_contours(contours)?);
    Ok(output)
}

pub fn encode_project_glyph(project: &FontProject, name: &str) -> Result<Vec<u8>, String> {
    let contours = project_glyph_contours(project, name)?;
    encode_type2_with_width(project.glyphs[name].width, &contours)
}

pub fn encode_project_glyph_cff2(project: &FontProject, name: &str) -> Result<Vec<u8>, String> {
    let contours = project_glyph_contours(project, name)?;
    let mut output = encode_type2_contours(&contours)?;
    // CFF2 removes the Type 2 endchar operator; the CharStrings INDEX item
    // ends after the final drawing operator.
    if output.last() == Some(&14) {
        output.pop();
    }
    Ok(output)
}

fn project_glyph_contours(project: &FontProject, name: &str) -> Result<Vec<Contour>, String> {
    let mut contours = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("グリフ '{}' がありません", name))?
        .contours
        .clone();
    let mut visiting = HashSet::new();
    let components = project.glyphs[name].components.clone();
    for component in &components {
        collect_component_contours(
            project,
            &component.base,
            component,
            &mut visiting,
            &mut contours,
        )?;
    }
    Ok(contours)
}

fn collect_component_contours(
    project: &FontProject,
    name: &str,
    component: &GlyphComponent,
    visiting: &mut HashSet<String>,
    output: &mut Vec<Contour>,
) -> Result<(), String> {
    if !visiting.insert(name.to_string()) {
        return Err(format!("コンポーネント循環参照: {}", name));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("コンポーネント '{}' がありません", name))?;
    for contour in &glyph.contours {
        output.push(Contour {
            points: contour
                .points
                .iter()
                .map(|point| {
                    let x = component.x_scale * point.x
                        + component.xy_scale * point.y
                        + component.x_offset;
                    let y = component.yx_scale * point.x
                        + component.y_scale * point.y
                        + component.y_offset;
                    crate::font_data::ContourPoint {
                        x,
                        y,
                        point_type: point.point_type,
                        smooth: point.smooth,
                    }
                })
                .collect(),
        });
    }
    for child in &glyph.components {
        let combined = compose_components(component, child);
        collect_component_contours(project, &child.base, &combined, visiting, output)?;
    }
    visiting.remove(name);
    Ok(())
}

fn compose_components(parent: &GlyphComponent, child: &GlyphComponent) -> GlyphComponent {
    GlyphComponent {
        base: child.base.clone(),
        x_scale: parent.x_scale * child.x_scale + parent.xy_scale * child.yx_scale,
        xy_scale: parent.x_scale * child.xy_scale + parent.xy_scale * child.y_scale,
        yx_scale: parent.yx_scale * child.x_scale + parent.y_scale * child.yx_scale,
        y_scale: parent.yx_scale * child.xy_scale + parent.y_scale * child.y_scale,
        x_offset: parent.x_scale * child.x_offset
            + parent.xy_scale * child.y_offset
            + parent.x_offset,
        y_offset: parent.yx_scale * child.x_offset
            + parent.y_scale * child.y_offset
            + parent.y_offset,
    }
}

fn checked_coordinate(x: f64, y: f64) -> Result<(i32, i32), String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("CFF座標が不正です".to_string());
    }
    let x = x.round();
    let y = y.round();
    if x < i32::MIN as f64 || x > i32::MAX as f64 || y < i32::MIN as f64 || y > i32::MAX as f64 {
        return Err("CFF座標が範囲外です".to_string());
    }
    Ok((x as i32, y as i32))
}

fn push_number(output: &mut Vec<u8>, value: i32) {
    if (-107..=107).contains(&value) {
        output.push((value + 139) as u8);
    } else if (108..=1131).contains(&value) {
        let value = value - 108;
        output.push((value / 256 + 247) as u8);
        output.push((value % 256) as u8);
    } else if (-1131..=-108).contains(&value) {
        let value = -value - 108;
        output.push((value / 256 + 251) as u8);
        output.push((value % 256) as u8);
    } else if (-32768..=32767).contains(&value) {
        output.push(28);
        output.extend_from_slice(&(value as i16).to_be_bytes());
    } else {
        output.push(29);
        output.extend_from_slice(&value.to_be_bytes());
    }
}

pub fn encode_dict_integer(value: i32) -> Vec<u8> {
    let mut output = Vec::new();
    if (-107..=107).contains(&value) {
        output.push((value + 139) as u8);
    } else if (108..=1131).contains(&value) {
        let value = value - 108;
        output.extend([(value / 256 + 247) as u8, (value % 256) as u8]);
    } else if (-1131..=-108).contains(&value) {
        let value = -value - 108;
        output.extend([(value / 256 + 251) as u8, (value % 256) as u8]);
    } else {
        output.push(29);
        output.extend_from_slice(&value.to_be_bytes());
    }
    output
}

pub fn encode_index(items: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if items.len() > 0xFFFF {
        return Err("CFF INDEXの要素数が多すぎます".to_string());
    }
    if items.is_empty() {
        return Ok(vec![0, 0]);
    }
    let data_len: usize = items.iter().map(Vec::len).sum();
    let end = data_len
        .checked_add(1)
        .ok_or("CFF INDEXのサイズが不正です")?;
    let off_size = if end <= 0xFF {
        1
    } else if end <= 0xFFFF {
        2
    } else if end <= 0xFF_FFFF {
        3
    } else {
        4
    };
    let mut output = Vec::new();
    output.extend_from_slice(&(items.len() as u16).to_be_bytes());
    output.push(off_size as u8);
    let mut offset = 1usize;
    for item in items {
        let value = offset;
        for shift in (0..off_size).rev() {
            output.push(((value >> (shift * 8)) & 0xFF) as u8);
        }
        offset += item.len();
    }
    let value = offset;
    for shift in (0..off_size).rev() {
        output.push(((value >> (shift * 8)) & 0xFF) as u8);
    }
    for item in items {
        output.extend_from_slice(item);
    }
    Ok(output)
}

pub fn encode_index2(items: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if items.len() > u32::MAX as usize {
        return Err("CFF2 INDEXの要素数が多すぎます".to_string());
    }
    if items.is_empty() {
        return Ok(0u32.to_be_bytes().to_vec());
    }
    let data_len: usize = items.iter().map(Vec::len).sum();
    let end = data_len
        .checked_add(1)
        .ok_or("CFF2 INDEXのサイズが不正です")?;
    let off_size = if end <= 0xFF {
        1
    } else if end <= 0xFFFF {
        2
    } else if end <= 0xFFFFFF {
        3
    } else {
        4
    };
    let mut output = Vec::new();
    output.extend_from_slice(&(items.len() as u32).to_be_bytes());
    output.push(off_size);
    let mut offset = 1usize;
    for item in items {
        write_offset(&mut output, offset, off_size);
        offset += item.len();
    }
    write_offset(&mut output, offset, off_size);
    for item in items {
        output.extend_from_slice(item);
    }
    Ok(output)
}

fn write_offset(output: &mut Vec<u8>, value: usize, off_size: u8) {
    for shift in (0..usize::from(off_size)).rev() {
        output.push(((value >> (shift * 8)) & 0xFF) as u8);
    }
}

pub fn build_minimal_cff2(charstrings: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if charstrings.is_empty() {
        return Err("CFF2には少なくとも.notdefが必要です".to_string());
    }
    let global_subrs = encode_index2(&[])?;
    let charstrings_index = encode_index2(charstrings)?;
    let private_dict = Vec::new();
    let mut top_dict_len = 12usize;
    let mut fd_dict_len = 6usize;
    let mut top_dict = Vec::new();
    for _ in 0..8 {
        let global_offset = 5 + top_dict_len;
        let charstrings_offset = global_offset + global_subrs.len();
        let fd_array_offset = charstrings_offset + charstrings_index.len();
        let fd_array = encode_index2(&[{
            let private_offset = fd_array_offset + 7 + fd_dict_len;
            let mut dict = Vec::new();
            dict.extend(encode_dict_integer(0));
            dict.extend(encode_dict_integer(private_offset as i32));
            dict.push(18); // Private (size, offset)
            dict
        }])?;
        top_dict.clear();
        top_dict.extend(encode_dict_integer(charstrings_offset as i32));
        top_dict.push(17); // CharStrings
        top_dict.extend(encode_dict_integer(fd_array_offset as i32));
        top_dict.extend([12, 36]); // FDArray
        if top_dict.len() == top_dict_len {
            if fd_array.len() == 7 + fd_dict_len {
                let mut output = vec![2, 0, 5];
                output.extend_from_slice(&(top_dict.len() as u16).to_be_bytes());
                output.extend_from_slice(&top_dict);
                output.extend_from_slice(&global_subrs);
                output.extend_from_slice(&charstrings_index);
                output.extend_from_slice(&fd_array);
                output.extend_from_slice(&private_dict);
                return Ok(output);
            }
            fd_dict_len = fd_array.len() - 7;
        } else {
            top_dict_len = top_dict.len();
        }
    }
    Err("CFF2オフセットの計算に失敗しました".to_string())
}

pub fn build_minimal_cff(font_name: &str, charstrings: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if font_name.is_empty() || font_name.len() > 127 {
        return Err("CFFフォント名の長さが不正です".to_string());
    }
    if charstrings.is_empty() {
        return Err("CFFには少なくとも.notdefが必要です".to_string());
    }
    let name_index = encode_index(&[font_name.as_bytes().to_vec()])?;
    let strings: Vec<Vec<u8>> = (1..charstrings.len())
        .map(|gid| format!("gid{gid}").into_bytes())
        .collect();
    let strings_index = encode_index(&strings)?;
    let global_subrs = encode_index(&[])?;
    let charset = {
        let mut bytes = vec![0];
        for sid in 1..charstrings.len() {
            let sid = u16::try_from(391usize + sid).map_err(|_| "CFFのグリフ数が多すぎます")?;
            bytes.extend_from_slice(&sid.to_be_bytes());
        }
        bytes
    };
    let charstrings_index = encode_index(charstrings)?;
    let header_len = 4;
    let top_dict_offset = header_len + name_index.len();
    let mut top_dict_index_len = 13;
    let mut top_dict_index = Vec::new();
    for _ in 0..4 {
        let strings_offset = top_dict_offset + top_dict_index_len;
        let global_subrs_offset = strings_offset + strings_index.len();
        let charset_offset = global_subrs_offset + global_subrs.len();
        let charstrings_offset = charset_offset + charset.len();
        let mut top_dict = Vec::new();
        top_dict.extend(encode_dict_integer(0));
        top_dict.push(20); // defaultWidthX
        top_dict.extend(encode_dict_integer(0));
        top_dict.push(21); // nominalWidthX
        top_dict.extend(encode_dict_integer(charset_offset as i32));
        top_dict.push(15); // charset
        top_dict.extend(encode_dict_integer(charstrings_offset as i32));
        top_dict.push(17); // CharStrings
        top_dict.extend(encode_dict_integer(2));
        top_dict.extend([12, 6]); // CharStringType
        top_dict_index = encode_index(&[top_dict])?;
        if top_dict_index.len() == top_dict_index_len {
            break;
        }
        top_dict_index_len = top_dict_index.len();
    }
    let mut output = vec![1, 0, 4, 4];
    output.extend(name_index);
    output.extend(top_dict_index);
    output.extend(strings_index);
    output.extend(global_subrs);
    output.extend(charset);
    output.extend(charstrings_index);
    Ok(output)
}

pub fn rebuild_sfnt_with_table(
    sfnt: &[u8],
    flavor: [u8; 4],
    replacement_tag: [u8; 4],
    replacement_data: &[u8],
) -> Result<Vec<u8>, String> {
    if sfnt.len() < 12 {
        return Err("SFNTヘッダーが不正です".to_string());
    }
    let count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    if sfnt.len() < 12 + count * 16 {
        return Err("SFNTテーブルディレクトリが不正です".to_string());
    }
    let mut tables = Vec::new();
    let mut replaced = false;
    for index in 0..count {
        let base = 12 + index * 16;
        let tag: [u8; 4] = sfnt[base..base + 4].try_into().unwrap();
        let offset = u32::from_be_bytes(sfnt[base + 8..base + 12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(sfnt[base + 12..base + 16].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(length)
            .ok_or("SFNTテーブル範囲が不正です")?;
        if end > sfnt.len() {
            return Err("SFNTテーブル範囲が不正です".to_string());
        }
        if tag == replacement_tag {
            tables.push((tag, replacement_data.to_vec()));
            replaced = true;
        } else if tag != *b"glyf" && tag != *b"loca" {
            tables.push((tag, sfnt[offset..end].to_vec()));
        }
    }
    if !replaced {
        tables.push((replacement_tag, replacement_data.to_vec()));
    }
    tables.sort_by_key(|(tag, _)| *tag);
    let directory_len = 12 + tables.len() * 16;
    let mut output = vec![0u8; directory_len];
    output[0..4].copy_from_slice(&flavor);
    output[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
    let mut offset = directory_len;
    for (index, (tag, data)) in tables.iter().enumerate() {
        while offset % 4 != 0 {
            offset += 1;
            output.push(0);
        }
        let base = 12 + index * 16;
        output[base..base + 4].copy_from_slice(tag);
        output[base + 4..base + 8].copy_from_slice(&checksum(data).to_be_bytes());
        output[base + 8..base + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        output[base + 12..base + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(data);
        offset += data.len();
    }
    Ok(output)
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut bytes = [0u8; 4];
        bytes[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(bytes));
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_data::ContourPoint;

    #[test]
    fn encodes_line_contour_with_relative_moveto_and_lines() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(10.0, 20.0),
                ContourPoint::on_curve(110.0, 20.0),
                ContourPoint::on_curve(110.0, 120.0),
            ],
        };
        let bytes = encode_type2_contours(&[contour]).unwrap();
        assert_eq!(bytes.last(), Some(&14));
        assert!(bytes.contains(&21));
        assert!(bytes.contains(&5));
    }

    #[test]
    fn rejects_off_curve_points() {
        let contour = Contour {
            points: vec![ContourPoint::off_curve(0.0, 0.0)],
        };
        assert!(encode_type2_contours(&[contour]).is_err());
    }

    #[test]
    fn encodes_cubic_curve_as_rrcurveto() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(10.0, 0.0),
                ContourPoint::off_curve(90.0, 100.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let bytes = encode_type2_contours(&[contour]).unwrap();
        assert!(bytes.contains(&8));
        assert_eq!(bytes.last(), Some(&14));
    }

    #[test]
    fn uses_type2_integer_encoding_boundaries() {
        let mut bytes = Vec::new();
        push_number(&mut bytes, -107);
        push_number(&mut bytes, 107);
        push_number(&mut bytes, 108);
        push_number(&mut bytes, -108);
        push_number(&mut bytes, 1131);
        push_number(&mut bytes, -1131);
        push_number(&mut bytes, 32767);
        push_number(&mut bytes, -32768);
        assert_eq!(bytes[0], 32);
        assert_eq!(bytes[1], 246);
        assert_eq!(bytes[2], 247);
        assert_eq!(bytes[4], 251);
        assert!(bytes.contains(&28));
    }

    #[test]
    fn encodes_empty_and_nonempty_cff_indexes() {
        assert_eq!(encode_index(&[]).unwrap(), vec![0, 0]);
        let index = encode_index(&[vec![1, 2], vec![3]]).unwrap();
        assert_eq!(&index[0..6], &[0, 2, 1, 1, 3, 4]);
        assert_eq!(&index[6..], &[1, 2, 3]);
    }

    #[test]
    fn cff_charstrings_carry_explicit_widths_and_empty_endchar() {
        let bytes = encode_type2_with_width(600.0, &[]).unwrap();
        assert_eq!(bytes.last(), Some(&14));
        assert_eq!(bytes[0], 248); // 600, Type 2 two-byte integer
    }

    #[test]
    fn builds_minimal_cff_with_required_indexes() {
        let table = build_minimal_cff("GlyphStudio", &[vec![14], vec![139, 139, 21, 14]]).unwrap();
        assert_eq!(&table[0..4], &[1, 0, 4, 4]);
        assert!(table.windows(2).any(|pair| pair == [0, 2]));
        assert!(table.ends_with(&[139, 139, 21, 14]));
    }

    #[test]
    fn builds_minimal_cff2_with_charstrings_and_private_dict() {
        let table = build_minimal_cff2(&[Vec::new(), vec![139, 139, 21]]).unwrap();
        assert_eq!(&table[0..3], &[2, 0, 5]);
        assert!(table.len() > 20);
        assert!(table.windows(2).any(|pair| pair == [12, 36]));
        assert_eq!(table.last(), Some(&18)); // Font DICT's Private operator
    }

    #[test]
    fn rebuilds_sfnt_and_replaces_outline_tables() {
        let mut sfnt = vec![0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        sfnt.extend_from_slice(b"CFF ");
        sfnt.extend_from_slice(&[0; 4]);
        sfnt.extend_from_slice(&(44u32).to_be_bytes());
        sfnt.extend_from_slice(&(4u32).to_be_bytes());
        sfnt.extend_from_slice(b"loca");
        sfnt.extend_from_slice(&[0; 4]);
        sfnt.extend_from_slice(&(48u32).to_be_bytes());
        sfnt.extend_from_slice(&(4u32).to_be_bytes());
        sfnt.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let rebuilt = rebuild_sfnt_with_table(&sfnt, *b"OTTO", *b"CFF ", &[9, 8]).unwrap();
        assert_eq!(&rebuilt[0..4], b"OTTO");
        assert_eq!(u16::from_be_bytes([rebuilt[4], rebuilt[5]]), 1);
        assert!(rebuilt.windows(4).any(|tag| tag == b"CFF "));
        assert!(!rebuilt.windows(4).any(|tag| tag == b"loca"));
    }
}
