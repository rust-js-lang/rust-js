//! `Clone`, `Default` and `PartialEq` where rust-js writes the
//! implementation: derived ones and std types' (ADRs 0052, 0053). A
//! hand-written one is called instead.

use super::recognition::{StdItem, std_item};
use super::representation::Num;
use super::{FnCx, R, Shape, lower_first};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind, UnaryOp};
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};

/// How JS compares a value as Rust's `==` does (`FnCx::js_eq`): by `===`
/// only, a float's; or by SameValueZero too, `includes`' and a `Map`'s.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum JsEq {
    Strict,
    SameValue,
}
impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn clone_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP)
    }

    /// Can a clone of `ty` be told apart from the value itself? Only if one
    /// of them can change in place, like a `Vec`, a cell, or a type in
    /// `mutated` (ADR 0020), or if cloning runs code: a hand-written `clone`,
    /// or a `T`'s, which might be one.
    pub(super) fn needs_clone(&self, ty: Ty<'tcx>) -> bool {
        self.needs_clone_in(ty, &mut Vec::new())
    }

    fn needs_clone_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if self.contains_mutated(ty) {
            return true;
        }
        if let Some(&needs) = self.walks().clones.borrow().get(&ty) {
            return needs;
        }
        // A recursive type: its other fields decide, so what's found under
        // it is only as sure as the walk further out, as in `unsupported_in`.
        if let Some(at) = seen.iter().position(|&t| t == ty) {
            self.walks().clone_assumed.set(self.walks().clone_assumed.get().min(at));
            return false;
        }
        let depth = seen.len();
        let outer = self.walks().clone_assumed.replace(usize::MAX);
        seen.push(ty);
        let std = |item: StdItem| self.is_std_type(ty, item);
        let needs = match ty.kind() {
            // A clone of a `&T` is the same reference.
            ty::Ref(..) => false,
            ty::Tuple(tys) => tys.iter().any(|t| self.needs_clone_in(t, seen)),
            // Arrays and cells are JS objects that change in place, and so
            // is a `Vec` something takes `&mut` of.
            ty::Array(..) => true,
            // A slice a `Box<[T]>` holds, as a `Vec`'s items: where something
            // changes one in place, `contains_mutated`'s, or its items need a
            // clone (ADR 0530).
            ty::Slice(item) => self.needs_clone_in(*item, seen),
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.vec_changed(ty) || self.needs_clone_in(args.type_at(0), seen)
            }
            ty::Adt(..)
                if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) || std(StdItem::OnceCell) =>
            {
                true
            }
            // A map or a set changes in place (ADR 0059).
            ty::Adt(..) if self.is_map(ty) => true,
            // A counted `Rc`'s clone counts one more, and a `Weak`'s (ADR 0320).
            ty::Adt(..) if self.counted_rc(ty).is_some() || self.weak_of(ty).is_some() => true,
            // A path is its text, which JS never changes in place (ADR 0173).
            ty::Adt(..)
                if self.is_rc(ty)
                    || self.is_lang_adt(ty, LangItem::String)
                    || self.recognition().is_path_like(ty)
                    || self.is_js_object(ty) =>
            {
                false
            }
            ty::Adt(..) if self.has_user_impl(self.clone_trait(), ty) => true,
            ty::Adt(adt, args)
                if ty.is_box()
                    || self.recognition().pinned(ty).is_some()
                    || !self.is_std(adt.did())
                    || self.is_known_std(ty) =>
            {
                adt.all_fields()
                    .any(|f| self.needs_clone_in(self.field_ty(f, args), seen))
            }
            // Another std type: `clone_value` says it can't.
            ty::Adt(..) => true,
            // A closure cloned apart makes itself again (ADR 0522).
            ty::Closure(id, _) => self.krate.closure_facts.clones_apart(*id),
            _ => false,
        };
        seen.pop();
        let assumed = self.walks().clone_assumed.get();
        self.walks().clone_assumed.set(outer.min(assumed));
        if needs || assumed >= depth {
            self.walks().clones.borrow_mut().insert(ty, needs);
        }
        needs
    }

    /// A type rust-js compiled: the crate's own, or a library's (ADR 0100).
    pub(super) fn is_rust_adt(&self, id: DefId) -> bool {
        id.is_local() || self.krate.foreign.in_library(id)
    }

    /// std enums whose fields are what JS has: `Option`, `Result`, `Ordering`.
    fn is_known_std(&self, ty: Ty<'tcx>) -> bool {
        self.is_lang_adt(ty, LangItem::Option)
            || self.is_std_type(ty, StdItem::Result)
            || self.is_lang_adt(ty, LangItem::OrderingEnum)
            // Its `PartialEq` and `Clone` are its field's; its order isn't
            // (`cmp_value`).
            || self.is_reverse(ty)
            // Its `PartialEq`, `Clone` and order are its number's (ADR 0175).
            || self.recognition().wrapping_of(ty).is_some()
    }

    /// `Clone::clone` of the `ty` at `place`: the place itself when nothing
    /// could tell a clone from it, and otherwise a copy of the parts that
    /// could, calling each hand-written `clone` on the way.
    pub(super) fn clone_value(&mut self, place: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        // A closure is its JS function, which shares what it captured: a
        // clone is the same function where nothing could tell (ADR 0522);
        // one cloned apart makes itself again, of a clone of what it took.
        if let ty::Closure(id, _) = *ty.kind() {
            if self.krate.closure_facts.clones_apart(id) {
                return Ok(Expr::call_helper("$clone", vec![place]));
            }
            let drops = |t| self.drops(t) != super::drops::Drops::Nothing;
            return match super::analysis::clone_shares(self.tcx, self.typing_env, ty, drops) {
                true => Ok(place),
                false => Err(self.unsupported(span, "cloning a closure where it isn't seen to be cloned")),
            };
        }
        // An iterator cloned apart makes itself again (ADR 0525); a copy of
        // one is the iterator it copies (ADR 0480), its clone that one's.
        // So does memchr's iterator, which carries its clone (ADR 0529).
        if self.iterator_cloned_apart(ty) || self.recognition().is_memchr_iter(ty) {
            return Ok(Expr::call_helper("$clone", vec![place]));
        }
        if let Some(source) = super::recognition::copied_source(self.tcx, ty) {
            return self.clone_value(place, source, span, out);
        }
        // A channel's sender: one more, of the same queue (ADR 0142).
        if self.recognition().channel_end(ty) == Some(super::recognition::ChannelEnd::Sender) {
            return Ok(Expr::call(Expr::helper("$cloneSender"), vec![place]));
        }
        // A counted `Rc`, or a `Weak`: one more, of the same value (ADR 0320).
        if self.counted_rc(ty).is_some() || self.weak_of(ty).is_some() {
            let clone = if self.weak_of(ty).is_some() {
                "$weakClone"
            } else {
                "$rcClone"
            };
            return Ok(Expr::call(Expr::var(clone), vec![place]));
        }
        // A path is its text, which nothing changes in place (ADR 0173).
        if !self.needs_clone(ty) || self.recognition().is_path_like(ty) {
            return Ok(place);
        }
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]);
            if let Some(dictionary) = self.evidence_for(tr) {
                return Ok(Expr::call(Expr::member(dictionary, "clone"), vec![place]));
            }
            // A `T: Copy`: its dictionary copies (ADR 0049).
            return Ok(self.copy(place, ty));
        }
        if self.has_user_impl(self.clone_trait(), ty) {
            let method = self.tcx.require_lang_item(LangItem::CloneFn, span);
            let args = self.args_of(self.clone_trait(), ty);
            return self.impl_call(method, args, vec![place], span);
        }
        // A bound on the type itself, `Peekable<I>: Clone`'s, whose dictionary
        // the caller gives (ADR 0049): its `clone`.
        if ty.has_param()
            && let Some(dictionary) = self.evidence_for(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]))
        {
            return Ok(Expr::call(Expr::member(dictionary, "clone"), vec![place]));
        }
        // A constant, like `"Dot"`, is a value no one else holds.
        if place.is_constant() {
            return Ok(place);
        }
        // serde_json's `Value` and `Map`, inside themselves (ADR 0083).
        if let Some(clone) = self.json_value_clone(place.clone(), ty) {
            return Ok(clone);
        }
        if self.is_copy(ty) {
            return Ok(self.copy(place, ty));
        }
        // A type inside itself, `Value` in `Obj(BTreeMap<String, Value>)`: its
        // clone is a function, which calls itself for the ones inside.
        if self.is_recursive(ty) {
            if let Some((_, name)) = self.cloning.iter().find(|(t, _)| *t == ty) {
                return Ok(Expr::call(Expr::var(name), vec![place]));
            }
            let type_name = match ty.kind() {
                ty::Adt(adt, _) => self.tcx.item_name(adt.did()).to_string(),
                _ => "Value".to_string(),
            };
            let name = self.fresh(&format!("clone{type_name}"));
            let param = self.fresh(&lower_first(&type_name));
            self.cloning.push((ty, name.clone()));
            let mut body = Vec::new();
            let value = self.clone_parts(Expr::var(&param), ty, span, &mut body);
            self.cloning.pop();
            body.push(StmtKind::Return(Some(value?)).at(js::Span::NONE));
            let f = Expr::arrow(vec![param.into()], body);
            out.push(StmtKind::Const(name.clone(), f).at(js::Span::NONE));
            return Ok(Expr::call(Expr::var(&name), vec![place]));
        }
        self.clone_parts(place, ty, span, out)
    }

    /// Is `ty`, one of the crate's own or a library's (ADR 0100), inside
    /// itself, through its fields or what a std type holds? A std type that
    /// holds one is cloned in place.
    fn is_recursive(&self, ty: Ty<'tcx>) -> bool {
        let mut seen = Vec::new();
        matches!(ty.kind(), ty::Adt(adt, _) if self.is_rust_adt(adt.did())) && self.holds(ty, ty, &mut seen)
    }

    /// Does `outer` hold `target` anywhere inside it?
    fn holds(&self, outer: Ty<'tcx>, target: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let parts: Vec<Ty<'tcx>> = match outer.kind() {
            ty::Adt(adt, args) if self.is_rust_adt(adt.did()) => {
                adt.all_fields().map(|f| self.field_ty(f, args)).collect()
            }
            ty::Adt(_, args) => args.types().collect(),
            ty::Tuple(tys) => tys.to_vec(),
            ty::Array(item, _) | ty::Slice(item) | ty::Ref(_, item, _) => vec![*item],
            _ => Vec::new(),
        };
        parts.into_iter().any(|part| {
            if part == target {
                return true;
            }
            if seen.contains(&part) {
                return false;
            }
            seen.push(part);
            self.holds(part, target, seen)
        })
    }

    /// `clone_value` of a `ty` that needs a copy, part by part.
    fn clone_parts(&mut self, place: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let std = |item: StdItem| self.is_std_type(ty, item);
        // Each read once.
        match ty.kind() {
            // An array, or a slice a `Box<[T]>` holds, a JS array (ADR 0530).
            ty::Array(item, _) | ty::Slice(item) => return self.clone_items(place, *item, span),
            ty::Adt(_, args) if self.is_vec_like(ty) => return self.clone_items(place, args.type_at(0), span),
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                return self.clone_value(place, args.type_at(0), span, out);
            }
            // A `RefCell`'s, while it's borrowed (ADR 0328): checked free to,
            // and held where its value's clone may ask.
            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {
                let mut body = Vec::new();
                let value = self.clone_value(Expr::var("value"), args.type_at(0), span, &mut body)?;
                body.push(StmtKind::Return(Some(value)).at(js::Span::NONE));
                let value = Expr::call(
                    Expr::helper("$withBorrow"),
                    vec![place, Expr::arrow(vec!["value".into()], body)],
                );
                return Ok(Expr::object(vec![Prop::Field(js::HELD.into(), value)]));
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let cell = match std(StdItem::RefCell) {
                    true => Expr::call(Expr::helper("$borrow"), vec![place]),
                    false => place,
                };
                let value = self.clone_value(Expr::held(cell), args.type_at(0), span, out)?;
                return Ok(Expr::object(vec![Prop::Field(js::HELD.into(), value)]));
            }
            // A `OnceCell` holds an `Option` (ADR 0317).
            ty::Adt(_, args) if std(StdItem::OnceCell) => {
                let option = Ty::new_option(self.tcx, args.type_at(0));
                let value = self.clone_value(Expr::held(place), option, span, out)?;
                return Ok(Expr::object(vec![Prop::Field(js::HELD.into(), value)]));
            }
            _ => {}
        }
        // Read more than once below.
        let place = if !place.reads_same() {
            self.spill("value", place, out)
        } else {
            place
        };
        match ty.kind() {
            // `new Map(m)`, cloning each value that needs it, and each key: a
            // primitive one never does, and one found by value may (ADR 0121).
            ty::Adt(_, args) if self.is_map(ty) => {
                let set = self.is_set(ty);
                let key = args.types().next();
                let class = self.map_class(set, key, span)?;
                let key = key.filter(|&k| self.is_value_key(k) && self.needs_clone(k));
                let value = args.types().nth(1).filter(|&v| !set && self.needs_clone(v));
                if key.is_none() && value.is_none() {
                    return Ok(Expr::new_(class, vec![place]));
                }
                let mut cloned = |this: &mut Self, name: &str, ty: Option<Ty<'tcx>>| match ty {
                    Some(ty) => this.clone_value(Expr::var(name), ty, span, out),
                    None => Ok(Expr::var(name)),
                };
                let (params, item) = if set {
                    (vec!["item".into()], cloned(self, "item", key)?)
                } else {
                    let pair = Expr::array(vec![cloned(self, "key", key)?, cloned(self, "value", value)?]);
                    (
                        vec![js::Pattern::Array(vec![Some("key".into()), Some("value".into())])],
                        pair,
                    )
                };
                let f = Expr::arrow(params, vec![StmtKind::Return(Some(item)).at(js::Span::NONE)]);
                let all = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![place]);
                Ok(Expr::new_(class, vec![Expr::call(Expr::member(all, "map"), vec![f])]))
            }
            // `{ start: r.start, end: r.end }` (ADR 0129).
            ty::Adt(_, args) if let Some(kind) = self.range_kind(ty) => {
                let props = kind
                    .bounds()
                    .iter()
                    .map(|&name| {
                        let bound = self.clone_value(Expr::member(place.clone(), name), args.type_at(0), span, out)?;
                        Ok(Prop::Field(name.into(), bound))
                    })
                    .collect::<R<_>>()?;
                Ok(Expr::object(props))
            }
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) => {
                let inner = args.type_at(0);
                let some = if self.boxed_payload(inner) {
                    let value = self.some_value(place.clone());
                    let clone = self.clone_value(value, inner, span, out)?;
                    self.some(clone)
                } else {
                    self.clone_value(place.clone(), inner, span, out)?
                };
                let none = Expr::bin(Op::LooseEq, place.clone(), Expr::null());
                Ok(Expr::cond(none, place, some))
            }
            // An untagged enum's variant is its payload (ADR 0214): one that
            // needs it gets the payload's clone, told by its kind: being changed
            // in place, as a tagged one's object is, is the payload's.
            ty::Adt(adt, args) if self.untagged(ty).is_some() => {
                let mut value = place.clone();
                for variant in adt.variants().iter().rev() {
                    let Some(field) = variant.fields.iter().next() else {
                        continue;
                    };
                    let payload = self.field_ty(field, args);
                    if !self.needs_clone(payload) {
                        continue;
                    }
                    let copy = self.clone_value(place.clone(), payload, span, out)?;
                    let test = self.untagged_variant_test(ty, variant, &place);
                    value = Expr::cond(test, copy, value);
                }
                Ok(value)
            }
            // A `Cow`'s is too: its `Owned`'s clone is the `to_owned()` of what it borrows.
            ty::Adt(adt, args)
                if adt.is_enum()
                    && (!self.is_std(adt.did())
                        || self.is_known_std(ty)
                        || self.recognition().cow_parts(ty).is_some()) =>
            {
                // `{ TAG: "Line", _0: .. }` (ADR 0033): a variant with fields
                // that need it gets a copy, and every other value is itself.
                let itself = self.mutated_itself(ty);
                // Each variant copied whole, with no field to clone: one `{ ...place }`.
                let fields: Vec<_> = adt.variants().iter().map(|v| self.variant_fields(v, args)).collect();
                if itself
                    && !fields.is_empty()
                    && fields
                        .iter()
                        .all(|f| !f.is_empty() && f.iter().all(|&(_, t)| !self.needs_clone(t)))
                {
                    return self.clone_fields(place, fields[0].clone(), span, out);
                }
                let mut value = place.clone();
                for variant in adt.variants().iter().rev() {
                    let fields = self.variant_fields(variant, args);
                    if fields.is_empty() || !(itself || fields.iter().any(|&(_, t)| self.needs_clone(t))) {
                        continue;
                    }
                    let copy = self.clone_fields(place.clone(), fields, span, out)?;
                    let index = adt.variant_index_with_id(variant.def_id);
                    let test = self.variant_test(place.clone(), ty, *adt, index);
                    value = Expr::cond(test, copy, value);
                }
                Ok(value)
            }
            // A std hasher, a `$SipHasher`: one of its state (ADR 0463).
            ty::Adt(..) if self.recognition().std_hasher(ty).is_some() => {
                Ok(Expr::call(Expr::member(place, "clone"), Vec::new()))
            }
            // A C-variadic function's `VaList`, the array of what it hasn't
            // read: a copy, read on its own (ADR 0428).
            ty::Adt(..) if super::recognition::is_va_list(self.tcx, ty) => {
                Ok(Expr::call(Expr::member(place, "slice"), Vec::new()))
            }
            // A `Cycle` a struct holds: the one it carries, of clones of what
            // it started with and what it steps (ADR 0544).
            ty::Adt(..) if self.held_adapter(ty) && super::recognition::cycled_source(self.tcx, ty).is_some() => {
                Ok(Expr::call_helper("$clone", vec![place]))
            }
            // A `Fuse`, `$fuse(..)`: one of a clone of what it steps, or of
            // none once it's ended (ADR 0473).
            ty::Adt(..) if let Some(inner) = self.recognition().fuse_of(ty) => {
                let iter = Expr::member(place.clone(), "iter");
                let clone = self.clone_fn("value", inner, span)?;
                let copy = Expr::call_helper("$iteratorWith", vec![iter.clone(), clone]);
                let fused = Expr::member(place, "fused");
                Ok(Expr::call_helper("$fuse", vec![Expr::bin(Op::And, iter, copy), fused]))
            }
            // std's iterator over an array is a `$iter` that knows where it is
            // (ADR 0364): a clone, a `$iter` of what it has left, each item a
            // clone of one where the iterator owns them; of its `rev()` too,
            // its array reversed (ADR 0530).
            ty::Adt(..) if self.steps_an_array(ty) => {
                let source = super::recognition::reversed_source(self.tcx, ty).unwrap_or(ty);
                let owns = self.array_source(source).unwrap_or(false);
                let left = self.stepped_left(place, ty, owns, span)?;
                Ok(self.stepped_items(left))
            }
            // Another's `rev()`: of a type parameter's given as it is, the
            // `$iterator` of it its `next_back` steps, made again of a clone
            // of its value, as a `Fuse`'s is; of one that knows its back,
            // `$rev(it)`, one of a clone of what it steps.
            ty::Adt(..) if let Some(source) = super::recognition::reversed_source(self.tcx, ty) => {
                if self.iterator_given_as_it_is(source) {
                    let clone = self.clone_fn("value", source, span)?;
                    return Ok(Expr::call_helper("$iteratorWith", vec![place, clone]));
                }
                let clone = self.clone_value(Expr::member(place, "source"), source, span, out)?;
                Ok(Expr::call_helper("$rev", vec![clone]))
            }
            _ if !matches!(ty.kind(), ty::Adt(adt, _) if self.is_std(adt.did())) => match self.shape(ty) {
                Shape::Object(fields) => self.clone_fields(place, fields, span, out),
                Shape::Array(tys) => Ok(Expr::array(
                    tys.into_iter()
                        .enumerate()
                        .map(|(i, t)| self.clone_value(Expr::index(place.clone(), Expr::int(i as i128)), t, span, out))
                        .collect::<R<_>>()?,
                )),
                Shape::Other => Err(self.unsupported(span, &format!("cloning `{ty}`"))),
            },
            _ => Err(self.unsupported(span, &format!("cloning `{ty}`"))),
        }
    }

    /// `{ ...place, v: <clone of place.v> }`: a new object, with a clone of
    /// each field that needs one.
    fn clone_fields(
        &mut self,
        place: Expr,
        fields: Vec<(String, Ty<'tcx>)>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut props = vec![Prop::Spread(place.clone())];
        for (name, t) in fields {
            // A `Cell` in a field is what it holds (ADR 0288): a clone of
            // one is a copy of that, which the spread makes if it needs none.
            let held = match t.kind() {
                ty::Adt(_, args) if self.is_std_type(t, StdItem::Cell) => Some(args.type_at(0)),
                _ => None,
            };
            if held.map_or(self.needs_clone(t), |item| self.needs_clone(item)) {
                let read = self.held(Expr::member(place.clone(), name.clone()), t);
                let field = self.clone_value(read, t, span, out)?;
                props.push(Prop::Field(name, self.holding(field, t)));
            }
        }
        Ok(Expr::object(props))
    }

    /// A clone of an array: `items.slice()`, or `items.map((item) => ..)`
    /// if its items need cloning too.
    pub(super) fn clone_items(&mut self, items: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        if !self.needs_clone(item) {
            // An array made here, which nothing else holds: itself.
            if super::iterators::made_array(&items) {
                return Ok(items);
            }
            return Ok(Expr::call(Expr::member(items, "slice"), Vec::new()));
        }
        let clone = self.clone_fn("item", item, span)?;
        Ok(Expr::call(Expr::member(items, "map"), vec![clone]))
    }

    /// A clone function of `ty`, for a helper that clones, or `undefined`
    /// where a clone is the value itself.
    pub(super) fn clone_arg(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.needs_clone(ty) {
            true => self.clone_fn("value", ty, span),
            false => Ok(Expr::undefined()),
        }
    }

    /// `(name) => <clone of name>`.
    pub(super) fn clone_fn(&mut self, name: &str, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let value = self.clone_value(Expr::var(name), ty, span, &mut body)?;
        body.push(StmtKind::Return(Some(value)).at(js::Span::NONE));
        Ok(Expr::arrow(vec![name.into()], body))
    }

    /// A derived `Default` of an enum is its `#[default]` variant, which
    /// has no fields: the constructor the derived body names, `Mode::Off`.
    fn default_variant(&self, default: DefId, ty: Ty<'tcx>) -> Option<DefId> {
        let tr = ty::TraitRef::new(self.tcx, default, [self.tcx.erase_and_anonymize_regions(ty)]);
        let Ok(ImplSource::UserDefined(imp)) = self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr))
        else {
            return None;
        };
        let method = self.tcx.associated_item_def_ids(imp.impl_def_id)[0].as_local()?;
        let mut expr = self.tcx.hir_body_owned_by(method).value;
        while let hir::ExprKind::Block(hir::Block { expr: Some(inner), .. }, _) = expr.kind {
            expr = inner;
        }
        let hir::ExprKind::Path(ref path) = expr.kind else {
            return None;
        };
        match self.tcx.typeck(method).qpath_res(path, expr.hir_id) {
            Res::Def(DefKind::Ctor(..), ctor) => Some(ctor),
            _ => None,
        }
    }

    /// `Default::default()` of `ty`: `0`, `""`, `[]`, a struct of its fields'
    /// defaults, or a call of a hand-written `default`.
    pub(super) fn default_value(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let default = std_item(self.tcx, StdItem::Default);
        // What a bound gave this function for it, a type parameter's or
        // a where clause's of one made of them, `[Self::Item; N]: Default`:
        // its dictionary's.
        let tr = ty::TraitRef::new(self.tcx, default, [ty]);
        if let Some(dictionary) = self.evidence_for(tr) {
            return Ok(Expr::call(Expr::member(dictionary, "default"), Vec::new()));
        }
        if self.is_unknown(ty) {
            return Err(self.no_evidence(span, tr));
        }
        // A React element's, React's empty node (ADR 0192): the react
        // crate's `impl` has no JS of its own.
        if let ty::Adt(adt, _) = ty.kind()
            && self
                .tcx
                .get_attrs_by_path(adt.did(), &[Symbol::intern("rust_js"), Symbol::intern("jsx_element")])
                .next()
                .is_some()
        {
            return Ok(Expr::undefined());
        }
        // react's `Rest`: no props, `undefined`, which a spread spreads
        // nothing of, and a JSX attribute isn't written for (ADR 0195).
        if super::bindings::is_rest(self.tcx, ty) {
            return Ok(Expr::undefined());
        }
        if self.has_user_impl(default, ty) {
            let method = self.tcx.associated_item_def_ids(default)[0];
            return self.impl_call(method, self.args_of(default, ty), Vec::new(), span);
        }
        self.check_value_ty(ty, span)?;
        // serde_json's `Value` is `Null` (ADR 0083).
        if self.json_type(ty) == Some(super::serde::Json::Value) {
            return Ok(Expr::str("Null"));
        }
        let std = |item: StdItem| self.is_std_type(ty, item);
        Ok(match ty.kind() {
            _ if let Some(num) = Num::of(ty) => num.literal(0),
            ty::Bool => Expr::bool(false),
            ty::Char => Expr::str("\0"),
            // `&str`'s is `""`, a slice's `[]`, and an array's its items'.
            ty::Ref(_, inner, _) if inner.is_str() => Expr::str(""),
            ty::Ref(_, inner, _) if inner.is_slice() => Expr::array(Vec::new()),
            // A `Box<str>`'s and a `Box<[T]>`'s, and `PhantomData`, which holds nothing.
            ty::Str => Expr::str(""),
            ty::Slice(_) => Expr::array(Vec::new()),
            _ if self.is_lang_adt(ty, LangItem::PhantomData) => Expr::undefined(),
            ty::Array(item, len) => {
                let len = len
                    .try_to_target_usize(self.tcx)
                    .ok_or_else(|| self.unsupported(span, &format!("`Default` of `{ty}`")))?;
                Expr::array((0..len).map(|_| self.default_value(*item, span)).collect::<R<_>>()?)
            }
            _ if ty.is_unit() || self.option_of(ty).is_some() => Expr::undefined(),
            _ if self.is_lang_adt(ty, LangItem::String) => Expr::str(""),
            _ if self.is_vec_like(ty) => Expr::array(Vec::new()),
            ty::Adt(_, args) if self.is_map(ty) => {
                let class = self.map_class(self.is_set(ty), args.types().next(), span)?;
                Expr::new_(class, Vec::new())
            }
            ty::Adt(_, args) if self.counted_rc(ty).is_some() => {
                Self::new_rc(self.default_value(args.type_at(0), span)?)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => self.default_value(args.type_at(0), span)?,
            ty::Adt(..) if std(StdItem::OnceCell) => {
                Expr::object(vec![Prop::Field(js::HELD.into(), Expr::undefined())])
            }
            // `LazyCell::new(T::default)` (ADR 0318).
            ty::Adt(_, args) if std(StdItem::LazyCell) => {
                let made = self.default_value(args.type_at(0), span)?;
                let init = Expr::arrow(vec![], vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]);
                Expr::object(vec![Prop::Field("init".into(), init)])
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) => Expr::object(
                vec![Prop::Field(js::HELD.into(), self.default_value(args.type_at(0), span)?)],
            ),
            ty::Adt(adt, _) if adt.is_enum() && !self.is_std(adt.did()) => {
                let variant = self
                    .default_variant(default, ty)
                    .ok_or_else(|| self.unsupported(span, &format!("`Default` of `{ty}`")))?;
                super::bindings::unit_variant(self.tcx, adt.did(), adt.variant_with_ctor_id(variant))
            }
            // A std hasher's, as its `new()` (ADR 0463).
            ty::Adt(..) if let Some(which) = self.recognition().std_hasher(ty) => {
                super::std_types::hash::new_hasher(which, Vec::new())
            }
            ty::Adt(adt, _) if self.is_std(adt.did()) => {
                return Err(self.unsupported(span, &format!("`Default` of `{ty}`")));
            }
            ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty() => Expr::undefined(),
            _ => match self.shape(ty) {
                // A flattened field's defaults are the struct's own (ADR 0204).
                Shape::Object(fields) => {
                    let made = Expr::object(
                        fields
                            .into_iter()
                            .map(|(name, t)| {
                                let value = self.default_value(t, span)?;
                                Ok(Prop::Field(name, self.holding(value, t)))
                            })
                            .collect::<R<_>>()?,
                    );
                    self.flattened_object(made, ty)
                }
                Shape::Array(tys) => {
                    Expr::array(tys.into_iter().map(|t| self.default_value(t, span)).collect::<R<_>>()?)
                }
                Shape::Other => return Err(self.unsupported(span, &format!("`Default` of `{ty}`"))),
            },
        })
    }

    pub(super) fn partial_eq_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::PartialEq, rustc_span::DUMMY_SP)
    }

    /// How JS's own comparison of a `ty` agrees with Rust's `==`, if it
    /// does: of a JS primitive, a string, a number, a `bool`, `()`, a
    /// fieldless enum of no `eq` of the crate's, `===` does; and
    /// SameValueZero, `includes`' and a `Map`'s, does too but of a float,
    /// whose `NaN` it finds where `==` doesn't. The one answer, which `==`,
    /// `contains` and a map's keys ask.
    pub(super) fn js_eq(&self, ty: Ty<'tcx>) -> Option<JsEq> {
        // A `&mut` to a number is a cell, an object (ADR 0099).
        if self.has_cell_layer(ty) {
            return None;
        }
        let ty = ty.peel_refs();
        // A parse error is its message, which tells its kind, as its derived
        // `==` compares (ADR 0063); a `TypeId` its type's key (ADR 0331).
        let primitive = self.is_string_like(ty)
            || self.is_parse_error(ty)
            || self.recognition().is_type_id(ty)
            || self.recognition().is_discriminant(ty)
            // A function pointer, by identity, as Rust's by address (ADR 0262).
            || matches!(ty.kind(), ty::FnPtr(..))
            || Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _) if super::representation::is_string_enum(self.tcx, *adt)
                && !self.has_user_impl(self.partial_eq_trait(), ty));
        primitive.then(|| match Num::of(ty).is_some_and(Num::float) {
            true => JsEq::Strict,
            false => JsEq::SameValue,
        })
    }

    /// Does `==` on `ty` run code of its own anywhere in it: a hand-written
    /// `eq`, or a `T`'s, which might be one? If not, it compares field by
    /// field, element by element, which `$eq` does.
    pub(super) fn custom_eq(&self, ty: Ty<'tcx>) -> bool {
        self.custom_eq_in(ty, &mut Vec::new())
    }

    fn custom_eq_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) || self.js_eq(ty).is_some() {
            return false;
        }
        seen.push(ty);
        let custom = match ty.kind() {
            _ if self.is_unknown(ty) => true,
            ty::Tuple(tys) => tys.iter().any(|t| self.custom_eq_in(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.custom_eq_in(*item, seen),
            ty::Adt(..) if self.has_user_impl(self.partial_eq_trait(), ty) => true,
            // A counted `Rc` compares what it points at, not its counts (ADR 0320).
            ty::Adt(..) if self.counted_rc(ty).is_some() => true,
            // A `Cow` compares what it borrows, not its variant (ADR 0400).
            ty::Adt(..) if self.recognition().is_cow(ty) => true,
            // A raw pointer, by its handle, not what it points to (ADR 0407).
            _ if self.is_handle_pointer(ty) => true,
            // `Vec`, `Box`, `Rc` and cells compare what they hold, and every
            // std type, its type's arguments, not its private fields: a
            // `BTreeMap`'s `NonNull` nodes are no pointer the program compares.
            ty::Adt(adt, args) if self.is_std_wrapper(ty) || self.recognition().is_std(adt.did()) => {
                args.types().any(|t| self.custom_eq_in(t, seen))
            }
            ty::Adt(adt, args) => adt
                .all_fields()
                .any(|f| self.custom_eq_in(self.field_ty(f, args), seen)),
            _ => false,
        };
        seen.pop();
        custom
    }

    /// `a == b` for values of type `ty`: `a === b` for JS primitives, a call
    /// of a hand-written `eq`, `TPartialEq.eq(a, b)` in generic code, and
    /// `$eq(a, b)` for what compares field by field. A derived `==` of a
    /// type with a custom part compares its parts one by one.
    /// `a == b` of an `a` of `ty` and a `b` of `rhs`, `PartialEq<Rhs>`'s: of
    /// items of two types, `[A::Item] == [B::Item]`, each by its own; of two
    /// types a bound compares, `A::Item: PartialEq<B::Item>`, its
    /// dictionary's; else `eq_value`'s, of `ty`.
    pub(super) fn eq_values(
        &mut self,
        a: Expr,
        b: Expr,
        (ty, rhs): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (x, lhs) = self.through_refs(a.clone(), ty);
        let (y, other) = self.through_refs(b.clone(), rhs);
        if lhs != other {
            let items = |ty: Ty<'tcx>| match ty.kind() {
                ty::Array(item, _) | ty::Slice(item) => Some(*item),
                ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
                _ => None,
            };
            if let (Some(item), Some(other_item)) = (items(lhs), items(other))
                && item != other_item
            {
                return self.eq_items(x, y, (item, other_item), span, out);
            }
            let tr = ty::TraitRef::new(self.tcx, self.partial_eq_trait(), [lhs, other]);
            if (lhs.has_param() || other.has_param())
                && let Some(dictionary) = self.evidence_for(tr)
            {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![x, y]));
            }
        }
        self.eq_value(a, b, ty, span, out)
    }

    pub(super) fn eq_value(&mut self, a: Expr, b: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (mut a, _) = self.through_refs(a, ty);
        let (mut b, mut ty) = self.through_refs(b, ty);
        // A `Box`, an `Rc` and a `Pin` compare what they point at, which
        // they are in JS (ADRs 0023, 0329), its own `==` too, and a `Cow`
        // what it borrows.
        loop {
            match ty.kind() {
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none() => {
                    ty = args.type_at(0);
                }
                ty::Adt(_, args) if self.recognition().pinned(ty).is_some() => {
                    let pointer = args.type_at(0);
                    (a, _) = self.through_refs(a, pointer);
                    (b, ty) = self.through_refs(b, pointer);
                }
                // A `Cow` what it borrows (ADR 0400).
                ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {
                    (a, b, ty) = (x, self.cow_deref(&b, ty).expect("a `Cow`").0, borrowed);
                }
                _ => break,
            }
        }
        // `o == Some(true)` of an `Option<bool>`: `!!o`, which a test reads
        // as `o`, as JS tests a flag that may be missing (ADR 0298).
        if self.option_of(ty).is_some_and(|inner| inner.is_bool()) {
            match (&a.kind, &b.kind) {
                (_, js::ExprKind::Bool(true)) => return Ok(Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, a))),
                (js::ExprKind::Bool(true), _) => return Ok(Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, b))),
                _ => {}
            }
        }
        // A raw pointer: the same place, or the same address (ADR 0562).
        if self.is_handle_pointer(ty) {
            return Ok(Expr::call(Expr::helper("$ptrEq"), vec![a, b]));
        }
        // A path's components, `a/b` and `a//b` alike (ADR 0401).
        if self.recognition().is_path(ty) {
            return Ok(Expr::call(Expr::helper("$pathEq"), vec![a, b]));
        }
        if self.js_eq(ty).is_some() {
            return Ok(Expr::bin(Op::Eq, a, b));
        }
        // A type of a type parameter whose `PartialEq` a bound gives, a
        // method's `where Self: PartialEq` of a `Wrapping<T>`: its dictionary's,
        // as its parts' needn't be.
        if ty.has_param() && !self.is_unknown(ty) {
            let tr = ty::TraitRef::new_from_args(
                self.tcx,
                self.partial_eq_trait(),
                self.args_of(self.partial_eq_trait(), ty),
            );
            if let Some(dictionary) = self.evidence_for(tr) {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![a, b]));
            }
        }
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new_from_args(
                self.tcx,
                self.partial_eq_trait(),
                self.args_of(self.partial_eq_trait(), ty),
            );
            if let Some(dictionary) = self.evidence_for(tr) {
                return Ok(Expr::call(Expr::member(dictionary, "eq"), vec![a, b]));
            }
            // A `T: Ord` or `T: PartialOrd` only: equal is `Equal` (ADR 0057).
            let ord = ty::TraitRef::new(self.tcx, self.ord_trait(), [ty]);
            let partial_ord = ty::TraitRef::new_from_args(
                self.tcx,
                self.partial_ord_trait(),
                self.args_of(self.partial_ord_trait(), ty),
            );
            if self.has_evidence(ord) || self.has_evidence(partial_ord) {
                let order = self.cmp_value(a, b, ty, !self.has_evidence(ord), span, out)?;
                return Ok(Expr::bin(Op::Eq, order, Expr::int(0)));
            }
            return Err(self.no_evidence(span, tr));
        }
        if self.has_user_impl(self.partial_eq_trait(), ty) {
            let eq = self.tcx.associated_item_def_ids(self.partial_eq_trait())[0];
            let args = self.args_of(self.partial_eq_trait(), ty);
            return self.impl_call(eq, args, vec![a, b], span);
        }
        // A tagged enum's fieldless variant, `{ kind: "On" }` (ADR 0284):
        // its tag, as the derive compares its variants.
        if let ty::Adt(adt, _) = ty.kind()
            && super::representation::is_fieldless_enum(*adt)
            && let Some(key) = super::bindings::declared_tag(self.tcx, adt.did())
        {
            return Ok(Expr::bin(Op::Eq, Expr::member(a, &key), Expr::member(b, &key)));
        }
        // A constant, like a fieldless variant's `"Nothing"`, is a JS
        // primitive: only itself is equal to it. `None` is also `null`.
        if a.is_constant() || b.is_constant() {
            let none = [&a, &b].iter().any(|x| matches!(x.kind, js::ExprKind::Undefined));
            let op = if none { Op::LooseEq } else { Op::Eq };
            return Ok(Expr::bin(op, a, b));
        }
        if let Some(inner) = self.option_of(ty) {
            // `==`, so that `null` and `undefined` are both `None`; of a
            // `Some` that's its payload, not one boxed, an object of its own
            // (ADR 0051), which compares by what's in it.
            if self.js_eq(inner).is_some() && !self.boxed_payload(inner) {
                return Ok(Expr::bin(Op::LooseEq, a, b));
            }
        }
        // Its bounds' (ADR 0129): `a.start === b.start && a.end === b.end`.
        if let Some(kind) = self.range_kind(ty) {
            let index = self.range_index(ty);
            let a = self.range_parts(a, kind, out);
            let b = self.range_parts(b, kind, out);
            let mut all: Option<Expr> = None;
            for (x, y) in a.into_iter().zip(b) {
                let part = self.eq_value(x, y, index.expect("a bound's type"), span, out)?;
                all = Some(match all {
                    Some(all) => Expr::bin(Op::And, all, part),
                    None => part,
                });
            }
            return Ok(all.unwrap_or_else(|| Expr::bool(true)));
        }
        // Two maps, or two sets (ADR 0059): as many of each, and each key of
        // one the other's, of an equal value: `$mapEq(a, b, (x, y) => ..)`,
        // `$setEq(a, b)`.
        if self.is_map(ty)
            && let ty::Adt(_, args) = ty.kind()
        {
            if self.is_set(ty) {
                return Ok(Expr::call_helper("$setEq", vec![a, b]));
            }
            let (x, y) = (self.fresh("x"), self.fresh("y"));
            let mut made = Vec::new();
            let same = self.eq_value(Expr::var(&x), Expr::var(&y), args.type_at(1), span, &mut made)?;
            let is = |e: &Expr, name: &str| matches!(&e.kind, js::ExprKind::Var(v) if v == name);
            let mut list = vec![a, b];
            match &same.kind {
                // `===`, which it compares by where it's given none.
                js::ExprKind::Binary(Op::Eq, l, r) if made.is_empty() && is(l, &x) && is(r, &y) => {}
                // A helper of the two, `$eq`: the helper.
                js::ExprKind::Call(callee, given)
                    if made.is_empty()
                        && callee.helper_name().is_some()
                        && matches!(&given[..], [l, r] if is(l, &x) && is(r, &y)) =>
                {
                    list.push((**callee).clone());
                }
                _ => {
                    made.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
                    list.push(Expr::arrow(vec![x.into(), y.into()], made));
                }
            }
            return Ok(Expr::call_helper("$mapEq", list));
        }
        let structural = matches!(ty.kind(), ty::Tuple(_) | ty::Array(..) | ty::Slice(_))
            || self.is_std_wrapper(ty)
            || matches!(ty.kind(), ty::Adt(adt, _) if !self.is_std(adt.did()) || self.is_known_std(ty));
        if !structural || self.is_js_object(ty) || self.is_lang_adt(ty, LangItem::String) {
            return Err(self.unsupported(span, &format!("`==` on `{ty}`")));
        }
        if !self.custom_eq(ty) {
            return Ok(Expr::call(Expr::helper("$eq"), vec![a, b]));
        }
        // Each is read more than once below.
        let a = if a.reads_same() { a } else { self.spill("left", a, out) };
        let b = if b.reads_same() { b } else { self.spill("right", b, out) };
        let std = |item: StdItem| self.is_std_type(ty, item);
        match ty.kind() {
            ty::Array(item, _) | ty::Slice(item) => self.eq_items(a, b, (*item, *item), span, out),
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.eq_items(a, b, (args.type_at(0), args.type_at(0)), span, out)
            }
            ty::Adt(_, args) if let Some(pointee) = self.counted_rc(ty) => {
                let (a, b) = (self.counted_pointee(a, pointee), self.counted_pointee(b, pointee));
                self.eq_value(a, b, args.type_at(0), span, out)
            }
            // Two `RefCell`s', each while it's borrowed (ADR 0328), held where
            // their values' `==` may ask.
            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {
                let mut body = Vec::new();
                let eq = self.eq_value(Expr::var("x"), Expr::var("y"), args.type_at(0), span, &mut body)?;
                body.push(StmtKind::Return(Some(eq)).at(js::Span::NONE));
                let inner = Expr::call(
                    Expr::helper("$withBorrow"),
                    vec![b, Expr::arrow(vec!["y".into()], body)],
                );
                Ok(Expr::call(
                    Expr::helper("$withBorrow"),
                    vec![
                        a,
                        Expr::arrow(vec!["x".into()], vec![StmtKind::Return(Some(inner)).at(js::Span::NONE)]),
                    ],
                ))
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let (a, b) = match std(StdItem::RefCell) {
                    true => (
                        Expr::call(Expr::helper("$borrow"), vec![a]),
                        Expr::call(Expr::helper("$borrow"), vec![b]),
                    ),
                    false => (a, b),
                };
                self.eq_value(Expr::held(a), Expr::held(b), args.type_at(0), span, out)
            }
            ty::Adt(_, args) if std(StdItem::OnceCell) => self.eq_value(
                Expr::held(a),
                Expr::held(b),
                Ty::new_option(self.tcx, args.type_at(0)),
                span,
                out,
            ),
            ty::Adt(_, args) if self.option_of(ty).is_some() => {
                let inner = args.type_at(0);
                let (x, y) = if self.boxed_payload(inner) {
                    (self.some_value(a.clone()), self.some_value(b.clone()))
                } else {
                    (a.clone(), b.clone())
                };
                let some = self.eq_value(x, y, inner, span, out)?;
                let none = Expr::bin(
                    Op::Or,
                    Expr::bin(Op::LooseEq, a.clone(), Expr::null()),
                    Expr::bin(Op::LooseEq, b.clone(), Expr::null()),
                );
                Ok(Expr::cond(none, Expr::bin(Op::LooseEq, a, b), some))
            }
            // An untagged enum's variant is its payload (ADR 0214): the same
            // variant, told by its kind, and its payloads equal.
            ty::Adt(adt, args) if self.untagged(ty).is_some() => {
                let mut value = Expr::bool(false);
                for variant in adt.variants().iter().rev() {
                    // One without fields, its name's string, is `b`'s the same.
                    let Some(field) = variant.fields.iter().next() else {
                        let both = self.untagged_variant_test(ty, variant, &b);
                        value = Expr::cond(self.untagged_variant_test(ty, variant, &a), both, value);
                        continue;
                    };
                    let payload = self.field_ty(field, args);
                    let same = self.eq_value(a.clone(), b.clone(), payload, span, out)?;
                    let both = Expr::bin(Op::And, self.untagged_variant_test(ty, variant, &b), same);
                    value = Expr::cond(self.untagged_variant_test(ty, variant, &a), both, value);
                }
                Ok(value)
            }
            ty::Adt(adt, args) if adt.is_enum() => {
                // `{ TAG: "Line", _0: .. }` (ADR 0033): a variant with a custom
                // part compares its fields, and `$eq` the rest.
                let mut value = Expr::call(Expr::helper("$eq"), vec![a.clone(), b.clone()]);
                for variant in adt.variants().iter().rev() {
                    let fields = self.variant_fields(variant, args);
                    if !fields.iter().any(|&(_, t)| self.custom_eq(t)) {
                        continue;
                    }
                    let index = adt.variant_index_with_id(variant.def_id);
                    let tag = |cx: &Self, x: &Expr| cx.variant_test(x.clone(), ty, *adt, index);
                    let mut same = tag(self, &b);
                    for (field, t) in fields {
                        let field = self.eq_value(
                            self.held(Expr::member(a.clone(), field.clone()), t),
                            self.held(Expr::member(b.clone(), field), t),
                            t,
                            span,
                            out,
                        )?;
                        same = Expr::bin(Op::And, same, field);
                    }
                    value = Expr::cond(tag(self, &a), same, value);
                }
                Ok(value)
            }
            _ => {
                let parts: Vec<(Expr, Expr, Ty<'tcx>)> = match self.shape(ty) {
                    Shape::Object(fields) => fields
                        .into_iter()
                        .map(|(name, t)| {
                            let x = self.held(Expr::member(a.clone(), name.clone()), t);
                            (x, self.held(Expr::member(b.clone(), name), t), t)
                        })
                        .collect(),
                    Shape::Array(tys) => tys
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| {
                            (
                                Expr::index(a.clone(), Expr::int(i as i128)),
                                Expr::index(b.clone(), Expr::int(i as i128)),
                                t,
                            )
                        })
                        .collect(),
                    Shape::Other => return Err(self.unsupported(span, &format!("`==` on `{ty}`"))),
                };
                let mut all: Option<Expr> = None;
                for (x, y, t) in parts {
                    let part = self.eq_value(x, y, t, span, out)?;
                    all = Some(match all {
                        Some(all) => Expr::bin(Op::And, all, part),
                        None => part,
                    });
                }
                Ok(all.unwrap_or_else(|| Expr::bool(true)))
            }
        }
    }

    /// `a.length === b.length && a.every((x, i) => <x == b[i]>)`, of `a`'s
    /// items and `b`'s, each held where it's more than a place.
    fn eq_items(
        &mut self,
        a: Expr,
        b: Expr,
        (item, other_item): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let a = if a.reads_same() { a } else { self.spill("items", a, out) };
        let b = if b.reads_same() { b } else { self.spill("other", b, out) };
        let mut body = Vec::new();
        let other = Expr::index(b.clone(), Expr::var("i"));
        let same = self.eq_values(Expr::var("x"), other, (item, other_item), span, &mut body)?;
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        let every = Expr::call(
            Expr::member(a.clone(), "every"),
            vec![Expr::arrow(vec!["x".into(), "i".into()], body)],
        );
        let lengths = Expr::bin(Op::Eq, Expr::member(a, "length"), Expr::member(b, "length"));
        Ok(Expr::bin(Op::And, lengths, every))
    }

    /// `(a, b) => <a == b>` for a dictionary's `eq`, or `$eq` itself.
    pub(super) fn eq_fn(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let same = self.eq_value(Expr::var("a"), Expr::var("b"), ty, span, &mut body)?;
        if body.is_empty()
            && let js::ExprKind::Call(callee, _) = &same.kind
            && callee.helper_name() == Some("$eq")
        {
            return Ok(Expr::helper("$eq"));
        }
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
    }
}
