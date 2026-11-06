//! How a place holds an `Option`'s `None` (#819, ADR 1085's change 6): one
//! answer, `NoneForm`, of each kind of place an `Option` is written to, a
//! struct's field, a thread-local, a function's result or parameter and a
//! binding's argument, and what a write of it stores, `NoneForm::storage`.
//! Each writer asks it, never `#[rust_js::nullable]`'s declaration itself;
//! and each reader what it may trust of a `None` there, of who writes it,
//! `NoneForm::trust`.
//! The type-level answer, whether a `Some` is boxed (ADR 0051), is
//! `options.rs`'s.

use crate::js;
use crate::lower::bindings;
use rustc_middle::ty::{FieldDef, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// How a place holds an `Option`'s `None`. Each match of it is exhaustive,
/// so a form added is a compile error wherever one is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::lower) enum NoneForm {
    /// `undefined`, a plain `Option`'s: its `Some` its value, or `$some` of
    /// one that may look like `None` (`boxed_payload`).
    #[default]
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
    /// JS's own, a binding's result it declares neither way: `undefined`
    /// or `null`, as JS gives it (ADR 0298, amended).
    Either,
}

/// Who writes a place a reader reads, and so what it may trust of its
/// `None` (`NoneForm::trust`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Writers {
    /// The crate's own, each storing by the place's form (`storage()`):
    /// a nullable one's `None` one thing, `null` (`?? null`), any other's
    /// as it's given.
    Crate,
    /// Each value exactly of the form: a literal `None` made, `undefined`,
    /// a value `null` or a value already, or JS's held to the type a
    /// binding declares, `T | null` or `T | undefined`.
    Exact,
    /// JS's too, which may leave a field out, `undefined`, of a struct it
    /// makes: nothing to trust (ADR 1072).
    Loose,
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
            NoneForm::Undefined | NoneForm::Nested | NoneForm::Value | NoneForm::Either => false,
        }
    }

    /// The form of the `Option` a `Some` of this place holds: of a nested
    /// one, `null` (ADR 1017), of any other, none of its own.
    pub(in crate::lower) fn of_some(self) -> NoneForm {
        match self {
            NoneForm::Nested => NoneForm::Null,
            NoneForm::Undefined | NoneForm::Null | NoneForm::Optional | NoneForm::Value | NoneForm::Either => {
                NoneForm::Undefined
            }
        }
    }

    /// What a reader of the place, written by `writers`, may trust its
    /// `None` is, the one answer of it (#819): `Known::NoneNull`, tested
    /// `=== null`, or `Known::NoneUndefined`, `=== undefined`; `None` where
    /// it may be either, tested `== null`. A nullable place's is `null`,
    /// stored so; a plain one's `undefined` only where each value is
    /// exactly so, as a plain write stores what it's given; an optional
    /// one's may be left out, and a held one, nested or of a value, is
    /// read through its own form (ADRs 1017, rd645).
    pub(in crate::lower) fn trust(self, writers: Writers) -> Option<js::Known> {
        match (self, writers) {
            (_, Writers::Loose) => None,
            (NoneForm::Null, Writers::Crate | Writers::Exact) => Some(js::Known::NoneNull),
            (NoneForm::Undefined, Writers::Exact) => Some(js::Known::NoneUndefined),
            (NoneForm::Undefined, Writers::Crate)
            | (NoneForm::Optional | NoneForm::Nested | NoneForm::Value | NoneForm::Either, _) => None,
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
    /// The `Cell` or `RefCell` of an `Option` a thread-local of this key
    /// holds, which, and what it lends, stores so however it's written (ADR
    /// 1078, #541).
    LocalCell(DefId),
    /// The result of this body, a function's or a nullable thread-local's
    /// `init`.
    Result(DefId),
    /// A function's parameter of this name.
    Param(DefId, rustc_span::Symbol),
    /// What a call of this function gives: a binding's, as it declares it,
    /// `#[rust_js::nullable]`'s `T | null` or `#[rust_js::optional]`'s `T |
    /// undefined` (ADR 0298, amended); not a trait's method's, whose impl
    /// the call may run, nor the crate's own function's, which may give
    /// back as it is a nullable field's value an object JS made holds
    /// `undefined` in.
    CallResult(DefId),
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
///   `#[rust_js::nullable(..)]` names, `T | null` (ADR 0861), and what a
///   call gives, as a binding declares it (ADR 0298, amended);
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
        NonePlace::LocalCell(key) => NoneForm::null_if(bindings::nullable_local_cell(tcx, key)),
        NonePlace::Result(body) => NoneForm::null_if(bindings::null_result(tcx, body)),
        NonePlace::Param(def_id, name) => NoneForm::null_if(bindings::nullable_params(tcx, def_id).contains(&name)),
        NonePlace::CallResult(def_id) if !bindings::is_binding(tcx, def_id) || tcx.trait_of_assoc(def_id).is_some() => {
            NoneForm::Either
        }
        NonePlace::CallResult(def_id) if bindings::nullable_result(tcx, def_id) => NoneForm::Null,
        NonePlace::CallResult(def_id) if bindings::optional_result(tcx, def_id) => NoneForm::Undefined,
        NonePlace::CallResult(_) => NoneForm::Either,
        NonePlace::BindingArg(def_id, i, payload) => {
            NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i) && bindings::holds_none_as_null(tcx, payload))
        }
        NonePlace::BindingEntries(def_id, i) => NoneForm::null_if(bindings::takes_nullable_entries(tcx, def_id, i)),
        NonePlace::ForeignParam(foreign, def_id, i) => NoneForm::null_if(foreign.nulled_param(def_id, i)),
    }
}
