//! Untagged enums (ADR 0214): an enum marked `#[rust_js::untagged]` is its
//! payload, as TS's `string | Blob` is, and a variant is told apart by its
//! payload's runtime kind, `typeof`, `Array.isArray`, `instanceof`, or the
//! function its type says tells one, `isValidElement`.

use super::FnCx;
use super::bindings;
use super::recognition::{Recognition, StdItem, conversion_target, deref_target, serde_impl, std_item};
use super::representation::{Num, is_string_enum};
use crate::js::{Expr, Op, UnaryOp};
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_middle::ty::{self, AdtDef, GenericArgsRef, Ty, TyCtxt, VariantDef};
use rustc_span::def_id::DefId;

/// What JS can tell a value is.
#[derive(Clone, PartialEq)]
pub(super) enum Kind {
    String,
    Number,
    BigInt,
    Boolean,
    Symbol,
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
    /// A variant without fields that's `undefined`, `#[rust_js::undefined]`,
    /// or `null`, `#[rust_js::null]` (ADR 0676).
    Undefined,
    Null,
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

    /// What `typeof` says of it, for the kinds it tells: not `null`, which
    /// it says is an object, as `=== null` tells it.
    fn type_of(&self) -> Option<&'static str> {
        match self {
            Kind::String => Some("string"),
            Kind::Number => Some("number"),
            Kind::BigInt => Some("bigint"),
            Kind::Boolean => Some("boolean"),
            Kind::Symbol => Some("symbol"),
            Kind::Function => Some("function"),
            Kind::Object => Some("object"),
            Kind::Literal(_) => Some("string"),
            Kind::Undefined => Some("undefined"),
            Kind::Array | Kind::Class(..) | Kind::Test(_) | Kind::Null => None,
        }
    }

    /// The kind of a JS type TypeScript names as `typeof` does one, a
    /// primitive's, `symbol`.
    fn of_types(types: &str) -> Option<Kind> {
        TYPE_OF
            .into_iter()
            .find(|kind| kind.type_of() == Some(types) && *kind != Kind::Object)
    }

    /// What it is in an error: TS's name for it.
    fn describe(&self) -> String {
        match self {
            Kind::Array => "an array".into(),
            Kind::Class(name, _) => format!("a `{name}`"),
            Kind::Test(test) => format!("what `{test}` says is one"),
            Kind::Object => "an object".into(),
            Kind::Literal(name) => format!("the string {name:?}"),
            Kind::Null => "`null`".into(),
            Kind::Undefined => "`undefined`".into(),
            _ => format!("a `{}`", self.type_of().expect("a `typeof` kind")),
        }
    }
}

/// The kinds `typeof` tells of a value that's neither `undefined` nor
/// `null`, as an untagged enum's is but where a variant is one of them
/// (ADR 0676): each it can say (ADR 0668).
const TYPE_OF: [Kind; 7] = [
    Kind::String,
    Kind::Number,
    Kind::BigInt,
    Kind::Boolean,
    Kind::Symbol,
    Kind::Function,
    Kind::Object,
];

