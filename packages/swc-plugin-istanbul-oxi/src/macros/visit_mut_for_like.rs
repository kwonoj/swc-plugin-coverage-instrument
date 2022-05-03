/// A macro creates body for the for-variant visitors (for, for-of, for-in) which
/// shares same logic. This also works for other loops like while, do-while.
#[macro_export]
macro_rules! visit_mut_for_like {
    ($self: ident, $for_like_stmt: ident) => {
        use swc_plugin::{syntax_pos::DUMMY_SP, utils::take::Take};

        let (old, ignore_current) = $self.on_enter($for_like_stmt);

        match ignore_current {
            Some(istanbul_oxi_instrument::hint_comments::IgnoreScope::Next) => {}
            _ => {
                // cover_statement's is_stmt prepend logic for individual child stmt visitor
                $self.mark_prepend_stmt_counter(&$for_like_stmt.span);

                let body = *$for_like_stmt.body.take();
                // if for stmt body is not block, wrap it before insert statement counter
                let body = if let Stmt::Block(body) = body {
                    body
                } else {
                    let stmts = vec![body];
                    BlockStmt {
                        span: DUMMY_SP,
                        stmts,
                    }
                };

                $for_like_stmt.body = Box::new(Stmt::Block(body));
                // Iterate children for inner stmt's counter insertion
                $for_like_stmt.visit_mut_children_with($self);
            }
        }

        $self.on_exit(old);
    };
}
