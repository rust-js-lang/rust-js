//! Untagged enums (ADR 0214): an enum marked `#[rust_js::untagged]` is its
//! payload, as TS's `string | Blob` is, and a variant is told apart by its
//! payload's runtime kind, `typeof`, `Array.isArray`, `instanceof`, or the
//! function its type says tells one, `isValidElement`.

use super::FnCx;
use super::bindings;
use super::recognition::{Recognition, StdItem, conversion_target, serde_impl, std_item};
use super::representation::{Num, is_fieldless_enum};
use crate::js::{Expr, Op, UnaryOp};
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_middle::ty::{self, AdtDef, GenericArgsRef, Ty, TyCtxt, VariantDef};
use rustc_span::def_id::DefId;
use rustc_span::sym;

/// What JS can tell a value is.
#[derive(Clone, PartialEq)]
pub(super) enum Kind {
    String,
    Number,
    BigInt,
    Boolean,
    Array,
    Function,
    Object,
    /// A class, `instanceof`: its JS name, and the Rust type that's it, whose
    /// `Deref` names the class it extends.
    Class(String, Option<DefId>),
    /// What a function says is one, `isValidElement(v)`: its JS path, a
    /// JS object type's `#[rust_js::test]`. An object no other kind is.
    Test(String),
    /// A variant without fields, its name's string, `"blocking"`.
    Literal(String),
}

impl Kind {
    /// Whether a value of one could be one of `other`'s too: a literal
    /// is a string, and the same literal.
    fn overlaps(&self, other: &Kind) -> bool {
        match (self, other) {
            (Kind::Literal(_), Kind::String) | (Kind::String, Kind::Literal(_)) => true,
            (a, b) => a == b,
        }
    }

    /// What `typeof` says of it, for the kinds it tells.
    fn type_of(&self) -> Option<&'static str> {
        match self {
            Kind::String => Some("string"),
            Kind::Number => Some("number"),
            Kind::BigInt => Some("bigint"),
            Kind::Boolean => Some("boolean"),
            Kind::Function => Some("function"),
            Kind::Object => Some("object"),
            Kind::Literal(_) => Some("string"),
            Kind::Array | Kind::Class(..) | Kind::Test(_) => None,
        }
    }

    /// What it is in an error: TS's name for it.
    fn describe(&self) -> String {
        match self {
            Kind::Array => "an array".into(),
            Kind::Class(name, _) => format!("a `{name}`"),
            Kind::Test(test) => format!("what `{test}` says is one"),
            Kind::Object => "an object".into(),
            Kind::Literal(name) => format!("the string {name:?}"),
            _ => format!("a `{}`", self.type_of().expect("a `typeof` kind")),
        }
    }
}

/// The functions `variant`'s test calls, of its enum's payloads' types'
/// `#[rust_js::test]`s: its own; or, of what's left out, each, as the
/// `otherwise` variant's and a plain object's tests leave them out.
pub(super) fn tests_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt: AdtDef<'tcx>,
    args: GenericArgsRef<'tcx>,
    variant: &VariantDef,
) -> Vec<String> {
    if !adt.is_enum() || !bindings::is_untagged(tcx, adt.did()) {
        return Vec::new();
    }
    let payload = |v: &VariantDef| {
        v.fields
            .iter()
            .next()
            .map(|f| f.ty(tcx, args).skip_normalization().peel_refs())
    };
    let test = |v: &VariantDef| match payload(v)?.kind() {
        ty::Adt(payload, _) => bindings::test_of(tcx, payload.did()),
        _ => None,
    };
    if let Some(own) = test(variant) {
        return vec![own];
    }
    // A struct of fields is an object, whose test leaves out a tested one's.
    let object = payload(variant).is_some_and(|ty| {
        matches!(ty.kind(), ty::Adt(s, own) if s.is_struct() && s.non_enum_variant().ctor_kind().is_none()
            && !s.non_enum_variant().fields.iter().next().is_some_and(|f| f.ty(tcx, own).skip_normalization().is_phantom_data()))
    });
    if bindings::is_otherwise(tcx, variant.def_id) || object {
        return adt.variants().iter().filter_map(test).collect();
    }
    Vec::new()
}

