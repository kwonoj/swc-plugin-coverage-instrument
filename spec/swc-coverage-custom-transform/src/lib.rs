#![recursion_limit = "2048"]
#![allow(dead_code)]

mod util;

#[macro_use]
extern crate napi_derive;
/// Explicit extern crate to use allocator.
extern crate swc_node_base;

use std::{env, panic::set_hook, sync::Arc};

use backtrace::Backtrace;
use swc::Compiler;
use swc_common::{self, sync::Lazy, FilePathMapping, SourceMap};

use std::path::Path;

use anyhow::Context as _;
use napi::bindgen_prelude::Buffer;
use swc::{config::Options, TransformOutput};
use swc_common::FileName;
use swc_ecma_ast::Program;

use crate::util::{deserialize_json, get_deserialized, try_with, MapErr};

static COMPILER: Lazy<Arc<Compiler>> = Lazy::new(|| {
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));

    Arc::new(Compiler::new(cm))
});

#[napi::module_init]
fn init() {
    if cfg!(debug_assertions) || env::var("SWC_DEBUG").unwrap_or_default() == "1" {
        set_hook(Box::new(|panic_info| {
            let backtrace = Backtrace::new();
            println!("Panic: {:?}\nBacktrace: {:?}", panic_info, backtrace);
        }));
    }
}

fn get_compiler() -> Arc<Compiler> {
    COMPILER.clone()
}

#[napi(js_name = "Compiler")]
pub struct JsCompiler {
    _compiler: Arc<Compiler>,
}

#[napi]
impl JsCompiler {
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            _compiler: COMPILER.clone(),
        }
    }
}

pub type ArcCompiler = Arc<Compiler>;

#[napi]
pub fn transform_sync(s: String, is_module: bool, opts: Buffer) -> napi::Result<TransformOutput> {
    let c = get_compiler();

    let mut options: Options = get_deserialized(&opts)?;

    if !options.filename.is_empty() {
        options.config.adjust(Path::new(&options.filename));
    }

    try_with(c.cm.clone(), !options.config.error.filename, |handler| {
        c.run(|| {
            if is_module {
                let program: Program =
                    deserialize_json(s.as_str()).context("failed to deserialize Program")?;
                c.process_js(handler, program, &options)
            } else {
                let fm = c.cm.new_source_file(
                    if options.filename.is_empty() {
                        FileName::Anon
                    } else {
                        FileName::Real(options.filename.clone().into())
                    },
                    s,
                );
                c.process_js_file(fm, handler, &options)
            }
        })
    })
    .convert_err()
}
