use swc_ecma_quote::{
    swc_common::DUMMY_SP,
    swc_ecma_ast::{Expr, Lit, Number, Str},
};

pub fn create_str(value: &str) -> Str {
    Str {
        value: value.clone().into(),
        raw: Some(format!(r#""{}""#, value).into()),
        span: DUMMY_SP,
    }
}

pub fn create_str_lit_expr(value: &str) -> Expr {
    Expr::Lit(Lit::Str(create_str(value)))
}

pub fn create_num_lit_expr(value: u32) -> Expr {
    Expr::Lit(Lit::Num(Number {
        value: value.clone().into(),
        raw: Some(value.to_string().into()),
        span: DUMMY_SP,
    }))
}
