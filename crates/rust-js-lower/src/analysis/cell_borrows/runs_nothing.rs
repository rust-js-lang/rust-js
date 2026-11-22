//! Which calls run none of the crate's code, so a borrow held across one
//! meets no other and it sets none of the crate's variables (ADRs 0277,
//! 0867, 0898): a binding's that runs nothing or only what it's given, a JS
//! object's `Deref`, an untagged enum's `From`, and a promise's resolve
//! given what JS reads with no code. `cell_borrows`' answers, apart.

use crate::bindings::{JsForm, is_binding, js_form, runs_nothing, runs_only_given};
use crate::recognition::{Recognition, StdItem, is_js_object_deref};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{ExprId, ExprKind, Thir};

use super::at_hand::AtHand;
use crate::analysis::fn_pointers::FnPointers;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// Whether a binding runs no code of the crate's: what makes a JSX element
/// or an object, or casts a value, a component given called when React
/// renders it; `in` or `instanceof`; whether the
/// host has a global, `typeof X !== "undefined"`; JS's `null`, `js::null()`;
/// a function made of a closure, `function::with_this`, which runs it only
/// where it's called, a call of a `dyn Fn` (ADR 1009); and one that says it
/// runs nothing, `#[rust_js::runs_nothing]` (ADR 0867).
pub(crate) fn binding_runs_nothing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_binding(tcx, id)
        && (matches!(
            js_form(tcx, id),
            JsForm::Jsx(_)
                | JsForm::Prop(_)
                | JsForm::Object(_)
                | JsForm::This
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
/// untagged enum that's the value it's given, webapi's `BlobPart::from(chunk)`
/// (rd535; `untagged::from_gives_its_argument`, #1775). The one answer a
/// borrow held across it and a thread-local made as its module loads ask
/// (`may_run::may_run`, ADR 1006, amended).
/// A binding's method is its own only of a receiver no JS caller handed
/// over (`receiver_runs_its_own`, #2111), asked of the call's arguments,
/// `given`, where the asker has them; of none, of the receiver's type
/// alone. Every asker asks it, `may_run` too, whose model took every
/// receiver's method to be its own (#2119).
pub(crate) fn call_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (id, args): (DefId, ty::GenericArgsRef<'tcx>),
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    (binding_runs_nothing(tcx, id) && receiver_runs_its_own(recognition, (id, args), given))
        || is_js_object_deref(tcx, tcx.parent(id))
        || crate::untagged::from_gives_its_argument(recognition.foreign, id)
}

/// Whether `def_id`, a binding that runs no code but its own and what it's
/// given, `#[rust_js::runs_only_given]` (ADR 0898), is given, of `args`,
/// only what runs nothing as JS reads it (`value_runs_nothing`).
/// Of each argument, where the call's are known, `given`, also one
/// written `None`, whatever its type: it's nothing JS reads, `undefined`,
/// as `Intl.Locale(tag, None)`'s options are (#2082).
fn given_what_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) || !runs_only_given(tcx, def_id) {
        return false;
    }
    let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
    let written_none = |at: usize| {
        given.is_some_and(|given| {
            given
                .args
                .get(at)
                .is_some_and(|&arg| is_none_written(recognition, given.thir, arg))
        })
    };
    (sig.skip_binder().inputs().iter().enumerate())
        .all(|(at, &input)| value_runs_nothing(recognition, input) || written_none(at))
}

/// Whether `arg` is `None` as the call writes it, an `Option`'s variant of
/// no payload, through the scopes THIR wraps it in.
fn is_none_written<'tcx>(recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, arg: ExprId) -> bool {
    let at = crate::body_queries::strip(thir, arg);
    matches!(&thir[at].kind, ExprKind::Adt(adt)
        if recognition.is_lang_adt(thir[at].ty, LangItem::Option)
            && adt.variant_index.as_u32() == 0
            && adt.fields.is_empty())
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
pub(crate) fn resolve_runs_nothing<'tcx>(
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
/// What's known of a call: its arguments in `thir`, the first its receiver,
/// what of the crate is a value at hand (`AtHand`), and which structs JS
/// may make or give a value of (`FnPointers`), where the caller has them.
#[derive(Clone, Copy)]
pub(crate) struct CallArgs<'a, 'tcx> {
    pub(crate) thir: &'a Thir<'tcx>,
    pub(crate) args: &'a [ExprId],
    pub(crate) at_hand: &'a AtHand,
    pub(crate) pointers: Option<&'a FnPointers<'tcx>>,
}

