use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::ast::*,
};

use super::create_increase_counter_expr::create_increase_counter_expr;
use crate::constants::idents::IDENT_BT;

/// Reads the logic expression conditions and conditionally increments truthy counter.
/// This is always known to be b_t type counter does not need to accept what type of ident it'll create.
pub fn create_increase_true_expr(
    id: u32,
    idx: u32,
    var_name: &Ident,
    temp_var_name: &Ident,
    expr: Expr,
) -> Expr {
    let member = Expr::Member(MemberExpr {
        obj: Box::new(Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Ident(var_name.clone()))),
            ..CallExpr::dummy()
        })),
        prop: MemberProp::Ident(temp_var_name.clone()),
        ..MemberExpr::dummy()
    });

    let assignment = Expr::Assign(AssignExpr {
        op: AssignOp::Assign,
        left: PatOrExpr::Expr(Box::new(member.clone())),
        right: Box::new(expr), // Only evaluates once.
        ..AssignExpr::dummy()
    });

    let paren = Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(Expr::Cond(CondExpr {
            test: Box::new(validate_true_non_trivial(var_name, temp_var_name)),
            cons: Box::new(create_increase_counter_expr(
                &IDENT_BT,
                id,
                var_name,
                Some(idx),
            )),
            alt: Box::new(Expr::Lit(Lit::Null(Null::dummy()))),
            ..CondExpr::dummy()
        })),
    });

    let ret = Expr::Seq(SeqExpr {
        span: DUMMY_SP,
        exprs: vec![Box::new(assignment), Box::new(paren), Box::new(member)],
    });

    ret
}

fn validate_true_non_trivial(var_name: &Ident, temp_var_name: &Ident) -> Expr {
    // TODO: duplicate code with create_increase_true_expr
    let member = Expr::Member(MemberExpr {
        obj: Box::new(Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Ident(var_name.clone()))),
            ..CallExpr::dummy()
        })),
        prop: MemberProp::Ident(temp_var_name.clone()),
        ..MemberExpr::dummy()
    });

    let left_for_right = Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(Expr::Bin(BinExpr {
            op: BinaryOp::LogicalOr,
            left: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Bang,
                arg: Box::new(Expr::Call(CallExpr {
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        obj: Box::new(Expr::Ident(Ident {
                            sym: "Array".into(),
                            ..Ident::dummy()
                        })),
                        prop: MemberProp::Ident(Ident {
                            sym: "isArray".into(),
                            ..Ident::dummy()
                        }),
                        ..MemberExpr::dummy()
                    }))),
                    args: vec![ExprOrSpread {
                        expr: Box::new(member.clone()),
                        spread: None,
                    }],
                    ..CallExpr::dummy()
                })),
                ..UnaryExpr::dummy()
            })),
            right: Box::new(Expr::Member(MemberExpr {
                obj: Box::new(member.clone()),
                prop: MemberProp::Ident(Ident {
                    sym: "length".into(),
                    ..Ident::dummy()
                }),
                ..MemberExpr::dummy()
            })),
            ..BinExpr::dummy()
        })),
    });
    let right_for_right = Expr::Paren(ParenExpr {
        expr: Box::new(Expr::Bin(BinExpr {
            op: BinaryOp::LogicalOr,
            left: Box::new(Expr::Bin(BinExpr {
                op: BinaryOp::NotEqEq,
                left: Box::new(Expr::Call(CallExpr {
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        obj: Box::new(Expr::Ident(Ident {
                            sym: "Object".into(),
                            ..Ident::dummy()
                        })),
                        prop: MemberProp::Ident(Ident {
                            sym: "getPrototypeOf".into(),
                            ..Ident::dummy()
                        }),
                        ..MemberExpr::dummy()
                    }))),
                    args: vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(member.clone()),
                    }],
                    ..CallExpr::dummy()
                })),
                right: Box::new(Expr::Member(MemberExpr {
                    obj: Box::new(Expr::Ident(Ident {
                        sym: "Object".into(),
                        ..Ident::dummy()
                    })),
                    prop: MemberProp::Ident(Ident {
                        sym: "prototype".into(),
                        ..Ident::dummy()
                    }),
                    ..MemberExpr::dummy()
                })),
                ..BinExpr::dummy()
            })),
            right: Box::new(Expr::Member(MemberExpr {
                obj: Box::new(Expr::Call(CallExpr {
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        obj: Box::new(Expr::Ident(Ident {
                            sym: "Object".into(),
                            ..Ident::dummy()
                        })),
                        prop: MemberProp::Ident(Ident {
                            sym: "values".into(),
                            ..Ident::dummy()
                        }),
                        ..MemberExpr::dummy()
                    }))),
                    args: vec![ExprOrSpread {
                        expr: Box::new(member.clone()),
                        spread: None,
                    }],
                    ..CallExpr::dummy()
                })),
                prop: MemberProp::Ident(Ident {
                    sym: "length".into(),
                    ..Ident::dummy()
                }),
                ..MemberExpr::dummy()
            })),
            ..BinExpr::dummy()
        })),
        ..ParenExpr::dummy()
    });

    let right = Expr::Bin(BinExpr {
        op: BinaryOp::LogicalAnd,
        left: Box::new(left_for_right),
        right: Box::new(right_for_right),
        ..BinExpr::dummy()
    });

    let ret = Expr::Bin(BinExpr {
        op: BinaryOp::LogicalAnd,
        left: Box::new(member),
        right: Box::new(right),
        ..BinExpr::dummy()
    });
    ret
}
