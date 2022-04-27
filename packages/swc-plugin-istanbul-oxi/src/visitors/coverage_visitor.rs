// TODO: remove
#![allow(unused)]
use istanbul_oxi_instrument::{BranchType, SourceCoverage};
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
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    instrument::create_increase_expression_expr,
    template::{
        create_coverage_fn_decl::create_coverage_fn_decl,
        create_global_stmt_template::create_global_stmt_template,
    },
    utils::{
        lookup_range::{get_expr_span, get_range_from_span, get_stmt_span},
        node::Node,
    },
    visit_mut_prepend_statement_counter, InstrumentOptions,
};

use super::stmt_like_visitor::{StmtVisitor, StmtVisitor2};

pub struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}

/// pattern for istanbul to ignore the whole file
/// This is not fully identical to original file comments
/// https://github.com/istanbuljs/istanbuljs/blob/6f45283feo31faaa066375528f6b68e3a9927b2d5/packages/istanbul-lib-instrument/src/visitor.js#L10=
/// as regex package doesn't support lookaround
static COMMENT_FILE_REGEX: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"^\s*istanbul\s+ignore\s+(file)(\W|$)").unwrap());

/// pattern for istanbul to ignore a section
static COMMENT_RE: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"^\s*istanbul\s+ignore\s+(if|else|next)(\W|$)").unwrap());