/// Whether `ty` is an untagged enum, `#[rust_js::untagged]`.
pub(super) fn untagged_adt<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(AdtDef<'tcx>, GenericArgsRef<'tcx>)> {
    match *ty.kind() {
        ty::Adt(adt, args) if adt.is_enum() && bindings::is_untagged(tcx, adt.did()) => Some((adt, args)),
        _ => None,
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// What JS can tell a payload of type `ty` is, if anything: not an
    /// `Option`'s `undefined`, a `()`, a generic `T` or an enum.
    pub(super) fn untagged_kind(&self, ty: Ty<'tcx>) -> Option<Kind> {
        let ty = ty.peel_refs();
        let ty = ty.boxed_ty().unwrap_or(ty);
        if self.is_string_like(ty) || ty.is_char() {
            return Some(Kind::String);
        }
        if let Some(num) = Num::of(ty) {
            return Some(if num.big() { Kind::BigInt } else { Kind::Number });
        }
        if ty.is_bool() {
            return Some(Kind::Boolean);
        }
        if self.is_map(ty) {
            return Some(Kind::Class("Map".into(), None));
        }
        if self.is_set(ty) {
            return Some(Kind::Class("Set".into(), None));
        }
        if ty.is_array()
            || ty.is_slice()
            || self.is_vec_like(ty)
            || matches!(ty.kind(), ty::Tuple(items) if !items.is_empty())
        {
            return Some(Kind::Array);
        }
        if let ty::Adt(adt, _) = ty.kind()
            && self.is_js_object(ty)
            && let Some(test) = bindings::test_of(self.tcx, adt.did())
        {
            return Some(Kind::Test(test));
        }
        // One TypeScript types as an object literal, `Dict`'s `{ [key: string]:
        // T }`, is a plain object, of no class to test.
        if let ty::Adt(adt, _) = ty.kind()
            && self.is_js_object(ty)
            && super::declarations::written_types(self.tcx, adt.did()).is_some_and(|t| t.trim_start().starts_with('{'))
        {
            return Some(Kind::Object);
        }
        if let ty::Adt(adt, _) = ty.kind()
            && self.is_js_object(ty)
        {
            return Some(Kind::Class(bindings::class_name(self.tcx, adt.did()), Some(adt.did())));
        }
        let fn_trait = |id: DefId| self.tcx.fn_trait_kind_from_def_id(id).is_some();
        match ty.kind() {
            ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) => Some(Kind::Function),
            ty::Dynamic(predicates, ..) if predicates.principal_def_id().is_some_and(fn_trait) => Some(Kind::Function),
            // A struct of fields is an object, and of a tuple's an array (ADR 0033).
            ty::Adt(adt, _) if adt.is_struct() && !self.is_std(adt.did()) => match adt.non_enum_variant().ctor_kind() {
                None => Some(Kind::Object),
                Some(CtorKind::Fn) if !adt.non_enum_variant().fields.is_empty() => Some(Kind::Array),
                Some(_) => None,
            },
            // A fieldless enum is its variant's name (ADR 0013), a string.
            ty::Adt(adt, _) if is_fieldless_enum(*adt) && !adt.variants().is_empty() && !self.is_std(adt.did()) => {
                Some(Kind::String)
            }
            _ => None,
        }
    }

    /// What JS can tell `variant` is: its payload's kind, or of one without
    /// fields, its name's string.
    pub(super) fn untagged_variant_kind(&self, variant: &VariantDef, args: GenericArgsRef<'tcx>) -> Option<Kind> {
        if variant.fields.is_empty() && variant.ctor_kind() == Some(CtorKind::Const) {
            return Some(Kind::Literal(bindings::variant_name(self.tcx, variant)));
        }
        self.untagged_kind(variant.fields.iter().next()?.ty(self.tcx, args).skip_normalization())
    }

    /// A `From` into an untagged enum, or `into()` to one: the trait's method,
    /// or the crate's `from` a call of it resolves to. Each is the value
    /// itself, as its `from` is the variant of its argument, which is checked
    /// where it's declared.
    pub(super) fn converts_to_untagged(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
        conversion_target(self.tcx, def_id, args).is_some_and(|target| untagged_adt(self.tcx, target).is_some())
    }

    /// The classes the JS object type `def_id` extends, nearest first: what
    /// its `Deref` goes to, and on, as the webapi crate's types say theirs.
    fn superclasses(&self, def_id: DefId) -> Vec<DefId> {
        let tcx = self.tcx;
        let Some(deref) = tcx.lang_items().get(LangItem::Deref) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        let mut at = def_id;
        while let Some(parent) = tcx.all_impls(deref).find_map(|imp| {
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            let ty::Adt(adt, _) = self_ty.kind() else { return None };
            if adt.did() != at {
                return None;
            }
            let target = tcx
                .associated_items(imp)
                .filter_by_name_unhygienic(sym::Target)
                .next()?;
            match tcx
                .type_of(target.def_id)
                .instantiate_identity()
                .skip_normalization()
                .kind()
            {
                ty::Adt(parent, _) => Some(parent.did()),
                _ => None,
            }
        }) {
            if found.contains(&parent) {
                break;
            }
            found.push(parent);
            at = parent;
        }
        found
    }

    /// `variant`'s test of `value`: its payload's kind, leaving out
    /// what another variant's more exactly is, so each test holds of its
    /// own values only, whatever order they're tried in. A class's leaves out
    /// its subclasses in the enum; an object's every array and class.
    pub(super) fn untagged_test(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variant: &VariantDef,
        value: &Expr,
        class: &dyn Fn(&str) -> Expr,
    ) -> Expr {
        let otherwise = |v: &VariantDef| bindings::is_otherwise(self.tcx, v.def_id);
        // What the others aren't: none of their tests.
        if otherwise(variant) {
            let none = (adt.variants().iter().filter(|v| !otherwise(v)))
                .map(|v| Expr::unary(UnaryOp::Not, self.untagged_test(adt, args, v, value, class)));
            return none
                .reduce(|all, test| Expr::bin(Op::And, all, test))
                .unwrap_or(Expr::bool(true));
        }
        let kinds: Vec<Kind> = adt
            .variants()
            .iter()
            .filter(|v| !otherwise(v))
            .filter_map(|v| self.untagged_variant_kind(v, args))
            .collect();
        let own = (self.untagged_variant_kind(variant, args))
            .expect("an untagged enum's variant has a kind, as its declaration was checked");
        let test = |kind: &Kind| match kind {
            Kind::Array => Expr::call(Expr::member(Expr::var("Array"), "isArray"), vec![value.clone()]),
            Kind::Class(name, _) => Expr::bin(Op::InstanceOf, value.clone(), class(name)),
            Kind::Test(test) => Expr::call(class(test), vec![value.clone()]),
            Kind::Literal(name) => Expr::bin(Op::Eq, value.clone(), Expr::str(name)),
            other => Expr::bin(
                Op::Eq,
                Expr::unary(UnaryOp::Typeof, value.clone()),
                Expr::str(other.type_of().expect("a `typeof` kind")),
            ),
        };
        let narrower: Vec<&Kind> = kinds
            .iter()
            .filter(|kind| match (&own, kind) {
                (Kind::Object, Kind::Array | Kind::Class(..) | Kind::Test(_)) => true,
                (Kind::Class(_, Some(base)), Kind::Class(_, Some(sub))) => self.superclasses(*sub).contains(base),
                _ => false,
            })
            .collect();
        // A subclass of what's left out already is too: `!(v instanceof
        // Error)` leaves out each `TypeError`.
        let extends = |kind: &Kind, base: &Kind| match (kind, base) {
            (Kind::Class(_, Some(sub)), Kind::Class(_, Some(base))) => self.superclasses(*sub).contains(base),
            _ => false,
        };
        let narrower: Vec<&Kind> = narrower
            .iter()
            .filter(|kind| !narrower.iter().any(|base| extends(kind, base)))
            .copied()
            .collect();
        narrower.into_iter().fold(test(&own), |all, kind| {
            Expr::bin(Op::And, all, Expr::unary(UnaryOp::Not, test(kind)))
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `ty` is an untagged enum, `#[rust_js::untagged]` (ADR 0214).
    pub(super) fn untagged(&self, ty: Ty<'tcx>) -> Option<(AdtDef<'tcx>, GenericArgsRef<'tcx>)> {
        untagged_adt(self.tcx, ty)
    }

    /// `variant` of the untagged enum `ty`'s test of `value`.
    pub(super) fn untagged_variant_test(&self, ty: Ty<'tcx>, variant: &VariantDef, value: &Expr) -> Expr {
        let (adt, args) = self.untagged(ty).expect("an untagged enum");
        self.recognition()
            .untagged_test(adt, args, variant, value, &|name| self.js_ref(name))
    }
}

/// Does something but a `match` tell `def_id`'s variants apart: its
/// `Clone`, `PartialEq` or `Debug`, or its drop (ADR 0214)?
fn tested_by_impls<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, recognition: &Recognition<'_, 'tcx>) -> bool {
    let traits = [
        tcx.lang_items().clone_trait(),
        tcx.lang_items().eq_trait(),
        Some(std_item(tcx, StdItem::Debug)),
    ];
    let implemented = traits.into_iter().flatten().any(|tr| {
        tcx.all_impls(tr).any(|imp| {
            matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if adt.did() == def_id)
        })
    });
    let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
    implemented || ty.needs_drop(tcx, recognition.typing_env)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Another variant of the untagged enum `ty` whose value is of
    /// `variant`'s kind, which a test can't tell from it (ADR 0214).
    pub(super) fn untagged_alike(&self, ty: Ty<'tcx>, variant: &VariantDef) -> Option<String> {
        let (adt, args) = self.untagged(ty)?;
        let recognition = self.recognition();
        let kind_of = |v: &VariantDef| {
            (!bindings::is_otherwise(self.tcx, v.def_id))
                .then(|| recognition.untagged_variant_kind(v, args))
                .flatten()
        };
        let own = kind_of(variant)?;
        (adt.variants().iter())
            .find(|other| other.def_id != variant.def_id && kind_of(other).is_some_and(|k| k.overlaps(&own)))
            .map(|other| other.name.to_string())
    }
}

/// Report each untagged enum JS couldn't tell the variants of apart, and
/// each `From` into one that isn't its variant of its argument. False if
/// there was one.
pub(super) fn validate<'tcx>(tcx: TyCtxt<'tcx>, foreign: &super::library::Foreign<'_, 'tcx>) -> bool {
    let mut valid = true;
    let mut error = |span, message: String| {
        tcx.dcx().span_err(span, message);
        valid = false;
    };
    for def_id in tcx.hir_crate_items(()).definitions() {
        if tcx.def_kind(def_id) == DefKind::Enum && bindings::is_untagged(tcx, def_id.to_def_id()) {
            let recognition = Recognition {
                tcx,
                typing_env: ty::TypingEnv::non_body_analysis(tcx, def_id),
                trait_impls: &[],
                foreign,
            };
            let adt = tcx.adt_def(def_id);
            let args = ty::GenericArgs::identity_for_item(tcx, def_id);
            let mut seen: Vec<(Kind, String)> = Vec::new();
            let last = adt.variants().len() - 1;
            for (i, variant) in adt.variants().iter().enumerate() {
                let span = tcx.def_span(variant.def_id);
                // One without fields is its name's string.
                if variant.fields.is_empty() && variant.ctor_kind() == Some(CtorKind::Const) {
                    let kind = Kind::Literal(bindings::variant_name(tcx, variant));
                    if let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))
                        && tested_by_impls(tcx, def_id.to_def_id(), &recognition)
                    {
                        error(
                            span,
                            format!(
                                "`{}` and `{other}` both hold {}, which JS can't tell apart in an untagged enum (ADR 0214)",
                                variant.name,
                                kind.describe()
                            ),
                        );
                    }
                    seen.push((kind, variant.name.to_string()));
                    continue;
                }
                let ([field], Some(CtorKind::Fn)) = (&variant.fields.raw[..], variant.ctor_kind()) else {
                    error(
                        span,
                        format!(
                            "an untagged enum's variant holds one value, as `{}(&'a str)` does, or none, its name's string (ADR 0214)",
                            variant.name
                        ),
                    );
                    continue;
                };
                // What the others aren't holds anything, and comes last.
                if bindings::is_otherwise(tcx, variant.def_id) {
                    if i != last {
                        error(
                            span,
                            format!(
                                "an untagged enum's `#[rust_js::otherwise]` variant, `{}`, is its last (ADR 0214)",
                                variant.name
                            ),
                        );
                    }
                    continue;
                }
                let ty = field.ty(tcx, args).skip_normalization();
                let Some(kind) = recognition.untagged_kind(ty) else {
                    error(
                        span,
                        format!(
                            "JS can't tell a `{ty}` from another variant's value: an untagged enum's variant holds a string, a number, a `bool`, an array, a JS object, a function or a struct (ADR 0214)"
                        ),
                    );
                    continue;
                };
                // Two variants of one kind are told apart nowhere but where
                // one is tested: an enum only made, `getStaticProps`'s `{ props }`
                // or `{ notFound }`, is its payloads. An impl or a drop that
                // tests them is refused here.
                if let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))
                    && tested_by_impls(tcx, def_id.to_def_id(), &recognition)
                {
                    error(
                        span,
                        format!(
                            "`{}` and `{other}` both hold {}, which JS can't tell apart in an untagged enum (ADR 0214)",
                            variant.name,
                            kind.describe()
                        ),
                    );
                }
                seen.push((kind, variant.name.to_string()));
            }
        }
        if let Some(message) = untagged_from_misuse(tcx, def_id.to_def_id()) {
            error(tcx.def_span(def_id), message);
        }
        // serde's codecs read and write a tagged enum (ADR 0079): one without
        // a tag is serde's own `#[serde(untagged)]`, which is to come.
        if serde_impl(tcx, def_id.to_def_id()).is_some()
            && let Some((adt, _)) = untagged_adt(tcx, tcx.type_of(def_id).instantiate_identity().skip_normalization())
        {
            error(
                tcx.def_span(def_id),
                format!(
                    "rust-js does not support serde of the untagged enum `{}` yet (ADR 0214)",
                    tcx.item_name(adt.did())
                ),
            );
        }
    }
    valid
}

