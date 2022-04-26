use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    source_map::PluginSourceMapProxy,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};

use crate::{
    constants::idents::*,
    instrument::create_increase_expression_expr,
    utils::lookup_range::{get_expr_span, get_range_from_span},
};

pub struct StmtVisitor2<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub cov: &'a mut SourceCoverage,
    pub var_name_ident: &'a Ident,
    pub before: Vec<Stmt>,
}

// TODO: duplicated path between CoverageVisitor
impl<'a> StmtVisitor2<'a> {
    pub fn new(
        source_map: &'a PluginSourceMapProxy,
        cov: &'a mut SourceCoverage,
        var_name_ident: &'a Ident,
    ) -> StmtVisitor2<'a> {
        StmtVisitor2 {
            source_map,
            cov,
            var_name_ident,
            before: vec![],
        }
    }

    fn create_increase_expr(&mut self, type_ident: &Ident, span: &Span, idx: Option<u32>) -> Expr {
        let stmt_range = get_range_from_span(self.source_map, span);

        let stmt_id = self.cov.new_statement(&stmt_range);
        crate::instrument::create_increase_expression_expr(
            type_ident,
            stmt_id,
            &self.var_name_ident,
            idx,
        )
    }

    // Mark to prepend statement increase counter to current stmt.
    // if (path.isStatement()) {
    //    path.insertBefore(T.expressionStatement(increment));
    // }
    fn mark_prepend_stmt_counter(&mut self, span: &Span) {
        let increment_expr = self.create_increase_expr(&IDENT_S, span, None);

        self.before.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        }));
    }

    // if (path.isExpression()) {
    //    path.replaceWith(T.sequenceExpression([increment, path.node]));
    //}
    fn replace_stmt_with_counter(&mut self, expr: &mut Expr) {
        let span = get_expr_span(expr);
        if let Some(span) = span {
            let init_range = get_range_from_span(self.source_map, span);

            let idx = self.cov.new_statement(&init_range);
            let increment_expr =
                create_increase_expression_expr(&IDENT_S, idx, &self.var_name_ident, None);

            let paren_expr = Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Seq(SeqExpr {
                    span: DUMMY_SP,
                    exprs: vec![Box::new(increment_expr), Box::new(expr.take())],
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

// TODO: duplicated path between CoverageVisitor
impl VisitMut for StmtVisitor2<'_> {
    // VariableDeclarator: entries(coverVariableDeclarator),
    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        if let Some(init) = &mut declarator.init {
            let init = &mut **init;
            self.replace_stmt_with_counter(init);
        }
    }
}

/// Visit statements, create a call to increase statement counter.
pub struct StmtVisitor<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub cov: &'a mut SourceCoverage,
    pub var_name: &'a Ident,
    pub before_stmts: Vec<Stmt>,
    pub after_stmts: Vec<Stmt>,
    pub replace: bool,
}

impl<'a> StmtVisitor<'a> {
    pub fn new(
        source_map: &'a PluginSourceMapProxy,
        cov: &'a mut SourceCoverage,
        var_name: &'a Ident,
    ) -> StmtVisitor<'a> {
        StmtVisitor {
            source_map,
            cov,
            var_name,
            before_stmts: vec![],
            after_stmts: vec![],
            replace: false,
        }
    }
}

impl<'a> StmtVisitor<'a> {
    fn insert_statement_counter(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Decl(Decl::Fn(_)) | Stmt::Decl(Decl::Var(_)) => {
                return;
            }
            _ => {}
        };

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
          // | Stmt::Decl(Decl::Fn(FnDecl { function: Function { span, .. }, ..}))
          // | Stmt::Decl(Decl::Var(VarDecl { span, ..}))
          // TODO: need this?
          | Stmt::Decl(Decl::TsInterface(TsInterfaceDecl { span, ..}))
          | Stmt::Decl(Decl::TsTypeAlias(TsTypeAliasDecl { span, ..}))
          | Stmt::Decl(Decl::TsEnum(TsEnumDecl { span, ..}))
          | Stmt::Decl(Decl::TsModule(TsModuleDecl { span, ..}))
          | Stmt::Expr(ExprStmt { span, .. })
          => span,
          _ => {todo!()}
      };

        let stmt_range = get_range_from_span(self.source_map, &stmt_span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr = create_increase_expression_expr(&IDENT_S, idx, self.var_name, None);
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
        //stmt.visit_mut_children_with(self);

        // If given statement is coverage counter inserted by any other visitor, skip to insert statement counter.
        // Currently this is required to visit counter in order of outer-to-inner like
        // ```
        // function a() {
        //   f[0]++;
        //   const x = function () { f[1]++ }
        // }
        // ```
        //

        self.insert_statement_counter(stmt);
    }

    /*
    fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
        let stmt_range = get_range_from_span(self.source_map, &var_decl.span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr = create_increase_expression_expr(&IDENT_S, idx, self.var_name, None);

        self.before_stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        }));
    } */
}
