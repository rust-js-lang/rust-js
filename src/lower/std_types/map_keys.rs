//! What a map's or a set's key is to JS (ADR 0059): one JS's own `Map`
//! finds as Rust does, a primitive; one found by its value, a `$KeyMap`'s,
//! and the parts `$key` finds it by; or one a B-tree finds by its `cmp`.

use crate::js::{self, Expr, StmtKind};
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Eligibility for JS Map/Set equality. A string-shaped enum alone is
    /// not enough: user equality or ordering may equate distinct variants.
    pub(in crate::lower) fn is_key(&self, ty: Ty<'tcx>, ordered: bool) -> bool {
        let peeled = ty.peel_refs();
        let primitive = !peeled.is_unit()
            && !Num::of(peeled).is_some_and(Num::float)
            && self.is_primitive_key(peeled)
            // One the crate compares by its address alone is JS's key (ADR 0864).
            && (!self.has_user_impl(self.partial_eq_trait(), peeled) || self.equal_only_to_itself(peeled))
            && (!ordered || !self.has_user_impl(self.ord_trait(), peeled));
        primitive
            || (self.is_value_key(ty) && (!ordered || self.orders_by_value(ty, &mut Vec::new())))
            || self.recognition().is_path(peeled)
            || self.is_cow_key(peeled)
    }

    /// A hash map's or a hash set's key it finds by its own `Hash` and
    /// `Eq`, in a `$hashedMap` or a `$hashedSet` (ADR 1120): one JS can't
    /// find by itself or by its value, its `==` its own, or whose `hash`
    /// runs code a program sees, which std's table calls as it searches.
    /// `()`, one value, is JS's `undefined`.
    pub(in crate::lower) fn found_by_hash(&self, key: Ty<'tcx>) -> bool {
        (!self.is_key(key, false) && !key.peel_refs().is_unit()) || self.hash_runs_code(key, &mut Vec::new())
    }

    /// Whether a map's entry, of type `entry`, of `key`s and of `value`s if
    /// it's a map's, is its one search, `$entry(m, k)`'s, its `or_insert`
    /// and the rest asking nothing again, as std's entry holds where it
    /// is: a B-tree of std's nodes (`tree_of_nodes`, #926), or a hash map
    /// of keys found by their own `Hash` and `Eq` (`found_by_hash`, ADR
    /// 1120), a type parameter's included, whose `hash` std's entry calls
    /// once (#1362). Another's searches ask what nothing sees, and
    /// `$orInsert(m, k, v)` is what JS writes (ADR 0262, no difference).
    pub(in crate::lower) fn entry_searched_once(
        &self,
        entry: Ty<'tcx>,
        key: Ty<'tcx>,
        value: Option<Ty<'tcx>>,
    ) -> bool {
        let entry = entry.peel_refs();
        if self.is_std_type(entry, crate::lower::recognition::StdItem::BTreeEntry) {
            return self.tree_of_nodes(key, value);
        }
        self.is_std_type(entry, crate::lower::recognition::StdItem::HashMapEntry) && self.found_by_hash(key)
    }

    /// A key a B-tree finds as a `$KeyMap` keeps it: one whose `Ord` is
    /// derived, or std's, all through, so two it orders `Equal` are equal
    /// field by field, as `$key` tells them, and its order is `cmp_fn`'s
    /// (ADR 0404).
    pub(in crate::lower) fn orders_by_value(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        match ty.kind() {
            ty::Adt(adt, _) if self.js_eq(ty).is_some() => {
                !self.krate.foreign.compiled_by_rust_js(adt.did()) || self.recognition().derives(self.ord_trait(), ty)
            }
            _ if self.js_eq(ty).is_some() => true,
            ty::Tuple(parts) => parts.iter().all(|t| self.orders_by_value(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.orders_by_value(*item, seen),
            // Every `PhantomData` is `Equal` to every other (#514).
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::PhantomData) => true,
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) || ty.is_box() || self.is_vec_like(ty) => {
                self.orders_by_value(args.type_at(0), seen)
            }
            ty::Adt(adt, args) => {
                self.recognition().derives(self.ord_trait(), ty)
                    && adt
                        .all_fields()
                        .all(|field| self.orders_by_value(self.field_ty(field, args), seen))
            }
            _ => false,
        }
    }

    /// A `Cow` a map finds by what it borrows, or by that itself, `&str` or
    /// `&[T]`, never an object like the `Cow`'s, `{ TAG, _0 }`, so `$CowMap`
    /// tells them apart (ADR 0402): a `Cow<str>`, or a `Cow<[T]>` of what
    /// compares by value.
    pub(in crate::lower) fn is_cow_key(&self, ty: Ty<'tcx>) -> bool {
        self.recognition()
            .cow_parts(ty.peel_refs())
            .is_some_and(|(borrowed, _)| match borrowed.kind() {
                ty::Str => true,
                // Not of what holds a part found by itself, which `$cowKey`
                // would find by what's in it (ADR 1015, amended).
                ty::Slice(item) => self.compares_by_value(*item, &mut Vec::new()) && !self.holds_identity(*item),
                _ => false,
            })
    }

    /// A key a map finds another by that it can be told apart from: a
    /// path, `a//b` finding `a/b` (ADR 0401), or a `Cow`, `Borrowed("a")`
    /// finding `Owned("a")` (ADR 0402), or a B-tree's found by its `cmp`
    /// (ADR 0404), or a hashed one's found by its own `Eq`, `Word("A")`
    /// finding `Word("a")` (ADR 1120). The key it holds given back isn't
    /// the one asked with, and of equal ones a B-tree made of items keeps
    /// the last, where `insert` keeps the first.
    pub(super) fn finds_others(&self, key: Ty<'tcx>, sorted: bool) -> bool {
        self.is_cow_key(key)
            || self.recognition().is_path(key)
            || match sorted {
                true => self.found_by_cmp(key),
                false => self.found_by_hash(key),
            }
    }

    /// A B-tree's key it finds by its `cmp`, in an `$OrdMap` (ADR 0404):
    /// one whose `Ord` isn't the order of its value, so two it orders
    /// `Equal` may differ.
    pub(in crate::lower) fn found_by_cmp(&self, key: Ty<'tcx>) -> bool {
        !self.is_key(key, true)
    }

    /// A key a `$KeyMap` finds by its value (ADR 0121): one that isn't its
    /// own JS key, and that a derived `Eq` compares field by field, as `$eq`
    /// and `$key` do.
    pub(in crate::lower) fn is_value_key(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        !ty.is_unit() && !self.is_primitive_key(ty) && self.compares_by_value(ty, &mut Vec::new())
    }

    /// A key a JS `Map` finds as Rust does: a primitive one (ADR 0059). One
    /// found by value that's one of these needs no `$KeyMap`. Not an
    /// `Option` of one, whose `None` is `undefined` or `null`, each its own
    /// key to JS's `Map`, where Rust has one `None` (ADRs 0030, 0935):
    /// `$KeyMap` finds either.
    pub(in crate::lower) fn is_js_key(&self, ty: Ty<'tcx>) -> bool {
        self.is_primitive_key(ty.peel_refs())
    }

    /// A key a JS `Map` holds as it is, a primitive `js_eq` compares, not
    /// `()`.
    pub(in crate::lower) fn is_primitive_key(&self, ty: Ty<'tcx>) -> bool {
        !ty.is_unit() && self.js_eq(ty).is_some()
    }

    /// Of a key found by its value, whether it holds a part `===` finds by
    /// itself, an object equal only to itself or an `Rc` of one (ADR 1015,
    /// amended), as its derived `Hash` and `Eq` find it, where `$key` would
    /// find it by what's in it: `std_impls`' `eq_of_objects`, its one answer.
    pub(in crate::lower) fn holds_identity(&self, ty: Ty<'tcx>) -> bool {
        self.holds_identity_in(ty, &mut Vec::new())
    }

    fn holds_identity_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        let holds = match ty.kind() {
            _ if self.js_eq(ty).is_some() => self.eq_of_objects(ty),
            ty::Tuple(parts) => parts.iter().any(|t| self.holds_identity_in(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.holds_identity_in(*item, seen),
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) || ty.is_box() || self.is_vec_like(ty) => {
                self.holds_identity_in(args.type_at(0), seen)
            }
            ty::Adt(adt, args) => (adt.variants().iter().flat_map(|variant| variant.fields.iter()))
                .any(|field| self.holds_identity_in(self.field_ty(field, args), seen)),
            _ => false,
        };
        seen.pop();
        holds
    }

    /// What `$key` finds `key`, of `ty`, by, where it holds a part found by
    /// itself (`holds_identity`): each such part its `$id`, the number that
    /// object is known by, and the rest as it is (ADR 1015, amended). A
    /// struct `{ ...key, fiber: $id(key.fiber) }`, a tuple `[$id(key[0]),
    /// key[1]]`, a `Vec` `key.map($id)`, an `Option` its `None` or its
    /// `Some`'s, an enum each variant's that holds one.
    fn key_parts(&mut self, key: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        if !self.holds_identity(ty) {
            return Ok(key);
        }
        let refused = |cx: &Self| {
            cx.unsupported(
                span,
                &format!(
                    "a map or a set keyed by `{ty}`, whose part equal only to itself rust-js can't find by itself there"
                ),
            )
        };
        match ty.kind() {
            // An untagged enum's payload, which may be a number an `$id` is too.
            _ if self.js_eq(ty).is_some() => match self.untagged(ty) {
                Some(_) => Err(refused(self)),
                None => Ok(Expr::call_helper("$id", vec![key])),
            },
            ty::Tuple(parts) => {
                let parts = parts
                    .iter()
                    .enumerate()
                    .map(|(i, t)| self.key_parts(Expr::index(key.clone(), Expr::int(i as i128)), t, span))
                    .collect::<R<Vec<_>>>()?;
                Ok(Expr::array(parts))
            }
            ty::Array(item, _) | ty::Slice(item) => self.items_key_parts(key, *item, span),
            ty::Adt(_, args) if self.is_vec_like(ty) => self.items_key_parts(key, args.type_at(0), span),
            ty::Adt(_, args) if ty.is_box() => self.key_parts(key, args.type_at(0), span),
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) => {
                let inner = args.type_at(0);
                // A `Some` boxed, `$some(..)`, isn't what it holds.
                if self.boxed_payload(inner) {
                    return Err(refused(self));
                }
                // `$id` of `None` is `None`.
                if self.js_eq(inner.peel_refs()).is_some() {
                    return self.key_parts(key, inner, span);
                }
                let some = self.key_parts(key.clone(), inner, span)?;
                Ok(Expr::cond(Expr::presence(key.clone(), false), key, some))
            }
            ty::Adt(adt, args) if adt.is_enum() => {
                let mut value = key.clone();
                for (index, variant) in adt.variants().iter_enumerated().rev() {
                    let fields = self.variant_fields(variant, args);
                    if !fields.iter().any(|&(_, t)| self.holds_identity(t)) {
                        continue;
                    }
                    let mut props = vec![js::Prop::Spread(key.clone())];
                    for (name, t) in fields {
                        if self.holds_identity(t) {
                            let part = self.key_parts(Expr::member(key.clone(), &name), t, span)?;
                            props.push(js::Prop::Field(name, part));
                        }
                    }
                    let test = self.variant_test(key.clone(), ty, *adt, index, false);
                    value = Expr::cond(test, Expr::object(props), value);
                }
                Ok(value)
            }
            _ => match self.shape(ty) {
                crate::lower::Shape::Object(fields) => {
                    let mut props = vec![js::Prop::Spread(key.clone())];
                    for (i, (name, t)) in Self::named_fields(fields).into_iter().enumerate() {
                        if self.holds_identity(t) {
                            let part = self.key_parts(self.project(key.clone(), ty, i), t, span)?;
                            props.push(js::Prop::Field(name, part));
                        }
                    }
                    Ok(Expr::object(props))
                }
                crate::lower::Shape::Array(tys) => {
                    let parts = tys
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| self.key_parts(self.project(key.clone(), ty, i), t, span))
                        .collect::<R<Vec<_>>>()?;
                    Ok(Expr::array(parts))
                }
                crate::lower::Shape::Other => Err(refused(self)),
            },
        }
    }

    /// `items.map((item) => <item's parts>)`, `items.map($id)` of objects
    /// found by themselves: `key_parts` of an array's items.
    fn items_key_parts(&mut self, items: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let each = self.key_parts_fn("item", item, span)?;
        Ok(Expr::call(Expr::member(items, "map"), vec![each]))
    }

    /// `(name) => <name's parts>`, the function of a `ty`'s `key_parts`, or
    /// `$id` itself, of an object found by itself.
    pub(super) fn key_parts_fn(&mut self, name: &str, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let parts = self.key_parts(Expr::var(name), ty, span)?;
        Ok(match &parts.kind {
            js::ExprKind::Call(callee, args)
                if callee.helper_name() == Some("$id")
                    && matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(v) if v == name)) =>
            {
                (**callee).clone()
            }
            _ => Expr::arrow(
                vec![name.into()],
                vec![StmtKind::Return(Some(parts)).at(js::Span::NONE)],
            ),
        })
    }

    pub(in crate::lower) fn compares_by_value(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) {
            return true;
        }
        seen.push(ty);
        match ty.kind() {
            _ if let Some(eq) = self.js_eq(ty) => eq == crate::lower::std_impls::JsEq::SameValue,
            ty::Tuple(parts) => parts.iter().all(|t| self.compares_by_value(t, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.compares_by_value(*item, seen),
            // A `PhantomData` is equal to every other, as its `undefined`,
            // a field left out of its object, is to `$key` (#514).
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::PhantomData) => true,
            ty::Adt(_, args) if self.is_lang_adt(ty, LangItem::Option) || ty.is_box() || self.is_vec_like(ty) => {
                self.compares_by_value(args.type_at(0), seen)
            }
            // Of std's, one whose `PartialEq` is derived, a path's `Component`,
            // or a C string's, its bytes (#1157), of what its values are made of.
            ty::Adt(adt, args) => {
                (self.krate.foreign.compiled_by_rust_js(adt.did())
                    || self.std_enum_derives_eq(ty)
                    || crate::lower::recognition::is_c_string_like(self.tcx, ty))
                    && self.recognition().derives(self.partial_eq_trait(), ty)
                    && adt
                        .variants()
                        .iter()
                        .filter(|variant| !crate::lower::recognition::never_made(self.tcx, variant))
                        .flat_map(|variant| variant.fields.iter())
                        .all(|field| self.compares_by_value(self.field_ty(field, args), seen))
            }
            _ => false,
        }
    }
}
