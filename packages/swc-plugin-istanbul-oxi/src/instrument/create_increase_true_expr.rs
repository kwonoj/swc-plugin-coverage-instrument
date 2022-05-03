use swc_plugin::ast::*;

/// Reads the logic expression conditions and conditionally increments truthy counter.
/// This is always known to be b_t type counter does not need to accept what type of ident it'll create.
pub(crate) fn create_increase_true_expr(id: u32, idx: u32, var_name: &Ident) -> Expr {
    todo!("not implemented");
}
