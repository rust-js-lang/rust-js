//! `Clone`, `Default` and `PartialEq` where rust-js writes the
//! implementation: derived ones and std types' (ADRs 0052, 0053). A
//! hand-written one is called instead.

use super::recognition::{StdItem, std_item};
use super::representation::Num;
use super::{FnCx, R, Shape, is_fieldless_enum, lower_first};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind, UnaryOp};
use crate::runtime::Helper;
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};

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
            // A range is its bounds (ADR 0129), changed in place only if
            // `contains_mutated` says so.
            ty::Adt(_, args) if self.range_kind(ty).is_some() => args
                .types()
                .next()
                .is_some_and(|index| self.needs_clone_in(index, seen)),
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
        // clone can be the same function only if nothing could tell, that
        // is, if it never changes what it holds, and holds no `Cell`.
        if let ty::Closure(_, args) = ty.kind() {
            let closure = args.as_closure();
            let shared = closure.kind() == ty::ClosureKind::Fn
                && closure
                    .upvar_tys()
                    .iter()
                    .all(|t| t.is_freeze(self.tcx, self.typing_env));
            return if shared {
                Ok(place)
            } else {
                Err(self.unsupported(span, "cloning a closure that changes what it captured"))
            };
        }
        // A channel's sender: one more, of the same queue (ADR 0142).
        if self.recognition().channel_end(ty) == Some(super::recognition::ChannelEnd::Sender) {
            self.runtime.insert(Helper::Channel);
            return Ok(Expr::call(Expr::var("$cloneSender"), vec![place]));
        }
        // A counted `Rc`, or a `Weak`: one more, of the same value (ADR 0320).
        if self.counted_rc(ty).is_some() || self.weak_of(ty).is_some() {
            self.runtime.insert(Helper::Rc);
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
            ty::Array(item, _) => return self.clone_items(place, *item, span),
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
                self.runtime.insert(Helper::Borrow);
                let value = Expr::call(
                    Expr::var("$withBorrow"),
                    vec![place, Expr::arrow(vec!["value".into()], body)],
                );
                return Ok(Expr::object(vec![Prop::Field("value".into(), value)]));
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let cell = match std(StdItem::RefCell) {
                    true => {
                        self.runtime.insert(Helper::Borrow);
                        Expr::call(Expr::var("$borrow"), vec![place])
                    }
                    false => place,
                };
                let value = self.clone_value(Expr::member(cell, "value"), args.type_at(0), span, out)?;
                return Ok(Expr::object(vec![Prop::Field("value".into(), value)]));
            }
            // A `OnceCell` holds an `Option` (ADR 0317).
            ty::Adt(_, args) if std(StdItem::OnceCell) => {
                let option = Ty::new_option(self.tcx, args.type_at(0));
                let value = self.clone_value(Expr::member(place, "value"), option, span, out)?;
                return Ok(Expr::object(vec![Prop::Field("value".into(), value)]));
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
                let class = self.map_class(set, key);
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
                    let test = Expr::bin(
                        Op::Eq,
                        Expr::member(place.clone(), super::bindings::tag_key(self.tcx, adt.did())),
                        super::bindings::variant_tag(self.tcx, variant),
                    );
                    value = Expr::cond(test, copy, value);
                }
                Ok(value)
            }
            // std's iterator over an array is the array (ADR 0061), which iterating
            // doesn't change: a clone of one that borrows its items is the array
            // itself, of one that owns them, a copy of each that needs one (ADR 0181).
            ty::Adt(_, args) if let Some(owns) = self.array_source(ty) => {
                let item = args.types().next().expect("an iterator's item");
                match owns && self.needs_clone(item) {
                    true => self.clone_items(place, item, span),
                    false => Ok(place),
                }
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
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new(self.tcx, default, [ty]);
            return match self.evidence_for(tr) {
                Some(dictionary) => Ok(Expr::call(Expr::member(dictionary, "default"), Vec::new())),
                None => Err(self.no_evidence(span, tr)),
            };
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
                let class = self.map_class(self.is_set(ty), args.types().next());
                Expr::new_(class, Vec::new())
            }
            ty::Adt(_, args) if self.counted_rc(ty).is_some() => {
                Self::new_rc(self.default_value(args.type_at(0), span)?)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => self.default_value(args.type_at(0), span)?,
            ty::Adt(..) if std(StdItem::OnceCell) => Expr::object(vec![Prop::Field("value".into(), Expr::undefined())]),
            // `LazyCell::new(T::default)` (ADR 0318).
            ty::Adt(_, args) if std(StdItem::LazyCell) => {
                let made = self.default_value(args.type_at(0), span)?;
                let init = Expr::arrow(vec![], vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]);
                Expr::object(vec![Prop::Field("init".into(), init)])
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) => Expr::object(
                vec![Prop::Field("value".into(), self.default_value(args.type_at(0), span)?)],
            ),
            ty::Adt(adt, _) if adt.is_enum() && !self.is_std(adt.did()) => {
                let variant = self
                    .default_variant(default, ty)
                    .ok_or_else(|| self.unsupported(span, &format!("`Default` of `{ty}`")))?;
                super::bindings::unit_variant(self.tcx, adt.did(), adt.variant_with_ctor_id(variant))
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

    /// Is `==` on `ty` JS's `===`: strings, numbers, `bool`s, `()` and
    /// fieldless enums, all JS primitives?
    fn is_primitive_eq(&self, ty: Ty<'tcx>) -> bool {
        // A `&mut` to a number is a cell, an object (ADR 0099).
        if self.has_cell_layer(ty) {
            return false;
        }
        let ty = ty.peel_refs();
        // A parse error is its message, which tells its kind, as its derived
        // `==` compares (ADR 0063).
        self.is_string_like(ty)
            || self.is_parse_error(ty)
            // A `TypeId` is its type's key (ADR 0331).
            || self.recognition().is_type_id(ty)
            || Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _) if is_fieldless_enum(*adt))
    }

    /// Does `==` on `ty` run code of its own anywhere in it: a hand-written
    /// `eq`, or a `T`'s, which might be one? If not, it compares field by
    /// field, element by element, which `$eq` does.
    pub(super) fn custom_eq(&self, ty: Ty<'tcx>) -> bool {
        self.custom_eq_in(ty, &mut Vec::new())
    }

    fn custom_eq_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        let ty = ty.peel_refs();
        if seen.contains(&ty) || self.is_primitive_eq(ty) {
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
            // `Vec`, `Box`, `Rc` and cells compare what they hold.
            ty::Adt(_, args) if self.is_std_wrapper(ty) => args.types().any(|t| self.custom_eq_in(t, seen)),
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
    pub(super) fn eq_value(&mut self, a: Expr, b: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (mut a, _) = self.through_refs(a, ty);
        let (mut b, mut ty) = self.through_refs(b, ty);
        // A `Box`, an `Rc` and a `Pin` compare what they point at, which
        // they are in JS (ADRs 0023, 0329), its own `==` too.
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
        if self.is_primitive_eq(ty) {
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
        // A constant, like a fieldless variant's `"Nothing"`, is a JS
        // primitive: only itself is equal to it. `None` is also `null`.
        if a.is_constant() || b.is_constant() {
            let none = [&a, &b].iter().any(|x| matches!(x.kind, js::ExprKind::Undefined));
            let op = if none { Op::LooseEq } else { Op::Eq };
            return Ok(Expr::bin(op, a, b));
        }
        if let Some(inner) = self.option_of(ty) {
            // `==`, so that `null` and `undefined` are both `None`.
            if self.is_primitive_eq(inner) {
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
        let structural = matches!(ty.kind(), ty::Tuple(_) | ty::Array(..) | ty::Slice(_))
            || self.is_std_wrapper(ty)
            || matches!(ty.kind(), ty::Adt(adt, _) if !self.is_std(adt.did()) || self.is_known_std(ty));
        if !structural || self.is_js_object(ty) || self.is_lang_adt(ty, LangItem::String) {
            return Err(self.unsupported(span, &format!("`==` on `{ty}`")));
        }
        if !self.custom_eq(ty) {
            self.runtime.insert(Helper::Eq);
            return Ok(Expr::call(Expr::var("$eq"), vec![a, b]));
        }
        // Each is read more than once below.
        let a = if a.reads_same() { a } else { self.spill("left", a, out) };
        let b = if b.reads_same() { b } else { self.spill("right", b, out) };
        let std = |item: StdItem| self.is_std_type(ty, item);
        match ty.kind() {
            ty::Array(item, _) | ty::Slice(item) => self.eq_items(a, b, *item, span),
            ty::Adt(_, args) if self.is_vec_like(ty) => self.eq_items(a, b, args.type_at(0), span),
            ty::Adt(_, args) if self.counted_rc(ty).is_some() => self.eq_value(
                Expr::member(a, "value"),
                Expr::member(b, "value"),
                args.type_at(0),
                span,
                out,
            ),
            // Two `RefCell`s', each while it's borrowed (ADR 0328), held where
            // their values' `==` may ask.
            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {
                let mut body = Vec::new();
                let eq = self.eq_value(Expr::var("x"), Expr::var("y"), args.type_at(0), span, &mut body)?;
                body.push(StmtKind::Return(Some(eq)).at(js::Span::NONE));
                self.runtime.insert(Helper::Borrow);
                let inner = Expr::call(Expr::var("$withBorrow"), vec![b, Expr::arrow(vec!["y".into()], body)]);
                Ok(Expr::call(
                    Expr::var("$withBorrow"),
                    vec![
                        a,
                        Expr::arrow(vec!["x".into()], vec![StmtKind::Return(Some(inner)).at(js::Span::NONE)]),
                    ],
                ))
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let (a, b) = match std(StdItem::RefCell) {
                    true => {
                        self.runtime.insert(Helper::Borrow);
                        (
                            Expr::call(Expr::var("$borrow"), vec![a]),
                            Expr::call(Expr::var("$borrow"), vec![b]),
                        )
                    }
                    false => (a, b),
                };
                self.eq_value(
                    Expr::member(a, "value"),
                    Expr::member(b, "value"),
                    args.type_at(0),
                    span,
                    out,
                )
            }
            ty::Adt(_, args) if std(StdItem::OnceCell) => self.eq_value(
                Expr::member(a, "value"),
                Expr::member(b, "value"),
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
                self.runtime.insert(Helper::Eq);
                let mut value = Expr::call(Expr::var("$eq"), vec![a.clone(), b.clone()]);
                for variant in adt.variants().iter().rev() {
                    let fields = self.variant_fields(variant, args);
                    if !fields.iter().any(|&(_, t)| self.custom_eq(t)) {
                        continue;
                    }
                    let name = super::bindings::variant_tag(self.tcx, variant);
                    let key = super::bindings::tag_key(self.tcx, adt.did());
                    let tag = |x: &Expr| Expr::bin(Op::Eq, Expr::member(x.clone(), key.clone()), name.clone());
                    let mut same = tag(&b);
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
                    value = Expr::cond(tag(&a), same, value);
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

    /// `a.length === b.length && a.every((x, i) => <x == b[i]>)`.
    fn eq_items(&mut self, a: Expr, b: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let other = Expr::index(b.clone(), Expr::var("i"));
        let same = self.eq_value(Expr::var("x"), other, item, span, &mut body)?;
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
            && let js::ExprKind::Var(name) = &callee.kind
            && name == "$eq"
        {
            return Ok(Expr::var("$eq"));
        }
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
    }
}

/// `!(a == b)`, as JS writes it: `a !== b` for `a === b`.
pub(super) fn negate(eq: Expr) -> Expr {
    match eq.kind {
        js::ExprKind::Binary(Op::Eq, a, b) => Expr::bin(Op::Ne, *a, *b),
        js::ExprKind::Binary(Op::LooseEq, a, b) => Expr::bin(Op::LooseNe, *a, *b),
        js::ExprKind::Binary(Op::Ne, a, b) => Expr::bin(Op::Eq, *a, *b),
        js::ExprKind::Binary(Op::LooseNe, a, b) => Expr::bin(Op::LooseEq, *a, *b),
        _ => Expr::unary(UnaryOp::Not, eq),
    }
}
