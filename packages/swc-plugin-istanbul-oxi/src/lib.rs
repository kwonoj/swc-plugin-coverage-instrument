use istanbul_oxi_instrument::SourceCoverage;
use serde_json::Value;
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

mod constants;
mod instrument;
mod template;
#[macro_use]
mod macros;
#[macro_use]
mod utils;
mod options;
mod visitors;
pub use options::InstrumentOptions;
use template::create_coverage_fn_decl::{create_coverage_fn_ident, COVERAGE_FN_IDENT};
use tracing_subscriber::fmt::format::FmtSpan;
pub use visitors::coverage_visitor;

use tracing::Level;

use visitors::coverage_visitor::CoverageVisitor;

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let context: Value = serde_json::from_str(&metadata.transform_context)
        .expect("Should able to deserialize context");
    let filename = if let Some(filename) = (&context["filename"]).as_str() {
        filename
    } else {
        "unknown.js"
    };

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
    };

    tracing_subscriber::fmt()
        // TODO: runtime config
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_ansi(false)
        .event_format(tracing_subscriber::fmt::format().pretty())
        .init();

    create_coverage_fn_ident(filename);

    let mut cov = SourceCoverage::new(filename.to_string(), instrument_options.report_logic);
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
        Default::default(),
        None,
    );

    program.fold_with(&mut as_folder(visitor))
}
