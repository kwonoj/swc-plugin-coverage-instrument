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
    plugin_transform,
    syntax_pos::DUMMY_SP,
    utils::{quote, take::Take},
    TransformPluginProgramMetadata,
};

use regex::Regex as Regexp;

/// This is not fully identical to original file comments
/// https://github.com/istanbuljs/istanbuljs/blob/6f45283feo31faaa066375528f6b68e3a9927b2d5/packages/istanbul-lib-instrument/src/visitor.js#L10=
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
    file_path: String,
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
            file_path: var_name.to_string(),
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

/// Returns `var $var_decl_name = $value;`
fn get_assignment_stmt(var_decl_name: &str, value: Expr) -> (Ident, Stmt) {
    let ident = Ident::new(var_decl_name.into(), DUMMY_SP);

    let stmt = Stmt::Decl(Decl::Var(VarDecl {
        kind: VarDeclKind::Var,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Assign(AssignPat {
                span: DUMMY_SP,
                left: Box::new(Pat::Ident(BindingIdent::from(ident.clone()))),
                right: Box::new(value),
                type_ann: None,
            }),
            init: None,
            definite: false,
        }],
        ..VarDecl::dummy()
    }));

    (ident, stmt)
}

/// Create a function declaration for the collecting coverage.
fn create_coverage_fn_decl(
    coverage_variable: &str,
    global_ident: Ident,
    coverage_template: Stmt,
    var_name: &str,
    file_path: &str,
) -> (Ident, ModuleItem) {
    // Actual fn body statements will be injected
    let mut stmts = vec![];

    // var path = $file_path;
    let (path_ident, path_stmt) = get_assignment_stmt(
        "path",
        Expr::Lit(Lit::Str(Str {
            value: file_path.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(path_stmt);

    // var hash = $HASH;
    let (hash_ident, hash_stmt) = get_assignment_stmt(
        "hash",
        Expr::Lit(Lit::Str(Str {
            value: "TODO".into(),
            ..Str::dummy()
        })),
    );
    stmts.push(hash_stmt);

    // var global = new Function("return $global_coverage_scope")();
    stmts.push(coverage_template);

    // var gcv = ${coverage_variable};
    let (gcv_ident, gcv_stmt) = get_assignment_stmt(
        "gcv",
        Expr::Lit(Lit::Str(Str {
            value: coverage_variable.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(gcv_stmt);

    // var coverageData = INITIAL;
    let (coverage_data_ident, coverage_data_stmt) = get_assignment_stmt(
        "coverageData",
        Expr::Lit(Lit::Str(Str {
            value: "TODO".into(),
            ..Str::dummy()
        })),
    );
    stmts.push(coverage_data_stmt);

    let coverage_ident = Ident::new("coverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $coverage = $global[$gcv] || ($global[$gcv] = {})" as Stmt,
        coverage = coverage_ident.clone(),
        gcv = gcv_ident.clone(),
        global = global_ident.clone()
    ));

    stmts.push(quote!(
        r#"
    if (!$coverage[$path] || $coverage[$path].$hash !== $hash) {
        $coverage[$path] = $coverage_data;
    }
    "# as Stmt,
        coverage = coverage_ident.clone(),
        path = path_ident.clone(),
        hash = hash_ident.clone(),
        coverage_data = coverage_data_ident.clone()
    ));

    // var actualCoverage = coverage[path];
    let actual_coverage_ident = Ident::new("actualCoverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $actual_coverage = $coverage[$path];" as Stmt,
        actual_coverage = actual_coverage_ident.clone(),
        coverage = coverage_ident.clone(),
        path = path_ident.clone()
    ));

    let coverage_fn_ident = Ident::new(var_name.into(), DUMMY_SP);
    //
    //COVERAGE_FUNCTION = function () {
    //   return actualCoverage;
    //}
    // TODO: need to add @ts-ignore leading comment
    let coverage_fn_assign_expr = Expr::Assign(AssignExpr {
        left: PatOrExpr::Pat(Box::new(Pat::Ident(BindingIdent::from(
            coverage_fn_ident.clone(),
        )))),
        right: Box::new(Expr::Fn(FnExpr {
            ident: None,
            function: Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(Box::new(Expr::Ident(actual_coverage_ident.clone()))),
                    })],
                }),
                ..Function::dummy()
            },
        })),
        ..AssignExpr::dummy()
    });

    stmts.push(Stmt::Block(BlockStmt {
        stmts: vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(coverage_fn_assign_expr),
        })],
        ..BlockStmt::dummy()
    }));

    stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(Box::new(Expr::Ident(actual_coverage_ident.clone()))),
    }));

    // moduleitem for fn decl includes body defined above
    let module_item = ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
        ident: coverage_fn_ident.clone(),
        declare: false,
        function: Function {
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts,
            }),
            ..Function::dummy()
        },
    })));

    (coverage_fn_ident, module_item)
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
        for item in items.iter_mut() {
            item.visit_mut_children_with(self);
        }

        if self.is_instrumented_already() {
            return;
        }

        self.cov.freeze();

        //TODO: option: global coverage variable scope. (optional, default `this`)
        let coverage_global_scope = "this";
        //TODO: option: use an evaluated function to find coverageGlobalScope.
        let coverage_global_scope_func = true;
        // TODO: option: name of global coverage variable. (optional, default `__coverage__`)
        let coverage_variable = "__coverage__";

        let sealed = self.cov.get_sealed_inner();

        /*
        TODO:
        const coverageNode = T.valueToNode(coverageData);
        delete coverageData[MAGIC_KEY];
        delete coverageData.hash;
        */

        let gv_template = if coverage_global_scope_func {
            // TODO: path.scope.getBinding('Function')
            let is_function_binding_scope = false;

            if is_function_binding_scope {
                /*
                gvTemplate = globalTemplateAlteredFunction({
                    GLOBAL_COVERAGE_SCOPE: T.stringLiteral(
                        'return ' + opts.coverageGlobalScope
                    )
                });
                 */
                unimplemented!("");
            } else {
                // var global = new Function("return $global_coverage_scope")();
                let expr = Expr::New(NewExpr {
                    callee: Box::new(Expr::Ident(Ident::new("Function".into(), DUMMY_SP))),
                    args: Some(vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Str(Str {
                            value: format!("return {}", coverage_global_scope).into(),
                            ..Str::dummy()
                        }))),
                    }]),
                    ..NewExpr::dummy()
                });

                get_assignment_stmt(
                    "global",
                    Expr::Call(CallExpr {
                        callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                            span: DUMMY_SP,
                            expr: Box::new(expr),
                        }))),
                        ..CallExpr::dummy()
                    }),
                )
            }
        } else {
            unimplemented!("");
            /*
            gvTemplate = globalTemplateVariable({
                GLOBAL_COVERAGE_SCOPE: opts.coverageGlobalScope
            });
            */
        };

        // INITIAL (valueToNode(coverageData)), HASH
        let (coverage_fn_ident, coverage_template) = create_coverage_fn_decl(
            &coverage_variable,
            gv_template.0,
            gv_template.1,
            &self.var_name,
            &self.file_path,
        );

        // prepend template to the top of the code
        items.insert(0, coverage_template);

        // explicitly call this.varName to ensure coverage is always initialized
        let m = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Ident(coverage_fn_ident))),
                ..CallExpr::dummy()
            })),
        }));
        items.insert(1, m);
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

    let visitor = CoverageVisitor::new(
        metadata.comments,
        filename,
        UnknownReserved,
        None,
        SourceCoverage::from_file_path(filename.to_string(), report_logic),
        UnknownReserved,
        UnknownReserved,
        None,
        report_logic,
    );

    program.fold_with(&mut as_folder(visitor))
}
