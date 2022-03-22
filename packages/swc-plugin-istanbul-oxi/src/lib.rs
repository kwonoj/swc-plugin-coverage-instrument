use swc_plugin::{ast::*, plugin_transform};

struct CoverageVisitor;

impl VisitMut for CoverageVisitor {}

#[plugin_transform]
pub fn process(program: Program, _plugin_config: String, _context: String) -> Program {
    program.fold_with(&mut as_folder(CoverageVisitor))
}
