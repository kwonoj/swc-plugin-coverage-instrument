use swc_core::{
    ecma::{ast::Program, visit::*},
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};
use swc_coverage_instrument::{
    create_coverage_instrumentation_visitor, InstrumentLogOptions, InstrumentOptions,
};

use tracing_subscriber::fmt::format::FmtSpan;
use wax::Pattern;

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
    let filename = metadata.get_context(&TransformPluginMetadataContextKind::Filename);
    let filename = if let Some(filename) = filename.as_deref() {
        filename
    } else {
        "unknown.js"
    };

    let plugin_config = metadata.get_transform_plugin_config();
    let instrument_options: InstrumentOptions = if let Some(plugin_config) = plugin_config {
        serde_json::from_str(&plugin_config).unwrap_or_else(|f| {
            println!("Could not deserialize instrumentation option");
            println!("{:#?}", f);
            Default::default()
        })
    } else {
        Default::default()
    };

    // Unstable option to exclude files from coverage. If pattern is wax(https://crates.io/crates/wax)
    // compatible glob and the filename matches the pattern, the file will not be instrumented.
    // Note that the filename is provided by swc's core, may not be the full absolute path to the file name.
    if let Some(exclude) = &instrument_options.unstable_exclude {
        match wax::any(exclude.iter().map(|s| s.as_ref()).collect::<Vec<&str>>()) {
            Ok(p) => {
                if p.is_match(filename) {
                    return program;
                }
            }
            Err(e) => {
                println!("Could not parse unstable_exclude option, will be ignored");
                println!("{:#?}", e);
            }
        }
    }

    initialize_instrumentation_log(&instrument_options.instrument_log);

    let visitor = create_coverage_instrumentation_visitor(
        std::sync::Arc::new(metadata.source_map),
        metadata.comments.as_ref(),
        instrument_options,
        filename.to_string(),
    );

    program.apply(&mut visit_mut_pass(visitor))
}
