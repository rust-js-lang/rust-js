//! What a function's JS takes its `self` as: its first parameter, `this`
//! or nothing, as its body is lowered, a trait of classes' default is put
//! on a prototype and a binding's method is called; the one answer a JS
//! caller's signature asks (ADR 1091).

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use super::{Member, is_class_trait, member};

/// What a function's JS takes its `self` as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Receiver {
    /// Its first parameter, as Rust's is, or it has none: a free
    /// function's, an object of methods' (ADR 0047), a dictionary's entry's.
    First,
    /// `this`: a class's method or getter (ADR 0761), an object's own
    /// method (ADR 0943), a trait of classes' method, each class's
    /// prototype's (ADR 1035), and a binding's method its object is called
    /// by, `object.isPersistent()`.
    This,
    /// Nothing: a namespace's method, its module's function (ADR 1037).
    Absent,
}

/// What `function`'s body takes its `self` as, as it's lowered
/// (`receiver_as_this`): a member's of a class, an object or a namespace.
/// A trait of classes' default is lowered of its first parameter, into its
/// impl's dictionary, before each prototype takes it as `this`
/// (`defaults::prototype_defaults`).
pub(super) fn body_receiver(tcx: TyCtxt<'_>, function: DefId) -> Receiver {
    match member(tcx, function) {
        Some(Member::Namespace) => Receiver::Absent,
        Some(Member::Method | Member::Getter | Member::Own) => Receiver::This,
        _ => Receiver::First,
    }
}

/// What `function`'s JS takes its `self` as where a JS caller calls it:
/// its body's (`body_receiver`); a trait of classes' method's `this`, each
/// prototype's (`defaults::prototype_defaults`); and a binding's method's,
/// its object it's called on (`bindings::js_form`). The one answer of what
/// a JS caller gives it (`signatures::JsSignature`).
pub(crate) fn receiver(tcx: TyCtxt<'_>, function: DefId) -> Receiver {
    let own = body_receiver(tcx, function);
    if own != Receiver::First {
        return own;
    }
    let method = tcx.def_kind(function) == DefKind::AssocFn && tcx.associated_item(function).is_method();
    let prototype = (tcx.trait_of_assoc(function)).is_some_and(|of| is_class_trait(tcx, of));
    let bound = crate::lower::bindings::is_binding(tcx, function)
        && matches!(
            crate::lower::bindings::js_form(tcx, function),
            crate::lower::bindings::JsForm::Call(_)
        );
    match method && (prototype || bound) {
        true => Receiver::This,
        false => Receiver::First,
    }
}
