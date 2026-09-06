
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorGradientKind {
    #[default]
    Linear,
    Radial,
    Sweep,
}
