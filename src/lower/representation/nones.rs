//! How a place holds an `Option`'s `None` (#819, ADR 1085's change 6): one
//! answer, `NoneForm`, of each kind of place an `Option` is written to, a
//! struct's field, a thread-local, a function's result or parameter and a
//! binding's argument, and what a write of it stores, `NoneForm::storage`.
//! Each writer asks it, never `#[rust_js::nullable]`'s declaration itself.
//! The type-level answer, whether a `Some` is boxed (ADR 0051), is
//! `options.rs`'s.

use crate::js;
use crate::lower::bindings;
use rustc_middle::ty::{FieldDef, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// How a place holds an `Option`'s `None`. Each match of it is exhaustive,
/// so a form added is a compile error wherever one is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum NoneForm {
    /// `undefined`, a plain `Option`'s: its `Some` its value, or `$some` of
    /// one that may look like `None` (`boxed_payload`).
    Undefined,
    /// `null`, a `#[rust_js::nullable]` place's (ADRs 0196, 0275): what may
    /// be `undefined` stored `?? null` (`Storage::NullForNone`).
    Null,
    /// A `#[rust_js::nullable]` and `#[rust_js::optional]` field's: a
    /// literal's `None` left out, `undefined`, anything else as a nullable
    /// place stores it (ADR 0810, amended).
    Optional,
    /// A nullable `Option<Option<T>>` field's, ADR 1017's held form: its
    /// `None` not there, `undefined`, its `Some(None)` `null` (#470).
    Nested,
    /// A nullable `Option<V>` field of a `V` never `null` that may be
    /// `undefined`: its `None` `null`, its `Some(v)` `v` (rd645).
    Value,
}

impl NoneForm {
    /// How a write of the place stores what's written: a nullable place's
    /// `None` `null`, `made ?? null` where it may be `undefined`
    /// (`Storage::written`); any other as it's given.
    pub(in crate::lower) fn storage(self) -> js::Storage {
        match self.stores_null() {
            true => js::Storage::NullForNone,
            false => js::Storage::AsGiven,
        }
    }

    /// Whether a write of the place stores a `None` `null`
    /// (`Storage::NullForNone`).
    pub(in crate::lower) fn stores_null(self) -> bool {
        match self {
            NoneForm::Null | NoneForm::Optional => true,
            NoneForm::Undefined | NoneForm::Nested | NoneForm::Value => false,
        }
    }

    /// The form of the `Option` a `Some` of this place holds: of a nested
    /// one, `null` (ADR 1017), of any other, none of its own.
    pub(in crate::lower) fn of_some(self) -> NoneForm {
        match self {
            NoneForm::Nested => NoneForm::Null,
            NoneForm::Undefined | NoneForm::Null | NoneForm::Optional | NoneForm::Value => NoneForm::Undefined,
        }
    }

    fn null_if(null: bool) -> NoneForm {
        match null {
            true => NoneForm::Null,
            false => NoneForm::Undefined,
        }
    }
}

/// How `field` holds a `None`, of an `Option` or a cell of one, `field_ty`
/// where the field's type there is asked: a held value's, a nullable
/// one's, an optional nullable one's, or a plain one's, a nested one's too,
/// which no writer stores apart.
pub(in crate::lower) fn field_none_form<'tcx>(
    tcx: TyCtxt<'tcx>,
    field: &FieldDef,
    field_ty: Option<Ty<'tcx>>,
) -> NoneForm {
    if bindings::holds_value(tcx, field) {
        return NoneForm::Value;
    }
    let held = field_ty.is_none_or(|ty| bindings::nullable_option(tcx, ty).is_some());
    match (
        bindings::is_nullable(tcx, field) && held,
        bindings::is_optional(tcx, field),
    ) {
        (true, true) => NoneForm::Optional,
        (true, false) => NoneForm::Null,
        (false, _) => NoneForm::Undefined,
    }
}

/// How a thread-local or a `const` of `key` holds a `None`
/// (`#[rust_js::nullable]`, ADR 0586).
pub(in crate::lower) fn local_none_form(tcx: TyCtxt<'_>, key: DefId) -> NoneForm {
    NoneForm::null_if(bindings::nullable_local(tcx, key))
}

/// How the `Option` a thread-local of `key` holds itself, not a cell's, holds
/// a `None` (ADR 0586, amended).
pub(in crate::lower) fn local_option_none_form(tcx: TyCtxt<'_>, key: DefId) -> NoneForm {
    NoneForm::null_if(bindings::nullable_local_option(tcx, key))
}

/// How the result of `body`, a function's or a nullable thread-local's
/// `init`, holds a `None` (ADRs 0841, 0586).
pub(in crate::lower) fn result_none_form(tcx: TyCtxt<'_>, body: DefId) -> NoneForm {
    NoneForm::null_if(bindings::null_result(tcx, body))
}

/// How binding `def_id`'s `i`th argument, an `Option` of `payload`, holds a
/// `None`: one it names `#[rust_js::nullable(..)]`, or a collection's value
/// it holds `None` of as `null`, of a payload not of any shape (ADR 0275).
pub(in crate::lower) fn binding_arg_none_form(tcx: TyCtxt<'_>, def_id: DefId, i: usize, payload: Ty<'_>) -> NoneForm {
    NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i) && bindings::holds_none_as_null(tcx, payload))
}

/// How `def_id`'s parameter `name` holds a `None`: one its
/// `#[rust_js::nullable(..)]` names, `T | null` (ADR 0861).
pub(in crate::lower) fn param_none_form(tcx: TyCtxt<'_>, def_id: DefId, name: rustc_span::Symbol) -> NoneForm {
    NoneForm::null_if(bindings::nullable_params(tcx, def_id).contains(&name))
}

/// How the values of the entries binding `def_id`'s `i`th argument holds,
/// `[[key, value], ..]`, hold a `None`: of a collection whose values it
/// holds `None` of as `null` (ADR 0884, amended).
pub(in crate::lower) fn binding_entries_none_form(tcx: TyCtxt<'_>, def_id: DefId, i: usize) -> NoneForm {
    NoneForm::null_if(bindings::takes_nullable_entries(tcx, def_id, i))
}

/// How another crate's function's `i`th parameter, which its body tests
/// `!== null`, holds a `None`: as its manifest says (ADR 0861).
pub(in crate::lower) fn foreign_param_none_form(
    foreign: &crate::lower::library::Foreign<'_, '_>,
    def_id: DefId,
    i: usize,
) -> NoneForm {
    NoneForm::null_if(foreign.nulled_param(def_id, i))
}
