//! What a body declares once at its top of the dictionaries it makes
//! (ADR 0979): those it writes twice or more, named by their type's and
//! their trait's words, of what a body's top can read. Split from
//! dictionaries.rs by concept, as a file is at most 1500 lines.

use super::super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of `made`, a body's dictionaries and their names, each that `body`
    /// writes twice or more: named there, and declared, for its top, each
    /// before what names it (ADR 0979).
    pub(in crate::lower) fn declare_repeated(
        &mut self,
        mut made: Vec<(String, Expr)>,
        body: &mut [js::Stmt],
    ) -> Vec<js::Stmt> {
        // The largest first, so one inside another is counted in its `const`.
        made.sort_by_key(|(_, e)| std::cmp::Reverse(size(e)));
        let mut declared: Vec<js::Stmt> = Vec::new();
        for (hint, dictionary) in made {
            let mut count = 0;
            let mut counting = |e: &mut Expr| count += usize::from(*e == dictionary);
            js::each_expr_mut(body, &mut counting);
            js::each_expr_mut(&mut declared, &mut counting);
            if count < 2 {
                continue;
            }
            let name = self.fresh(&hint);
            let mut put = |e: &mut Expr| {
                if *e == dictionary {
                    *e = Expr::var(&name);
                }
            };
            js::each_expr_mut(body, &mut put);
            js::each_expr_mut(&mut declared, &mut put);
            declared.insert(0, StmtKind::Const(name, dictionary).at(js::Span::NONE));
        }
        declared
    }

    /// The name of a declared dictionary of `tr`: its type's and its
    /// trait's, `chainDoubleEndedIterator` or `u32Clone`, or its trait's
    /// alone of a type with no one name, `intoIterator`.
    pub(super) fn dictionary_name(&self, tr: ty::TraitRef<'tcx>) -> String {
        let of = tr.self_ty().peel_refs();
        let of = match of.kind() {
            ty::Adt(adt, _) => self.tcx.item_name(adt.did()).to_string(),
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => of.to_string(),
            ty::Tuple(parts) if parts.is_empty() => "unit".to_string(),
            ty::Tuple(_) => "tuple".to_string(),
            ty::Array(..) | ty::Slice(_) => "items".to_string(),
            _ => String::new(),
        };
        crate::lower::lower_first(&format!("{of}{}", self.tcx.item_name(tr.def_id)))
    }
}

/// Whether `e`, a dictionary made, holds a function: a compiled method
/// given its sources' dictionaries, not `{ next: $next }` alone.
pub(super) fn holds_a_function(e: &Expr) -> bool {
    // A runtime maker's of an object, `$partialEq({ eq })`'s: the object's.
    if let js::ExprKind::Call(callee, given) = &e.kind
        && callee.helper_name().is_some()
        && let [object] = given.as_slice()
    {
        return holds_a_function(object);
    }
    let js::ExprKind::Object(props, _) = &e.kind else {
        return false;
    };
    props
        .iter()
        .any(|prop| matches!(prop, Prop::Field(_, value) if matches!(value.kind, js::ExprKind::Arrow(..))))
}

/// How many expressions `e` is made of.
fn size(e: &Expr) -> usize {
    let mut count = 0;
    e.clone().each_mut(&mut |_| count += 1);
    count
}

/// Whether `e`, a dictionary made, is a generic impl's, a call of its
/// function, `holderTouch(TTouch)`: the program's item, given dictionaries.
pub(super) fn is_impl_call(e: &Expr) -> bool {
    // A runtime maker's of one, std's reader's defaults over a `Chain`'s
    // `Read`, `$readerDefaults(chainRead(..))`: that one's (#1156).
    if let js::ExprKind::Call(callee, given) = &e.kind
        && callee.helper_name().is_some()
        && let [made] = given.as_slice()
    {
        return is_impl_call(made);
    }
    matches!(&e.kind, js::ExprKind::Call(callee, _)
        if matches!(callee.kind, js::ExprKind::Var(_)) && callee.facts.has(js::Known::Item))
}

/// Whether `e` reads only what its body's top can: the runtime's helpers,
/// the program's items, the names its own functions bind, and `given`,
/// what the function was given (`given_names`, ADR 0979, amended).
pub(super) fn reads_only_at_top(e: &Expr, given: &IndexSet<String>) -> bool {
    let mut bound: Vec<String> = Vec::new();
    let mut each = e.clone();
    each.each_mut(&mut |part| {
        if let js::ExprKind::Arrow(params, _) = &mut part.kind {
            for param in params {
                param.each_name_mut(&mut |name| bound.push(name.clone()));
            }
        }
    });
    let mut only = true;
    let mut each = e.clone();
    each.each_mut(&mut |part| {
        if let js::ExprKind::Var(name) = &part.kind {
            // A JS global, `Array` of `Array.from`, which no local's name
            // is (ADR 0352), as a helper is: read the same at the top (#1239).
            only &= part.helper_name().is_some()
                || crate::names::is_js_global(name)
                || part.facts.has(js::Known::Item)
                || bound.contains(name)
                || given.contains(name);
        }
    });
    only
}
