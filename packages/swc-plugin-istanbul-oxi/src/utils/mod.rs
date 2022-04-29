//! Naive wrapper to create commonly used ast types

use swc_plugin::ast::*;

pub(crate) mod ast_builder;
pub(crate) mod hint_comments;
pub(crate) mod lookup_range;
pub(crate) mod node;
pub(crate) mod visitor_macros;

/// Determines if var_declarator::init's expr should be wrapped into parenExpr with
/// statementcounter.
pub fn is_var_declarator_init_to_be_wrapped_into_paren(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_) | Expr::Call(_) | Expr::Assign(_) | Expr::Object(_) | Expr::Member(_) => true,
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
        | Expr::JSXFragment(_) => false,
        _ => false,
    }
}
