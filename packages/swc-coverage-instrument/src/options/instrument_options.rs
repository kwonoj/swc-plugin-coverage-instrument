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
    pub debug_initial_coverage_comment: bool,
    // Allow to specify which files should be excluded from instrumentation.
    // This option accepts an array of wax(https://crates.io/crates/wax)-compatible glob patterns
    // and will match against the filename provided by swc's core.
    pub unstable_exclude: Option<Vec<String>>,
    // The expression used to locate the global scope object where coverage
    // counters are stored. Mirrors babel-plugin-istanbul's `coverageGlobalScope`
    // (default `"this"`). When `coverage_global_scope_func` is false, this string
    // is emitted verbatim as the initializer (e.g. an inline
    // `globalThis`/`self`/`window`/`global` lookup) instead of being wrapped in a
    // `Function` constructor, which some Content Security Policies forbid.
    // See https://github.com/istanbuljs/babel-plugin-istanbul/issues/212
    pub coverage_global_scope: String,
    // When true (default, matching istanbul), the global scope is located via an
    // evaluated `Function` constructor. Set to false to emit `coverage_global_scope`
    // directly and avoid the eval-like construct.
    pub coverage_global_scope_func: bool,
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
            debug_initial_coverage_comment: false,
            unstable_exclude: Default::default(),
            coverage_global_scope: "this".to_string(),
            coverage_global_scope_func: true,
        }
    }
}
