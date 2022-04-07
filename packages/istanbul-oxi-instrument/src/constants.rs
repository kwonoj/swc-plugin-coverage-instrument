use once_cell::sync::Lazy;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub static COVERAGE_MAGIC_KEY: Lazy<String> = Lazy::new(|| {
    return "_coverageSchema".to_string();
});

pub static COVERAGE_MAGIC_VALUE: Lazy<String> = Lazy::new(|| {
    let mut s = DefaultHasher::new();
    let name = "istanbul-oxi-instrument";
    format!("{}@{}", name, 1).hash(&mut s);
    s.finish().to_string()
});
