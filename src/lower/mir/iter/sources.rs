//! What a chain steps: an array, a range, a std iterator, as a JS
//! iterator; and the callbacks it's given.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, an iterator of type `ty`, as a JS iterator: a range stepped
    /// by its `next`, the crate's by its impl's, a JS one as it is.
    pub(in crate::lower) fn as_js_iterator(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // The crate's own behind an `impl Iterator` too.
        let ty = self.reveal(ty);
        // A `&mut` to a range, `take2(&mut it)`'s: the range, stepped where
        // it is, as its lender goes on from where it's left.
        let ranged = match ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) if self.range_kind(*inner).is_some() => *inner,
            _ => ty,
        };
        match self.range_kind(ranged) {
            Some(kind) => match kind.step(false) {
                Some(step) => Ok(self.stepped_by(value, Expr::helper(step))),
                None => Err(self.unsupported(span, "iterating this range, from its MIR")),
            },
            // A `char`'s or a `str`'s escape, its text, of its `char`s, which
            // JS's string steps by code point as Rust's are.
            None if crate::lower::recognition::is_text_escape(self.tcx, ty) => Ok(self.js_iterator(value)),
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_iterator(value, ty, trait_id, "next", span)
            }
            // One given its dictionary, stepped by it, its `Some` unboxed
            // (ADR 0411).
            None if let Some(it) = self.given_js_iterator(value.clone(), ty, false, false) => Ok(it),
            // A type parameter's, which may be given an array (ADR 0061): JS's
            // iterator of it. An associated type's is what a dictionary
            // gives, a JS iterator already.
            None if matches!(ty.kind(), ty::Param(_)) && !is_js_iterator(&value) => Ok(self.js_iterator(value)),
            None => Ok(value),
        }
    }

    /// How many items a `$iter` of an array has left, its `len()`: of one
    /// made here, `$iter(v)`, which nothing has stepped, its array's length,
    /// `v.length`; else `it.end - it.at`, `it` held if it's made here.
    pub(super) fn items_left(&mut self, it: Expr, made: &mut Vec<Stmt>) -> Expr {
        if crate::lower::iterators::is_stepped_items(&it)
            && let js::ExprKind::Call(_, items) = &it.kind
        {
            return Expr::member(items[0].clone(), "length");
        }
        let it = match it.reads_same() {
            true => it,
            false => self.spill("it", it, made),
        };
        Expr::bin(Op::Sub, Expr::member(it.clone(), "end"), Expr::member(it, "at"))
    }

    /// Whether std's iterator `ty` is a `$iter` of an array, as
    /// `std_iterator` makes it: an array's own, a string's `chars()`,
    /// `bytes()` or `split`, a `VecDeque`'s, a slice's `iter_mut()`, a
    /// `BTreeMap`'s, its `range()`'s too: one that steps from its back and
    /// knows its length. The one list.
    pub(in crate::lower) fn steps_an_array(&self, ty: Ty<'tcx>) -> bool {
        self.array_source(ty).is_some()
            || self.recognition().iterates_items(ty)
            // A `rev()` of one, its array reversed (ADR 0530).
            || crate::lower::recognition::reversed_source(self.tcx, ty).is_some_and(|source| self.steps_an_array(source))
            // A `copied()` of one, which is it in JS (ADR 0547).
            || (self.is_its_source(ty)
                && (crate::lower::recognition::copied_source(self.tcx, ty))
                    .or_else(|| crate::lower::recognition::cloned_source(self.tcx, ty))
                    .is_some_and(|source| self.steps_an_array(source)))
            || self.recognition().is_str_chars(ty)
            || self.recognition().is_string_drain(ty)
            || self.is_str_split(ty)
            || self.recognition().is_deque_iter(ty)
            || self.recognition().is_slice_iter_mut(ty)
            || self.recognition().is_tree_iter(ty)
            || self.recognition().is_process_items(ty)
            || self.recognition().is_path_items(ty)
    }

    /// A std iterator of type `ty` made of `value`, an array or a text, as
    /// MIR steps it: over an array, a `$iter`, which knows where it is, so a
    /// clone of it does (ADR 0181); another, the JS iterator of `value`.
    pub(in crate::lower) fn std_iterator(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
        if is_js_iterator(&value) {
            value
        } else if made_array(&value) || self.steps_an_array(ty) {
            // An array made of it, `s.split(",")`'s, or one `steps_an_array`
            // lists, a `VecDeque`'s say: a `$iter` of it, as of an array's.
            self.stepped_items(value)
        } else {
            self.js_iterator(value)
        }
    }

    /// `$iterator({ start: a, end: b }, $rangeNext).map(f)`, a range of
    /// numbers `iter_ty` steps, each item mapped, all collected: its array's,
    /// `$range(a, b).map(f)`, made as big as what's collected
    /// anyway; what more than one map does in Rust's order, one item through
    /// them all at a time, only lazily. Not filtered, which may keep few of
    /// many, `$lazyRange(a, b).filter(f)` (`range_items`). Of `a..=b`,
    /// `$range(a, b + 1)`; stepped from its end, `.rev()`, `.toReversed()`
    /// first.
    pub(super) fn mapped_range(&mut self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        let mut maps = Vec::new();
        let mut at = it;
        while let js::ExprKind::Call(callee, args) = &at.kind
            && let js::ExprKind::Member(source, name) = &callee.kind
            && name == "map"
            && let [f] = &args[..]
        {
            maps.push((name.clone(), f.clone()));
            at = source;
        }
        let range = stepped_range(at)?;
        let numbers = (iter_ty.walk())
            .filter_map(|arg| arg.as_type())
            .find(|&t| matches!(self.range_kind(t), Some(RangeKind::Exclusive | RangeKind::Inclusive)))
            .and_then(|t| self.range_index(t))
            .and_then(Num::of)
            .is_some_and(|num| !num.big());
        if !numbers || maps.iter().filter(|(_, f)| crate::lower::iterators::does(f)).count() > 1 && maps.len() > 1 {
            return None;
        }
        let end = match range.inclusive {
            true => match range.end.as_int() {
                Some(n) => Expr::int(n + 1),
                None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),
            },
            false => range.end.clone(),
        };
        let mut items = Expr::call_helper("$range", vec![range.start.clone(), end]);
        if range.back {
            items = Expr::call(Expr::member(items, "toReversed"), Vec::new());
        }
        Some(maps.into_iter().rev().fold(items, |items, (name, f)| {
            Expr::call(Expr::member(items, &name), vec![f])
        }))
    }

    /// Of `Iterator.from(list)` collected, `it`, each item mapped or
    /// filtered on the way: the array it makes, `Array.from(nodes).map(f)`
    /// of a binding's list. Not of a generic
    /// value's, which may be an iterator with only a `next`, of which
    /// `Array.from` makes an empty array (ADR 0140).
    pub(super) fn collected_items(&self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        let mut source = self.reveal(iter_ty);
        while let ty::Adt(_, args) = source.kind()
            && let Some(inner) = args.types().next()
            && self.implements_iterator(inner)
        {
            source = inner;
        }
        if crate::lower::recognition::known_by_bounds(source) {
            return None;
        }
        collected(it)
    }

    /// Whether `ty` is an iterator: one whose `Item` rustc says, a type or
    /// another trait's associated type, smallvec's `A::Item`, not one of
    /// std's iterator traits' still.
    pub(in crate::lower) fn implements_iterator(&self, ty: Ty<'tcx>) -> bool {
        let iterator_traits = [StdItem::Iterator, StdItem::IntoIterator].map(|item| std_item(self.tcx, item));
        let said = |item: Ty<'tcx>| match item.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) => self
                .tcx
                .trait_of_assoc(*def_id)
                .is_some_and(|tr| !iterator_traits.contains(&tr)),
            ty::Alias(..) => false,
            _ => true,
        };
        self.iterator_item(ty).is_some_and(said) || matches!(ty.kind(), ty::Param(_))
    }

    /// A function JS's helpers give an item, and its index too, as one
    /// that takes the item alone: an arrow of one parameter as it is,
    /// anything else called with the item alone.
    pub(super) fn unary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,
            js::ExprKind::Var(_) if own => f,
            _ => {
                let x = self.fresh("item");
                let call = Expr::call(f, vec![Expr::var(&x)]);
                Expr::arrow(vec![x.into()], vec![StmtKind::Return(Some(call)).at(js::Span::NONE)])
            }
        }
    }

    /// Is `ty` a function item of the crate's own, not a JS binding's or a
    /// trait's method, whose JS is called with what Rust gives it alone?
    pub(super) fn is_crate_fn_item(&self, ty: Ty<'tcx>) -> bool {
        match *ty.kind() {
            ty::FnDef(def_id, _) => {
                def_id.is_local()
                    && self.tcx.trait_of_assoc(def_id).is_none()
                    && !bindings::is_binding(self.tcx, def_id)
            }
            _ => false,
        }
    }

    /// As `unary_callback`, of a function of two: `reduce`'s, given an index
    /// too. `f64::max`'s and `f64::min`'s helpers take two alone.
    /// `f`, of an item, as a fold's callback, of what it's made so far and
    /// the item: `(_, x) => ..` of `(x) => ..`.
    pub(super) fn unary_folded(&mut self, f: Expr) -> Expr {
        let made = self.fresh("_");
        match f.kind {
            js::ExprKind::Arrow(mut params, body) if params.len() == 1 => {
                params.insert(0, made.into());
                Expr::arrow(params, body)
            }
            _ => {
                let x = self.fresh("x");
                let call = Expr::call(f, vec![Expr::var(&x)]);
                Expr::arrow(
                    vec![made.into(), x.into()],
                    vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
        }
    }

    pub(super) fn binary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,
            js::ExprKind::Var(_) if own => f,
            _ if matches!(f.helper_name(), Some("$f64Max" | "$f64Min")) => f,
            _ => {
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let call = Expr::call(f, vec![Expr::var(&a), Expr::var(&b)]);
                Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
        }
    }
}

/// `counted_chain`'s `$iter(xs).drop(1).take(2)`: the array it steps,
/// `xs.slice(1).slice(0, 2)`.
pub(super) fn sliced(e: &Expr) -> Expr {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(source, name) = &callee.kind
                && let [n] = &args[..] =>
        {
            let items = sliced(source);
            match name.as_str() {
                "drop" => Expr::call(Expr::member(items, "slice"), vec![n.clone()]),
                _ => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), n.clone()]),
            }
        }
        _ => crate::lower::iterators::iterated(e.clone()),
    }
}

/// Whether `e` is an array's items made there, `$iter(xs)`, skipped or taken
/// from, `.drop(1)`, by a count that does nothing (`len()`'s).
pub(super) fn counted_chain(e: &Expr) -> bool {
    let mut at = e;
    let mut stages = 0;
    while let js::ExprKind::Call(callee, args) = &at.kind
        && let js::ExprKind::Member(source, name) = &callee.kind
        && matches!(name.as_str(), "drop" | "take")
        && let [n] = &args[..]
        && !n.has_effects()
    {
        stages += 1;
        at = source;
    }
    stages > 0 && crate::lower::iterators::is_stepped_items(at)
}
