use istanbul_oxi_instrument::SourceCoverage;
use serde_json::Value;
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

mod constants;
mod instrument;
mod template;
#[macro_use]
mod macros;
mod options;
mod utils;
mod visitors;
pub use options::InstrumentOptions;
use template::create_coverage_fn_decl::create_coverage_fn_ident;
use tracing_subscriber::fmt::format::FmtSpan;
pub use visitors::coverage_visitor;

use tracing::Level;

use visitors::coverage_visitor::CoverageVisitor;

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    tracing_subscriber::fmt()
        // TODO: runtime config
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_ansi(false)
        .event_format(tracing_subscriber::fmt::format().pretty())
        .init();

    let context: Value = serde_json::from_str(&metadata.transform_context)
        .expect("Should able to deserialize context");
    let filename = if let Some(filename) = (&context["filename"]).as_str() {
        filename
    } else {
        "unknown.js"
    };

    // create a function name ident for the injected coverage instrumentation counters.
    create_coverage_fn_ident(filename);

    let instrument_options_value: Value = serde_json::from_str(&metadata.plugin_config)
        .expect("Should able to deserialize plugin config");
    let instrument_options = InstrumentOptions {
        coverage_variable: instrument_options_value["coverageVariable"]
            .as_str()
            .unwrap_or("__coverage__")
            .to_string(),
        compact: instrument_options_value["compact"]
            .as_bool()
            .unwrap_or(false),
        report_logic: instrument_options_value["reportLogic"]
            .as_bool()
            .unwrap_or(false),
        ignore_class_methods: instrument_options_value["ignoreClassMethods"]
            .as_array()
            .map(|v| {
                v.iter()
                    .map(|m| m.as_str().expect("Should be a valid string").to_string())
                    .collect()
            })
            .unwrap_or_default(),
    };

    let mut cov = SourceCoverage::new(filename.to_string(), instrument_options.report_logic);
    let source_map: Option<istanbul_oxi_instrument::SourceMap> =
        serde_json::from_str(&instrument_options_value["inputSourceMap"].to_string()).ok();
    cov.set_input_source_map(source_map);

    let nodes = vec![];
    let visitor = CoverageVisitor::new(
        &metadata.source_map,
        metadata.comments.as_ref(),
        &mut cov,
        &instrument_options,
        &nodes,
        None,
        filename.to_string(),
        Default::default(),
        None,
        Default::default(),
        None,
    );

    program.fold_with(&mut as_folder(visitor))
}
