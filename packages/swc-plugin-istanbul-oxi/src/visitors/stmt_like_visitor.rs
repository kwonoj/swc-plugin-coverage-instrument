use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    comments::PluginCommentsProxy,
    source_map::PluginSourceMapProxy,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    insert_counter_helper,
    instrument::create_increase_expression_expr,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
};

pub struct StmtVisitor2<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub comments: Option<&'a PluginCommentsProxy>,
    pub cov: &'a mut SourceCoverage,
    pub var_name_ident: Ident,
    pub before: Vec<Stmt>,
    pub nodes: Vec<Node>,
}

// TODO: duplicated path between CoverageVisitor
impl<'a> StmtVisitor2<'a> {
    pub fn new(
        source_map: &'a PluginSourceMapProxy,
        comments: Option<&'a PluginCommentsProxy>,
        cov: &'a mut SourceCoverage,
        var_name_ident: &'a Ident,
        current_node: Node,
    ) -> StmtVisitor2<'a> {
        StmtVisitor2 {
            source_map,
            comments,
            cov,
            var_name_ident: var_name_ident.clone(),
            before: vec![],
            nodes: vec![current_node],
        }
    }

    insert_counter_helper!();

    /// Visit individual statements with stmt_visitor and update.
    fn insert_stmts_counter(&mut self, stmts: &mut Vec<Stmt>) {
        let mut new_stmts = vec![];

        for mut stmt in stmts.drain(..) {
            if !self.is_injected_counter_stmt(&stmt) {
                let span = crate::utils::lookup_range::get_stmt_span(&stmt);
                if let Some(span) = span {
                    let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                    new_stmts.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(increment_expr),
                    }));
                } else {
                    // if given stmt is not a plain stmt and omit to insert stmt counter,
                    // visit it to collect inner stmt counters
                    let mut visitor = StmtVisitor2::new(
                        self.source_map,
                        self.comments,
                        &mut self.cov,
                        &self.var_name_ident,
                        self.nodes.last().expect("Should exist").clone(),
                    );

                    stmt.visit_mut_with(&mut visitor);
                    new_stmts.extend(visitor.before.drain(..))
                }
            }

            new_stmts.push(stmt);
        }

        *stmts = new_stmts;
    }
}

impl VisitMut for StmtVisitor2<'_> {
    // BlockStatement: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
        self.nodes.push(Node::BlockStmt);

        self.insert_stmts_counter(&mut block_stmt.stmts);

        //block_stmt.visit_mut_children_with(self);
        self.nodes.pop();
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
