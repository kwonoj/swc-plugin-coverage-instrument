//! Naive wrapper to create commonly used ast types
/// Temporal type for unknown.
#[derive(Debug)]
pub struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}
