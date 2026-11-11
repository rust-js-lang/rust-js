//! std's byte sources (#1156): `&[u8]` and `Cursor<T>`, read through the
//! runtime's dictionary of each, `$sliceReader` and `$cursorReader(TAsRef)`,
//! whether the source's type is known or generic code's, one form of
//! both; and `Cursor<T>`, std's struct, `{ inner, pos }`, its `pos` a `u64`.
//! A `VecDeque<u8>`, read by `$dequeReader` where its type is known, but
//! its `read` and `fill_buf` (#1302). `io::empty()`, std's unit struct,
//! `undefined`, read by `$emptyReader`, and `io::repeat(byte)`, std's
//! struct, `{ byte }`, by `$repeatReader`.

use crate::js::{Expr, Prop};
use crate::lower::bindings;
use crate::lower::calls::Call;
use crate::lower::recognition::{ByteSource, IoOp, Std};
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
                // A `VecDeque<u8>`'s own, but its `read` and `fill_buf`,
                // which read its front slice alone, which a JS array has
                // none of (#1302).
                let deque = self.recognition().byte_source(tr.self_ty()) == Some(ByteSource::Deque);
                let name = tcx.item_name(call.def_id);
                if deque && matches!(name.as_str(), "read" | "fill_buf") {
                    return Err(self.unsupported(
                        call.span,
                        &format!("a `VecDeque<u8>`'s `{name}`, which reads its front slice alone (#1302)"),
                    ));
                }
                let dictionary = match deque {
                    true => Expr::helper("$dequeReader"),
                    false => self.dictionary(tr, call.span)?,
                };
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
            // Its reader in generic code's box, read through `R`'s `Read`.
            IoOp::ReadByte => {
                let reader = next();
                let read = crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::IoRead);
                let tr = ty::TraitRef::new(self.tcx, read, [call.generic_args.type_at(0)]);
                let dictionary = self.dictionary(tr, call.span)?;
                Expr::call_helper("$readByte", vec![reader, dictionary])
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
            IoOp::Empty => Expr::undefined(),
            IoOp::Repeat => Expr::object(vec![Prop::Field("byte".into(), next())]),
        }))
    }
}
