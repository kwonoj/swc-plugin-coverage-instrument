//! Static ident declarations being used across template

use once_cell::sync::Lazy;
use swc_ecma_quote::swc_ecma_ast::Ident;
use swc_plugin::utils::take::Take;

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
