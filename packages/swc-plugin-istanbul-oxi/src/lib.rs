use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use once_cell::sync::Lazy;
use serde_json::Value;
use swc_plugin::{
    ast::*,
    comments::{Comment, Comments, PluginCommentsProxy},
    plugin_transform, TransformPluginProgramMetadata,
};

use regex::Regex as Regexp;

/// This is not fully identical to original file comments
/// https://github.com/istanbuljs/istanbuljs/blob/6f45283fe31faaa066375528f6b68e3a9927b2d5/packages/istanbul-lib-instrument/src/visitor.js#L10=
/// as regex package doesn't support lookaround
static COMMENT_FILE_REGEX: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"^\s*istanbul\s+ignore\s+(file)(\W|$)").unwrap());

struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}

/// Internal visitor
struct CoverageState {
    var_name: String,
    attrs: UnknownReserved,
    next_ignore: Option<UnknownReserved>,
    cov: UnknownReserved,
    ignore_class_method: UnknownReserved,
    types: UnknownReserved,
    source_mapping_url: Option<UnknownReserved>,
    report_logic: bool,
}

impl CoverageState {
    pub fn new(
        var_name: &str,
        attrs: UnknownReserved,
        next_ignore: Option<UnknownReserved>,
        cov: UnknownReserved,
        ignore_class_method: UnknownReserved,
        types: UnknownReserved,
        source_mapping_url: Option<UnknownReserved>,
        report_logic: bool,
    ) -> CoverageState {
        let var_name_hash = CoverageState::get_var_name_hash(var_name);

        CoverageState {
            var_name: var_name_hash,
            attrs,
            next_ignore,
            cov,
            ignore_class_method,
            types,
            source_mapping_url,
            report_logic,
        }
    }

    fn get_var_name_hash(name: &str) -> String {
        let mut s = DefaultHasher::new();
        name.hash(&mut s);
        return format!("cov_{}", s.finish());
    }
}

impl VisitMut for CoverageState {}

/// Parent visitor
struct CoverageVisitor {
    comments: Option<PluginCommentsProxy>,
    filename: String,
}

impl CoverageVisitor {
    fn should_ignore_file(&mut self, program: &Program) -> bool {
        if let Some(comments) = &self.comments {
            let pos = match program {
                Program::Module(module) => module.span,
                Program::Script(script) => script.span,
            };

            let validate_comments = |comments: &Option<Vec<Comment>>| {
                if let Some(comments) = comments {
                    comments
                        .iter()
                        .any(|comment| COMMENT_FILE_REGEX.is_match(&comment.text))
                } else {
                    false
                }
            };

            vec![
                comments.get_leading(pos.lo),
                comments.get_leading(pos.hi),
                comments.get_trailing(pos.lo),
                comments.get_trailing(pos.hi),
            ]
            .iter()
            .any(|c| validate_comments(c))
        } else {
            false
        }
    }

    /// Not implemented.
    /// TODO: is this required?
    fn is_instrumented_already(&self) -> bool {
        return false;
    }
}

impl VisitMut for CoverageVisitor {
    fn visit_mut_program(&mut self, program: &mut Program) {
        if self.should_ignore_file(program) {
            return;
        }

        if self.is_instrumented_already() {
            return;
        }

        let mut state = CoverageState::new(
            self.filename.as_str(),
            UnknownReserved,
            None,
            UnknownReserved,
            UnknownReserved,
            UnknownReserved,
            None,
            false,
        );
        program.visit_mut_children_with(&mut state);
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

    program.fold_with(&mut as_folder(CoverageVisitor {
        comments: metadata.comments,
        filename: filename.to_string(),
    }))
}
