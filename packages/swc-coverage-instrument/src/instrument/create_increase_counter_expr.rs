use swc_core::{common::DUMMY_SP, ecma::ast::*};

/// Creates a expr like `cov_17709493053001988098().s[0]++;`
/// idx indicates index of vec-based counters (i.e branches).
/// If it exists, creates a expr with idx like
/// 1cov_17709493053001988098().b[0][idx]++;` instead.
pub fn create_increase_counter_expr(
    type_ident: &Ident,
    id: u32,
    var_name: &Ident,
    idx: Option<u32>,
) -> Expr {
    let call = CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Ident(var_name.clone()))),
        args: vec![],
        type_args: None,
        ..Default::default()
    };

    let c = MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Call(call)),
        prop: MemberProp::Ident(type_ident.clone().into()),
    };

    let expr = MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(c)),
        prop: MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: id as f64,
                raw: None,
            }))),
        }),
    };

    let expr = if let Some(idx) = idx {
        MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Member(expr)),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: idx as f64,
                    raw: None,
                }))),
            }),
        }
    } else {
        expr
    };

    Expr::Update(UpdateExpr {
        span: DUMMY_SP,
        op: UpdateOp::PlusPlus,
        prefix: false,
        arg: Box::new(Expr::Member(expr)),
    })
}
