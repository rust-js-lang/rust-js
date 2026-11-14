//! `Clone` where rust-js writes the implementation: a derived one, a std
//! type's, or a bound's dictionary's (ADRs 0052, 0053). A hand-written
//! one is called instead. Whether a clone could be told apart from the
//! value, `needs_clone`, and the clone itself, `clone_value`.

use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::recognition::StdItem;
use crate::lower::recursive_fns::Kind;
use crate::lower::temporary_names::{Base, Role};
use crate::lower::{FnCx, R, Shape, lower_first};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(in crate::lower) fn clone_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP)
    }

    /// Can a clone of `ty` be told apart from the value itself? Only if one
    /// of them can change in place, like a `Vec`, a cell, or a type in
    /// `mutated` (ADR 0020), or if cloning runs code: a hand-written `clone`,
    /// or a `T`'s, which might be one.
    pub(in crate::lower) fn needs_clone(&self, ty: Ty<'tcx>) -> bool {
        self.needs_clone_in(ty, &mut Vec::new())
    }

    fn needs_clone_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        // An uncounted `Weak` is shared, as what its `Rc` is, whatever's
        // changed of what it points at (ADR 0843).
        if self.uncounted_weak(ty).is_some() {
            return false;
        }
        // A `#[repr(transparent)]` struct of the program's is its field (ADR
        // 0584): one of what's no object, a function, is told apart from
        // its clone as that is, whatever's written over where it's kept,
        // `*task = next`, unless its own `clone` runs code.
        if let ty::Adt(adt, _) = ty.kind()
            && !self.is_std(adt.did())
            && let Some(held) = self.transparent(ty)
            && !self.may_be_object(held)
            && !self.has_user_impl(self.clone_trait(), ty)
        {
            return self.needs_clone_in(held, seen);
        }
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
            // changes one in place, as a slice or as the `Vec` its
            // `into_vec()` makes, `items_changed`'s, or its items need a
            // clone (ADR 0530, amended).
            ty::Slice(item) => self.items_changed(*item) || self.needs_clone_in(*item, seen),
            // A `Vec`'s, of what `into_boxed_slice()` makes of it too.
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.vec_changed(ty)
                    || (std(StdItem::Vec) && self.items_changed(args.type_at(0)))
                    || self.needs_clone_in(args.type_at(0), seen)
            }
            ty::Adt(..)
                if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) || std(StdItem::OnceCell) =>
            {
                true
            }
            // A map or a set changes in place (ADR 0059).
            ty::Adt(..) if self.is_map(ty) => true,
            // A counted `Rc`'s clone counts one more, and a `Weak`'s (ADR 0320).
            ty::Adt(..) if self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() => true,
            // A path is its text, which JS never changes in place (ADR 0173),
            // and a parse error its message, or what it's made of, which
            // nothing changes (#118).
            ty::Adt(..)
                if self.is_rc(ty)
                    || self.is_lang_adt(ty, LangItem::String)
                    || self.recognition().is_path_like(ty)
                    || self.is_js_object(ty)
                    || self.is_parse_error(ty) =>
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

    /// `Clone::clone` of the `ty` at `place`: the place itself when nothing
    /// could tell a clone from it, and otherwise a copy of the parts that
    /// could, calling each hand-written `clone` on the way.
    pub(in crate::lower) fn clone_value(
        &mut self,
        place: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // What an `impl Trait` is arrives revealed, as MIR's types are
        // after analysis, a returned struct's or closure's, held or not
        // (`impl_trait_clone_held.rs`). A closure is its JS function, which shares what it captured: a
        // clone is the same function where nothing could tell (ADR 0522);
        // one cloned apart makes itself again, of a clone of what it took.
        // As the crate decided of each it clones, a library's as its library
        // did (#1202).
        if let ty::Closure(id, _) = *ty.kind() {
            match self.krate.closure_facts.clone_is_itself(id) {
                Some(true) => return Ok(place),
                Some(false) => return Ok(Expr::call_helper("$clone", vec![place])),
                None => {}
            }
            let drops = |t| self.drops(t) != crate::lower::drops::Drops::Nothing;
            return match crate::lower::analysis::clone_shares(self.tcx, self.typing_env, ty, drops) {
                true => Ok(place),
                false => Err(self.unsupported(span, "cloning a closure where it isn't seen to be cloned")),
            };
        }
        // An iterator cloned apart makes itself again (ADR 0525); a copy of
        // one is the iterator it copies (ADR 0480), its clone that one's.
        // So does memchr's iterator, which carries its clone (ADR 0529).
        // Not a held one std compiles, a `Map`, its struct (ADR 0871), whose
        // clone is its compiled `Clone`: one made in place is part of what
        // `clone_in_place` clones.
        let held_compiled = crate::lower::recognition::is_held_compiled(self.tcx, ty);
        if (self.iterator_cloned_apart(ty) && !held_compiled) || self.recognition().is_memchr_iter(ty) {
            return Ok(Expr::call_helper("$clone", vec![place]));
        }
        // A `copied()` std compiles held, `--std-iterators`', its struct too.
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty)
            && !held_compiled
        {
            return self.copied_clone(place, (ty, source), span, out);
        }
        // A channel's sender: one more, of the same queue (ADR 0142).
        if self.recognition().channel_end(ty) == Some(crate::lower::recognition::ChannelEnd::Sender) {
            return Ok(Expr::call(Expr::helper("$cloneSender"), vec![place]));
        }
        // A counted `Rc`, or a `Weak`: one more, of the same value (ADR 0320).
        if self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() {
            let clone = if self.counted_weak(ty).is_some() {
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
            // A `T: Copy` alone: its dictionary's supertrait `Clone`'s where
            // its caller gives it one, else its copy (ADR 0049; ADR 0623,
            // amended, #1451).
            return Ok(self.cloned_by_copy_bound(place, ty));
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
        if self.is_copy(ty) && !self.copy_clone_runs_code(ty) {
            return Ok(self.copy(place, ty));
        }
        // A type inside itself, `Value` in `Obj(BTreeMap<String, Value>)`: its
        // clone is a function, which calls itself for the ones inside
        // (`recursive_fns`).
        if self.is_recursive(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a recursive type is an ADT");
            };
            let type_name = self.tcx.item_name(adt.did()).to_string();
            let (base, param) = (format!("clone{type_name}"), lower_first(&type_name));
            let name = self.recursive_fn(
                (Kind::Clone, ty, &base),
                &[Base::Named(&param)],
                |cx, params, body| cx.clone_parts(params[0].clone(), ty, span, body),
                out,
            )?;
            return Ok(Expr::call(Expr::var(&name), vec![place]));
        }
        self.clone_parts(place, ty, span, out)
    }

    /// How each entry of the map or set `ty` is cloned, of std's `(K, V)`,
    /// its key's clone then its value's, the key dropped as its value's
    /// clone unwinds (#897): `[key, value] => [..]` or `item => ..`, the
    /// entry's type, and whether a clone may panic; `None` where neither
    /// needs one. Each key that needs one, whatever finds it: a primitive
    /// one never does, and one found by value (ADR 0121), by a B-tree's
    /// `cmp` or a caller's class may (#924). A clone's, and a hashed map's
    /// `clone_from`'s (#922).
    pub(in crate::lower) fn entry_clone(
        &mut self,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<(Expr, Ty<'tcx>, bool)>> {
        let ty::Adt(_, args) = ty.kind() else {
            unreachable!("a map's or a set's type")
        };
        let set = self.is_set(ty);
        let (key_ty, value_ty) = (args.type_at(0), (!set).then(|| args.type_at(1)));
        let key = Some(key_ty).filter(|&k| self.needs_clone(k));
        let value = value_ty.filter(|&v| self.needs_clone(v));
        if key.is_none() && value.is_none() {
            return Ok(None);
        }
        let mut cloned = |this: &mut Self, name: &str, ty: Option<Ty<'tcx>>| match ty {
            Some(ty) => this.clone_value(Expr::var(name), ty, span, out),
            None => Ok(Expr::var(name)),
        };
        let may_panic = |this: &Self, ty: Option<Ty<'tcx>>| ty.is_some_and(|t| this.clone_may_panic(t));
        let (params, item, entry) = if set {
            (vec!["item".into()], cloned(self, "item", key)?, key_ty)
        } else {
            let parts = [
                (key_ty, may_panic(self, key)),
                (value_ty.expect("a map's"), may_panic(self, value)),
            ];
            let named = [("key", key), ("value", value)];
            let (pair, made) = self.parts_in_turn(&parts, span, |cx, i| cloned(cx, named[i].0, named[i].1))?;
            let pattern = js::Pattern::Array(vec![Some("key".into()), Some("value".into())]);
            let entry = Ty::new_tup(self.tcx, &[key_ty, parts[1].0]);
            (vec![pattern], Self::in_turn(Expr::array(pair), made), entry)
        };
        let f = Expr::arrow(params, vec![StmtKind::Return(Some(item)).at(js::Span::NONE)]);
        Ok(Some((f, entry, may_panic(self, key) || may_panic(self, value))))
    }

    /// Whether `Clone::clone` of a `Copy` `ty` runs code, so isn't its bits:
    /// a hand-written `clone` of it, or one a part's clone calls, as std's
    /// clone of a tuple, an array, an `Option` or a derived one of a generic
    /// type clones each part (ADR 0623, amended). Not one std's own impls
    /// say is its bits, `TrivialClone`, rustc's answer of this toolchain's
    /// std: `#[derive(Clone, Copy)]`'s, a reference's, an array of such.
    pub(in crate::lower) fn copy_clone_runs_code(&self, ty: Ty<'tcx>) -> bool {
        let trivial = self.tcx.lang_items().get(LangItem::TrivialClone);
        if trivial.is_some_and(|trivial| self.recognition().implements_trait(ty, trivial)) {
            return false;
        }
        match ty.kind() {
            ty::Tuple(tys) => tys.iter().any(|t| self.copy_clone_runs_code(t)),
            ty::Array(item, _) => self.copy_clone_runs_code(*item),
            ty::Adt(adt, args) => {
                self.has_user_impl(self.clone_trait(), ty)
                    || adt
                        .all_fields()
                        .any(|f| self.copy_clone_runs_code(self.field_ty(f, args)))
            }
            _ => false,
        }
    }

    /// Whether a clone of `ty` shows, beyond what changes in place, which
    /// `needs_clone` asks: a hand-written `clone` of it, or of a part a
    /// derived one or std's clones, whose value or effects may differ from
    /// what it was given; an `Rc`'s or a `Weak`'s that counts one more; a
    /// type only a caller knows. Not a `Copy` value's bits
    /// (`copy_clone_runs_code` asks whether those are its clone). Where it
    /// doesn't, a clone made later can't be told from one made sooner: a
    /// `cycle()`'s of an array's items (#1630).
    pub(in crate::lower) fn clone_shows(&self, ty: Ty<'tcx>) -> bool {
        self.clone_shows_in(ty, &mut Vec::new())
    }

    fn clone_shows_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if self.is_copy(ty) {
            return self.copy_clone_runs_code(ty);
        }
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        let shows = match ty.kind() {
            ty::Ref(..) => false,
            _ if crate::lower::generic_facts::clone_given_by_caller(ty) => true,
            ty::Tuple(parts) => parts.iter().any(|part| self.clone_shows_in(part, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.clone_shows_in(*item, seen),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).any(|upvar| self.clone_shows_in(upvar, seen))
            }
            ty::Adt(..) if self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() => true,
            ty::Adt(..)
                if self.has_user_impl(self.clone_trait(), ty) && !self.is_derived_impl(self.clone_trait(), ty) =>
            {
                true
            }
            ty::Adt(adt, args) if !self.is_std(adt.did()) => adt
                .all_fields()
                .any(|f| self.clone_shows_in(self.field_ty(f, args), seen)),
            ty::Adt(_, args) => args.types().any(|arg| self.clone_shows_in(arg, seen)),
            _ => false,
        };
        seen.pop();
        shows
    }

    /// Is `ty`, one of the crate's own or a library's (ADR 0100), inside
    /// itself, through its fields or what a std type holds? A std type that
    /// holds one is cloned in place.
    pub(in crate::lower) fn is_recursive(&self, ty: Ty<'tcx>) -> bool {
        let mut seen = Vec::new();
        matches!(ty.kind(), ty::Adt(adt, _) if self.krate.foreign.compiled_by_rust_js(adt.did()))
            && self.holds(ty, ty, &mut seen)
    }

    /// Does `outer` hold `target` anywhere inside it?
    fn holds(&self, outer: Ty<'tcx>, target: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let parts: Vec<Ty<'tcx>> = match outer.kind() {
            ty::Adt(adt, args) if self.krate.foreign.compiled_by_rust_js(adt.did()) => {
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
            // A `ManuallyDrop`'s, its value's (ADR 0584).
            ty::Adt(..) if let Some(held) = self.recognition().std_wrapped(ty) => {
                return self.clone_value(place, held, span, out);
            }
            // A `RefCell` of an object's, the object's clone (ADR 0863),
            // while it's borrowed: checked free to, and held where its
            // clone may ask (ADR 0328).
            ty::Adt(_, args) if std(StdItem::RefCell) && self.is_value_cell(ty) => {
                if !self.recognition().std_alone(args.type_at(0)) {
                    let param = self.fresh(Role::Value);
                    let mut body = Vec::new();
                    let value = self.clone_value(Expr::var(&param), args.type_at(0), span, &mut body)?;
                    body.push(StmtKind::Return(Some(value)).at(js::Span::NONE));
                    return Ok(Expr::call(
                        Expr::helper("$withBorrow"),
                        vec![place, Expr::arrow(vec![param.as_str().into()], body), Expr::bool(true)],
                    ));
                }
                let cell = match self.quiet_ref_cell(ty, false) {
                    true => place,
                    false => Expr::call(Expr::helper("$borrow"), vec![place]),
                };
                return self.clone_value(cell, args.type_at(0), span, out);
            }
            // A `RefCell`'s, while it's borrowed (ADR 0328): checked free to,
            // and held where its value's clone may ask.
            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {
                // Its parameter named apart from what its body names, a
                // part read more than once of a `RefCell` inside (#200).
                let param = self.fresh(Role::Value);
                let mut body = Vec::new();
                let value = self.clone_value(Expr::var(&param), args.type_at(0), span, &mut body)?;
                body.push(StmtKind::Return(Some(value)).at(js::Span::NONE));
                let value = Expr::call(
                    Expr::helper("$withBorrow"),
                    vec![place, Expr::arrow(vec![param.as_str().into()], body)],
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
        let written = place.clone();
        let place = if !place.reads_same() {
            self.spill(Role::Value, place, out)
        } else {
            place
        };
        match ty.kind() {
            // `new Map(m)`, cloning each value and each key that needs it,
            // whatever finds it: a primitive one never does, and one found
            // by value (ADR 0121), by a B-tree's `cmp` or a caller's class
            // may (#924).
            ty::Adt(_, args) if self.is_map(ty) => {
                let set = self.is_set(ty);
                let key = args.types().next();
                let value = args.types().nth(1).filter(|_| !set);
                let sorted = self.is_sorted(ty);
                let class = self.map_class(set, sorted, (key, value), span)?;
                let key_ty = args.type_at(0);
                // A B-tree of keys found by their own `cmp`, or its caller's,
                // made of its entries in their order, asking it nothing, as
                // std's `clone_subtree` asks nothing (#924): `ofSorted`; of
                // std's nodes, in its source's shape (#926).
                let nodes = sorted && self.tree_of_nodes(key_ty, value);
                let in_order = sorted && self.found_by_cmp(key_ty) || nodes;
                let shape = nodes.then(|| place.clone());
                let made_in_order = |made: Expr| match in_order {
                    true => Expr::call(
                        Expr::member(class.clone(), "ofSorted"),
                        [made].into_iter().chain(shape.clone()).collect(),
                    ),
                    false => Expr::new_(class.clone(), vec![made]),
                };
                let Some((f, entry, may_panic)) = self.entry_clone(ty, span, out)? else {
                    return Ok(made_in_order(place));
                };
                let made = match may_panic && self.drops(entry) != crate::lower::drops::Drops::Nothing {
                    // Of std's nodes, given its key's and value's drops (#926).
                    true if shape.is_some() && !set => self.made_each_of_nodes(place, f, entry, args, span)?,
                    true => self.made_each(place, f, (entry, true), span)?,
                    false => {
                        let all = Expr::builtin(Expr::js_global("Array"), "from", vec![place]);
                        Expr::builtin(all, "map", vec![f])
                    }
                };
                Ok(made_in_order(made))
            }
            // `{ start: r.start, end: r.end }` (ADR 0129); of an inclusive one
            // of numbers or `char`s, which a step may leave exhausted, `{ ...r
            // }`, its `exhausted` too (#414).
            ty::Adt(_, args)
                if self.range_kind(ty) == Some(crate::lower::std_types::range::RangeKind::Inclusive)
                    && (args.type_at(0).is_integral() || args.type_at(0).is_char())
                    && !self.never_exhausted(&place) =>
            {
                Ok(Expr::object(vec![Prop::Spread(place)]))
            }
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
                // Of a payload whose clone is itself, the `Option` itself,
                // not `key == null ? key : key`; as it's written, not held
                // first to be read once, `items.at(-1)` (#587).
                if some == place {
                    if written != place
                        && let js::ExprKind::Var(held) = &place.kind
                        && out
                            .last()
                            .is_some_and(|s| matches!(&s.kind, StmtKind::Const(name, _) if name == held))
                    {
                        self.names.swap_remove(held);
                        out.pop();
                        return Ok(written);
                    }
                    return Ok(place);
                }
                let none = Expr::bin(Op::LooseEq, place.clone(), Expr::null());
                Ok(Expr::cond(none, place, some))
            }
            // An untagged enum's variant is its payload (ADR 0214): one that
            // needs it gets the payload's clone, told by its kind: being changed
            // in place, as a tagged one's object is, is the payload's.
            // One whose array is changed in place itself (rd490) gets a
            // copy of each payload, and of arrays of what needs no clone of
            // its own, the array's `slice()`.
            ty::Adt(adt, args) if self.untagged(ty).is_some() => {
                let itself = self.untagged_changed_itself(ty);
                if itself && self.sliced_untagged(ty, &|t| self.needs_clone(t)) {
                    return Ok(Expr::builtin(place, "slice", Vec::new()));
                }
                let mut value = place.clone();
                for variant in adt.variants().iter().rev() {
                    let Some(field) = variant.fields.iter().next() else {
                        continue;
                    };
                    let payload = self.field_ty(field, args);
                    let copy = match self.needs_clone(payload) {
                        true => self.clone_value(place.clone(), payload, span, out)?,
                        false if itself => self.copy(place.clone(), payload),
                        false => continue,
                    };
                    let test = self.untagged_variant_test(ty, &[variant], &[], &place, false);
                    value = Expr::cond(test, copy, value);
                }
                Ok(value)
            }
            // A `Cow`'s is too: its `Owned`'s clone is the `to_owned()` of what it borrows.
            ty::Adt(adt, args)
                if adt.is_enum()
                    && (!self.is_std(adt.did())
                        || self.is_known_std(ty)
                        || self.std_enum_derives_clone(ty)
                        || self.recognition().cow_parts(ty).is_some()) =>
            {
                // `{ TAG: "Line", _0: .. }` (ADR 0033): a variant with fields
                // that need it gets a copy, and every other value is itself.
                // A variant with a cell that's the property it holds, set in
                // place, is copied, of what it holds a clone (#503).
                let cells = |cx: &Self, variant: &ty::VariantDef, fields: &[(String, Ty<'tcx>)]| {
                    (variant.fields.iter().zip(fields)).any(|(def, &(_, t))| cx.property_cell(Some(def), t).is_some())
                };
                let itself = self.mutated_itself(ty);
                // Whether a variant's field needs a clone of its own: of such a
                // cell, what it holds.
                let needs = |cx: &Self, variant: &ty::VariantDef, fields: &[(String, Ty<'tcx>)]| {
                    (variant.fields.iter().zip(fields)).any(|(def, &(_, t))| {
                        (cx.property_cell(Some(def), t)).map_or(cx.needs_clone(t), |item| cx.needs_clone(item))
                    })
                };
                // Each variant copied whole, with no field to clone: one `{ ...place }`.
                let fields: Vec<_> = adt.variants().iter().map(|v| self.variant_fields(v, args)).collect();
                let variants = || adt.variants().iter().zip(&fields);
                if (itself || variants().any(|(v, f)| cells(self, v, f)))
                    && !fields.is_empty()
                    && variants().all(|(v, f)| !f.is_empty() && !needs(self, v, f))
                {
                    return self.clone_fields(place, (ty, None), fields[0].clone(), span, out);
                }
                let mut value = place.clone();
                for variant in adt.variants().iter().rev() {
                    let fields = self.variant_fields(variant, args);
                    if fields.is_empty() || !(itself || cells(self, variant, &fields) || needs(self, variant, &fields))
                    {
                        continue;
                    }
                    let copy = self.clone_fields(place.clone(), (ty, Some(variant)), fields, span, out)?;
                    let index = adt.variant_index_with_id(variant.def_id);
                    let test = self.variant_test(place.clone(), ty, *adt, index, false);
                    value = Expr::cond(test, copy, value);
                }
                Ok(value)
            }
            // What clones itself: a std hasher, a `$SipHasher`, one of its
            // state (ADR 0463); a string's `chars()` or `char_indices()`, the
            // runtime's `$chars` where it's cloned, one where it is (#1186).
            ty::Adt(..)
                if self.recognition().std_hasher(ty).is_some()
                    || self.recognition().is_str_chars(ty)
                    || self.recognition().is_char_indices(ty) =>
            {
                Ok(Expr::call(Expr::member(place, "clone"), Vec::new()))
            }
            // A `BuildHasherDefault`, which holds nothing, `undefined` (#1155):
            // itself, a `HashMap<K, V, S>`'s `S: Clone` given one (#1420).
            ty::Adt(..) if self.is_std_type(ty, crate::lower::recognition::StdItem::BuildHasherDefault) => Ok(place),
            // And a `RandomState`, which holds nothing JS sees (#1420).
            ty::Adt(..) if self.is_std_type(ty, crate::lower::recognition::StdItem::RandomState) => Ok(place),
            // A C-variadic function's `VaList`, the array of what it hasn't
            // read: a copy, read on its own (ADR 0428).
            ty::Adt(..) if crate::lower::recognition::is_va_list(self.tcx, ty) => {
                Ok(Expr::builtin(place, "slice", Vec::new()))
            }
            // A `Cycle` a struct holds: the one it carries, of clones of what
            // it started with and what it steps (ADR 0544).
            ty::Adt(..) if self.held_cycle(ty).is_some() => Ok(Expr::call_helper("$clone", vec![place])),
            // A `repeat(x)`'s and a `repeat_n(x, n)`'s: the one it carries,
            // std's derived `Clone`, of a clone of its value, a `repeat_n`'s
            // count copied (repeat.rs:71, repeat_n.rs:68, #1139, #1579).
            ty::Adt(..)
                if crate::lower::recognition::is_repeat(self.tcx, ty)
                    || crate::lower::recognition::is_repeat_n(self.tcx, ty) =>
            {
                Ok(Expr::call_helper("$clone", vec![place]))
            }
            // std's iterator over an array is a `$iter` that knows where it is
            // (ADR 0364): a clone, a `$iter` of what it has left, each item a
            // clone of one where the iterator owns them. A `Rev`'s is std's,
            // its compiled `Clone` (ADR 0871).
            // So is one of its items alone, an `Option`'s `into_iter()` and
            // `once(x)`, which owns them: a borrowing one's, `&T` or `Copy`,
            // need no clone.
            ty::Adt(..) if self.steps_an_array(ty) => {
                let owns = self.array_source(ty).unwrap_or(false) || self.recognition().iterates_items(ty);
                let left = self.stepped_left(place, ty, owns, span)?;
                Ok(self.stepped_items_of(left, ty))
            }
            _ if self.cloned_by_shape(ty) => match self.shape(ty) {
                Shape::Object(fields) => self.clone_fields(place, (ty, None), Self::named_fields(fields), span, out),
                // Each part's clone in turn, those made before one that
                // unwinds dropped (`held_in_turn`).
                Shape::Array(tys) => {
                    let parts: Vec<_> = (tys.iter())
                        .map(|&t| (t, self.needs_clone(t) && self.clone_may_panic(t)))
                        .collect();
                    let (items, made) = self.parts_in_turn(&parts, span, |cx, i| {
                        cx.clone_value(Expr::index(place.clone(), Expr::int(i as i128)), tys[i], span, out)
                    })?;
                    Ok(Self::in_turn(Expr::array(items), made))
                }
                Shape::Other => Err(self.uncloned(ty, span)),
            },
            _ => Err(self.uncloned(ty, span)),
        }
    }

    /// The refusal of a clone of `ty`.
    fn uncloned(&self, ty: Ty<'tcx>, span: Span) -> rustc_span::ErrorGuaranteed {
        self.unsupported(span, &format!("cloning `{ty}`"))
    }

    /// Whether a clone of `ty` is its parts', by its shape: of a type of the
    /// crate's, or of one of std's whose fields are what JS has, a C
    /// string's error (ADR 1119).
    fn cloned_by_shape(&self, ty: Ty<'tcx>) -> bool {
        !matches!(ty.kind(), ty::Adt(adt, _) if self.is_std(adt.did()) && !self.is_known_std(ty))
    }

    /// `{ ...place, v: <clone of place.v> }`: a new object, with a clone of
    /// each field that needs one, of `fields`, those of a `ty`, a struct, or
    /// of its `variant`, an enum's (#503). Each field's clone in turn, in
    /// `$madeInTurn` where a later one's may unwind past one made before it
    /// with a destructor, each such given to `made` though it needs no clone
    /// of its own, as a derived `Clone` drops them (`held_in_turn`, #897).
    fn clone_fields(
        &mut self,
        place: Expr,
        (ty, variant): (Ty<'tcx>, Option<&'tcx ty::VariantDef>),
        fields: Vec<(String, Ty<'tcx>)>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A cell in a field is what it holds (`property_cell`): a clone of
        // one is a copy of that, which the spread makes if it needs none.
        let def = |cx: &Self, i: usize| match variant {
            Some(variant) => variant.fields.get(rustc_abi::FieldIdx::from_usize(i)),
            None => cx.struct_field_def(ty, i),
        };
        let needs: Vec<bool> = (fields.iter().enumerate())
            .map(|(i, &(_, t))| {
                self.property_cell(def(self, i), t)
                    .map_or(self.needs_clone(t), |item| self.needs_clone(item))
            })
            .collect();
        let parts: Vec<_> = (fields.iter().zip(&needs))
            .map(|(&(_, t), &needs)| (t, needs && self.clone_may_panic(t)))
            .collect();
        let held = self.held_in_turn(&parts);
        let made = held.contains(&true).then(|| self.fresh(Role::Made));
        let mut props = vec![Prop::Spread(place.clone())];
        for (i, (name, t)) in fields.into_iter().enumerate() {
            if !needs[i] && !held[i] {
                continue;
            }
            let def = def(self, i);
            let read = self.held(Expr::member(place.clone(), name.clone()), def, t);
            let mut field = match needs[i] {
                true => self.clone_value(read, t, span, out)?,
                false => read,
            };
            if let (Some(made), true) = (&made, held[i]) {
                field = self.made_part(made, field, t, span)?;
            }
            props.push(Prop::Field(name, self.holding(field, def, t)));
        }
        Ok(Self::in_turn(Expr::object(props), made))
    }

    /// A clone of an array: `items.slice()`, or `items.map((item) => ..)`
    /// if its items need cloning too, `$madeEach`'s where an item's clone
    /// may panic past those made before it (`made_each`).
    pub(in crate::lower) fn clone_items(&mut self, items: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        if !self.needs_clone(item) {
            // An array made here, which nothing else holds: itself.
            return Ok(crate::lower::iterators::owned_items(items));
        }
        let clone = self.clone_fn("item", item, span)?;
        let may_panic = self.clone_may_panic(item);
        self.made_each(items, clone, (item, may_panic), span)
    }

    /// `items`, each a copy where it needs one, a `Copy` struct something
    /// changes in place: `items.map((item) => ..)`; else `items` itself.
    pub(in crate::lower) fn copies_of(&mut self, items: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        if !self.needs_clone(ty) {
            return Ok(items);
        }
        let clone = self.clone_fn("item", ty, span)?;
        Ok(Expr::builtin(items, "map", vec![clone]))
    }

    /// `items` of `from`s put in a collection of `into`s, as `Extend<&T>`
    /// puts a `&T`: each a copy of what it refers to where that's the
    /// collection's own `T`, `copies_of`; else `items` itself.
    pub(in crate::lower) fn copied_into(
        &mut self,
        items: Expr,
        from: Ty<'tcx>,
        into: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        match *from.kind() {
            ty::Ref(_, item, _) if into == Some(item) => self.copies_of(items, item, span),
            _ => Ok(items),
        }
    }

    /// A clone function of `ty`, for a helper that clones, or `undefined`
    /// where a clone is the value itself.
    pub(in crate::lower) fn clone_arg(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.needs_clone(ty) {
            true => self.clone_fn("value", ty, span),
            false => Ok(Expr::js_undefined()),
        }
    }

    /// `(name) => <clone of name>`.
    pub(in crate::lower) fn clone_fn(&mut self, name: &str, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // Its parameter is a name its body's own may not take, `const value
        // = $someValue(value)` of a nested `Option` (#1246), taken while
        // the body's made, its scope's alone.
        let taken = self.names.insert(name.to_string());
        let mut body = Vec::new();
        let value = self.clone_value(Expr::var(name), ty, span, &mut body);
        if taken {
            self.names.shift_remove(name);
        }
        body.push(StmtKind::Return(Some(value?)).at(js::Span::NONE));
        Ok(Expr::arrow(vec![name.into()], body))
    }
}
