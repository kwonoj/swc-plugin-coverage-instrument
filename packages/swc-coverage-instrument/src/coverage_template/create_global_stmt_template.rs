use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::{ast::*, utils::quote_ident},
};

use crate::constants::idents::IDENT_GLOBAL;

use super::create_assignment_stmt::create_assignment_stmt;

/// Creates an assignment statement for the global scope lookup function
/// `var gcov = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof self !== 'undefined' ? self : this;`
pub fn create_global_stmt_template(_coverage_global_scope: &str) -> Stmt {
    // Construct AST: typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : typeof self !== 'undefined' ? self : this
    let global_this_ident = quote_ident!(Default::default(), "globalThis");
    let window_ident = quote_ident!(Default::default(), "window");
    let self_ident = quote_ident!(Default::default(), "self");

    let make_typeof_check = |ident: Ident| -> Expr {
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::NotEqEq,
            left: Box::new(Expr::Unary(UnaryExpr {
                span: DUMMY_SP,
                op: UnaryOp::TypeOf,
                arg: Box::new(Expr::Ident(ident)),
            })),
            right: Box::new(Expr::Lit(Lit::Str(Str {
                value: "undefined".into(),
                ..Str::dummy()
            }))),
        })
    };

    // typeof self !== 'undefined' ? self : this
    let self_check = Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(make_typeof_check(self_ident.clone())),
        cons: Box::new(Expr::Ident(self_ident)),
        alt: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
    });

    // typeof window !== 'undefined' ? window : (self_check)
    let window_check = Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(make_typeof_check(window_ident.clone())),
        cons: Box::new(Expr::Ident(window_ident)),
        alt: Box::new(self_check),
    });

    // typeof globalThis !== 'undefined' ? globalThis : (window_check)
    let global_expr = Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(make_typeof_check(global_this_ident.clone())),
        cons: Box::new(Expr::Ident(global_this_ident)),
        alt: Box::new(window_check),
    });

    create_assignment_stmt(&IDENT_GLOBAL, global_expr)
}
