//! The shallow checks (#2403 step 2a, ADR 1192): each parameter of a
//! function JS may call, of a number, a `bool`, a `char`, a string, `()`,
//! a fieldless enum or an `Option` of one, checked first in its body, one
//! statement each, `$checkU32(n, "double(n)");`. What a value of each type
//! is to JS is its `Contract`, the one the declarations print (ADR 1190), so
//! a check and the `.d.ts` can't disagree. An `Option`'s `None` is what its
//! parameter's form reads as one: `null` and `undefined` of a plain one,
//! `null` of a nullable one, `undefined` of one marked `none_undefined`.
//!
//! Not checked here: what holds other values, a struct, a tuple, a `Vec`,
//! a map, an enum with fields, a callback (2b's deep checks); a `NonZero`,
//! a pattern type and a `#[repr]` enum, whose values are narrower than
//! their kind's; JS's own types, `Unknown` and type parameters, which are
//! any value; and `@rust-js/std`'s functions, which only Rust calls.

use rust_js_program::js::{self, Expr};
use rustc_middle::ty::{self, Ty};

use crate::FnCx;
use crate::declarations::contract::{self, Contract};
use crate::representation::Num;
use crate::signatures::{Form, JsParam, JsSignature, Origin};

/// The runtime's checks a function starts with, each a helper of
/// `boundary_checks.js`.
pub(crate) const CHECKS: &[&str] = &[
    "$checks",
    "$checkU8",
    "$checkU16",
    "$checkU32",
    "$checkU64",
    "$checkU128",
    "$checkI8",
    "$checkI16",
    "$checkI32",
    "$checkI64",
    "$checkI128",
    "$checkF32",
    "$checkF64",
    "$checkBool",
    "$checkChar",
    "$checkStr",
    "$checkUnit",
    "$checkOneOf",
    "$checkWhole",
    "$checkCallback",
];

/// What a parameter's check is: its helper, and the arguments after the
/// value and where it is.
pub(super) struct Check {
    pub(super) helper: &'static str,
    /// The names a fieldless enum's value is one of, given before `where`.
    pub(super) names: Option<Vec<String>>,
    /// Of a value that holds others, its checker, which `$checkWhole` calls
    /// (2b(b), ADR 1194).
    pub(super) whole: Option<Expr>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The check of a parameter JS gives whole, `param` of `signature`, by
    /// `name`, its JS's, at `place`, `count(order)`: its type's, or of a
    /// function, a callback's, wrapped where what it gives back is read (2c,
    /// ADR 1195); none of what isn't checked here. At no place of the Rust,
    /// the boundary's own code: a comment above the parameter stays above it.
    pub(super) fn whole_check(
        &self,
        (signature, param): (&JsSignature<'tcx>, &JsParam<'tcx>),
        name: &js::Name,
        place: String,
    ) -> Option<js::Stmt> {
        if let Some(callback) = self.callback(signature, param.form) {
            let mut args = vec![Expr::var(name), Expr::str(place)];
            let call = |args| Expr::call(Expr::helper("$checkCallback"), args);
            return Some(match callback.output {
                Some(output) => {
                    args.push(output);
                    js::StmtKind::Assign(Expr::var(name), call(args)).at(js::Span::NONE)
                }
                None => js::StmtKind::Expr(call(args)).at(js::Span::NONE),
            });
        }
        let (check, none) = self.parameter_check(param.form)?;
        // A `#[rust_js::nullable]` one its lowering takes `?? null` of, which
        // a call through a value may give `undefined`: that too, as the
        // nulled parameters' owner says (`NulledFound::is_nulled`, ADR 0861).
        let none = match (param.form, param.origin) {
            (Form::Nullable(_), Origin::Rust(at)) if !self.settled.nulled.is_nulled(self.item, at) => Some("nullish"),
            _ => none,
        };
        let mut args = vec![Expr::var(name)];
        if let Some(whole) = check.whole {
            args.insert(0, whole);
        }
        if let Some(names) = check.names {
            args.push(Expr::array(names.into_iter().map(Expr::str).collect()));
        }
        // Its JS name, which a JS caller knows, a word JS keeps given apart
        // as written, `new`.
        args.push(Expr::str(place));
        if let Some(none) = none {
            args.push(Expr::str(none));
        }
        Some(js::StmtKind::Expr(Expr::call(Expr::helper(check.helper), args)).at(js::Span::NONE))
    }