pub struct CoverageVisitor<'a> {
    comments: Option<&'a PluginCommentsProxy>,
    source_map: &'a PluginSourceMapProxy,
    // an identifier to the function name for coverage collection.
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
    nodes: Vec<Node>,
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
            nodes: vec![Node::Root],
        }
    }

    fn print_node(&self) -> String {
        if self.nodes.len() > 0 {
            format!(
                "{}",
                self.nodes
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(":")
            )
        } else {
            "unexpected".to_string()
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

    fn on_exit(&mut self, module_items: &mut Vec<ModuleItem>) {
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

        let coverage_template = create_coverage_fn_decl(
            &self.instrument_options.coverage_variable,
            gv_template,
            &self.var_name_ident,
            &self.file_path,
            self.cov.as_ref(),
        );

        // explicitly call this.varName to ensure coverage is always initialized
        let m = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Ident(self.var_name_ident.clone()))),
                ..CallExpr::dummy()
            })),
        }));

        // prepend template to the top of the code
        module_items.insert(0, coverage_template);
        module_items.insert(1, m);
    }

    fn visit_mut_fn(&mut self, ident: &Option<&Ident>, function: &mut Function) {
        let (span, name) = if let Some(ident) = &ident {
            (&ident.span, Some(ident.sym.to_string()))
        } else {
            (&function.span, None)
        };

        let range = get_range_from_span(self.source_map, span);
        let body_span = if let Some(body) = &function.body {
            body.span
        } else {
            // TODO: probably this should never occur
            function.span
        };

        let body_range = get_range_from_span(self.source_map, &body_span);
        let index = self.cov.new_function(&name, &range, &body_range);

        match &mut function.body {
            Some(blockstmt) => {
                let b =
                    create_increase_expression_expr(&IDENT_F, index, &self.var_name_ident, None);
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

    /// Visit individual statements with stmt_visitor and update.
    fn insert_stmts_counter(&mut self, stmts: &mut Vec<Stmt>) {
        let mut new_stmts = vec![];

        for mut stmt in stmts.drain(..) {
            if !self.is_injected_counter_stmt(&stmt) {
                let span = get_stmt_span(&stmt);
                if let Some(span) = span {
                    let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                    new_stmts.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(increment_expr),
                    }));
                } else {
                    // if given stmt is not a plain stmt and omit to insert stmt counter,
                    // visit it to collect inner stmt counters
                    stmt.visit_mut_with(self);
                    // Once visit completes, pick up stmt counter immediately - otherwise parent visitor will
                    // place this incorrect position outside of current scope.
                    // TODO: should we use new visitor instead? or should we need different storage property
                    // for better clarity?
                    new_stmts.extend(self.before.drain(..));
                }
            }

            new_stmts.push(stmt);
        }

        *stmts = new_stmts;
    }

    fn lookup_hint_comments(&mut self, expr: &Expr) -> Option<String> {
        let span = get_expr_span(expr);
        if let Some(span) = span {
            let h = self.comments.get_leading(span.hi);
            let l = self.comments.get_leading(span.lo);

            if let Some(h) = h {
                let h_value = h.iter().find_map(|c| {
                    if let Some(re_match) = COMMENT_RE.find_at(&c.text, 0) {
                        Some(re_match.as_str().to_string())
                    } else {
                        None
                    }
                });

                if let Some(h_value) = h_value {
                    return Some(h_value);
                }
            }

            if let Some(l) = l {
                let l_value = l.iter().find_map(|c| {
                    if let Some(re_match) = COMMENT_RE.find_at(&c.text, 0) {
                        Some(re_match.as_str().to_string())
                    } else {
                        None
                    }
                });

                if let Some(l_value) = l_value {
                    return Some(l_value);
                }
            }
        }

        return None;
    }

    /// Determine if given stmt is an injected counter by transform.
    pub fn is_injected_counter_stmt(&self, stmt: &swc_plugin::ast::Stmt) -> bool {
        use swc_plugin::ast::*;

        if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
            if let Expr::Update(UpdateExpr { arg, .. }) = &**expr {
                if let Expr::Member(MemberExpr { obj, .. }) = &**arg {
                    if let Expr::Member(MemberExpr { obj, .. }) = &**obj {
                        if let Expr::Call(CallExpr { callee, .. }) = &**obj {
                            if let Callee::Expr(expr) = callee {
                                if let Expr::Ident(ident) = &**expr {
                                    if ident == &self.var_name_ident {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            };
        }
        false
    }
}

/// Interfaces to mark counters. Parent node visitor should pick up and insert marked counter accordingly.
/// Unlike istanbul we can't have single insert logic to be called in any arbitary child node.
impl CoverageVisitor<'_> {
    #[tracing::instrument(skip(self, span, idx), fields(stmt_id))]
    fn create_stmt_increase_counter_expr(&mut self, span: &Span, idx: Option<u32>) -> Expr {
        let stmt_range = get_range_from_span(self.source_map, span);

        let stmt_id = self.cov.new_statement(&stmt_range);

        tracing::Span::current().record("stmt_id", &stmt_id);

        crate::instrument::create_increase_expression_expr(
            &IDENT_S,
            stmt_id,
            &self.var_name_ident,
            idx,
        )
    }

    // Mark to prepend statement increase counter to current stmt.
    // if (path.isStatement()) {
    //    path.insertBefore(T.expressionStatement(increment));
    // }
    #[tracing::instrument(skip_all)]
    fn mark_prepend_stmt_counter(&mut self, span: &Span) {
        let increment_expr = self.create_stmt_increase_counter_expr(span, None);
        self.before.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        }));
    }

    // if (path.isExpression()) {
    //    path.replaceWith(T.sequenceExpression([increment, path.node]));
    //}
    #[tracing::instrument(skip_all)]
    fn replace_expr_with_stmt_counter(&mut self, expr: &mut Expr) {
        self.replace_expr_with_counter(expr, |cov, var_name_ident, range| {
            let idx = cov.new_statement(&range);
            create_increase_expression_expr(&IDENT_S, idx, var_name_ident, None)
        });
    }

    #[tracing::instrument(skip_all)]
    fn replace_expr_with_branch_counter(&mut self, expr: &mut Expr, branch: u32) {
        self.replace_expr_with_counter(expr, |cov, var_name_ident, range| {
            let idx = cov.add_branch_path(branch, &range);

            create_increase_expression_expr(&IDENT_B, branch, var_name_ident, Some(idx))
        });
    }

    // Base wrapper fn to replace given expr to wrapped paren expr with counter
    fn replace_expr_with_counter<F>(&mut self, expr: &mut Expr, get_counter: F)
    where
        F: core::ops::Fn(&mut SourceCoverage, &Ident, &istanbul_oxi_instrument::Range) -> Expr,
    {
        let span = get_expr_span(expr);
        if let Some(span) = span {
            let init_range = get_range_from_span(self.source_map, span);
            let prepend_expr = get_counter(&mut self.cov, &self.var_name_ident, &init_range);

            let paren_expr = Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Seq(SeqExpr {
                    span: DUMMY_SP,
                    exprs: vec![Box::new(prepend_expr), Box::new(expr.take())],
                })),
            });

            // replace init with increase expr + init seq
            *expr = paren_expr;
        }
    }

    // if (path.isBlockStatement()) {
    //    path.node.body.unshift(T.expressionStatement(increment));
    // }
    fn mark_prepend_stmt_counter_for_body(&mut self) {
        todo!("not implemented");
    }

    /*
     if (
        this.counterNeedsHoisting(path) &&
        T.isVariableDeclarator(path.parentPath)
    ) {
        // make an attempt to hoist the statement counter, so that
        // function names are maintained.
        const parent = path.parentPath.parentPath;
        if (parent && T.isExportNamedDeclaration(parent.parentPath)) {
            parent.parentPath.insertBefore(
                T.expressionStatement(increment)
            );
        } else if (
            parent &&
            (T.isProgram(parent.parentPath) ||
                T.isBlockStatement(parent.parentPath))
        ) {
            parent.insertBefore(T.expressionStatement(increment));
        } else {
            path.replaceWith(T.sequenceExpression([increment, path.node]));
        }
    }
    */
    fn mark_prepend_stmt_counter_for_hoisted(&mut self) {}
}

