//! How a value is written over by a clone of another, as std's
//! `clone_from` of its type writes it (#919, #922, #927, #928): std's own
//! in place, `clone_into`'s; a type's own `clone_from`; an `Option`'s and a
//! `Result`'s value's; a value only a caller knows, its `Clone`
//! dictionary's; and what generic code's dictionary of a std type gives as
//! its `clone_from`. The one composition the direct call
//! (`mir/std_calls/clone_from.rs`), a slice's `fill` and `clone_from_slice`
//! and generic code's dictionary ask.

use crate::lower::temporary_names::Role;
use rustc_middle::ty;
use rustc_span::Span;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::copies::InPlaceWriter;
use crate::lower::recognition::trait_method;
use crate::lower::{FnCx, R};

/// One side of a `Result`'s `clone_from`, two `Ok`s or two `Err`s: the
/// test of it, and its write (`result_sides`).
pub(in crate::lower) type Side = (Expr, Vec<Stmt>);

/// A value written over in place by a clone of another, as std's own
/// `clone_from` of its type does (`clone_into`): `helper(to, from,
/// ...args)`.
pub(in crate::lower) struct CloneInto {
    helper: &'static str,
    args: Vec<Expr>,
}

impl CloneInto {
    /// `to` written over by a clone of `from`.
    pub(in crate::lower) fn call(self, to: Expr, from: Expr) -> Expr {
        Expr::call_helper(self.helper, [to, from].into_iter().chain(self.args).collect())
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// How an `item` is written over by a clone of another, as std's
    /// `clone_from` does it: a clone, where a clone is more than the value,
    /// and `(items, i, from) => ...`, its type's own `clone_from`, or a type
    /// parameter's dictionary's where its impl writes one. The one answer
    /// `fill` and `clone_from_slice` ask (ADR 0507).
    pub(in crate::lower) fn clone_over(
        &mut self,
        item: ty::Ty<'tcx>,
        what: &str,
        span: Span,
    ) -> R<(Option<Expr>, Option<Expr>)> {
        let made_i = self.unspelled("i");
        let made_items = self.unspelled("items");
        let from = js::Name::declared("from".to_string());
        let written = |called: Expr| {
            Expr::arrow(
                vec![made_items.clone().into(), made_i.clone().into(), from.clone().into()],
                vec![StmtKind::Expr(called).at(js::Span::NONE)],
            )
        };
        if self.recognition().own_clone_from(item) {
            let place = Expr::index(Expr::var(&made_items), Expr::var(&made_i));
            let target = match self.handles_mut(item) {
                true => Expr::handle(place),
                false => place,
            };
            let clone_from = trait_method(self.tcx, self.clone_trait(), "clone_from");
            let args = self.args_of(self.clone_trait(), item);
            let called = self.impl_call(clone_from, args, vec![target, Expr::var(&from)], span)?;
            return Ok((None, Some(written(called))));
        }
        // A type parameter's, or an associated type's, `A::Item`.
        if self.is_unknown(item) {
            let clone = self.clone_arg(item, span)?;
            let clone_from = self.given_clone_from(item, span)?;
            let place = Expr::call(Expr::helper("$ptr"), vec![Expr::var(&made_items), Expr::var(&made_i)]);
            let called = Expr::call(clone_from.clone(), vec![place, Expr::var(&from)]);
            return Ok((Some(clone), Some(Expr::bin(Op::And, clone_from, written(called)))));
        }
        // A `Vec`'s, a `VecDeque`'s, a map's, std's own, written in place,
        // where its order of clones and drops is seen: `clone_into`'s
        // (#919, #922).
        let slot = || Expr::index(Expr::var(&made_items), Expr::var(&made_i));
        if let Some(into) = self.clone_into(item, span)? {
            return Ok((None, Some(written(into.call(slot(), Expr::var(&from))))));
        }
        // An `Option`'s: its value's, where both are `Some`, and otherwise
        // a clone written over it, what it held dropped (#922).
        let pointer = Expr::call(Expr::helper("$ptr"), vec![Expr::var(&made_items), Expr::var(&made_i)]);
        if let Some(over) = self.option_clone_over(item, (slot(), pointer), Expr::var(&from), span)? {
            return Ok((
                None,
                Some(Expr::arrow(
                    vec![made_items.clone().into(), made_i.clone().into(), from.clone().into()],
                    over,
                )),
            ));
        }
        if self.recognition().reaches_clone_from(item) {
            return Err(self.unsupported(
                span,
                &format!("`{what}` of items whose `clone_from` may be the crate's, inside them or generic"),
            ));
        }
        Ok((Some(self.clone_arg(item, span)?), None))
    }

    /// `to.clone_from(from)` of an `Option` whose value is written in place,
    /// `written_over`'s, as std's is: of two `Some`s, its value's; otherwise
    /// `*to = from.clone()`, the clone written over it, then what it held
    /// dropped (#922). `to` is a place, read more than once, as `from` is,
    /// and the box generic code has of it, which is its value's too, as a
    /// `Some` of it is the value. `None` where its value isn't written in
    /// place.
    pub(in crate::lower) fn option_clone_over(
        &mut self,
        ty: ty::Ty<'tcx>,
        (to, pointer): (Expr, Expr),
        from: Expr,
        span: Span,
    ) -> R<Option<Vec<Stmt>>> {
        let Some(inner) = self.option_of(ty) else {
            return Ok(None);
        };
        // A `Some` of a value that may look like `None` is boxed, `$some(x)`
        // (ADR 0051): its value's place is read and written through the
        // box, `{ get value() { return $someValue(to); }, set value(v) { to
        // = $some(v); } }`, quickcheck's `Option<A>` (#927).
        let boxed = self.boxed_payload(inner);
        let (payload, payload_pointer, from_payload, made) = match boxed {
            true => {
                let name = self.fresh(Role::Payload);
                let getter = Expr::arrow(
                    Vec::new(),
                    vec![StmtKind::Return(Some(Expr::call_helper("$someValue", vec![to.clone()]))).at(js::Span::NONE)],
                );
                let v = self.fresh(Role::V);
                let setter = Expr::arrow(
                    vec![v.clone().into()],
                    vec![
                        StmtKind::Assign(to.clone(), Expr::call_helper("$some", vec![Expr::var(&v)]))
                            .at(js::Span::NONE),
                    ],
                );
                let made = Expr::object(vec![
                    js::Prop::Getter(js::HELD.into(), getter),
                    js::Prop::Setter(js::HELD.into(), setter),
                ]);
                let pointer = Expr::var(&name);
                let from_payload = Expr::call_helper("$someValue", vec![from.clone()]);
                (
                    Expr::member(pointer.clone(), js::HELD),
                    pointer,
                    from_payload,
                    Some(StmtKind::Const(name, made).at(js::Span::NONE)),
                )
            }
            false => (to.clone(), pointer, from.clone(), None),
        };
        // Its value written over in place, or itself an `Option` or a
        // `Result` whose value is, `Option<Option<T>>`'s (#927).
        let (asked, written) = match self.written_over(
            inner,
            (payload.clone(), payload_pointer.clone()),
            from_payload.clone(),
            span,
        )? {
            Some((asked, written)) => (asked, vec![StmtKind::Expr(written).at(js::Span::NONE)]),
            None => {
                match self.option_clone_over(inner, (payload.clone(), payload_pointer), from_payload.clone(), span)? {
                    Some(over) => (None, over),
                    None => match self.result_clone_over(inner, payload, from_payload, span)? {
                        Some(over) => (None, over),
                        None => return Ok(None),
                    },
                }
            }
        };
        let some = |e: &Expr| Expr::bin(Op::LooseNe, e.clone(), Expr::null());
        let mut both = Expr::bin(Op::And, some(&to), some(&from));
        if let Some(asked) = asked {
            both = Expr::bin(Op::And, both, asked);
        }
        let mut replaced = Vec::new();
        let clone = self.clone_value(from.clone(), ty, span, &mut replaced)?;
        let old = self.fresh(Role::Old);
        replaced.push(StmtKind::Const(old.clone(), to.clone()).at(js::Span::NONE));
        replaced.push(StmtKind::Assign(to.clone(), clone).at(js::Span::NONE));
        self.drop_value(Expr::var(&old), ty, span, &mut replaced)?;
        let over = made.into_iter().chain(written).collect();
        Ok(Some(vec![StmtKind::If(both, over, Some(replaced)).at(js::Span::NONE)]))
    }

    /// `to.clone_from(from)` of a `Result`, as std's is: of two `Ok`s its
    /// value's, of two `Err`s its error's, where that's written in place
    /// (`result_sides`); otherwise `*to = from.clone()`, the clone written
    /// over it, then what it held dropped, quickcheck's `Result<A, B>`
    /// (#927). `to` a place this writes, a box's `value`, an `Option`'s
    /// value's place or a local; `None` of another type, or where neither
    /// side is written in place.
    pub(in crate::lower) fn result_clone_over(
        &mut self,
        ty: ty::Ty<'tcx>,
        to: Expr,
        from: Expr,
        span: Span,
    ) -> R<Option<Vec<Stmt>>> {
        let Some(sides) = self.result_sides(ty, to.clone(), from.clone(), span)? else {
            return Ok(None);
        };
        let mut replaced = Vec::new();
        let clone = self.clone_value(from.clone(), ty, span, &mut replaced)?;
        let old = self.fresh(Role::Old);
        let held = self.moved_out(to.clone(), ty);
        replaced.push(StmtKind::Const(old.clone(), held).at(js::Span::NONE));
        replaced.push(StmtKind::Assign(to.clone(), clone).at(js::Span::NONE));
        self.drop_value(Expr::var(&old), ty, span, &mut replaced)?;
        Ok(Some(Self::sides_or(sides, replaced)))
    }

    /// Each side of `to.clone_from(from)` of a `Result` std writes as its
    /// value's, two `Ok`s or two `Err`s, where that's written in place
    /// (`written_over`): the test of it, and the write. Each side's place is
    /// `to`'s `_0`, and where the write takes a box of it, `{ get value() {
    /// return to._0; }, set value(v) { $assign(to, { TAG: "Ok", _0: v });
    /// } }`, the `Result` written in place, as a `&mut` to one is the object
    /// itself, asking the copy policy as each write in place does
    /// (`in_place`, #2028). `None` of another type, or where neither side is
    /// written in place.
    pub(in crate::lower) fn result_sides(
        &mut self,
        ty: ty::Ty<'tcx>,
        to: Expr,
        from: Expr,
        span: Span,
    ) -> R<Option<Vec<Side>>> {
        if !self.is_std_type(ty, crate::lower::recognition::StdItem::Result) {
            return Ok(None);
        }
        let ty::Adt(_, args) = ty.kind() else {
            return Ok(None);
        };
        let mut sides = Vec::new();
        for (ok, held) in [(true, args.type_at(0)), (false, args.type_at(1))] {
            let name = self.fresh(Role::Payload);
            let getter = Expr::arrow(
                Vec::new(),
                vec![StmtKind::Return(Some(js::result::held(to.clone()))).at(js::Span::NONE)],
            );
            let pointer = Expr::var(&name);
            let Some((asked, written)) = self.written_over(
                held,
                (js::result::held(to.clone()), pointer),
                js::result::held(from.clone()),
                span,
            )?
            else {
                continue;
            };
            let mut test = Expr::bin(
                Op::And,
                js::result::is(to.clone(), ok),
                js::result::is(from.clone(), ok),
            );
            if let Some(asked) = asked {
                test = Expr::bin(Op::And, test, asked);
            }
            let mut body = vec![StmtKind::Expr(written).at(js::Span::NONE)];
            // The box, where what's written takes one, a dictionary's
            // `cloneFrom`'s or a value's whose `&mut` is a box: the write in
            // place it holds asked of the copy policy only there.
            if js::mentions_of(&body, &name) > 0 {
                let v = self.fresh(Role::V);
                let replaced = js::result::made(ok, Expr::var(&v));
                let assigned = self.in_place(InPlaceWriter::Assign, ty, vec![to.clone(), replaced], span)?;
                let setter = Expr::arrow(vec![v.into()], vec![StmtKind::Expr(assigned).at(js::Span::NONE)]);
                let made = Expr::object(vec![
                    js::Prop::Getter(js::HELD.into(), getter),
                    js::Prop::Setter(js::HELD.into(), setter),
                ]);
                body.insert(0, StmtKind::Const(name, made).at(js::Span::NONE));
            }
            sides.push((test, body));
        }
        Ok((!sides.is_empty()).then_some(sides))
    }

    /// `sides`' tests, each its write, the last's `else` `otherwise`.
    pub(in crate::lower) fn sides_or(sides: Vec<Side>, otherwise: Vec<Stmt>) -> Vec<Stmt> {
        let mut otherwise = otherwise;
        for (test, body) in sides.into_iter().rev() {
            otherwise = vec![StmtKind::If(test, body, Some(otherwise)).at(js::Span::NONE)];
        }
        otherwise
    }

    /// `held` at `place`, `pointer` the box generic code has of it, written
    /// over in place by `from` as std's `clone_from` of it writes it, where
    /// that isn't `*place = from.clone()`: `clone_into`'s, a `Box`'s its
    /// value's; or of a type only a caller knows, its `Clone` dictionary's
    /// `clone_from`, which the caller's type has where std's or its impl's
    /// is its own (`dictionary_clone_from`), so the JS asks it first (#928):
    /// what's asked, and the call. `None` where it's `*place =
    /// from.clone()` here.
    pub(in crate::lower) fn written_over(
        &mut self,
        held: ty::Ty<'tcx>,
        (place, pointer): (Expr, Expr),
        from: Expr,
        span: Span,
    ) -> R<Option<(Option<Expr>, Expr)>> {
        if let Some(into) = self.clone_into(held, span)? {
            return Ok(Some((None, into.call(place, from))));
        }
        // A `Box`'s, its value's, as std's `Box::clone_from` writes
        // `(**self).clone_from(&**source)`, the box its value in JS (#927).
        let mut held = held;
        while held.is_box() {
            held = held.boxed_ty().expect("a `Box`");
        }
        // A `Reverse`'s, its value's, `self.0.clone_from(&source.0)`: of its
        // place `[x]`'s `x`, given a box of it as a `Result`'s `_0` is
        // (`result_clone_over`, #927).
        if self.recognition().is_reverse(held)
            && let ty::Adt(_, args) = held.kind()
        {
            let value = Expr::index(place, Expr::int(0));
            let v = self.fresh(Role::V);
            let getter = Expr::arrow(
                Vec::new(),
                vec![StmtKind::Return(Some(value.clone())).at(js::Span::NONE)],
            );
            let setter = Expr::arrow(
                vec![v.clone().into()],
                vec![StmtKind::Assign(value.clone(), Expr::var(&v)).at(js::Span::NONE)],
            );
            let pointer = Expr::object(vec![
                js::Prop::Getter(js::HELD.into(), getter),
                js::Prop::Setter(js::HELD.into(), setter),
            ]);
            return self.written_over(args.type_at(0), (value, pointer), Expr::index(from, Expr::int(0)), span);
        }
        // A type's own `clone_from`, its impl's, given the value, or the box
        // of a value JS can't change in place (`clone_over`'s, #927).
        if self.recognition().own_clone_from(held) {
            let target = match self.handles_mut(held) {
                true => pointer,
                false => place,
            };
            let clone_from = trait_method(self.tcx, self.clone_trait(), "clone_from");
            let args = self.args_of(self.clone_trait(), held);
            return Ok(Some((
                None,
                self.impl_call(clone_from, args, vec![target, from], span)?,
            )));
        }
        if !self.clone_given(held) {
            return Ok(None);
        }
        let clone_from = self.given_clone_from(held, span)?;
        Ok(Some((
            Some(clone_from.clone()),
            Expr::call(clone_from, vec![pointer, from]),
        )))
    }

    /// Whether `ty` is a type only a caller knows whose `Clone` dictionary
    /// a caller gives. Where none does, its dictionary is made here, of a
    /// value JS gives, and its `clone_from` is std's provided.
    pub(in crate::lower) fn clone_given(&self, ty: ty::Ty<'tcx>) -> bool {
        self.is_unknown(ty) && (self.evidence_for(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]))).is_some()
    }

    /// The `clone_from` of a type only a caller knows, its `Clone`
    /// dictionary's, `TClone.cloneFrom`: `undefined` where it's std's
    /// provided `*dest = source.clone()`.
    fn given_clone_from(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let dictionary = self.dictionary(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]), span)?;
        Ok(Expr::member(
            dictionary,
            crate::lower::bindings::trait_method_name(self.tcx, self.clone_trait(), "clone_from"),
        ))
    }

    /// The `clone_from` of a std type's `Clone` dictionary, where std's
    /// isn't `*dest = source.clone()`, `(to, from) => ..`, `to` the box
    /// generic code has of its `&mut`: `clone_into`'s, an `Option`'s of its
    /// value's (`written_over`), so a generic `clone_from` of it is std's
    /// (#928), as an impl's own `clone_from` is its dictionary's. `None`
    /// where std's is the provided one. Refused where std's forwards to a
    /// value's that rust-js doesn't write (#927), which a generic caller's
    /// `clone_from` would otherwise skip.
    pub(in crate::lower) fn dictionary_clone_from(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        // A type only a caller knows, whose dictionary is made here, of no
        // caller's: its `clone_from` is no caller's either, std's provided.
        if self.is_unknown(ty) {
            return Ok(None);
        }
        let (to_name, from_name) = (
            js::Name::declared("to".to_string()),
            js::Name::declared("from".to_string()),
        );
        let (to, from) = (Expr::var(&to_name), Expr::var(&from_name));
        let place = Expr::member(to.clone(), js::HELD);
        let body = match self.written_over(ty, (place.clone(), to.clone()), from.clone(), span)? {
            Some((None, written)) => vec![StmtKind::Expr(written).at(js::Span::NONE)],
            // A `Box` of a type parameter's, which is its value: the
            // value's dictionary's, given, where it has one.
            Some((Some(given), _)) => return Ok(Some(given)),
            None => match self.option_clone_over(ty, (place.clone(), to), from.clone(), span)? {
                Some(over) => over,
                None if let Some(over) = self.result_clone_over(ty, place, from, span)? => over,
                None if self.forwards_clone_from(ty, span)? => {
                    return Err(self.unsupported(
                        span,
                        &format!("a `Clone` dictionary of `{ty}` given to generic code, whose std `clone_from` is its value's, which the crate's or std's own writes in place (#927)"),
                    ));
                }
                None => return Ok(None),
            },
        };
        Ok(Some(Expr::arrow(vec![to_name.into(), from_name.into()], body)))
    }

    /// How a `ty`, a JS object, is written over in place by a clone of
    /// another, as std's own `clone_from` of its type does, where its order
    /// of clones and drops is seen (#919, #922): a `Vec`'s and a
    /// `BinaryHeap`'s, `$cloneIntoVec`; a `VecDeque`'s, which clears, then
    /// clones each, `$cloneIntoDeque`; a `LinkedList`'s, which writes each it
    /// keeps over, pops the rest from its back, then clones,
    /// `$cloneIntoList`; an array's, each item written over,
    /// `$cloneFromSlice`; a hashed map's or set's, which drops its entries,
    /// stopping at one that panics, as hashbrown's does, then clones each,
    /// `$cloneIntoMap`; a `Box`'s, its value's. `None` where std's is
    /// `*dest = source.clone()`, a B-tree's, or where no order is seen: of
    /// items with no destructor, no `clone_from` of their own and a clone
    /// that can't panic.
    pub(in crate::lower) fn clone_into(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<CloneInto>> {
        use crate::lower::recognition::StdItem;
        let seen = |cx: &Self, t| cx.drops(t) != crate::lower::drops::Drops::Nothing || cx.clone_may_panic(t);
        let ty::Adt(_, args) = ty.kind() else {
            // An array's `clone_from`, its slice's `clone_from_slice`.
            let ty::Array(item, _) = *ty.kind() else {
                return Ok(None);
            };
            let (clone, clone_from) = self.clone_over(item, "clone_from", span)?;
            if clone_from.is_none() && !seen(self, item) {
                return Ok(None);
            }
            let drop = self.std_drop_function(item, span)?;
            let args = Expr::optional_args(Vec::new(), vec![clone, clone_from, drop]);
            return Ok(Some(CloneInto {
                helper: "$cloneFromSlice",
                args,
            }));
        };
        if ty.is_box() {
            return self.clone_into(args.type_at(0), span);
        }
        if self.is_std_type(ty, StdItem::Vec) || self.is_std_type(ty, StdItem::BinaryHeap) {
            let args = self.vec_clone_into(args.type_at(0), span)?;
            return Ok(args.map(|args| CloneInto {
                helper: "$cloneIntoVec",
                args,
            }));
        }
        let deque = self.is_std_type(ty, StdItem::VecDeque);
        if deque || self.is_std_type(ty, StdItem::LinkedList) {
            let item = args.type_at(0);
            // A `VecDeque`'s asks no item's `clone_from`.
            let clone_from = match deque {
                true => None,
                false => self.clone_over(item, "clone_from", span)?.1,
            };
            if clone_from.is_none() && !seen(self, item) {
                return Ok(None);
            }
            let clone = match self.needs_clone(item) {
                true => Some(self.clone_fn("item", item, span)?),
                false => None,
            };
            let drop = self.std_drop_function(item, span)?;
            let (helper, list) = match deque {
                true => ("$cloneIntoDeque", vec![clone, drop]),
                false => ("$cloneIntoList", vec![clone, clone_from, drop]),
            };
            let args = Expr::optional_args(Vec::new(), list);
            return Ok(Some(CloneInto { helper, args }));
        }
        if self.is_map(ty) && !self.is_sorted(ty) {
            let mut out = Vec::new();
            let make = self.entry_clone(ty, span, &mut out)?;
            assert!(out.is_empty(), "an entry's clone is an arrow");
            let (make, entry, may_panic) = match make {
                Some((f, entry, may_panic)) => (Some(f), entry, may_panic),
                None if self.is_set(ty) => (None, args.type_at(0), false),
                None => (
                    None,
                    ty::Ty::new_tup(self.tcx, &[args.type_at(0), args.type_at(1)]),
                    false,
                ),
            };
            if self.drops(entry) == crate::lower::drops::Drops::Nothing && !may_panic {
                return Ok(None);
            }
            let drop = self.std_drop_function(entry, span)?;
            let args = Expr::optional_args(Vec::new(), vec![make, drop]);
            return Ok(Some(CloneInto {
                helper: "$cloneIntoMap",
                args,
            }));
        }
        Ok(None)
    }

    /// A `Vec`'s `clone_from` of `item`s, std's `SpecCloneIntoVec`: what it
    /// has past the source's length dropped, each it keeps written over by
    /// `clone_over`'s, then a clone of each of the rest pushed. The
    /// arguments `$cloneIntoVec(w, v, ..)` takes after the two, its items'
    /// clone, `clone_from` and drop, where that order is seen: of items with
    /// a destructor, or a `clone_from` of their own or a `Vec`'s, or a clone
    /// that may panic, which leaves `w` half written (#922); of none,
    /// `None`, as `*w = v.clone()` does alike, every clone then no drop
    /// that can be seen (#919).
    pub(in crate::lower) fn vec_clone_into(&mut self, item: ty::Ty<'tcx>, span: Span) -> R<Option<Vec<Expr>>> {
        let (_, clone_from) = self.clone_over(item, "clone_from", span)?;
        if clone_from.is_none()
            && self.drops(item) == crate::lower::drops::Drops::Nothing
            && !self.clone_may_panic(item)
        {
            return Ok(None);
        }
        // What the rest are cloned by, though those kept are by `clone_from`.
        let clone = match self.needs_clone(item) {
            true => Some(self.clone_fn("item", item, span)?),
            false => None,
        };
        let drop = self.std_drop_function(item, span)?;
        Ok(Some(Expr::optional_args(Vec::new(), vec![clone, clone_from, drop])))
    }
}
