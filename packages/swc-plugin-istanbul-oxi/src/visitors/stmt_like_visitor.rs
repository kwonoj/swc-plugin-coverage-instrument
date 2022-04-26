use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{ast::*, source_map::PluginSourceMapProxy, syntax_pos::DUMMY_SP};

use crate::{
    constants::idents::*, instrument::build_increase_expression_expr,
    utils::lookup_range::get_range_from_span,
};

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
        stmt.visit_mut_children_with(self);

        // If given statement is coverage counter inserted by any other visitor, skip to insert statement counter.
        // Currently this is required to visit counter in order of outer-to-inner like
        // ```
        // function a() {
        //   f[0]++;
        //   const x = function () { f[1]++ }
        // }
        // ```
        //
        if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
            if let Expr::Update(UpdateExpr { arg, .. }) = &**expr {
                if let Expr::Member(MemberExpr { obj, .. }) = &**arg {
                    if let Expr::Member(MemberExpr { obj, .. }) = &**obj {
                        if let Expr::Call(CallExpr { callee, .. }) = &**obj {
                            if let Callee::Expr(expr) = callee {
                                if let Expr::Ident(ident) = &**expr {
                                    if ident == self.var_name {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            };
        }

        self.insert_statement_counter(stmt);
    }

    fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
        let stmt_range = get_range_from_span(self.source_map, &var_decl.span);

        let idx = self.cov.new_statement(&stmt_range);
        let increment_expr = build_increase_expression_expr(&IDENT_S, idx, self.var_name, None);

        self.before_stmts.push(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(increment_expr),
        }));
    }
}