impl VisitMut for CoverageVisitor<'_> {
    #[instrument(skip_all, fields(node = %self.print_node()))]
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

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        if self.is_instrumented_already() {
            return;
        }

        self.nodes.push(Node::ModuleItems);
        let mut new_items = vec![];
        for mut item in items.drain(..) {
            item.visit_mut_children_with(self);

            if self.before.len() > 0 {
                new_items.extend(self.before.drain(..).map(|v| ModuleItem::Stmt(v)));
            }
            new_items.push(item);
        }
        *items = new_items;
        self.nodes.pop();

        self.on_exit(items);
    }

    // ArrowFunctionExpression: entries(convertArrowExpression, coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        self.nodes.push(Node::ArrowExpr);
        arrow_expr.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // AssignmentPattern: entries(coverAssignmentPattern),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_assign_pat(&mut self, assign_pat: &mut AssignPat) {
        self.nodes.push(Node::AssignPat);
        assign_pat.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // BlockStatement: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
        self.nodes.push(Node::BlockStmt);

        self.insert_stmts_counter(&mut block_stmt.stmts);

        //block_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ExportDefaultDeclaration: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_export_default_decl(&mut self, export_default_decl: &mut ExportDefaultDecl) {
        self.nodes.push(Node::ExportDefaultDecl);
        // noop
        export_default_decl.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ExportNamedDeclaration: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_export_decl(&mut self, export_named_decl: &mut ExportDecl) {
        self.nodes.push(Node::ExportDecl);
        // noop
        export_named_decl.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ClassMethod: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_class_method(&mut self, class_method: &mut ClassMethod) {
        self.nodes.push(Node::ClassMethod);
        class_method.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ClassDeclaration: entries(parenthesizedExpressionProp('superClass')),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
        self.nodes.push(Node::ClassDecl);
        class_decl.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ClassProperty: entries(coverClassPropDeclarator),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_class_prop(&mut self, class_prop: &mut ClassProp) {
        self.nodes.push(Node::ClassProp);
        class_prop.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ClassPrivateProperty: entries(coverClassPropDeclarator),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_private_prop(&mut self, private_prop: &mut PrivateProp) {
        self.nodes.push(Node::PrivateProp);
        private_prop.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ExpressionStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_expr_stmt(&mut self, expr_stmt: &mut ExprStmt) {
        self.nodes.push(Node::ExprStmt);
        self.mark_prepend_stmt_counter(&expr_stmt.span);
        expr_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // BreakStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_break_stmt(&mut self, break_stmt: &mut BreakStmt) {
        self.nodes.push(Node::BreakStmt);
        break_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // BreakStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_continue_stmt(&mut self, continue_stmt: &mut ContinueStmt) {
        self.nodes.push(Node::ContinueStmt);
        continue_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // DebuggerStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_debugger_stmt(&mut self, debugger_stmt: &mut DebuggerStmt) {
        self.nodes.push(Node::DebuggerStmt);
        debugger_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ReturnStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
        self.nodes.push(Node::ReturnStmt);
        return_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ThrowStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_throw_stmt(&mut self, throw_stmt: &mut ThrowStmt) {
        self.nodes.push(Node::ThrowStmt);
        throw_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // TryStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_try_stmt(&mut self, try_stmt: &mut TryStmt) {
        self.nodes.push(Node::TryStmt);
        try_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // VariableDeclaration: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
        self.nodes.push(Node::VarDecl);
        //noop?
        var_decl.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // VariableDeclarator: entries(coverVariableDeclarator),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        let parent = self.nodes.last().unwrap().clone();
        let parent_parent = self.nodes[self.nodes.len() - 2];
        self.nodes.push(Node::VarDeclarator);

        if let Some(init) = &mut declarator.init {
            let init = &mut **init;
            let span = get_expr_span(init);
            if let Some(span) = span {
                // This is ugly, poor man's substitute to istanbul's `insertCounter` to determine
                // when to replace givn expr to wrapped Paren or prepend stmt counter.
                // We can't do insert parent node's sibling in downstream's child node.
                // TODO: there should be a better way.
                match parent_parent {
                    Node::BlockStmt | Node::ModuleItems => {
                        self.mark_prepend_stmt_counter(span);
                    }
                    _ => {
                        self.replace_expr_with_stmt_counter(init);
                    }
                }
            }
        }

        declarator.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // IfStatement: entries(blockProp('consequent'), blockProp('alternate'), coverStatement, coverIfBranches)
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_if_stmt(&mut self, if_stmt: &mut IfStmt) {
        self.nodes.push(Node::IfStmt);
        if_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ForStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
        self.nodes.push(Node::ForStmt);

        self.mark_prepend_stmt_counter(&for_stmt.span);

        for_stmt.visit_mut_children_with(self);

        let mut body = *for_stmt.body.take();
        // if for stmt body is not block, wrap it before insert statement counter
        let body = if let Stmt::Block(mut body) = body {
            //self.visit_mut_stmts(&mut body.stmts);
            body
        } else {
            let mut stmts = vec![body];
            self.insert_stmts_counter(&mut stmts);

            BlockStmt {
                span: DUMMY_SP,
                stmts,
            }
        };
        for_stmt.body = Box::new(Stmt::Block(body));

        self.nodes.pop();
    }

    // ForInStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_for_in_stmt(&mut self, for_in_stmt: &mut ForInStmt) {
        self.nodes.push(Node::ForInStmt);

        self.mark_prepend_stmt_counter(&for_in_stmt.span);

        for_in_stmt.visit_mut_children_with(self);

        let mut body = *for_in_stmt.body.take();
        // if for stmt body is not block, wrap it before insert statement counter
        let body = if let Stmt::Block(mut body) = body {
            //self.visit_mut_stmts(&mut body.stmts);
            body
        } else {
            let mut stmts = vec![body];
            self.insert_stmts_counter(&mut stmts);

            BlockStmt {
                span: DUMMY_SP,
                stmts,
            }
        };
        for_in_stmt.body = Box::new(Stmt::Block(body));

        self.nodes.pop();
    }

    // ForOfStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_for_of_stmt(&mut self, for_of_stmt: &mut ForOfStmt) {
        self.nodes.push(Node::ForOfStmt);
        self.mark_prepend_stmt_counter(&for_of_stmt.span);

        for_of_stmt.visit_mut_children_with(self);

        let mut body = *for_of_stmt.body.take();
        // if for stmt body is not block, wrap it before insert statement counter
        let body = if let Stmt::Block(mut body) = body {
            //self.visit_mut_stmts(&mut body.stmts);
            body
        } else {
            let mut stmts = vec![body];
            self.insert_stmts_counter(&mut stmts);

            BlockStmt {
                span: DUMMY_SP,
                stmts,
            }
        };
        for_of_stmt.body = Box::new(Stmt::Block(body));
        self.nodes.pop();
    }

    // WhileStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_while_stmt(&mut self, while_stmt: &mut WhileStmt) {
        self.nodes.push(Node::WhileStmt);
        while_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // DoWhileStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_do_while_stmt(&mut self, do_while_stmt: &mut DoWhileStmt) {
        self.nodes.push(Node::DoWhileStmt);
        do_while_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // SwitchStatement: entries(createSwitchBranch, coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_switch_stmt(&mut self, switch_stmt: &mut SwitchStmt) {
        self.nodes.push(Node::SwitchStmt);
        switch_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // SwitchCase: entries(coverSwitchCase),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_switch_case(&mut self, switch_case: &mut SwitchCase) {
        self.nodes.push(Node::SwitchCase);
        switch_case.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // WithStatement: entries(blockProp('body'), coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_with_stmt(&mut self, with_stmt: &mut WithStmt) {
        self.nodes.push(Node::WithStmt);
        with_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // FunctionDeclaration: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
        self.nodes.push(Node::FnDecl);
        self.visit_mut_fn(&Some(&fn_decl.ident), &mut fn_decl.function);
        fn_decl.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // FunctionExpression: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
        self.nodes.push(Node::FnExpr);
        // We do insert counter _first_, then iterate child:
        // Otherwise inner stmt / fn will get the first idx to the each counter.
        // StmtVisitor filters out injected counter internally.
        self.visit_mut_fn(&fn_expr.ident.as_ref(), &mut fn_expr.function);
        fn_expr.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // FunctionExpression: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_labeled_stmt(&mut self, labeled_stmt: &mut LabeledStmt) {
        self.nodes.push(Node::LabeledStmt);
        labeled_stmt.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // ConditionalExpression: entries(coverTernary),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_cond_expr(&mut self, cond_expr: &mut CondExpr) {
        self.nodes.push(Node::CondExpr);

        let range = get_range_from_span(self.source_map, &cond_expr.span);
        let branch = self.cov.new_branch(BranchType::CondExpr, &range, false);

        let c_hint = self.lookup_hint_comments(&*cond_expr.cons);
        let a_hint = self.lookup_hint_comments(&*cond_expr.alt);

        if c_hint.as_deref() != Some("next") {
            // TODO: do we need this?
            // cond_expr.cons.visit_mut_children_with(self);

            // replace consequence to the paren for increase expr + expr itself
            self.replace_expr_with_branch_counter(&mut *cond_expr.cons, branch);
        }

        if a_hint.as_deref() != Some("next") {
            // TODO: do we need this?
            // cond_expr.alt.visit_mut_children_with(self);

            // replace consequence to the paren for increase expr + expr itself
            self.replace_expr_with_branch_counter(&mut *cond_expr.alt, branch);
        }

        cond_expr.visit_mut_children_with(self);
        self.nodes.pop();
    }

    // LogicalExpression: entries(coverLogicalExpression)
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
        self.nodes.push(Node::BinExpr);
        bin_expr.visit_mut_children_with(self);
        self.nodes.pop();
        /*
        BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                BinaryOrLogicalExpr::Logical(LogicalExpression {
                    base: ctx.base(self.span),
                    operator: self.op.babelify(ctx).into(),
                    left: Box::alloc().init(self.left.babelify(ctx).into()),
                    right: Box::alloc().init(self.right.babelify(ctx).into()),
                })
            }
        */
    }

    // ObjectMethod: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_method_prop(&mut self, method_prop: &mut MethodProp) {
        self.nodes.push(Node::MethodProp);
        method_prop.visit_mut_children_with(self);
        self.nodes.pop();
        // ObjectMethodKind::Method,
    }

    // ObjectMethod: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_getter_prop(&mut self, getter_prop: &mut GetterProp) {
        self.nodes.push(Node::GetterProp);
        getter_prop.visit_mut_children_with(self);
        self.nodes.pop();
        // ObjectMethodKind::Get,
    }

    // ObjectMethod: entries(coverFunction),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_setter_prop(&mut self, setter_prop: &mut SetterProp) {
        self.nodes.push(Node::SetterProp);
        setter_prop.visit_mut_children_with(self);
        self.nodes.pop();
        //ObjectMethodKind::Set,
    }
}

