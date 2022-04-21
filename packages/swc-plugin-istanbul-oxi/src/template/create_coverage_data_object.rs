use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, COVERAGE_MAGIC_KEY, COVERAGE_MAGIC_VALUE};
use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

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
        key: PropName::Ident(Ident::new("path".into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(
            coverage_data.path.to_string(),
        )))),
    })));
    props.push(path_prop);

    let statement_map_prop_values = coverage_data
        .statement_map
        .iter()
        .map(|(key, value)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(Ident::new(key.to_string().into(), DUMMY_SP)),
                //{"start":{"line":2,"column":11},"end":{"line":2,"column":3}}
                value: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new("start".to_string().into(), DUMMY_SP)),
                            //{"start":{"line":2,"column":11},"end":{"line":2,"column":3}}
                            value: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: vec![
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(Ident::new(
                                            "line".to_string().into(),
                                            DUMMY_SP,
                                        )),
                                        //{"start":{"line":2,"column":11},"end":{"line":2,"column":3}}
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.start.line as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(Ident::new(
                                            "column".to_string().into(),
                                            DUMMY_SP,
                                        )),
                                        //{"start":{"line":2,"column":11},"end":{"line":2,"column":3}}
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
                            key: PropName::Ident(Ident::new("end".to_string().into(), DUMMY_SP)),
                            value: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: vec![
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(Ident::new(
                                            "line".to_string().into(),
                                            DUMMY_SP,
                                        )),
                                        value: Box::new(Expr::Lit(Lit::Num(Number {
                                            span: DUMMY_SP,
                                            value: value.end.line as f64,
                                            raw: None,
                                        }))),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(Ident::new(
                                            "column".to_string().into(),
                                            DUMMY_SP,
                                        )),
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
        key: PropName::Ident(Ident::new("statementMap".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: statement_map_prop_values,
        })),
    })));
    props.push(statement_map_prop);

    let fn_map_prop_values = vec![];
    let fn_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("fnMap".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fn_map_prop_values,
        })),
    })));
    props.push(fn_map_prop);

    let branch_map_prop_values = vec![];
    let branch_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("branchMap".into(), DUMMY_SP)),
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
        key: PropName::Ident(Ident::new("s".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: s_prop_values,
        })),
    })));
    props.push(s_prop);

    let f_prop_values = vec![];
    let f_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("f".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: f_prop_values,
        })),
    })));
    props.push(f_prop);

    let b_prop_values = vec![];
    let b_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("b".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        })),
    })));
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = vec![];
        let b_t_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("bT".into(), DUMMY_SP)),
            value: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            })),
        })));
        props.push(b_t_prop);
    }

    // fill in _coverageSchema, and hash
    let coverage_schema_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new(COVERAGE_MAGIC_KEY.clone().into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(COVERAGE_MAGIC_VALUE.clone())))),
    })));
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object, but we use props ast instead.
    let mut hasher = DefaultHasher::new();
    props.hash(&mut hasher);
    let hash = hasher.finish().to_string();

    let hash_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("hash".into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(hash.clone())))),
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
