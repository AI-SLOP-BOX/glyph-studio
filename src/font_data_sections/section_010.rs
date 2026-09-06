
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorGradientExtend {
    #[default]
    Pad,
    Repeat,
    Reflect,
}
