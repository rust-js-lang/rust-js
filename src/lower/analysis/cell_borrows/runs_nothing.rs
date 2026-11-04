//! Which calls run none of the crate's code, so a borrow held across one
//! meets no other and it sets none of the crate's variables (ADRs 0277,
//! 0867, 0898): a binding's that runs nothing or only what it's given, a JS
//! object's `Deref`, an untagged enum's `From`, and a promise's resolve
//! given what JS reads with no code. `cell_borrows`' answers, apart.

use crate::lower::bindings::{JsForm, is_binding, js_form, runs_nothing, runs_only_given};
use crate::lower::recognition::{Recognition, StdItem, is_js_object_deref};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{ExprId, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// Whether a binding runs no code of the crate's: what makes a JSX element
/// or an object, or casts a value, a component given called when React
/// renders it; a read of a property, `node.sortIndex`, `in` or
/// `instanceof`, but by a JS getter that calls back into Rust; whether the
/// host has a global, `typeof X !== "undefined"`; JS's `null`, `js::null()`;
/// a function made of a closure, `function::with_this`, which runs it only
/// where it's called, a call of a `dyn Fn` (ADR 1009); and one that says it
/// runs nothing, `#[rust_js::runs_nothing]` (ADR 0867).
pub(in crate::lower) fn binding_runs_nothing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_binding(tcx, id)
        && (matches!(
            js_form(tcx, id),
            JsForm::Jsx(_)
                | JsForm::Prop(_)
                | JsForm::Object(_)
                | JsForm::This
                | JsForm::Get(_)
                | JsForm::GetIndex
                | JsForm::In
                | JsForm::InstanceOf(_)
                | JsForm::InstanceOfGiven
                | JsForm::Truthy
                | JsForm::StrictEquals
                | JsForm::Asks(_)
                | JsForm::WithThis
                | JsForm::TypeofObject
                | JsForm::Null
                | JsForm::Or
                | JsForm::Method
        ) || runs_nothing(tcx, id))
}

/// Whether a call of `id` runs no code of anyone's but its own, not even
/// what it's given: a binding's that runs nothing (`binding_runs_nothing`),
/// or a JS object type's `Deref` to the class it extends, which is the
/// object itself, `AggregateError`'s to `Error`; or a `From` into an
/// untagged enum, the value it's given, webapi's `BlobPart::from(chunk)`
/// (rd535). The one answer a borrow
/// held across it and a thread-local made as its module loads ask
/// (`may_run::may_run`, ADR 1006, amended).
pub(in crate::lower) fn call_runs_nothing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    binding_runs_nothing(tcx, id)
        || is_js_object_deref(tcx, tcx.parent(id))
        || crate::lower::untagged::is_untagged_from(tcx, id)
}

/// Whether `def_id`, a binding that runs no code but its own and what it's
/// given, `#[rust_js::runs_only_given]` (ADR 0898), is given, of `args`,
/// only what runs nothing as JS reads it (`value_runs_nothing`).
fn given_what_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) || !runs_only_given(tcx, def_id) {
        return false;
    }
    let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
    (sig.skip_binder().inputs().iter()).all(|&input| value_runs_nothing(recognition, input))
}

/// Whether JS reads a value of `ty` with no code of anyone's: a number, a
/// `bool`, a `char`, text, and `Option`s, tuples, arrays, slices and
/// `Vec`s of them, which `String(x)` and `+x` read so (ADR 0898). Not an
/// object of the crate's, whose getter or `toString` may be its code, nor
/// a JS value, which may be a function or a proxy of the crate's.
fn value_runs_nothing<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => true,
        ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => value_runs_nothing(recognition, *inner),
        ty::Tuple(items) => items.iter().all(|item| value_runs_nothing(recognition, item)),
        ty::Adt(_, _) if recognition.is_lang_adt(ty, LangItem::String) => true,
        ty::Adt(_, items)
            if recognition.is_lang_adt(ty, LangItem::Option) || recognition.is_std_type(ty, StdItem::Vec) =>
        {
            value_runs_nothing(recognition, items.type_at(0))
        }
        _ => false,
    }
}

/// Whether a call of `def_id` of `args`, a `dyn Fn`'s `call`, given
/// `call_args` in `thir`, calls a promise's resolve function, which JS gave
/// its executor (`resolvers::is_resolve`), given only what runs nothing as
/// JS reads it (`value_runs_nothing`): it fulfils the promise, which only
/// queues its reactions, and runs nothing, so a borrow held across it
/// meets no other (rd654, ADR 0898, amended).
pub(in crate::lower) fn resolve_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    thir: &Thir<'tcx>,
    (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    call_args: &[ExprId],
) -> bool {
    let tcx = recognition.tcx;
    let fn_trait = tcx
        .trait_of_assoc(def_id)
        .is_some_and(|of| tcx.fn_trait_kind_from_def_id(of).is_some());
    let (true, [callee, _]) = (fn_trait, call_args) else {
        return false;
    };
    matches!(args.type_at(0).peel_refs().kind(), ty::Dynamic(..))
        && (args.type_at(1).tuple_fields().iter()).all(|input| value_runs_nothing(recognition, input))
        && super::resolvers::is_resolve(tcx, thir, *callee)
}

/// Whether a call of `def_id`, a binding, of `args`, runs none of the
/// crate's code: one that runs nothing (`binding_runs_nothing`), or only
/// what it's given, given only what runs nothing
/// (`given_what_runs_nothing`). So a borrow held across it meets no other,
/// and it sets none of the crate's variables, which lowering marks its
/// call so (`Known::RunsNothing`, ADR 0277, amended).
pub(in crate::lower) fn binding_call_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    binding_runs_nothing(recognition.tcx, def_id) || given_what_runs_nothing(recognition, def_id, args)
}
