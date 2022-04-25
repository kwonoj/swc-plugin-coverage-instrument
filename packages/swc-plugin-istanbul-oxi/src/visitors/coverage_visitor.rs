use istanbul_oxi_instrument::SourceCoverage;
use once_cell::sync::Lazy;
use regex::Regex as Regexp;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use swc_plugin::{
    ast::*,
    comments::{Comment, CommentKind, Comments, PluginCommentsProxy},
    source_map::PluginSourceMapProxy,
    syntax_pos::DUMMY_SP,
    utils::take::Take,
};

use crate::{
    constants::idents::*,
    instrument::build_increase_expression_expr,
    template::{
        create_coverage_fn_decl::create_coverage_fn_decl,
        create_global_stmt_template::create_global_stmt_template,
    },
    utils::lookup_range::get_range_from_span,
    visit_mut_prepend_statement_counter, InstrumentOptions,
};

use super::stmt_like_visitor::StmtVisitor;

pub struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}

/// This is not fully identical to original file comments
/// https://github.com/istanbuljs/istanbuljs/blob/6f45283feo31faaa066375528f6b68e3a9927b2d5/packages/istanbul-lib-instrument/src/visitor.js#L10=
/// as regex package doesn't support lookaround
static COMMENT_FILE_REGEX: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"^\s*istanbul\s+ignore\s+(file)(\W|$)").unwrap());

pub struct CoverageVisitor<'a> {
    comments: Option<&'a PluginCommentsProxy>,
    source_map: &'a PluginSourceMapProxy,
    var_name: String,
    var_name_ident: Ident,
    file_path: String,
    attrs: UnknownReserved,
    next_ignore: Option<UnknownReserved>,
    cov: SourceCoverage,
    ignore_class_method: UnknownReserved,
    types: UnknownReserved,
    source_mapping_url: Option<UnknownReserved>,
    instrument_options: InstrumentOptions,
    before: Vec<Stmt>,
}

impl<'a> CoverageVisitor<'a> {
    pub fn new(
        comments: Option<&'a PluginCommentsProxy>,
        source_map: &'a PluginSourceMapProxy,
        var_name: &str,
        attrs: UnknownReserved,
        next_ignore: Option<UnknownReserved>,
        cov: SourceCoverage,
        ignore_class_method: UnknownReserved,
        types: UnknownReserved,
        source_mapping_url: Option<UnknownReserved>,
        instrument_options: InstrumentOptions,
    ) -> CoverageVisitor<'a> {
        let var_name_hash = CoverageVisitor::get_var_name_hash(var_name);

        CoverageVisitor {
            comments,
            source_map,
            var_name: var_name_hash.clone(),
            var_name_ident: Ident::new(var_name_hash.into(), DUMMY_SP),
            file_path: var_name.to_string(),
            attrs,
            next_ignore,
            cov,
            ignore_class_method,
            types,
            source_mapping_url,
            instrument_options,
            before: vec![],
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

    fn visit_mut_fn(&mut self, ident: &Option<&Ident>, function: &mut Function) {
        let (span, name) = if let Some(ident) = &ident {
            (&ident.span, Some(ident.sym.to_string()))
        } else {
            (&function.span, None)
        };

        let range = get_range_from_span(self.source_map, span);
        let body_range = get_range_from_span(self.source_map, &function.span);
        let index = self.cov.new_function(&name, &range, &body_range);

        match &mut function.body {
            Some(blockstmt) => {
                let b = build_increase_expression_expr(&IDENT_F, index, &self.var_name_ident, None);
                let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(b),
                })];
                prepended_vec.extend(blockstmt.stmts.take());
                blockstmt.stmts = prepended_vec;
            }
            _ => {
                unimplemented!("Unable to process function body node type")
            }
        }
    }
}

