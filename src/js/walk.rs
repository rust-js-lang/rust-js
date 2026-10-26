//! The walks of a tree's statements and expressions, each written once and
//! made twice: to read it, `each_list`, `each_expr` and `Expr::each`, and to
//! change it, `statement_lists`, `each_expr_mut` and `Expr::each_mut`. Both
//! go through the same nodes, in the same order, so what reads a tree
//! needn't copy it to walk it.

use super::*;

macro_rules! walks {
    (
        $([$($m:tt)*])?
        lists: $lists:ident($list:ty),
        slotted: $slotted:ident,
        exprs: $exprs:ident,
        expr: $expr:ident,
        cases: $cases:ident,
        iter: $iter:ident,
        body: |$function:ident| $function_body:expr,
    ) => {
        /// `stmts`, and each list nested in its statements, not in its closures.
        pub fn $lists(stmts: $list, f: &mut dyn FnMut($list)) {
            $slotted(stmts, Slot::Block, &mut |list, _| f(list));
        }

        /// `$lists`, each list given where it is, `stmts` at `slot`.
        pub fn $slotted(stmts: $list, slot: Slot, f: &mut dyn FnMut($list, Slot)) {
            f(stmts, slot);
            for stmt in stmts.$iter() {
                match & $($($m)*)? stmt.kind {
                    StmtKind::If(_, a, b) => {
                        $slotted(a, Slot::Block, f);
                        if let Some(b) = b {
                            $slotted(b, Slot::Else, f);
                        }
                    }
                    StmtKind::While { body, .. }
                    | StmtKind::ForOf { body, .. }
                    | StmtKind::For { body, .. }
                    | StmtKind::Labeled(_, body) => $slotted(body, Slot::Block, f),
                    StmtKind::Switch { cases, default, .. } => {
                        $cases(cases, default).for_each(|body| $slotted(body, Slot::Block, f));
                    }
                    StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                        $slotted(a, Slot::Block, f);
                        $slotted(b, Slot::Block, f);
                    }
                    // A function's, as a closure's, are its own.
                    StmtKind::Const(..)
                    | StmtKind::Let(..)
                    | StmtKind::Destructure { .. }
                    | StmtKind::Assign(..)
                    | StmtKind::Compound(..)
                    | StmtKind::Expr(_)
                    | StmtKind::Break(_)
                    | StmtKind::Continue(_)
                    | StmtKind::Directive(_) | StmtKind::Comments
                    | StmtKind::Return(_)
                    | StmtKind::Throw(_)
                    | StmtKind::Function(_) => {}
                }
            }
        }

        /// Each expression in `stmts`, outermost first, nested ones too, in
        /// closures' bodies, JSX and templates.
        pub fn $exprs(stmts: & $($($m)*)? [Stmt], f: &mut dyn FnMut(& $($($m)*)? Expr)) {
            for stmt in stmts {
                match & $($($m)*)? stmt.kind {
                    StmtKind::Const(_, e)
                    | StmtKind::Let(_, Some(e))
                    | StmtKind::Destructure { value: e, .. }
                    | StmtKind::Expr(e)
                    | StmtKind::Return(Some(e))
                    | StmtKind::Throw(e) => e.$expr(f),
                    StmtKind::Assign(a, b) | StmtKind::Compound(_, a, b) => {
                        a.$expr(f);
                        b.$expr(f);
                    }
                    StmtKind::If(test, a, b) => {
                        test.$expr(f);
                        $exprs(a, f);
                        if let Some(b) = b {
                            $exprs(b, f);
                        }
                    }
                    StmtKind::While { cond, body, .. } => {
                        cond.$expr(f);
                        $exprs(body, f);
                    }
                    StmtKind::ForOf { iterable, body, .. } => {
                        iterable.$expr(f);
                        $exprs(body, f);
                    }
                    StmtKind::For { start, test, body, .. } => {
                        start.$expr(f);
                        test.$expr(f);
                        $exprs(body, f);
                    }
                    StmtKind::Labeled(_, body) => $exprs(body, f),
                    StmtKind::Switch {
                        discriminant,
                        cases,
                        default,
                    } => {
                        discriminant.$expr(f);
                        for Case { values, body, .. } in cases.$iter() {
                            values.$iter().for_each(|value| value.$expr(f));
                            $exprs(body, f);
                        }
                        if let Some(default) = default {
                            $exprs(default, f);
                        }
                    }
                    StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                        $exprs(a, f);
                        $exprs(b, f);
                    }
                    StmtKind::Function($function) => $exprs($function_body, f),
                    StmtKind::Let(_, None)
                    | StmtKind::Break(_)
                    | StmtKind::Continue(_)
                    | StmtKind::Return(None)
                    | StmtKind::Directive(_) | StmtKind::Comments => {}
                }
            }
        }

        impl Expr {
            /// This, then each expression in it, as its statements' walk goes.
            pub fn $expr(& $($($m)*)? self, f: &mut dyn FnMut(& $($($m)*)? Expr)) {
                f(self);
                let props = |props: & $($($m)*)? [Prop], f: &mut dyn FnMut(& $($($m)*)? Expr)| {
                    for prop in props {
                        let (Prop::Field(_, value)
                        | Prop::Spread(value)
                        | Prop::Getter(_, value)
                        | Prop::Setter(_, value) | Prop::Method(_, value)) = prop;
                        value.$expr(f);
                    }
                };
                match & $($($m)*)? self.kind {
                    ExprKind::Member(a, _)
                    | ExprKind::OptionalMember(a, _)
                    | ExprKind::Unary(_, a)
                    | ExprKind::Await(a)
                    | ExprKind::Spread(a)
                    | ExprKind::Stringed(a)
                    | ExprKind::Handle(a, _)
                    | ExprKind::Assigned(_, a)
                    | ExprKind::Stepped(_, a)
                    | ExprKind::DropArgument(_, _, a) => a.$expr(f),
                    ExprKind::Index(a, b)
                    | ExprKind::OptionalIndex(a, b)
                    | ExprKind::Binary(_, a, b)
                    | ExprKind::Pair(a, b) => {
                        a.$expr(f);
                        b.$expr(f);
                    }
                    ExprKind::Cond(a, b, c) => {
                        a.$expr(f);
                        b.$expr(f);
                        c.$expr(f);
                    }
                    ExprKind::Call(callee, args)
                    | ExprKind::OptionalCall(callee, args)
                    | ExprKind::New(callee, args) => {
                        callee.$expr(f);
                        args.$iter().for_each(|a| a.$expr(f));
                    }
                    ExprKind::Array(items) | ExprKind::Template(_, items, _) => {
                        items.$iter().for_each(|a| a.$expr(f))
                    }
                    ExprKind::Object(fields, ..) => props(fields, f),
                    ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => $exprs(body, f),
                    ExprKind::Function($function) => $exprs($function_body, f),
                    ExprKind::Jsx(jsx) => {
                        if let JsxTag::Component(tag) = & $($($m)*)? jsx.tag {
                            tag.$expr(f);
                        }
                        props(& $($($m)*)? jsx.props, f);
                        jsx.children.$iter().for_each(|c| c.$expr(f));
                    }
                    ExprKind::Var(_)
                    | ExprKind::Num(_)
                    | ExprKind::BigInt(_)
                    | ExprKind::BigUint(_)
                    | ExprKind::Bool(_)
                    | ExprKind::Str(_)
                    | ExprKind::Written(_)
                    | ExprKind::Undefined
                    | ExprKind::GivenUndefined
                    | ExprKind::Null
                    | ExprKind::This
                    | ExprKind::Symbol(_)
                    | ExprKind::ItemHole(_)
                    | ExprKind::Import(_)
                    | ExprKind::Regex(_) => {}
                }
            }
        }
    };
}

/// Where a list of statements is: an `if`'s `else`, or another block, a
/// function's body, a branch's, a loop's or a case's.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Block,
    Else,
}

walks! {
    lists: each_list(&[Stmt]),
    slotted: each_list_in,
    exprs: each_expr,
    expr: each,
    cases: case_lists,
    iter: iter,
    body: |function| &function.body,
}

walks! {
    [mut]
    lists: statement_lists(&mut Vec<Stmt>),
    slotted: statement_lists_in,
    exprs: each_expr_mut,
    expr: each_mut,
    cases: case_lists_mut,
    iter: iter_mut,
    body: |function| &mut function.body,
}
