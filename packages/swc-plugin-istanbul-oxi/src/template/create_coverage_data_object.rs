use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, COVERAGE_MAGIC_VALUE};
use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

use crate::{
    constants::idents::{
        IDENT_B, IDENT_BRANCH_MAP, IDENT_BT, IDENT_COLUMN, IDENT_COVERAGE_MAGIC_KEY, IDENT_END,
        IDENT_F, IDENT_FN_MAP, IDENT_HASH, IDENT_LINE, IDENT_PATH, IDENT_S, IDENT_START,
        IDENT_STATEMENT_MAP,
    },
    utils::ast_builder::{create_num_lit_expr, create_str, create_str_lit_expr},
};

pub fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    let mut props = vec![];

    // Afaik there's no built-in way to iterate over struct properties
    if coverage_data.all {
        let all_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("all".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Bool(true.into()))),
        })));
        props.push(all_prop);
    }

    let path_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_PATH.clone()),
        value: Box::new(create_str_lit_expr(&coverage_data.path)),
    })));
    props.push(path_prop);

    let statement_map_prop_values = coverage_data
        .statement_map
        .iter()
        .map(|(key, value)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Str(create_str(&key.to_string())),
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
                                        value: Box::new(create_num_lit_expr(value.start.line)),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_COLUMN.clone()),
                                        value: Box::new(create_num_lit_expr(value.start.column)),
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
                                        value: Box::new(create_num_lit_expr(value.end.line)),
                                    }))),
                                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                        key: PropName::Ident(IDENT_COLUMN.clone()),
                                        value: Box::new(create_num_lit_expr(value.end.column)),
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
                key: PropName::Str(create_str(&key.to_string())),
                value: Box::new(create_num_lit_expr(*value)),
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
        value: Box::new(create_str_lit_expr(&COVERAGE_MAGIC_VALUE)),
    })));
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object, but we use props ast instead.
    let mut hasher = DefaultHasher::new();
    props.hash(&mut hasher);
    let hash = hasher.finish().to_string();

    let hash_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_HASH.clone()),
        value: Box::new(create_str_lit_expr(&hash)),
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
    use istanbul_oxi_instrument::{BranchType, FileCoverage, Range, SourceCoverage};
    use swc_ecma_quote::quote;
    use swc_plugin::ast::*;

    use crate::template::create_coverage_data_object::create_coverage_data_object;

    use pretty_assertions::assert_eq;

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

    #[test]
    fn should_create_statement_map() {
        let mut coverage_data = SourceCoverage::new("/test/src/statement.js".to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        coverage_data.new_statement(&dummy_range);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let expected = quote!(
            r#"
        {
            path: "/test/src/statement.js",
            statementMap: {
                "0": {
                    start: {
                        line: 2,
                        column: 3
                    },
                    end: {
                        line: 5,
                        column: 2
                    }
                }
            },
            fnMap: {},
            branchMap: {},
            s: {
                "0": 0
            },
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "673786009243969507"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }
}