impl VisitMut for CoverageVisitor<'_> {
    fn visit_mut_program(&mut self, program: &mut Program) {
        if should_ignore_file(&self.comments, program) {
            return;
        }

        if self.is_instrumented_already() {
            return;
        }

        program.visit_mut_children_with(self);

        let span = match &program {
            Program::Module(m) => m.span,
            Program::Script(s) => s.span,
        };

        let coverage_data_json_str = serde_json::to_string(self.cov.as_ref())
            .expect("Should able to serialize coverage data");

        // Append coverage data as stringified JSON comments at the bottom of transformed code.
        // Currently plugin does not have way to pass any other data to the host except transformed program.
        // This attaches arbitary data to the transformed code itself to retrieve it.
        self.comments.add_trailing(
            span.hi,
            Comment {
                kind: CommentKind::Block,
                span: DUMMY_SP,
                text: format!("__coverage_data_json_comment__::{}", coverage_data_json_str).into(),
            },
        );
    }

    fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
        fn_expr.visit_mut_children_with(self);

        self.visit_mut_fn(&fn_expr.ident.as_ref(), &mut fn_expr.function);
    }

    fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
        fn_decl.visit_mut_children_with(self);

        self.visit_mut_fn(&Some(&fn_decl.ident), &mut fn_decl.function);
    }

    /// Visit variable declarator, inject a statement increase expr by wrapping declaration init with paren.
    /// var x = 0
    /// ->
    /// var x = (cov_18biir0b3p().s[3]++, 0)
    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        if let Some(init) = &mut declarator.init {
            // TODO: this is not complete
            let expr_span = match &mut **init {
                Expr::Lit(Lit::Str(Str { span, .. }))
                | Expr::Lit(Lit::Num(Number { span, .. }))
                | Expr::Call(CallExpr { span, .. })
                | Expr::Assign(AssignExpr { span, .. })
                | Expr::Object(ObjectLit { span, .. }) => span,
                _ => {
                    todo!("not implemented")
                }
            };

            let init_range = get_range_from_span(self.source_map, expr_span);

            let idx = self.cov.new_statement(&init_range);
            let increment_expr =
                build_increase_expression_expr(&IDENT_S, idx, &self.var_name_ident, None);

            let paren_expr = Expr::Paren(ParenExpr {
                span: expr_span.take(),
                expr: Box::new(Expr::Seq(SeqExpr {
                    span: DUMMY_SP,
                    exprs: vec![Box::new(increment_expr), init.take()],
                })),
            });

            // replace init with increase expr + init seq
            **init = paren_expr;
        }

        declarator.visit_mut_children_with(self);
    }

    // Insert statement counter for For(in, of)Stmt
    visit_mut_prepend_statement_counter!(visit_mut_for_stmt, ForStmt);
    visit_mut_prepend_statement_counter!(visit_mut_for_of_stmt, ForOfStmt);
    visit_mut_prepend_statement_counter!(visit_mut_expr_stmt, ExprStmt);

    ///Visit statements, ask StmtVisitor to create a statement increasement counter.
    /// TODO: StmtVisitor seems not required
    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        let mut new_stmts: Vec<Stmt> = vec![];

        for mut stmt in stmts.drain(..) {
            let mut stmt_visitor = StmtVisitor {
                source_map: self.source_map,
                var_name: &self.var_name_ident,
                cov: &mut self.cov,
                before_stmts: vec![],
                after_stmts: vec![],
                replace: false,
            };

            stmt.visit_mut_with(&mut stmt_visitor);

            new_stmts.extend(stmt_visitor.before_stmts);
            if !stmt_visitor.replace {
                new_stmts.push(stmt);
            }
            new_stmts.extend(stmt_visitor.after_stmts);
        }

        *stmts = new_stmts;
    }

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        if self.is_instrumented_already() {
            return;
        }

        let mut new_items = vec![];
        for mut item in items.drain(..) {
            item.visit_mut_children_with(self);

            if self.before.len() > 0 {
                new_items.extend(self.before.drain(..).map(|v| ModuleItem::Stmt(v)));
            }
            new_items.push(item);
        }
        *items = new_items;

        self.cov.freeze();

        //TODO: option: global coverage variable scope. (optional, default `this`)
        let coverage_global_scope = "this";
        //TODO: option: use an evaluated function to find coverageGlobalScope.
        let coverage_global_scope_func = true;

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
                create_global_stmt_template(coverage_global_scope)
            }
        } else {
            unimplemented!("");
            /*
            gvTemplate = globalTemplateVariable({
                GLOBAL_COVERAGE_SCOPE: opts.coverageGlobalScope
            });
            */
        };

        let (coverage_fn_ident, coverage_template) = create_coverage_fn_decl(
            &self.instrument_options.coverage_variable,
            gv_template,
            &self.var_name,
            &self.file_path,
            self.cov.as_ref(),
        );

        // explicitly call this.varName to ensure coverage is always initialized
        let m = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Ident(coverage_fn_ident))),
                ..CallExpr::dummy()
            })),
        }));

        // prepend template to the top of the code
        items.insert(0, coverage_template);
        items.insert(1, m);
    }
}

fn should_ignore_file(comments: &Option<&PluginCommentsProxy>, program: &Program) -> bool {
    if let Some(comments) = *comments {
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
