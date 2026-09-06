
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontMaster {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub width: f64,
    pub is_bracket: bool,
    #[serde(default)]
    pub axes: HashMap<String, f64>,
}
