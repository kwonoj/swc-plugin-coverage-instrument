use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::SourceCoverage;
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
struct CoverageVisitor {
    comments: Option<PluginCommentsProxy>,
    var_name: String,
    attrs: UnknownReserved,
    next_ignore: Option<UnknownReserved>,
    cov: SourceCoverage,
    ignore_class_method: UnknownReserved,
    types: UnknownReserved,
    source_mapping_url: Option<UnknownReserved>,
    report_logic: bool,
}

impl CoverageVisitor {
    pub fn new(
        comments: Option<PluginCommentsProxy>,
        var_name: &str,
        attrs: UnknownReserved,
        next_ignore: Option<UnknownReserved>,
        cov: SourceCoverage,
        ignore_class_method: UnknownReserved,
        types: UnknownReserved,
        source_mapping_url: Option<UnknownReserved>,
        report_logic: bool,
    ) -> CoverageVisitor {
        let var_name_hash = CoverageVisitor::get_var_name_hash(var_name);

        CoverageVisitor {
            comments,
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

    /// Not implemented.
    /// TODO: is this required?
    fn is_instrumented_already(&self) -> bool {
        return false;
    }

    fn on_enter(&mut self) {}

    fn on_exit(&mut self) {}
}

impl VisitMut for CoverageVisitor {
    fn visit_mut_program(&mut self, program: &mut Program) {
        if should_ignore_file(&self.comments, program) {
            return;
        }

        if self.is_instrumented_already() {
            return;
        }

        program.visit_mut_children_with(self);
    }

    fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
        self.on_enter();

        match stmt {
            Stmt::For(for_stmt) => {}
            _ => {}
        }
        stmt.visit_mut_children_with(self);
        self.on_exit();
    }

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        for item in items {
            item.visit_mut_children_with(self);
        }

        if self.is_instrumented_already() {
            return;
        }

        self.cov.freeze();
        /*
        visitState.cov.freeze();
        const coverageData = visitState.cov.toJSON();
        if (shouldIgnoreFile(path.find(p => p.isProgram()))) {
            return {
                fileCoverage: coverageData,
                sourceMappingURL: visitState.sourceMappingURL
            };
        }
        coverageData[MAGIC_KEY] = MAGIC_VALUE;
        const hash = createHash(SHA)
            .update(JSON.stringify(coverageData))
            .digest('hex');
        coverageData.hash = hash;
        const coverageNode = T.valueToNode(coverageData);
        delete coverageData[MAGIC_KEY];
        delete coverageData.hash;
        let gvTemplate;
        if (opts.coverageGlobalScopeFunc) {
            if (path.scope.getBinding('Function')) {
                gvTemplate = globalTemplateAlteredFunction({
                    GLOBAL_COVERAGE_SCOPE: T.stringLiteral(
                        'return ' + opts.coverageGlobalScope
                    )
                });
            } else {
                gvTemplate = globalTemplateFunction({
                    GLOBAL_COVERAGE_SCOPE: T.stringLiteral(
                        'return ' + opts.coverageGlobalScope
                    )
                });
            }
        } else {
            gvTemplate = globalTemplateVariable({
                GLOBAL_COVERAGE_SCOPE: opts.coverageGlobalScope
            });
        }
        const cv = coverageTemplate({
            GLOBAL_COVERAGE_VAR: T.stringLiteral(opts.coverageVariable),
            GLOBAL_COVERAGE_TEMPLATE: gvTemplate,
            COVERAGE_FUNCTION: T.identifier(visitState.varName),
            PATH: T.stringLiteral(sourceFilePath),
            INITIAL: coverageNode,
            HASH: T.stringLiteral(hash)
        });
        // explicitly call this.varName to ensure coverage is always initialized
        path.node.body.unshift(
            T.expressionStatement(
                T.callExpression(T.identifier(visitState.varName), [])
            )
        );
        path.node.body.unshift(cv);
        return {
            fileCoverage: coverageData,
            sourceMappingURL: visitState.sourceMappingURL
        };
        */
    }
}

fn should_ignore_file(comments: &Option<PluginCommentsProxy>, program: &Program) -> bool {
    if let Some(comments) = &comments {
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

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let context: Value = serde_json::from_str(&metadata.transform_context)
        .expect("Should able to deserialize context");
    let filename = if let Some(filename) = (&context["filename"]).as_str() {
        filename
    } else {
        "unknown.js"
    };

    //TODO: support plugin options
    let report_logic = false;

    let mut visitor = CoverageVisitor::new(
        metadata.comments,
        filename,
        UnknownReserved,
        None,
        SourceCoverage::new(filename.to_string(), report_logic),
        UnknownReserved,
        UnknownReserved,
        None,
        report_logic,
    );

    program.fold_with(&mut as_folder(visitor))
}
