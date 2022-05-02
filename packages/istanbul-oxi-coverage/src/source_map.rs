use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceMap {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources_content: Option<Vec<Option<String>>>,
    pub names: Vec<String>,
    pub mappings: String,
}

impl Default for SourceMap {
    fn default() -> Self {
        SourceMap {
            version: 3,
            file: Default::default(),
            source_root: Default::default(),
            sources: Default::default(),
            sources_content: Default::default(),
            names: Default::default(),
            mappings: "".to_string(),
        }
    }
}
