use swc_core::{
    ast::*,
    common::{util::take::Take, DUMMY_SP},
    utils::quote_ident,
};

use crate::constants::idents::IDENT_GLOBAL;

use super::create_assignment_stmt::create_assignment_stmt;

/// Creates an assignment statement for the global scope lookup function
/// `var global = new Function("return $global_coverage_scope")();`
pub fn create_global_stmt_template(coverage_global_scope: &str) -> Stmt {
    // Note: we don't support function template based on scoped binding
    // like https://github.com/istanbuljs/istanbuljs/blob/c7693d4608979ab73ebb310e0a1647e2c51f31b6/packages/istanbul-lib-instrument/src/visitor.js#L793=
    // due to scope checking is tricky.
    let fn_ctor = quote_ident!("((function(){}).constructor)");

    let expr = Expr::New(NewExpr {
        callee: Box::new(Expr::Ident(fn_ctor)),
        args: Some(vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str {
                value: format!("return {}", coverage_global_scope).into(),
                ..Str::dummy()
            }))),
        }]),
        ..NewExpr::dummy()
    });

    create_assignment_stmt(
        &IDENT_GLOBAL,
        Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(expr),
            }))),
            ..CallExpr::dummy()
        }),
    )
}
