use istanbul_oxi_instrument::{create_coverage_instrumentation_visitor, InstrumentOptions};
use serde_json::Value;
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

use tracing_subscriber::fmt::format::FmtSpan;

use tracing::Level;

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
        input_source_map: serde_json::from_str(
            &instrument_options_value["inputSourceMap"].to_string(),
        )
        .ok(),
    };

    let visitor = create_coverage_instrumentation_visitor(
        &std::rc::Rc::new(metadata.source_map),
        &metadata.comments,
        &instrument_options,
        filename,
    );

    program.fold_with(&mut as_folder(visitor))
}