/// `given`, where the caller has it, the call's arguments, each `None`
/// of which runs nothing whatever its type (`given_what_runs_nothing`),
/// and its receiver, of which a property read runs nothing where it's at
/// hand (`read_of_what_is_at_hand`); lowering's, which has none, asks of
/// the types alone, and takes no property read to run nothing.
pub(crate) fn binding_call_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    (runs_nothing_but_its_receiver(recognition, def_id, args, given)
        && receiver_runs_its_own(recognition, (def_id, args), given))
        || read_of_what_is_at_hand(recognition, def_id, given)
}

/// What a binding says it runs, of what it's given: nothing
/// (`binding_runs_nothing`), only what it's given, given only what runs
/// nothing (`given_what_runs_nothing`), or what its method does, an
/// optional call's (`optional_call_runs_nothing`). Of a method, its own
/// method's word, which `receiver_runs_its_own` says it is.
fn runs_nothing_but_its_receiver<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    binding_runs_nothing(recognition.tcx, def_id)
        || given_what_runs_nothing(recognition, def_id, args, given)
        || optional_call_runs_nothing(recognition, def_id, args)
}

/// Whether a binding's method call, `this.name(..)`, `this[key](..)` or
/// `this.name?.(..)`, calls the binding's own method, which its word is
/// of, not one a JS caller's object answers it with, a subclass's override
/// or a getter of the name, which may call back into the program (#2111):
/// of a receiver no JS caller may have handed over, by its type
/// (`read_may_run`), or one at hand (`AtHand`), which a builtin made. Of
/// a call whose arguments aren't known, lowering's or a load's, by its
/// declared `this` alone. What isn't a method call has no receiver to ask.
fn receiver_runs_its_own<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) {
        return true;
    }
    let (receiver, declared) = match js_form(tcx, def_id) {
        JsForm::Call(_) | JsForm::CallKeyed(_) if crate::bindings::is_method(tcx, def_id) => {
            let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
            (
                given.and_then(|given| Some((given, *given.args.first()?))),
                sig.skip_binder().inputs().first().copied(),
            )
        }
        // `js::optional_call((value, method), args)`: its path's value.
        JsForm::OptionalCall { .. } => (
            given.and_then(|given| {
                let path = crate::body_queries::strip(given.thir, *given.args.first()?);
                match &given.thir[path].kind {
                    ExprKind::Tuple { fields } => Some((given, *fields.first()?)),
                    _ => None,
                }
            }),
            args.types()
                .next()
                .and_then(|path| path.tuple_fields().first().copied()),
        ),
        _ => return true,
    };
    match (receiver, declared) {
        (Some((given, receiver)), _) => {
            let read = read_of(recognition, given.thir, receiver);
            (!crate::generic_facts::may_be_any_type(read) && !read_may_run(recognition, given.pointers, read))
                || given.at_hand.holds(recognition, given.thir, receiver)
        }
        (None, Some(this)) => !crate::generic_facts::may_be_any_type(this) && !read_may_run(recognition, None, this),
        (None, None) => false,
    }
}

/// Whether `def_id`, `js::optional_call`, `value.method?.(args)` (ADR
/// 1156), runs nothing: what the method its path names runs, of its
/// arguments' types, as a call of it would (#2082), of a receiver whose
/// method is the binding's own, as a plain call's (`receiver_runs_its_own`,
/// #2111). Not
/// `optional_call_unchecked`'s, whose value is asserted of another
/// contract, so its method may not be the binding's.
fn optional_call_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) || !matches!(js_form(tcx, def_id), JsForm::OptionalCall { unchecked: false }) {
        return false;
    }
    let Some(ty::Tuple(path)) = args.types().next().map(|path| path.kind()) else {
        return false;
    };
    let method = path.iter().find_map(crate::fn_def);
    method.is_some_and(|(method, method_args)| runs_nothing_but_its_receiver(recognition, method, method_args, None))
}

/// Whether `def_id` is a property read, `this.name` or `this[key]`, of a
/// value at hand, whose getters are none of the program's (#2106): one a
/// builtin made, `AtHand`'s. A value a JS caller could have handed over, a
/// parameter's, a field's or what a collection kept, may answer it by a
/// getter that calls back into the program, a borrow it meets counted.
fn read_of_what_is_at_hand<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    given: Option<CallArgs<'_, 'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) || !matches!(js_form(tcx, def_id), JsForm::Get(_) | JsForm::GetIndex) {
        return false;
    }
    let Some(given) = given else {
        return false;
    };
    let Some(&receiver) = given.args.first() else {
        return false;
    };
    // Of a type parameter's, its caller's to say (`generic_read`); of a
    // JS object's, or a struct's JS may make or give, the program's own
    // cast to `Unknown`, `js::get(js::unknown(&**chunk), "reason")`, only
    // what's at hand (#2109).
    let read = read_of(recognition, given.thir, receiver);
    !crate::generic_facts::may_be_any_type(read) && !read_may_run(recognition, given.pointers, read)
        || given.at_hand.holds(recognition, given.thir, receiver)
}

