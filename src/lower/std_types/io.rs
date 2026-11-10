//! std's byte sources (#1156): `&[u8]` and `Cursor<T>`, read through the
//! runtime's dictionary of each, `$sliceReader` and `$cursorReader(TAsRef)`,
//! whether the source's type is known or generic code's, one form of
//! both; and `Cursor<T>`, std's struct, `{ inner, pos }`, its `pos` a `u64`.

use crate::js::{Expr, Prop};
use crate::lower::bindings;
use crate::lower::calls::Call;
use crate::lower::recognition::{IoOp, Std};
use crate::lower::{FnCx, R};
use rustc_middle::ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of std's io a recognizer answers: `None` if `known` is
    /// another.
    pub(in crate::lower) fn io_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> R<Option<Expr>> {
        let Std::Io(op) = known else {
            return Ok(None);
        };
        let mut next = || values.next().expect("rustc checked the arguments");
        Ok(Some(match op {
            IoOp::Dictionary => {
                let tcx = self.tcx;
                let (trait_id, args) = match tcx.trait_of_assoc(call.def_id) {
                    Some(trait_id) => (trait_id, call.generic_args),
                    None => {
                        let imp = tcx.trait_impl_of_assoc(call.def_id).expect("a source's impl's method");
                        let tr = (tcx.impl_trait_ref(imp))
                            .instantiate(tcx, call.generic_args)
                            .skip_normalization();
                        (tr.def_id, tr.args)
                    }
                };
                let tr =
                    ty::TraitRef::new_from_args(tcx, trait_id, tcx.mk_args(&args[..tcx.generics_of(trait_id).count()]));
                let dictionary = self.dictionary(tr, call.span)?;
                // The source as generic code gives it: in a box.
                let receiver = next();
                let receiver = match self.mut_ref_form(tr.self_ty()).is_cell() {
                    true => receiver,
                    false => crate::lower::mut_refs::boxed(receiver),
                };
                let mut given = vec![receiver];
                given.extend(values.by_ref());
                let name = bindings::fn_name(tcx, call.def_id);
                let called = Expr::call(Expr::member(dictionary, name), given);
                // `read_exact`'s, thrown, caught where it's a value (ADR 0462).
                self.fmt_result_value(call.def_id, call.generic_args, called)
            }
            IoOp::AppendToString => {
                let buf = next();
                let f = next();
                Expr::call_helper("$appendToString", vec![buf, f])
            }
            IoOp::CursorNew => Expr::object(vec![
                Prop::Field("inner".into(), next()),
                Prop::Field("pos".into(), Expr::bigint(0)),
            ]),
            IoOp::CursorPosition => Expr::member(next(), "pos"),
            IoOp::CursorSetPosition => {
                let cursor = next();
                Expr::assigned(Expr::member(cursor, "pos"), next())
            }
            IoOp::CursorInner => Expr::member(next(), "inner"),
        }))
    }
}
