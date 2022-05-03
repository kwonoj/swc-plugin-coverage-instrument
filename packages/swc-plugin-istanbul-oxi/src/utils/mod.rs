//! Naive wrapper to create commonly used ast types
pub(crate) mod ast_builder;
pub(crate) mod hint_comments;
pub(crate) mod lookup_range;

/// Temporal type for unknown.
#[derive(Debug)]
pub struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}