/// What a `switch (typeof value)` has of a variant of an untagged enum
/// (ADR 0668): its `case`, what `typeof` says of it, and the variants'
/// tests it isn't, `!Array.isArray(value)` of `Kind::Object`'s; or, of one
/// `typeof` says is another's, an array's `"object"`, its test, an `if`
/// before the `switch`; or `null`'s, which is in `case "object"`, its
/// test, `value === null`, an `if` in it (ADR 0676).
#[derive(Clone)]
pub(super) enum TypeofCase {
    Typeof(&'static str, Vec<Expr>),
    Tested(Expr),
    Inside(&'static str, Expr),
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

/// What a variant without fields of an untagged enum is: its name's string,
/// `"blocking"`, or `undefined` or `null`, as it's marked (ADR 0676).
fn unit_kind(tcx: TyCtxt<'_>, variant: &VariantDef) -> Option<Kind> {
    if !variant.fields.is_empty() || variant.ctor_kind() != Some(CtorKind::Const) {
        return None;
    }
    Some(match bindings::given_nullish(tcx, variant.def_id) {
        Some(bindings::Nullish::Null) => Kind::Null,
        Some(bindings::Nullish::Undefined) => Kind::Undefined,
        None => Kind::Literal(bindings::variant_name(tcx, variant)),
    })
}

/// Whether `variant` of an untagged enum is told by what it is, its
/// payload's kind, `undefined` or `null`, not compared to its string as a
/// literal's is (ADR 0676).
pub(super) fn told_by_kind(tcx: TyCtxt<'_>, variant: &VariantDef) -> bool {
    !matches!(unit_kind(tcx, variant), Some(Kind::Literal(_)))
}

/// The value a variant without fields of an untagged enum is in JS, the
/// one answer: `"blocking"`, `null` or `undefined` (ADR 0676).
pub(super) fn unit_value(tcx: TyCtxt<'_>, variant: &VariantDef) -> Option<Expr> {
    Some(match unit_kind(tcx, variant)? {
        Kind::Null => Expr::null(),
        Kind::Undefined => Expr::undefined(),
        Kind::Literal(name) => Expr::str(name),
        _ => unreachable!("a variant without fields is a literal, `null` or `undefined`"),
    })
}

/// Whether a value of the untagged enum `adt` may be `undefined` or `null`,
/// a variant's, `js::Kind::Undefined`'s: an `Option` of one is then a box
/// (ADR 0051).
pub(super) fn holds_nullish(tcx: TyCtxt<'_>, adt: AdtDef<'_>) -> bool {
    bindings::is_untagged(tcx, adt.did())
        && (adt.variants().iter()).any(|v| matches!(unit_kind(tcx, v), Some(Kind::Null | Kind::Undefined)))
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
        // One TypeScript types as `typeof` names a primitive, `js::Symbol`'s
        // `symbol`, is that kind.
        if let ty::Adt(adt, _) = ty.kind()
            && self.is_js_object(ty)
            && let Some(kind) = super::declarations::written_types(self.tcx, adt.did()).and_then(|t| Kind::of_types(&t))
        {
            return Some(kind);
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
            ty::Adt(adt, _)
                if is_string_enum(self.tcx, *adt) && !adt.variants().is_empty() && !self.is_std(adt.did()) =>
            {
                Some(Kind::String)
            }
            _ => None,
        }
    }

    /// What JS can tell `variant` is: its payload's kind, or of one without
    /// fields, its name's string.
    pub(super) fn untagged_variant_kind(&self, variant: &VariantDef, args: GenericArgsRef<'tcx>) -> Option<Kind> {
        if let Some(kind) = unit_kind(self.tcx, variant) {
            return Some(kind);
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
            let target = deref_target(tcx, imp)?;
            match tcx.type_of(target).instantiate_identity().skip_normalization().kind() {
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

    /// The test that `value` is one of `variants`, an or-pattern's, or the
    /// one: each one's payload's kind, leaving out what another variant's
    /// more exactly is, so each test holds of its own values only, whatever
    /// order they're tried in. A class's leaves out its subclasses in the
    /// enum; an object's every array and class, and `null`, but of a value
    /// `present`, `classify(Some(value))`'s, which is never `null` (ADR
    /// 0676). Of more than one, what they are together (`union_parts`):
    /// `Object | Array`'s `typeof value === "object" && value !== null`
    /// (ADR 0668).
    pub(super) fn untagged_test(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variants: &[&VariantDef],
        value: &Expr,
        present: bool,
        class: &dyn Fn(&str) -> Expr,
    ) -> Expr {
        let otherwise = |v: &VariantDef| bindings::is_otherwise(self.tcx, v.def_id);
        // What the others aren't: none of their tests, but where that's one
        // kind, as it is of `js::Kind`'s `Object`.
        if let Some(&variant) = (variants.iter()).find(|v| otherwise(v) && self.otherwise_kind(adt, args).is_none()) {
            let none = (adt.variants().iter().filter(|v| !otherwise(v)))
                .map(|v| Expr::unary(UnaryOp::Not, self.untagged_test(adt, args, &[v], value, present, class)));
            let none = none
                .reduce(|all, test| Expr::bin(Op::And, all, test))
                .unwrap_or(Expr::bool(true));
            let others: Vec<&VariantDef> = (variants.iter().copied())
                .filter(|v| v.def_id != variant.def_id)
                .collect();
            return match others.is_empty() {
                true => none,
                false => Expr::bin(
                    Op::Or,
                    self.untagged_test(adt, args, &others, value, present, class),
                    none,
                ),
            };
        }
        let test = |kind: &Kind| self.kind_test(kind, value, class);
        (self.union_parts(adt, args, variants, present).iter())
            .map(|(own, narrower)| {
                narrower.iter().fold(test(own), |all, kind| {
                    Expr::bin(Op::And, all, Expr::unary(UnaryOp::Not, test(kind)))
                })
            })
            .reduce(|either, test| Expr::bin(Op::Or, either, test))
            .expect("a variant to test")
    }

    /// What a value is that's one of `variants`, an or-pattern's: each
    /// one's kind and what its test leaves out (`untagged_parts`), but what
    /// another of them is whole, which they take in together; and of an
    /// array, or `null`, where an object that may be one is of them,
    /// nothing, as `typeof` says each is an object. So `Object | Array` is
    /// an object not `null`, `typeof value === "object" && value !== null`,
    /// as JS tests one, where each alone is `Array.isArray(value) || (typeof
    /// value === "object" && value !== null && !Array.isArray(value))`, and
    /// `Null | Object | Array` is `typeof value === "object"` (ADRs 0668,
    /// 0676). The one answer, of a test and of a `switch`'s case.
    fn union_parts(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variants: &[&VariantDef],
        present: bool,
    ) -> Vec<(Kind, Vec<Kind>)> {
        let parts: Vec<(Kind, Vec<Kind>)> = (variants.iter())
            .map(|v| self.untagged_parts(adt, args, v, present))
            .collect();
        let whole: Vec<Kind> = (parts.iter())
            .filter(|(_, narrower)| narrower.is_empty())
            .map(|(own, _)| own.clone())
            .collect();
        let parts: Vec<(Kind, Vec<Kind>)> = (parts.into_iter())
            .map(|(own, narrower)| (own, narrower.into_iter().filter(|kind| !whole.contains(kind)).collect()))
            .collect();
        // Whether an object of these may be `kind`, an array or `null`.
        let objects =
            |kind: &Kind| (parts.iter()).any(|(own, narrower)| *own == Kind::Object && !narrower.contains(kind));
        let (arrays, nulls) = (objects(&Kind::Array), objects(&Kind::Null));
        (parts.iter())
            .filter(|(own, _)| !(arrays && *own == Kind::Array || nulls && *own == Kind::Null))
            .cloned()
            .collect()
    }

    /// What `typeof` says is the value of an `otherwise` variant of `adt`,
    /// what the others aren't (ADR 0668): of the kinds it tells of a value,
    /// the one the others aren't, where one is left, `js::Kind`'s `Object`'s
    /// `"object"` beside its `String` to `Function`; of more, as of `Json`'s,
    /// none. A literal, `"auto"`, is one string, not every one.
    fn otherwise_kind(&self, adt: AdtDef<'tcx>, args: GenericArgsRef<'tcx>) -> Option<Kind> {
        let told: Vec<&'static str> = (adt.variants().iter())
            .filter(|v| !bindings::is_otherwise(self.tcx, v.def_id))
            .filter_map(|v| self.untagged_variant_kind(v, args))
            .filter(|kind| !matches!(kind, Kind::Literal(_)))
            .filter_map(|kind| kind.type_of())
            .collect();
        let left: Vec<Kind> = TYPE_OF
            .into_iter()
            .filter(|kind| !kind.type_of().is_some_and(|t| told.contains(&t)))
            .collect();
        match <[Kind; 1]>::try_from(left) {
            Ok([only]) => Some(only),
            Err(_) => None,
        }
    }

    /// `variant`'s kind and the kinds of the others its test leaves out, as
    /// more exactly theirs: a class's its subclasses in the enum; an
    /// object's every array and class, and `null` where the value may be
    /// it, not `present`. Of an `otherwise` variant, the kind left
    /// (`otherwise_kind`).
    fn untagged_parts(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variant: &VariantDef,
        present: bool,
    ) -> (Kind, Vec<Kind>) {
        let otherwise = |v: &VariantDef| bindings::is_otherwise(self.tcx, v.def_id);
        let kinds: Vec<Kind> = adt
            .variants()
            .iter()
            .filter(|v| !otherwise(v))
            .filter_map(|v| self.untagged_variant_kind(v, args))
            .collect();
        let own = match otherwise(variant) {
            true => self.otherwise_kind(adt, args),
            false => self.untagged_variant_kind(variant, args),
        };
        let own = own.expect("an untagged enum's variant has a kind, as its declaration was checked");
        let narrower: Vec<&Kind> = kinds
            .iter()
            .filter(|kind| match (&own, kind) {
                (Kind::Object, Kind::Array | Kind::Class(..) | Kind::Test(_)) => true,
                (Kind::Object, Kind::Null) => !present,
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
        let mut narrower: Vec<Kind> = narrower
            .iter()
            .filter(|kind| !narrower.iter().any(|base| extends(kind, base)))
            .map(|&kind| kind.clone())
            .collect();
        // `null` first, as JS writes an object's test: `typeof value ===
        // "object" && value !== null`.
        narrower.sort_by_key(|kind| *kind != Kind::Null);
        (own, narrower)
    }

    /// The test that `value` is of `kind`.
    fn kind_test(&self, kind: &Kind, value: &Expr, class: &dyn Fn(&str) -> Expr) -> Expr {
        match kind {
            Kind::Array => Expr::call(Expr::member(Expr::var("Array"), "isArray"), vec![value.clone()]),
            Kind::Class(name, _) => Expr::bin(Op::InstanceOf, value.clone(), class(name)),
            Kind::Test(test) => Expr::call(class(test), vec![value.clone()]),
            Kind::Literal(name) => Expr::bin(Op::Eq, value.clone(), Expr::str(name)),
            // `null` and `undefined` by `===`, as each is one value: never
            // `== null`, which is both.
            Kind::Null => Expr::bin(Op::Eq, value.clone(), Expr::null()),
            Kind::Undefined => Expr::bin(Op::Eq, value.clone(), Expr::undefined()),
            other => Expr::bin(
                Op::Eq,
                Expr::unary(UnaryOp::Typeof, value.clone()),
                Expr::str(other.type_of().expect("a `typeof` kind")),
            ),
        }
    }

    /// The cases in a `switch (typeof value)` (ADR 0668) of `variants`, an
    /// or-pattern's, or the one, as their test has them (`union_parts`):
    /// of each a kind `typeof` tells, or an array, whose test is an `if`
    /// before it, or `null`, whose test is an `if` in `case "object"` (ADR
    /// 0676); none of a class, a test function's or a literal, which it
    /// doesn't, nor of an `otherwise` that's none of more kinds. Of a value
    /// `present`, an object's case leaves out no `null`.
    pub(super) fn untagged_cases(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variants: &[&VariantDef],
        value: &Expr,
        present: bool,
    ) -> Option<Vec<TypeofCase>> {
        let otherwise = |v: &VariantDef| bindings::is_otherwise(self.tcx, v.def_id);
        if variants.iter().any(|v| otherwise(v)) && self.otherwise_kind(adt, args).is_none() {
            return None;
        }
        let no_class = |_: &str| unreachable!("a `typeof` kind's test names no class");
        (self.union_parts(adt, args, variants, present).into_iter())
            .map(|(own, narrower)| {
                // What it leaves out is an array or `null`, or it's no case:
                // an object no `Error` is, say, isn't one.
                if narrower.iter().any(|kind| !matches!(kind, Kind::Array | Kind::Null)) {
                    return None;
                }
                match (&own, own.type_of()) {
                    (Kind::Array, _) => Some(TypeofCase::Tested(self.kind_test(&own, value, &no_class))),
                    (Kind::Null, _) => Some(TypeofCase::Inside(
                        Kind::Object.type_of().expect("an object's `typeof`"),
                        self.kind_test(&own, value, &no_class),
                    )),
                    (_, None) => None,
                    (_, Some(case)) => Some(TypeofCase::Typeof(
                        case,
                        (narrower.iter())
                            .map(|kind| self.kind_test(kind, value, &no_class))
                            .collect(),
                    )),
                }
            })
            .collect()
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `ty` is an untagged enum, `#[rust_js::untagged]` (ADR 0214).
    pub(super) fn untagged(&self, ty: Ty<'tcx>) -> Option<(AdtDef<'tcx>, GenericArgsRef<'tcx>)> {
        untagged_adt(self.tcx, ty)
    }

    /// The test that `value` is one of `variants` of the untagged enum
    /// `ty`, an or-pattern's, or the one (`untagged_test`), which is
    /// `present` where it's never `undefined` nor `null` (ADR 0676).
    pub(super) fn untagged_variant_test(
        &self,
        ty: Ty<'tcx>,
        variants: &[&VariantDef],
        value: &Expr,
        present: bool,
    ) -> Expr {
        let (adt, args) = self.untagged(ty).expect("an untagged enum");
        self.recognition()
            .untagged_test(adt, args, variants, value, present, &|name| self.js_ref(name))
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
                // One without fields is its name's string, or `undefined`
                // or `null` (ADR 0676).
                if let Some(kind) = unit_kind(tcx, variant) {
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
                // `undefined` and `null` hold nothing.
                if let Some(nullish) = bindings::given_nullish(tcx, variant.def_id) {
                    let marked = format!("{nullish:?}").to_lowercase();
                    error(
                        span,
                        format!(
                            "an untagged enum's `#[rust_js::{marked}]` variant, `{}`, is `{marked}`, and holds nothing, as `js::Kind::{nullish:?}` does (ADR 0676)",
                            variant.name
                        ),
                    );
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
            let what = format!("serde of the untagged enum `{}`", tcx.item_name(adt.did()));
            let refused = super::refusal::Refused::Because(&what, "see ADR 0214");
            error(tcx.def_span(def_id), super::refusal::words(tcx, refused));
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
