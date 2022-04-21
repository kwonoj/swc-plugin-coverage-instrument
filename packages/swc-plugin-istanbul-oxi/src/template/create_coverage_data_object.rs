use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, COVERAGE_MAGIC_KEY, COVERAGE_MAGIC_VALUE};
use swc_plugin::{ast::*, syntax_pos::DUMMY_SP, utils::take::Take};

use crate::constants::idents::{
    IDENT_B, IDENT_BRANCH_MAP, IDENT_BT, IDENT_COLUMN, IDENT_COVERAGE_MAGIC_KEY, IDENT_END,
    IDENT_F, IDENT_FN_MAP, IDENT_HASH, IDENT_LINE, IDENT_PATH, IDENT_S, IDENT_START,
    IDENT_STATEMENT_MAP,
};

pub fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    let mut props = vec![];

    // Afaik there's no built-in way to iterate over struct properties
    if coverage_data.all {
        let all_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("all".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Bool(Bool::from(true)))),
        })));
        props.push(all_prop);
    }

    let path_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_PATH.clone()),
        value: Box::new(Expr::Lit(Lit::Str(Str {
            value: coverage_data.path.to_string().into(),
            raw: Some(format!(r#""{}""#, coverage_data.path).into()),
            ..Str::dummy()
        }))),
    })));
    props.push(path_prop);

    let statement_map_prop_values = coverage_data
        .statement_map
        .iter()
        .map(|(key, value)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(Ident::new(key.to_string().into(), DUMMY_SP)),
                value: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(IDENT_START.clone()),
                            value: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: vec![
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_LINE.clone()),
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.start.line as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_COLUMN.clone()),
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.start.column as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                ],
                            })),
                        }))),
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(IDENT_END.clone()),
                            value: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: vec![
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_LINE.clone()),
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.end.line as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_COLUMN.clone()),
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.end.column as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                ],
                            })),
                        }))),
                    ],
                })),
            })))
        })
        .collect();

    let statement_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_STATEMENT_MAP.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: statement_map_prop_values,
        })),
    })));
    props.push(statement_map_prop);

    let fn_map_prop_values = vec![];
    let fn_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_FN_MAP.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fn_map_prop_values,
        })),
    })));
    props.push(fn_map_prop);

    let branch_map_prop_values = vec![];
    let branch_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_BRANCH_MAP.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: branch_map_prop_values,
        })),
    })));
    props.push(branch_map_prop);

    let s_prop_values = coverage_data
        .s
        .iter()
        .map(|(key, value)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(Ident::new(key.to_string().into(), DUMMY_SP)),
                value: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: *value as f64,
                    raw: None,
                }))),
            })))
        })
        .collect();

    let s_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_S.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: s_prop_values,
        })),
    })));
    props.push(s_prop);

    let f_prop_values = vec![];
    let f_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_F.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: f_prop_values,
        })),
    })));
    props.push(f_prop);

    let b_prop_values = vec![];
    let b_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_B.clone()),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        })),
    })));
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = vec![];
        let b_t_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IDENT_BT.clone()),
            value: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            })),
        })));
        props.push(b_t_prop);
    }

    // fill in _coverageSchema, and hash
    let coverage_schema_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_COVERAGE_MAGIC_KEY.clone()),
        value: Box::new(Expr::Lit(Lit::Str(Str {
            value: COVERAGE_MAGIC_VALUE.into(),
            raw: Some(format!(r#""{}""#, COVERAGE_MAGIC_VALUE).into()),
            ..Str::dummy()
        }))),
    })));
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object, but we use props ast instead.
    let mut hasher = DefaultHasher::new();
    props.hash(&mut hasher);
    let hash = hasher.finish().to_string();

    let hash_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_HASH.clone()),
        value: Box::new(Expr::Lit(Lit::Str(Str {
            value: hash.clone().into(),
            raw: Some(format!(r#""{}""#, hash.clone()).into()),
            ..Str::dummy()
        }))),
    })));
    props.push(hash_prop);

    (
        hash,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        }),
    )
}

#[cfg(test)]
mod tests {
    use istanbul_oxi_instrument::FileCoverage;
    use swc_ecma_quote::quote;
    use swc_plugin::ast::*;

    use crate::template::create_coverage_data_object::create_coverage_data_object;

    use pretty_assertions::assert_eq;

    #[macro_export]
    macro_rules! try_assert {
        ($v: tt, $($tt:tt)*) => {{
            let expected_expr = swc_ecma_quote::quote!($($tt)* as Expr);
            if $v != expected_expr {

            }
        }};
    }

    #[test]
    fn should_create_empty() {
        let coverage_data = FileCoverage::empty("anon".to_string(), false);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let expected = quote!(
            r#"
        {
            path: "anon",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "7200395314456256211"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_empty_report_logic() {
        let coverage_data = FileCoverage::empty("/test/src/file.js".to_string(), true);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let expected = quote!(
            r#"
        {
            path: "/test/src/file.js",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            bT: {},
            _coverageSchema: "7101652470475984838",
            hash: "15473612320079640285"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }
}