/// Of a property read, `this.name` or `this[key]`, of what a type
/// parameter's value is, `T: Node`'s `get sortIndex` of a heap's element,
/// or a method call of one that's otherwise the binding's word (#2111):
/// its type, which each caller's instantiation says may run code or not
/// (`read_may_run`), as a drop of a type parameter's is its caller's.
pub(crate) fn generic_read<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    given: Option<CallArgs<'_, 'tcx>>,
) -> Option<Ty<'tcx>> {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) {
        return None;
    }
    let method = match js_form(tcx, def_id) {
        JsForm::Get(_) | JsForm::GetIndex => false,
        JsForm::Call(_) | JsForm::CallKeyed(_) if crate::bindings::is_method(tcx, def_id) => true,
        _ => return None,
    };
    let given = given?;
    if method && !runs_nothing_but_its_receiver(recognition, def_id, args, Some(given)) {
        return None;
    }
    let read = read_of(recognition, given.thir, *given.args.first()?);
    crate::generic_facts::may_be_any_type(read).then(|| tcx.erase_and_anonymize_regions(read))
}

/// Whether a property read of a value of `ty` may meet a getter of the
/// program's, which may call back into it (#2106, #2109): of a JS object,
/// or of a struct JS may make or give a value of, through a reference, a
/// `Box`, an `Rc` or an `Arc`; not of a struct only the crate constructs,
/// which JS gives back as the crate gave it (ADR 0262's difference 60),
/// nor of std's, a `String`'s, nor a number's or anything else with no
/// getter of the program's. A struct's is by which struct it is; what else
/// names a type parameter may be any. Where which structs JS may make isn't known, any may.
pub(crate) fn read_may_run<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    pointers: Option<&FnPointers<'tcx>>,
    ty: Ty<'tcx>,
) -> bool {
    let mut ty = ty.peel_refs();
    while let ty::Adt(_, args) = ty.kind()
        && (ty.is_box() || recognition.is_std_type(ty, StdItem::Rc) || recognition.is_std_type(ty, StdItem::Arc))
    {
        ty = args.type_at(0).peel_refs();
    }
    if recognition.is_js_object(ty) {
        return true;
    }
    // The program's own struct, by which it is, whatever its arguments.
    if let ty::Adt(adt, _) = ty.kind()
        && adt.is_struct()
        && !recognition.is_std(adt.did())
    {
        return pointers.is_none_or(|pointers| pointers.js_may_make(adt.did()) || pointers.js_may_give(adt.did()));
    }
    crate::generic_facts::may_be_any_type(ty)
}

/// Of a field's read or write, `chunk.reason`, of the place `lhs` in
/// `given.thir`: JS's own property access, which a getter or a setter of
/// the program's may answer where JS may make or give the struct
/// (`read_may_run`, #2113), as a binding's read may, unless what it's of
/// is at hand. `Some` of the struct's type where it may, which, naming a
/// type parameter, each caller's instance settles.
pub(crate) fn field_access_may_run<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    given: CallArgs<'_, 'tcx>,
    lhs: ExprId,
) -> Option<Ty<'tcx>> {
    let read = read_of(recognition, given.thir, lhs);
    (read_may_run(recognition, given.pointers, read) && !given.at_hand.holds(recognition, given.thir, lhs))
        .then(|| recognition.tcx.erase_and_anonymize_regions(read))
}

/// The type of what `receiver`, in `thir`, reads: through a borrow, a
/// deref and a cast to a JS value, `js::unknown(x)`'s, `x`'s own.
fn read_of<'tcx>(recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, receiver: ExprId) -> Ty<'tcx> {
    let tcx = recognition.tcx;
    let mut at = crate::body_queries::strip(thir, receiver);
    loop {
        match &thir[at].kind {
            ExprKind::Deref { arg } | ExprKind::Borrow { arg, .. } => at = crate::body_queries::strip(thir, *arg),
            ExprKind::Call { fun, args, .. }
                if let Some((callee, _)) = crate::fn_def(thir[*fun].ty)
                    && is_binding(tcx, callee)
                    && matches!(js_form(tcx, callee), JsForm::This)
                    && let Some(&of) = args.first() =>
            {
                at = crate::body_queries::strip(thir, of);
            }
            _ => return thir[at].ty.peel_refs(),
        }
    }
}
