use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

use crate::coverage_visitor::UnknownReserved;

/// Creates a expr like `cov_17709493053001988098().s[0]++;`
pub(crate) fn build_increase_expression_expr(
    type_ident: &Ident,
    id: u32,
    var_name: &Ident,
    i: Option<UnknownReserved>,
) -> Expr {
    if let Some(_i) = i {
        todo!("Not implemented yet!")
    } else {
        let call = CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(var_name.clone()))),
            args: vec![],
            type_args: None,
        };

        let c = MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Call(call)),
            prop: MemberProp::Ident(type_ident.clone()),
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

        Expr::Update(UpdateExpr {
            span: DUMMY_SP,
            op: UpdateOp::PlusPlus,
            prefix: false,
            arg: Box::new(Expr::Member(expr)),
        })
    }
}
