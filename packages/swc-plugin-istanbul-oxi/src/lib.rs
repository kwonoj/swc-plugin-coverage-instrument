use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use constants::idents::*;
use instrument::build_increase_expression_expr;
use istanbul_oxi_instrument::SourceCoverage;
use once_cell::sync::Lazy;
use serde_json::Value;
use swc_plugin::{
    ast::*,
    comments::{Comment, CommentKind, Comments, PluginCommentsProxy},
    plugin_transform,
    source_map::PluginSourceMapProxy,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
    TransformPluginProgramMetadata,
};

mod constants;
mod instrument;
mod template;
mod utils;

use regex::Regex as Regexp;
use template::{
    create_coverage_fn_decl::create_coverage_fn_decl,
    create_global_stmt_template::create_global_stmt_template,
};

use crate::utils::lookup_range::get_range_from_span;

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

struct CoverageVisitor<'a> {
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
}

/// Visit statements, create a call to increase statement counter.
struct StmtVisitor<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub cov: &'a mut SourceCoverage,
    pub var_name: &'a Ident,
    pub before_stmts: Vec<Stmt>,
    pub after_stmts: Vec<Stmt>,
    pub replace: bool,
}

impl<'a> StmtVisitor<'a> {
    fn insert_statement_counter(&mut self, stmt: &mut Stmt) {
        let stmt_span = match stmt {
            Stmt::Block(BlockStmt { span, .. })
            | Stmt::Empty(EmptyStmt { span, .. })
            | Stmt::Debugger(DebuggerStmt { span, .. })
            | Stmt::With(WithStmt { span, .. })
            | Stmt::Return(ReturnStmt { span, .. })
            | Stmt::Labeled(LabeledStmt { span, .. })
            | Stmt::Break(BreakStmt { span, .. })
            | Stmt::Continue(ContinueStmt { span, .. })
            | Stmt::If(IfStmt { span, .. })
            | Stmt::Switch(SwitchStmt { span, .. })
            | Stmt::Throw(ThrowStmt { span, .. })
            | Stmt::Try(TryStmt { span, .. })
            | Stmt::While(WhileStmt { span, .. })
            | Stmt::DoWhile(DoWhileStmt { span, .. })
            | Stmt::For(ForStmt { span, .. })
            | Stmt::ForIn(ForInStmt { span, .. })
            | Stmt::ForOf(ForOfStmt { span, .. })
            | Stmt::Decl(Decl::Class(ClassDecl { class: Class { span, .. }, ..}))
            | Stmt::Decl(Decl::Fn(FnDecl { function: Function { span, .. }, ..}))
            | Stmt::Decl(Decl::Var(VarDecl { span, ..}))
            // TODO: need this?
            | Stmt::Decl(Decl::TsInterface(TsInterfaceDecl { span, ..}))
            | Stmt::Decl(Decl::TsTypeAlias(TsTypeAliasDecl { span, ..}))
            | Stmt::Decl(Decl::TsEnum(TsEnumDecl { span, ..}))
            | Stmt::Decl(Decl::TsModule(TsModuleDecl { span, ..}))
            | Stmt::Expr(ExprStmt { span, .. })
            => span,
        };

        let stmt_range = get_range_from_span(self.source_map, &stmt_span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr = build_increase_expression_expr(&IDENT_S, idx, self.var_name, None);
        self.insert_counter(
            stmt,
            Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(increment_expr),
            }),
        );
    }

    fn insert_counter(&mut self, current: &Stmt, increment_expr: Stmt) {
        match current {
            _ => {
                self.before_stmts.push(increment_expr);
            }
        }
    }
}

impl VisitMut for StmtVisitor<'_> {
    fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
        self.insert_statement_counter(stmt);

        //stmt.visit_mut_children_with(self);
    }
}

impl<'a> CoverageVisitor<'a> {
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

    /// Create a call to increase statement counter for For statement.
    /// It is not possible to prepend created statement to the For statement directly,
    /// parent visitor (visit_module_items) deals with it instead.
    fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
        // TODO: Consolidate logic between StmtVisitor
        let stmt_range = get_range_from_span(self.source_map, &for_stmt.span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr =
            build_increase_expression_expr(&IDENT_S, idx, &self.var_name_ident, None);

        self.before = vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        })];

        for_stmt.visit_mut_children_with(self);
    }

    fn visit_mut_expr_stmt(&mut self, expr_stmt: &mut ExprStmt) {
        // TODO: Consolidate logic between StmtVisitor
        let stmt_range = get_range_from_span(self.source_map, &expr_stmt.span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr =
            build_increase_expression_expr(&IDENT_S, idx, &self.var_name_ident, None);

        self.before = vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        })];

        expr_stmt.visit_mut_children_with(self);
    }

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

struct InstrumentOptions {
    pub coverage_variable: String,
    pub compact: bool,
    pub report_logic: bool,
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

    let visitor = CoverageVisitor::new(
        metadata.comments.as_ref(),
        &metadata.source_map,
        filename,
        UnknownReserved,
        None,
        SourceCoverage::new(filename.to_string(), instrument_options.report_logic),
        UnknownReserved,
        UnknownReserved,
        None,
        instrument_options,
    );

    program.fold_with(&mut as_folder(visitor))
}