/// A `From` into an untagged enum is the value itself where it's called
/// (ADR 0214), so its `from` must be the variant of its argument,
/// `Src::Text(s)`: what else it did, a call that's the value would skip.
fn untagged_from_misuse(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    if tcx.def_kind(def_id) != DefKind::AssocFn || tcx.trait_impl_of_assoc(def_id).is_none() {
        return None;
    }
    let target = conversion_target(tcx, def_id, ty::GenericArgs::identity_for_item(tcx, def_id))?;
    let (adt, _) = untagged_adt(tcx, target)?;
    let local = def_id.as_local()?;
    let body = tcx.hir_body_owned_by(local);
    let param = body.params.first()?;
    let mut value = body.value;
    while let hir::ExprKind::Block(block, _) = value.kind
        && block.stmts.is_empty()
        && let Some(inner) = block.expr
    {
        value = inner;
    }
    let typeck = tcx.typeck(local);
    let variant_of_param = match value.kind {
        hir::ExprKind::Call(callee, [arg]) => {
            let ctor = match callee.kind {
                hir::ExprKind::Path(ref qpath) => typeck.qpath_res(qpath, callee.hir_id),
                _ => Res::Err,
            };
            let of_adt = matches!(ctor, Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), id)
                if tcx.parent(tcx.parent(id)) == adt.did());
            let of_param = matches!(arg.kind, hir::ExprKind::Path(hir::QPath::Resolved(None, path))
                if path.res == Res::Local(param.pat.hir_id));
            of_adt && of_param
        }
        _ => false,
    };
    (!variant_of_param).then(|| {
        let name = tcx.item_name(adt.did());
        format!(
            "a `From` into the untagged enum `{name}` is its variant of its argument, `{name}::Variant(value)`, as rust-js makes it the value itself (ADR 0214)"
        )
    })
}
