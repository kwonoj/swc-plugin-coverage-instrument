//! TODO: macro

use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

fn create_str(value: &str) -> Str {
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

pub fn create_ident_key_value_prop(key: &Ident, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(key.clone()),
        value: Box::new(value),
    })))
}

pub fn create_str_key_value_prop(key: &str, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(create_str(&key)),
        value: Box::new(value),
    })))
}
