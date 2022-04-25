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
    }
}
