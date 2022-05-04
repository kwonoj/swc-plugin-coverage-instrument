use serde_json::Value;
use swc_coverage_instrument::{
    create_coverage_instrumentation_visitor, InstrumentLogOptions, InstrumentOptions,
};
use swc_plugin::{
    ast::{as_folder, FoldWith, Program},
    plugin_transform, TransformPluginProgramMetadata,
};

use tracing_subscriber::fmt::format::FmtSpan;

fn initialize_instrumentation_log(log_options: &InstrumentLogOptions) {
    let log_level = match log_options.level.as_deref() {
        Some("error") => Some(tracing::Level::ERROR),
        Some("debug") => Some(tracing::Level::DEBUG),
        Some("info") => Some(tracing::Level::INFO),
        Some("warn") => Some(tracing::Level::WARN),
        Some("trace") => Some(tracing::Level::TRACE),
        _ => None,
    };

    if let Some(log_level) = log_level {
        let builder = tracing_subscriber::fmt().with_max_level(log_level);

        let builder = if log_options.enable_trace {
            builder.with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        } else {
            builder
        };

        builder
            .with_ansi(false)
            .event_format(tracing_subscriber::fmt::format().pretty())
            .init();
    }
}

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let context: Value = serde_json::from_str(&metadata.transform_context)
        .expect("Should able to deserialize context");
    let filename = if let Some(filename) = (&context["filename"]).as_str() {
        filename
    } else {
        "unknown.js"
    };

    let instrument_options: InstrumentOptions = serde_json::from_str(&metadata.plugin_config)
        .unwrap_or_else(|f| {
            println!("Could not deserialize instrumentation option");
            println!("{:#?}", f);
            Default::default()
        });

    initialize_instrumentation_log(&instrument_options.instrument_log);

    let visitor = create_coverage_instrumentation_visitor(
        &std::sync::Arc::new(metadata.source_map),
        metadata.comments.as_ref(),
        &instrument_options,
        filename,
    );

    program.fold_with(&mut as_folder(visitor))
}
