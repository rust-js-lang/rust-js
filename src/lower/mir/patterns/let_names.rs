//! What a body's `let`s and patterns bind, by name, for tidy
//! (`tidy::LetNames`): the `let`s the program wrote, of those the ones it
//! said it changes, that end a borrow or hold a frozen one, a drop guard's,
//! and a pattern's binding named as the field its scrutinee reads.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What the body's `let`s bind, by the names `state` gives them: a
    /// struct or tuple pattern's bindings (`let_pattern_bindings`); a name
    /// alone, `let init = lazy.init;`, a statement of its own, which no
    /// pattern of a part read beside it is (ADR 0598, amended), but a copy
    /// of a pattern's binding named as it is, `let x = *x;`, the pattern's
    /// (#546); a cast's too, `let lazy: &LazyComponent = unsafe { js::cast(children) };`,
    /// `const lazy = children;`; and of those, one an `if let` takes
    /// apart, its scrutinee the variable alone, `let types =
    /// t.types.borrow().clone(); if let Some(types) = types` (ADR 0866),
    /// not a cast's, the program's name for a value of the type it gives.
    pub(in crate::lower::mir) fn let_names(
        &self,
        body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> crate::lower::mir::tidy::LetNames {
        let thir = self.frame.thir;
        // Through an `unsafe { .. }` of the value alone, and what `strip` is through.
        let through = |mut e: thir::ExprId| loop {
            let inner = match &thir[e].kind {
                thir::ExprKind::Block { block } if thir[*block].stmts.is_empty() => thir[*block].expr,
                _ => Some(crate::lower::body_queries::strip(thir, e)),
            };
            match inner {
                Some(inner) if inner != e => e = inner,
                _ => return e,
            }
        };
        // The field a borrow's receiver is, `&current.child`, by its Rust name.
        let field_named = |receiver: Option<thir::ExprId>| {
            let mut e = through(receiver?);
            while let thir::ExprKind::Borrow { arg, .. } | thir::ExprKind::Deref { arg } = thir[e].kind {
                e = through(arg);
            }
            let thir::ExprKind::Field {
                lhs,
                variant_index,
                name,
            } = thir[e].kind
            else {
                return None;
            };
            let ty::Adt(adt, _) = thir[lhs].ty.peel_refs().kind() else {
                return None;
            };
            Some(adt.variant(variant_index).fields[name].name)
        };
        // A `RefCell`'s field's value cloned, `current.child.borrow().clone()`:
        // the value read, the borrow ended, and the field's name (ADR 0866,
        // amended).
        let borrow_cloned = |e: thir::ExprId| -> Option<rustc_span::Symbol> {
            let thir::ExprKind::Call { fun, ref args, .. } = thir[through(e)].kind else {
                return None;
            };
            let [cloned] = args[..] else {
                return None;
            };
            if !fn_def(thir[fun].ty).is_some_and(|(id, _)| self.tcx.is_lang_item(id, LangItem::CloneFn)) {
                return None;
            }
            // Through `&*`, and a `Ref`'s `Deref::deref` of it.
            let mut e = through(cloned);
            loop {
                e = match thir[e].kind {
                    thir::ExprKind::Borrow { arg, .. } | thir::ExprKind::Deref { arg } => through(arg),
                    thir::ExprKind::Call { fun, ref args, .. } => {
                        let (id, generic_args) = fn_def(thir[fun].ty)?;
                        let borrow = Std::Borrow {
                            lock: false,
                            mutable: false,
                        };
                        if self.recognition().classify(id, generic_args) == Some(borrow) {
                            return field_named(args.first().copied());
                        }
                        match args[..] {
                            [arg] if self.tcx.trait_of_assoc(id) == self.tcx.lang_items().deref_trait() => through(arg),
                            _ => return None,
                        }
                    }
                    _ => return None,
                };
            }
        };
        // A shared borrow, `&T` or a `RefCell`'s `Ref<T>`, of a `T` with no
        // cell of its own: what it's to can't change while it's held (rd658).
        let frozen_borrow = |ty: Ty<'tcx>| {
            let held = match ty.kind() {
                ty::Ref(_, held, ty::Mutability::Not) => Some(*held),
                ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCellRef) => args.types().next(),
                _ => None,
            };
            held.is_some_and(|held| !held.has_escaping_bound_vars() && held.is_freeze(self.tcx, self.typing_env))
        };
        // A copy of a struct pattern's field's binding, named as it is, `let
        // parent_object = *parent_object;`: the pattern's own, the value it
        // reads, which nothing runs between, not a statement of its own (#546).
        let copied_field = |binding: rustc_span::Symbol, initializer: Option<thir::ExprId>| {
            let Some(thir::ExprKind::Deref { arg }) = initializer.map(|e| &thir[through(e)].kind) else {
                return false;
            };
            matches!(thir[through(*arg)].kind, thir::ExprKind::VarRef { id }
                if self.tcx.hir_name(id.0) == binding
                    && matches!(self.tcx.parent_hir_node(id.0), rustc_hir::Node::PatField(_)))
        };
        let mut named = Vec::new();
        let mut mutable = Vec::new();
        let mut ends_borrow = Vec::new();
        let mut frozen = Vec::new();
        let mut guards = Vec::new();
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                pattern, initializer, ..
            } = &stmt.kind
                && let thir::PatKind::Binding {
                    subpattern: None,
                    name: binding,
                    mode,
                    ..
                } = pattern.kind
            {
                if !copied_field(binding, *initializer) {
                    named.push(pattern.span);
                }
                if mode.1.is_mut() {
                    mutable.push(pattern.span);
                }
                // Named as the field it reads, by their Rust names, whatever
                // the JS ones: `r#return`'s `return$` of `.return`, and a
                // field's `#[rust_js::name]`, `_debugHookTypes`.
                if initializer.and_then(borrow_cloned) == Some(binding) {
                    ends_borrow.push(pattern.span);
                }
                if frozen_borrow(pattern.ty) {
                    frozen.push(pattern.span);
                }
            }
        }
        // An `if let`'s or a let chain's binding of its scrutinee's whole
        // payload, `Some(debug_task)`, named as the field the scrutinee
        // reads, through a call's first argument, a method's receiver or a
        // cast's value, `fiber.debug_task.get()`, by their Rust names, of a
        // field `#[rust_js::name = "_debugTask"]` too (ADR 0866, amended,
        // rd706).
        fn payload(pat: &thir::Pat<'_>) -> Option<(rustc_span::Symbol, Span)> {
            match &pat.kind {
                thir::PatKind::Binding {
                    subpattern: None, name, ..
                } => Some((*name, pat.span)),
                thir::PatKind::Variant { subpatterns, .. } | thir::PatKind::Leaf { subpatterns } => {
                    match &subpatterns[..] {
                        [field] => payload(&field.pattern),
                        _ => None,
                    }
                }
                thir::PatKind::Deref { subpattern, .. } => payload(subpattern),
                _ => None,
            }
        }
        let read_field = |scrutinee: thir::ExprId| {
            let mut e = through(scrutinee);
            loop {
                e = match &thir[e].kind {
                    thir::ExprKind::Borrow { arg, .. } | thir::ExprKind::Deref { arg } => through(*arg),
                    thir::ExprKind::Call { args, .. } if !args.is_empty() => through(args[0]),
                    _ => return field_named(Some(e)),
                };
            }
        };
        let mut own_field = Vec::new();
        for expr in thir.exprs.iter() {
            if let thir::ExprKind::Let { expr, pat } = &expr.kind
                && let Some((binding, span)) = payload(pat)
                && read_field(*expr) == Some(binding)
            {
                own_field.push(span);
            }
        }
        // A drop guard's `let`, of a type of the program's with a `Drop` of
        // its own, not std's `Vec` or `Rc`, and the end of the block that
        // drops it, where the `try` its drop ends starts (ADR 0865, amended).
        for block in thir.blocks.iter() {
            for &stmt in block.stmts.iter() {
                if let thir::StmtKind::Let { pattern, .. } = &thir[stmt].kind
                    && let thir::PatKind::Binding { subpattern: None, .. } = pattern.kind
                    && let Some(adt) = pattern.ty.ty_adt_def()
                    && adt.has_dtor(self.tcx)
                    && !crate::lower::recognition::is_std_crate(self.tcx, adt.did())
                {
                    guards.push((self.js_span(pattern.span).lo, self.js_span(block.span).hi));
                }
            }
        }
        let names = |spans: &[Span]| {
            (body.var_debug_info.iter())
                .filter_map(|info| match info.value {
                    mir::VarDebugInfoContents::Place(place)
                        if place.projection.is_empty() && spans.contains(&info.source_info.span) =>
                    {
                        Some(state.locals.names[place.local].clone())
                    }
                    _ => None,
                })
                .collect()
        };
        crate::lower::mir::tidy::LetNames {
            patterns: (self.let_pattern_bindings(body).into_iter())
                .map(|(l, _)| state.locals.names[l].clone())
                .collect(),
            named: names(&named),
            mutable: names(&mutable),
            ends_borrow: names(&ends_borrow),
            frozen: names(&frozen),
            own_field: names(&own_field),
            guards,
        }
    }
}