/*
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
        // We do insert counter _first_, then iterate child:
        // Otherwise inner stmt / fn will get the first idx to the each counter.
        // StmtVisitor filters out injected counter internally.
        self.visit_mut_fn(&fn_expr.ident.as_ref(), &mut fn_expr.function);
        fn_expr.visit_mut_children_with(self);
    }

    fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
        self.visit_mut_fn(&Some(&fn_decl.ident), &mut fn_decl.function);
        fn_decl.visit_mut_children_with(self);
    }

    fn visit_mut_cond_expr(&mut self, cond_expr: &mut CondExpr) {
        let range = get_range_from_span(self.source_map, &cond_expr.span);
        let branch = self.cov.new_branch(BranchType::CondExpr, &range, false);

        let c_hint = self.lookup_hint_comments(&*cond_expr.cons);
        let a_hint = self.lookup_hint_comments(&*cond_expr.alt);

        if c_hint.as_deref() != Some("next") {
            let mut expr = cond_expr.cons.take();
            let span = get_expr_span(&expr).expect("Should have span");

            let range = get_range_from_span(self.source_map, &span);

            let idx = self.cov.add_branch_path(branch, &range);

            let increment_expr =
                build_increase_expression_expr(&IDENT_B, branch, &self.var_name_ident, Some(idx));

            expr.visit_mut_children_with(self);
            let paren_expr = Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Seq(SeqExpr {
                    span: DUMMY_SP,
                    exprs: vec![Box::new(increment_expr), expr],
                })),
            });

            // replace consequence to the paren for increase expr + expr itself
            *cond_expr.cons = paren_expr;
        }

        if a_hint.as_deref() != Some("next") {
            let mut expr = cond_expr.alt.take();
            let span = get_expr_span(&expr).expect("Should have span");

            let range = get_range_from_span(self.source_map, &span);

            let idx = self.cov.add_branch_path(branch, &range);

            let increment_expr =
                build_increase_expression_expr(&IDENT_B, branch, &self.var_name_ident, Some(idx));

            expr.visit_mut_children_with(self);
            let paren_expr = Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Seq(SeqExpr {
                    span: DUMMY_SP,
                    exprs: vec![Box::new(increment_expr), expr],
                })),
            });

            // replace alternative to the paren for increase expr + expr itself
            *cond_expr.alt = paren_expr;
        }
    }

    /// Visit variable declarator, inject a statement increase expr by wrapping declaration init with paren.
    /// var x = 0
    /// ->
    /// var x = (cov_18biir0b3p().s[3]++, 0)
    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        // TODO: this is not complete
        if let Some(init) = &mut declarator.init {
            match &mut **init {
                Expr::Lit(Lit::Str(Str { span, .. }))
                | Expr::Lit(Lit::Num(Number { span, .. }))
                | Expr::Call(CallExpr { span, .. })
                | Expr::Assign(AssignExpr { span, .. })
                | Expr::Object(ObjectLit { span, .. })
                | Expr::Member(MemberExpr { span, .. }) => {
                    let init_range = get_range_from_span(self.source_map, span);

                    let idx = self.cov.new_statement(&init_range);
                    let increment_expr =
                        build_increase_expression_expr(&IDENT_S, idx, &self.var_name_ident, None);

                    let paren_expr = Expr::Paren(ParenExpr {
                        span: span.take(),
                        expr: Box::new(Expr::Seq(SeqExpr {
                            span: DUMMY_SP,
                            exprs: vec![Box::new(increment_expr), init.take()],
                        })),
                    });

                    // replace init with increase expr + init seq
                    **init = paren_expr;
                }
                Expr::This(_)
                | Expr::Array(_)
                | Expr::Fn(_)
                | Expr::Unary(_)
                | Expr::Update(_)
                | Expr::Bin(_)
                | Expr::SuperProp(_)
                | Expr::Cond(_)
                | Expr::New(_)
                | Expr::Seq(_)
                | Expr::Ident(_)
                | Expr::Tpl(_)
                | Expr::TaggedTpl(_)
                | Expr::Arrow(_)
                | Expr::Class(_)
                | Expr::Yield(_)
                | Expr::MetaProp(_)
                | Expr::Await(_)
                | Expr::Paren(_)
                | Expr::JSXMember(_)
                | Expr::JSXNamespacedName(_)
                | Expr::JSXEmpty(_)
                | Expr::JSXElement(_)
                | Expr::JSXFragment(_) => {
                    println!("p======================");
                }
                _ => {
                    println!("r==========================");
                }
            };
        }

        declarator.visit_mut_children_with(self);
    }

    // Insert statement counter for For(in, of)Stmt
    visit_mut_prepend_statement_counter!(visit_mut_for_stmt, ForStmt);
    visit_mut_prepend_statement_counter!(visit_mut_for_of_stmt, ForOfStmt);
    visit_mut_prepend_statement_counter!(visit_mut_expr_stmt, ExprStmt);

    ///Visit statements, ask StmtVisitor to create a statement increasement counter.
    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        let mut new_stmts: Vec<Stmt> = vec![];

        for mut stmt in stmts.drain(..) {
            self.in_stmt_visitor = true;
            stmt.visit_mut_children_with(self);

            let mut stmt_visitor =
                StmtVisitor::new(self.source_map, &mut self.cov, &self.var_name_ident);

            stmt.visit_mut_with(&mut stmt_visitor);

            new_stmts.extend(&mut stmt_visitor.before_stmts.drain(..));
            if !stmt_visitor.replace {
                new_stmts.push(stmt);
            }
            new_stmts.extend(&mut stmt_visitor.after_stmts.drain(..));
            self.in_stmt_visitor = false;
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

        let coverage_template = create_coverage_fn_decl(
            &self.instrument_options.coverage_variable,
            gv_template,
            &self.var_name_ident,
            &self.file_path,
            self.cov.as_ref(),
        );

        // explicitly call this.varName to ensure coverage is always initialized
        let m = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Ident(self.var_name_ident.clone()))),
                ..CallExpr::dummy()
            })),
        }));

        // prepend template to the top of the code
        items.insert(0, coverage_template);
        items.insert(1, m);
    }
}*/

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
