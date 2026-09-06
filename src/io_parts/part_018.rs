fn parse_glyphs_feature_source(source: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("feature ") {
        let candidate = &rest[start + "feature ".len()..];
        let Some(open) = candidate.find('{') else {
            break;
        };
        let tag = candidate[..open].trim();
        let Some(close) = candidate[open + 1..].find('}') else {
            break;
        };
        result.push((
            tag.to_string(),
            candidate[open + 1..open + 1 + close].trim().to_string(),
        ));
        rest = &candidate[open + 1 + close + 1..];
    }
    result
}
