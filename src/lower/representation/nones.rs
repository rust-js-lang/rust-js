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

/// A place an `Option` is written to, of each kind `none_form` answers: a
/// new kind is a variant its match can't miss.
#[derive(Clone, Copy)]
pub(in crate::lower) enum NonePlace<'p, 'tcx> {
    /// A struct's field, of an `Option` or a cell of one, its type there
    /// where that's asked.
    Field(&'p FieldDef, Option<Ty<'tcx>>),
    /// A thread-local or a `const` of this key.
    Local(DefId),
    /// The `Option` a thread-local of this key holds itself, not a cell's.
    LocalOption(DefId),
    /// The result of this body, a function's or a nullable thread-local's
    /// `init`.
    Result(DefId),
    /// A function's parameter of this name.
    Param(DefId, rustc_span::Symbol),
    /// A binding's argument, by its index, an `Option` of this payload.
    BindingArg(DefId, usize, Ty<'tcx>),
    /// The values of the entries a binding's argument holds,
    /// `[[key, value], ..]`, by its index.
    BindingEntries(DefId, usize),
    /// Another crate's function's parameter, by its index, which its body
    /// tests `!== null`, as its manifest says.
    ForeignParam(&'p crate::lower::library::Foreign<'p, 'tcx>, DefId, usize),
}

/// How `place` holds a `None`, the one answer of each kind of place:
/// - a field's: a held value's, a nullable one's, an optional nullable
///   one's, or a plain one's, a nested one's too, which no writer stores
///   apart;
/// - a thread-local's or a `const`'s, and the `Option` a thread-local holds
///   itself (`#[rust_js::nullable]`, ADR 0586, amended);
/// - a function's result (ADR 0841) and its parameter its
///   `#[rust_js::nullable(..)]` names, `T | null` (ADR 0861);
/// - a binding's argument it names `#[rust_js::nullable(..)]`, or a
///   collection's value it holds `None` of as `null`, of a payload not of
///   any shape (ADR 0275), and the values of entries it's given (ADR 0884,
///   amended);
/// - another crate's function's parameter, as its manifest says (ADR 0861).
pub(in crate::lower) fn none_form<'tcx>(tcx: TyCtxt<'tcx>, place: NonePlace<'_, 'tcx>) -> NoneForm {
    match place {
        NonePlace::Field(field, field_ty) => {
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
        NonePlace::Local(key) => NoneForm::null_if(bindings::nullable_local(tcx, key)),
        NonePlace::LocalOption(key) => NoneForm::null_if(bindings::nullable_local_option(tcx, key)),
        NonePlace::Result(body) => NoneForm::null_if(bindings::null_result(tcx, body)),
        NonePlace::Param(def_id, name) => NoneForm::null_if(bindings::nullable_params(tcx, def_id).contains(&name)),
        NonePlace::BindingArg(def_id, i, payload) => {
            NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i) && bindings::holds_none_as_null(tcx, payload))
        }
        NonePlace::BindingEntries(def_id, i) => NoneForm::null_if(bindings::takes_nullable_entries(tcx, def_id, i)),
        NonePlace::ForeignParam(foreign, def_id, i) => NoneForm::null_if(foreign.nulled_param(def_id, i)),
    }
}