    /// Whether `item`, whose THIR is `thir`, is a function whose
    /// parameters JS gives are checked: one JS may call, not a closure, a
    /// binding, an `async fn` or `@rust-js/std`'s, nor one that passes them
    /// on to a function that checks them itself.
    pub(crate) fn checks_its_parameters(
        &self,
        item: rustc_span::def_id::DefId,
        thir: &rustc_middle::thir::Thir<'tcx>,
    ) -> bool {
        !(self.krate.compiled_once
            || self.tcx.is_closure_like(item)
            || crate::bindings::is_binding(self.tcx, item)
            || !self.js_callable(item)
            || self.forwards_to_a_checked_function(item, thir))
    }

    /// Whether JS outside the crate's Rust may call `item`, so its
    /// parameters are checked (#2403 lever (3), Tu's decision): a function
    /// its module exports (`js_facing`) that the crate's root gives JS, its
    /// own or by a `pub use`, as its package's entry does (ADR 1008), or
    /// that a module a hand-written JS file of the program or its workspace
    /// imports gives (`reexports::JsReached`, the one owner of both). Not one
    /// `pub mod` paths alone reach, which only Rust names, its calls the
    /// body's. A library's, as its own crate decided it. JS that
    /// deep-imports a module no such file imports reaches it unchecked, the
    /// limit the guide says.
    pub(crate) fn js_callable(&self, item: rustc_span::def_id::DefId) -> bool {
        crate::analysis::js_facing(self.tcx, self.krate.foreign, item)
            && match item.as_local() {
                Some(local) => self.krate.js_reached.may_call(self.tcx, local),
                // A library's, where its own JS checks it, as its manifest
                // says: one it doesn't, a forwarder of the crate's checks
                // itself, never its re-export (#2403 (Y)).
                None => self.krate.foreign.item(item).is_none_or(|imported| imported.checked),
            }
    }

    /// Whether any parameter of `item` is checked: one Rust names plainly,
    /// of a kind checked here.
    pub(crate) fn checks_a_parameter(&self, item: rustc_span::def_id::DefId) -> bool {
        let idents = self.tcx.fn_arg_idents(item);
        let signature = self.js_signature(item);
        (signature.params.iter()).any(|param| {
            matches!(param.origin, Origin::Rust(at) if named_plainly(idents.get(at)))
                && (self.parameter_check(param.form).is_some() || self.callback(&signature, param.form).is_some())
        })
    }

    /// Whether `item` only passes its parameters on to a function JS may
    /// call of no type arguments, which checks the same values itself: its
    /// module re-exports that function in its place where it can (rd575,
    /// ADR 1064), `export { mixed } from "./area.js"`, as a check of its
    /// own would keep it a function. One of a generic function's
    /// instantiation is checked, as that function's `T` is any value.
    pub(crate) fn forwards_to_a_checked_function(
        &self,
        item: rustc_span::def_id::DefId,
        thir: &rustc_middle::thir::Thir<'tcx>,
    ) -> bool {
        crate::passed_on::forwarder(self.tcx, item, thir)
            .is_some_and(|(target, args)| args.types().next().is_none() && self.js_callable(target))
    }

