//! A class's getter, `#[rust_js::getter]` (#357, rd679): which method is
//! one, its own or one of the class it extends, and what's refused of
//! one.

use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Is `def_id` marked `#[rust_js::getter]`?
pub(super) fn marked_getter(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("getter")])
        .next()
        .is_some()
}

/// Is `method`, a class's, one a getter is: of `&self` alone, no type
/// parameters, giving a value.
pub(super) fn reads_as_getter(tcx: TyCtxt<'_>, method: DefId) -> bool {
    let sig = tcx
        .fn_sig(method)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder();
    matches!(sig.inputs(), [receiver] if matches!(receiver.kind(), ty::Ref(_, _, ty::Mutability::Not)))
        && !sig.output().is_unit()
        && !tcx.asyncness(method).is_async()
        && tcx
            .generics_of(method)
            .own_params
            .iter()
            .all(|p| !p.kind.is_ty_or_const())
}

/// Is `item`, a trait's method, a getter of the binding's class the
/// trait is, which a class that extends it writes, webpack's `get type()`
/// of `ModuleDependency` (rd679): a binding crate's mark, which rustc
/// alone builds; a Rust call of it is its read, `dependency.type`.
pub(crate) fn extended_getter(tcx: TyCtxt<'_>, item: DefId) -> bool {
    let trait_id = tcx.parent(item);
    tcx.def_kind(item) == rustc_hir::def::DefKind::AssocFn
        && marked_getter(tcx, item)
        && super::base_class(tcx, trait_id).is_some()
        && !crate::lower::bindings::is_binding(tcx, item)
}

/// Is `own`, a method of a class's trait impl, a getter: one the trait
/// marks `#[rust_js::getter]`, as webpack's `ModuleDependency` declares
/// `get type()`, which a class that extends it overrides with its own
/// (rd679). The trait's mark says it, as the class it binds has it; the
/// impl's says it again where its JS is, which [`trait_getter_misuse`]
/// holds to the trait's.
pub(super) fn trait_getter(tcx: TyCtxt<'_>, own: DefId) -> bool {
    tcx.trait_item_of(own).is_some_and(|item| marked_getter(tcx, item))
}

/// What's refused of `own`, a method of a class's impl of the binding's
/// class it extends or of a trait of classes, as a getter (rd679): the
/// trait's getter not marked on it, which would read as a method; its mark
/// where the trait's is a method, which a getter would hide; the trait's
/// mark of a method it binds, `get ..` by its `link_name`; or a getter of
/// other than `&self` alone giving a value. `None` of what's fine.
pub(super) fn trait_getter_misuse(tcx: TyCtxt<'_>, own: DefId) -> Option<String> {
    let item = tcx.trait_item_of(own)?;
    let name = crate::lower::bindings::fn_name(tcx, item);
    let of = tcx.item_name(tcx.parent(item));
    match (marked_getter(tcx, item), marked_getter(tcx, own)) {
        (true, _) if crate::lower::bindings::is_binding(tcx, item) => Some(format!(
            "getter of `{of}`'s is bound by its `link_name`, `\"get {name}\"`, not `#[rust_js::getter]`, which marks one its class writes"
        )),
        (true, false) => Some(format!(
            "method is a getter where `{of}`'s `{name}` is one, `get {name}()`: mark it `#[cfg_attr(rust_js, rust_js::getter)]` as the trait's is"
        )),
        (false, true) => Some(format!(
            "getter overrides a getter, and `{of}`'s `{name}` is a method, which it would hide: mark the trait's `#[cfg_attr(rust_js, rust_js::getter)]` where its class's is `get {name}()`"
        )),
        (true, true) if !reads_as_getter(tcx, own) => Some(
            "getter is a method of `&self` alone that gives a value, `fn left_diagonal(&self) -> i32`, read `position.leftDiagonal`"
                .to_owned(),
        ),
        _ => None,
    }
}
