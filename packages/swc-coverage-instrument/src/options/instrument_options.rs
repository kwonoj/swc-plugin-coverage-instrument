use istanbul_oxide::SourceMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstrumentLogOptions {
    pub level: Option<String>,
    pub enable_trace: bool,
}

impl Default for InstrumentLogOptions {
    fn default() -> Self {
        InstrumentLogOptions {
            level: None,
            enable_trace: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstrumentOptions {
    pub coverage_variable: String,
    pub compact: bool,
    pub report_logic: bool,
    pub ignore_class_methods: Vec<String>,
    pub input_source_map: Option<SourceMap>,
    pub instrument_log: InstrumentLogOptions,
}

impl Default for InstrumentOptions {
    fn default() -> Self {
        InstrumentOptions {
            coverage_variable: "__coverage__".to_string(),
            compact: false,
            report_logic: false,
            ignore_class_methods: Default::default(),
            input_source_map: Default::default(),
            instrument_log: Default::default(),
        }
    }
}