    /// What an error calls the function: its JS name, a method's after its
    /// type's, `Cart.add`.
    pub(super) fn checked_function_name(&self, item: rustc_span::def_id::DefId) -> String {
        let name = crate::bindings::fn_name(self.tcx, item);
        match self.tcx.inherent_impl_of_assoc(item) {
            Some(imp) => match self.tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
                ty::Adt(adt, _) => format!("{}.{name}", self.tcx.item_name(adt.did())),
                _ => name,
            },
            None => name,
        }
    }

    /// A parameter's check of its form, and the `None` it allows, "null",
    /// "undefined" or "nullish"; none of what isn't checked here.
    pub(super) fn parameter_check(&self, form: Form<'tcx>) -> Option<(Check, Option<&'static str>)> {
        match form {
            Form::Value { ty, .. } => match self.contract(ty) {
                // Its `Some` a box where its value might look like `None`
                // (ADR 0051): checked whole, by `checker`, which opens it.
                Contract::Option(inner) if self.boxed_payload(inner) => Some((
                    Check {
                        helper: "$checkWhole",
                        names: None,
                        whole: Some(self.checker(ty)?),
                    },
                    None,
                )),
                Contract::Option(inner) => Some((self.shallow_check(inner)?, Some("nullish"))),
                _ => Some((self.shallow_check(ty)?, None)),
            },
            Form::Nullable(inner) => Some((self.shallow_check(inner)?, Some("null"))),
            Form::Undefinable { inner, .. } => Some((self.shallow_check(inner)?, Some("undefined"))),
            _ => None,
        }
    }

    /// What a value of `ty` is to JS, as the declarations say it.
    pub(super) fn contract(&self, ty: Ty<'tcx>) -> Contract<'tcx> {
        let facts = contract::Facts {
            tcx: self.tcx,
            counted: self.krate.counted,
            ref_cells: self.ref_cells(),
            foreign: self.krate.foreign,
        };
        Contract::of(&facts, ty)
    }

    /// The shallow check of a value of `ty`, if it's one of the kinds
    /// checked here.
    pub(super) fn shallow_check(&self, ty: Ty<'tcx>) -> Option<Check> {
        // A `NonZero` and a pattern type are numbers narrower than their
        // kind's: 2b's.
        let narrowed = matches!(ty.kind(), ty::Pat(..))
            || matches!(ty.kind(), ty::Adt(adt, _) if crate::recognition::is_non_zero(adt.did()));
        let helper = match self.contract(ty) {
            Contract::Num(_) if narrowed => return None,
            Contract::Num(num) => match num {
                Num::U8 => "$checkU8",
                Num::U16 => "$checkU16",
                Num::U32 => "$checkU32",
                Num::U64 => "$checkU64",
                Num::U128 => "$checkU128",
                Num::I8 => "$checkI8",
                Num::I16 => "$checkI16",
                Num::I32 => "$checkI32",
                Num::I64 => "$checkI64",
                Num::I128 => "$checkI128",
                Num::F32 => "$checkF32",
                Num::F64 => "$checkF64",
            },
            Contract::Bool => "$checkBool",
            Contract::Char => "$checkChar",
            Contract::Str => "$checkStr",
            Contract::Unit => "$checkUnit",
            // A shared borrow is the value it borrows, `&str` a string.
            Contract::Shared(inner) => return self.shallow_check(inner),
            Contract::Local(did, _) if let Some(names) = self.fieldless_names(did) => {
                return Some(Check {
                    helper: "$checkOneOf",
                    names: Some(names),
                    whole: None,
                });
            }
            // What holds other values, checked whole (2b(b), ADR 1194).
            _ => {
                return self.deep_checker(ty).map(|whole| Check {
                    helper: "$checkWhole",
                    names: None,
                    whole: Some(whole),
                });
            }
        };
        Some(Check {
            helper,
            names: None,
            whole: None,
        })
    }

    /// The names a value of the crate's enum `did` is one of, where each of
    /// its variants is its name's string: no fields, neither untagged nor
    /// of a tag of its own, no `#[repr]` numbers, no `bool`s.
    fn fieldless_names(&self, did: rustc_span::def_id::DefId) -> Option<Vec<String>> {
        let tcx = self.tcx;
        if tcx.def_kind(did) != rustc_hir::def::DefKind::Enum {
            return None;
        }
        let adt = tcx.adt_def(did);
        let plain = !adt.variants().is_empty()
            && adt.repr().int.is_none()
            && !crate::bindings::is_untagged(tcx, did)
            && crate::bindings::declared_tag(tcx, did).is_none()
            && (adt.variants().iter())
                .all(|v| v.fields.is_empty() && crate::bindings::given_bool(tcx, v.def_id).is_none());
        plain.then(|| {
            (adt.variants().iter())
                .map(|v| crate::bindings::variant_name(tcx, v))
                .collect()
        })
    }
}

/// Whether a parameter Rust names `ident` is named plainly, `n: u32`: not
/// a pattern, which names none, nor an `async fn`'s pattern, which rustc
/// names `__arg0` (2b(d): a pattern's check waits on its design).
pub(super) fn named_plainly(ident: Option<&Option<rustc_span::Ident>>) -> bool {
    matches!(ident, Some(Some(ident)) if !ident.name.as_str().starts_with("__arg"))
}
