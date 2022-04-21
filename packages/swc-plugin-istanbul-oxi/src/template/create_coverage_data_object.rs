use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, Range, COVERAGE_MAGIC_VALUE};
use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

use crate::{
    constants::idents::*,
    utils::ast_builder::{
        create_ident_key_value_prop, create_num_lit_expr, create_str_key_value_prop,
        create_str_lit_expr,
    },
};

fn create_range_prop(key: &str, value: &Range) -> PropOrSpread {
    create_str_key_value_prop(
        key,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
                create_ident_key_value_prop(
                    &IDENT_START,
                    Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![
                            create_ident_key_value_prop(
                                &IDENT_LINE,
                                create_num_lit_expr(value.start.line),
                            ),
                            create_ident_key_value_prop(
                                &IDENT_COLUMN,
                                create_num_lit_expr(value.start.column),
                            ),
                        ],
                    }),
                ),
                create_ident_key_value_prop(
                    &IDENT_END,
                    Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![
                            create_ident_key_value_prop(
                                &IDENT_LINE,
                                create_num_lit_expr(value.end.line),
                            ),
                            create_ident_key_value_prop(
                                &IDENT_COLUMN,
                                create_num_lit_expr(value.end.column),
                            ),
                        ],
                    }),
                ),
            ],
        }),
    )
}

pub fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    // Afaik there's no built-in way to iterate over struct properties via keys.

    let mut props = vec![];

    // assign coverage['all']
    if coverage_data.all {
        let all_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("all".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Bool(true.into()))),
        })));

        props.push(all_prop);
    }

    // assign coverage['path']
    props.push(create_ident_key_value_prop(
        &IDENT_PATH,
        create_str_lit_expr(&coverage_data.path),
    ));

    // assign coverage['statementMap']
    let statement_map_prop_values = coverage_data
        .statement_map
        .iter()
        .map(|(key, value)| create_range_prop(&key.to_string(), value))
        .collect();

    let statement_map_prop = create_ident_key_value_prop(
        &IDENT_STATEMENT_MAP,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: statement_map_prop_values,
        }),
    );
    props.push(statement_map_prop);

    // assign coverage['fnMap']
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
            create_str_key_value_prop(&key.to_string(), create_num_lit_expr(*value))
        })
        .collect();

    let s_prop = create_ident_key_value_prop(
        &IDENT_S,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: s_prop_values,
        }),
    );
    props.push(s_prop);

    let f_prop_values = vec![];
    let f_prop = create_ident_key_value_prop(
        &IDENT_F,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: f_prop_values,
        }),
    );
    props.push(f_prop);

    let b_prop_values = vec![];
    let b_prop = create_ident_key_value_prop(
        &IDENT_B,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        }),
    );
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = vec![];
        let b_t_prop = create_ident_key_value_prop(
            &IDENT_BT,
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            }),
        );
        props.push(b_t_prop);
    }

    // fill in _coverageSchema, and hash
    let coverage_schema_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IDENT_COVERAGE_MAGIC_KEY.clone()),
        value: Box::new(create_str_lit_expr(&COVERAGE_MAGIC_VALUE)),
    })));
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object. In here uses str-serialized object instead.
    let coverage_str =
        serde_json::to_string(coverage_data).expect("Should able to serialize coverage data");
    let mut hasher = DefaultHasher::new();
    coverage_str.hash(&mut hasher);
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
            hash: "2749072808032864045"
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
            hash: "5324777076056671972"
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
            hash: "14358638674647738158"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }
}
