use istanbul_oxi_instrument::Range;
//TODO : swc_plugin need to import Pos
use swc_ecma_quote::swc_common::source_map::Pos;
use swc_plugin::{ast::*, source_map::PluginSourceMapProxy, syntax_pos::Span};

pub fn get_range_from_span(source_map: &PluginSourceMapProxy, span: &Span) -> Range {
    let span_hi_loc = source_map.lookup_char_pos(span.hi);
    let span_lo_loc = source_map.lookup_char_pos(span.lo);

    Range::new(
        span_lo_loc.line as u32,
        span_lo_loc.col.to_u32(),
        span_hi_loc.line as u32,
        span_hi_loc.col.to_u32(),
    )
}

pub fn get_expr_span(expr: &Expr) -> Option<&Span> {
    match expr {
        Expr::This(ThisExpr { span, .. })
        | Expr::Array(ArrayLit { span, .. })
        | Expr::Object(ObjectLit { span, .. })
        //| Expr::Fn(FnExpr { function: Function { span, .. }, .. })
        | Expr::Unary(UnaryExpr { span, .. })
        | Expr::Update(UpdateExpr { span, .. })
        | Expr::Bin(BinExpr { span, .. })
        | Expr::Assign(AssignExpr { span, .. })
        | Expr::Member(MemberExpr { span, .. })
        | Expr::SuperProp(SuperPropExpr { span, .. })
        | Expr::Cond(CondExpr { span, .. })
        | Expr::Call(CallExpr { span, .. })
        | Expr::New(NewExpr { span, .. })
        | Expr::Seq(SeqExpr { span, .. })
        | Expr::Ident(Ident { span, .. })
        | Expr::Lit(Lit::Str(Str { span, .. }))
        | Expr::Lit(Lit::Bool(Bool { span, .. }))
        | Expr::Lit(Lit::Null(Null { span, .. }))
        | Expr::Lit(Lit::Num(Number { span, .. }))
        | Expr::Lit(Lit::Regex(Regex { span, .. }))
        | Expr::Lit(Lit::JSXText(JSXText { span, .. }))
        | Expr::Lit(Lit::BigInt(BigInt { span, .. }))
        | Expr::Tpl(Tpl { span, .. })
        | Expr::TaggedTpl(TaggedTpl { span, .. })
        | Expr::Arrow(ArrowExpr { span, .. })
        | Expr::Class(ClassExpr {
            class: Class { span, .. },
            ..
        })
        | Expr::Yield(YieldExpr { span, .. })
        | Expr::MetaProp(MetaPropExpr { span, .. })
        | Expr::Await(AwaitExpr { span, .. })
        | Expr::Paren(ParenExpr { span, .. })
        | Expr::PrivateName(PrivateName { span, .. })
        | Expr::OptChain(OptChainExpr { span, .. }) => Some(span),
        _ => None,
    }
}

/// Determines span of the given stmt if given stmt can be treated as plain stmt
/// with inserting stmt counter.
pub fn get_stmt_span(stmt: &Stmt) -> Option<&Span> {
    match stmt {
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
        // | Stmt::Decl(Decl::Class(ClassDecl { class: Class { span, .. }, ..}))
        // | Stmt::Decl(Decl::Fn(FnDecl { function: Function { span, .. }, ..}))
        // | Stmt::Decl(Decl::Var(VarDecl { span, ..}))
        // TODO: need this?
        | Stmt::Decl(Decl::TsInterface(TsInterfaceDecl { span, ..}))
        | Stmt::Decl(Decl::TsTypeAlias(TsTypeAliasDecl { span, ..}))
        | Stmt::Decl(Decl::TsEnum(TsEnumDecl { span, ..}))
        | Stmt::Decl(Decl::TsModule(TsModuleDecl { span, ..}))
        | Stmt::Expr(ExprStmt { span, .. })
        => Some(span),
        _ => {
            None
        }
    }
}
