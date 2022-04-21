//! Static ident declarations being used across template

use istanbul_oxi_instrument::COVERAGE_MAGIC_KEY;
use once_cell::sync::Lazy;
use swc_ecma_quote::swc_ecma_ast::Ident;
use swc_plugin::utils::take::Take;

pub static IDENT_ALL: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "all".into(),
    ..Ident::dummy()
});

pub static IDENT_HASH: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "hash".into(),
    ..Ident::dummy()
});

pub static IDENT_PATH: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "path".into(),
    ..Ident::dummy()
});

pub static IDENT_GCV: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "gcv".into(),
    ..Ident::dummy()
});

pub static IDENT_COVERAGE_DATA: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "coverageData".into(),
    ..Ident::dummy()
});

pub static IDENT_GLOBAL: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "global".into(),
    ..Ident::dummy()
});

pub static IDENT_START: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "start".into(),
    ..Ident::dummy()
});

pub static IDENT_END: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "end".into(),
    ..Ident::dummy()
});

pub static IDENT_LINE: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "line".into(),
    ..Ident::dummy()
});

pub static IDENT_COLUMN: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "column".into(),
    ..Ident::dummy()
});

pub static IDENT_STATEMENT_MAP: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "statementMap".into(),
    ..Ident::dummy()
});

pub static IDENT_FN_MAP: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "fnMap".into(),
    ..Ident::dummy()
});

pub static IDENT_BRANCH_MAP: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "branchMap".into(),
    ..Ident::dummy()
});

pub static IDENT_S: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "s".into(),
    ..Ident::dummy()
});

pub static IDENT_F: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "f".into(),
    ..Ident::dummy()
});

pub static IDENT_B: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "b".into(),
    ..Ident::dummy()
});

pub static IDENT_BT: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "bT".into(),
    ..Ident::dummy()
});

pub static IDENT_COVERAGE_MAGIC_KEY: Lazy<Ident> = Lazy::new(|| Ident {
    sym: COVERAGE_MAGIC_KEY.into(),
    ..Ident::dummy()
});

pub static IDENT_NAME: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "name".into(),
    ..Ident::dummy()
});

pub static IDENT_DECL: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "decl".into(),
    ..Ident::dummy()
});

pub static IDENT_LOC: Lazy<Ident> = Lazy::new(|| Ident {
    sym: "loc".into(),
    ..Ident::dummy()
});
