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
        | Expr::Fn(FnExpr {
            function: Function { span, .. },
            ..
        })
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
