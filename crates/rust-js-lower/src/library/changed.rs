//! What a library's contract says its code changes in place (ADR 0914,
//! amended), which its JS copies, and which another crate's JS sharing a
//! value with it then copies for too (ADR 0100, amended, #1871).

use super::{Foreign, head_key, may_be_shape};
use rustc_middle::ty::Ty;

impl<'tcx> Foreign<'_, 'tcx> {
    /// Does any of the crate's libraries change a value of `ty` in place, as
    /// its contract says? What its JS shares with the crate's is
    /// changed only by code that changes it: the crate's own, a library's,
    /// whose contract lists it, or a consumer's, which compiles its own of
    /// what may copy it (ADR 0914, amended). So the crate copies a library's
    /// type, a tuple, an array or a `Vec` that one changes, not each one
    /// another crate might (ADR 0100, amended, #1871). `@rust-js/std`'s
    /// changes are its calls', which the crate's own analysis sees.
    pub(crate) fn changed_by_a_library(&self, ty: Ty<'tcx>) -> bool {
        (self.dependencies.libraries.values())
            .filter(|library| !library.builtin)
            .any(|library| (library.changed.iter()).any(|shape| may_be_shape(self.tcx, shape, ty)))
    }

    /// And of a `Vec` of `item`s, whatever its allocator: one a library
    /// changes, or of a shape that may be any.
    pub(crate) fn changes_a_vec_of(&self, item: Ty<'tcx>) -> bool {
        let Some(vec) = crate::recognition::opt_std_item(self.tcx, crate::recognition::StdItem::Vec) else {
            return self.any();
        };
        let vec = head_key(self.tcx, &crate::analysis::Head::Def("", vec));
        (self.dependencies.libraries.values())
            .filter(|library| !library.builtin)
            .flat_map(|library| library.changed.iter())
            .any(|shape| {
                shape.of.as_ref().is_none_or(|of| *of == vec)
                    && shape.args.first().is_none_or(|arg| may_be_shape(self.tcx, arg, item))
            })
    }
}
