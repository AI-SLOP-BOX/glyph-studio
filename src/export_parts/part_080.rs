
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
