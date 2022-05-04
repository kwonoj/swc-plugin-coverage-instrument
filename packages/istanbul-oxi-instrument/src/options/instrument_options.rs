use istanbul_oxi_coverage::SourceMap;

#[derive(Debug, Clone)]
pub struct InstrumentOptions {
    pub coverage_variable: String,
    pub compact: bool,
    pub report_logic: bool,
    pub ignore_class_methods: Vec<String>,
    pub input_source_map: Option<SourceMap>,
}
