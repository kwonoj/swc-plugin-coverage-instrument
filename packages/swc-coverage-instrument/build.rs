use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

/// Create compile-time constant values for the coverage schema hash & coverage lib version hash (magic-value)
fn main() {
    let magic_key = "_coverageSchema";
    let mut hasher = DefaultHasher::new();
    let name = std::env::var("CARGO_PKG_NAME").unwrap();
    // Use major as schema version, changing schema means major breaking anyway.
    let version = std::env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    format!("{}@{}", name, version).hash(&mut hasher);
    let magic_value = hasher.finish().to_string();

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("constants.rs");

    std::fs::write(
        &path,
        format!(
            r#"pub static COVERAGE_MAGIC_KEY: &'static str = "{}";
pub static COVERAGE_MAGIC_VALUE: &'static str = "{}";"#,
            magic_key, magic_value
        ),
    )
    .unwrap();

    let out_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::PathBuf::from(&out_dir)
        .join("../../spec/util/")
        .join("constants.ts");

    std::fs::write(
        &path,
        format!(
            r#"const COVERAGE_MAGIC_KEY = "{}";
const COVERAGE_MAGIC_VALUE = "{}";

export {{
  COVERAGE_MAGIC_KEY,
  COVERAGE_MAGIC_VALUE
}}"#,
            magic_key, magic_value
        ),
    )
    .unwrap();
}
