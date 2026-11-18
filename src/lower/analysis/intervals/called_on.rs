//! What a call tells the MIR intervals of what it gives of a `Vec`: its
//! `deref` or `deref_mut`, a slice of its items, a copy of it (#1672 M3,
//! step 5). What a call may write, of a `Vec` lent it or one a pointer it
//! keeps reaches, is the effects query's (`analysis::Effects`, #2112 B);
//! and `carries_code`, what of a type may run code std's given it runs,
//! which it asks.

use super::*;
use crate::lower::generic_facts::is_type_parameter;
use crate::lower::recognition::{StdItem, StdPath, deref_call, is_std_crate, is_std_def, is_std_path};
use rustc_span::def_id::DefId;

impl<'tcx> Walk<'_, 'tcx> {
    /// A `Vec`'s `deref` or `deref_mut`, a slice of its items: of its
    /// length, a copy of it, so `swap(i, j)` of the slice is of the `Vec`.
    pub(super) fn derefed(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        local: Local,
        state: &State,
        after: &mut State,
    ) {
        if func
            .const_fn_def()
            .is_some_and(|(callee, _)| deref_call(self.tcx, callee))
            && let Some(receiver) = args.first()
            && (receiver.node.ty(&*self.body, self.tcx).builtin_deref(true))
                .is_some_and(|of| matches!(of.kind(), ty::Adt(adt, _) if is_std_def(self.tcx, adt.did(), StdItem::Vec)))
            && let Some(of) = receiver.node.place().and_then(|p| p.as_local())
        {
            after.copies.insert(local, state.root(of));
        }
    }
}

/// Whether `ty` carries code a std function given it may run, which may
/// write through a pointer kept of an escaped `Vec` (#1966): a closure, a
/// `dyn`, a function pointer, a function or a type not std's, a type
/// parameter; and, where the body's formatting may run code
/// (`formats_code`), std's `fmt::Arguments` and `Argument`, which hide
/// the `Display` impls they're made of.
pub(in crate::lower) fn carries_code<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, formats_code: bool) -> bool {
    ty.walk().any(|part| {
        part.as_type().is_some_and(|t| match *t.kind() {
            _ if is_type_parameter(t) => true,
            ty::Adt(adt, _) if formats_code && is_formatting(tcx, adt.did()) => true,
            ty::Adt(adt, _) => !is_std_crate(tcx, adt.did()),
            ty::FnDef(def, _) => !is_std_crate(tcx, def),
            ty::Closure(..)
            | ty::CoroutineClosure(..)
            | ty::Coroutine(..)
            | ty::Dynamic(..)
            | ty::FnPtr(..)
            | ty::Alias(..)
            | ty::Foreign(_) => true,
            _ => false,
        })
    })
}

/// Whether `id` is std's `fmt::Arguments` or `fmt::rt::Argument`.
fn is_formatting(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_path(tcx, id, StdPath::FmtArguments) || is_std_path(tcx, id, StdPath::RtArgument)
}
