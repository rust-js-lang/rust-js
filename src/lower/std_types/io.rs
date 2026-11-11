//! std's byte sources (#1156): `&[u8]` and `Cursor<T>`, read through the
//! runtime's dictionary of each, `$sliceReader` and `$cursorReader(TAsRef)`,
//! whether the source's type is known or generic code's, one form of
//! both; and `Cursor<T>`, std's struct, `{ inner, pos }`, its `pos` a `u64`.
//! A `VecDeque<u8>`, read by `$dequeReader` where its type is known, but
//! its `read` and `fill_buf` (#1302). `io::empty()`, std's unit struct,
//! `undefined`, read by `$emptyReader`, and `io::repeat(byte)`, std's
//! struct, `{ byte }`, by `$repeatReader`. `BufReader<R>`, std's fields
//! flattened, `{ inner, buf, pos, filled }`, by `$bufReader(RRead)`.

use crate::js::{Expr, Prop, Stmt};
use crate::lower::bindings;
use crate::lower::calls::Call;
use crate::lower::generic_facts::Form;
use crate::lower::plain;
use crate::lower::recognition::{ByteSource, IoOp, Std, StdItem};
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
        out: &mut Vec<Stmt>,
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
                    None => match tcx.trait_impl_of_assoc(call.def_id) {
                        Some(imp) => {
                            let tr = (tcx.impl_trait_ref(imp))
                                .instantiate(tcx, call.generic_args)
                                .skip_normalization();
                            (tr.def_id, tr.args)
                        }
                        // `BufReader::seek_relative`, which its `Seek` impl
                        // calls: `Seek`'s of the `BufReader<R>`.
                        None => {
                            let imp = tcx.inherent_impl_of_assoc(call.def_id).expect("a source's method");
                            let reader = tcx
                                .type_of(imp)
                                .instantiate(tcx, call.generic_args)
                                .skip_normalization();
                            let seek = crate::lower::recognition::std_item(tcx, StdItem::IoSeek);
                            (seek, tcx.mk_args(&[reader.into()]))
                        }
                    },
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
                // A `&mut [u8]` writer's box re-points what it holds past
                // what's written (`$sliceWriter`, #1282): of a variable, a
                // handle on it, so the caller's slice is re-pointed too,
                // where a box made for the call would be dropped with it.
                let slice_writer = crate::lower::recognition::is_byte_slice(tr.self_ty().peel_refs());
                let receiver = match &receiver.kind {
                    crate::js::ExprKind::Object(props, ..)
                        if slice_writer
                            && let [crate::js::Prop::Field(key, held)] = props.as_slice()
                            && key == crate::js::HELD
                            && matches!(held.kind, crate::js::ExprKind::Var(_)) =>
                    {
                        Expr::handle(held.clone())
                    }
                    _ => receiver,
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
            IoOp::Inner => {
                let inner = Expr::member(next(), "inner");
                self.buf_reader_inner_back(&call, inner)?
            }
            // std's fields flattened, its `buf` a zeroed array of its
            // capacity: std's room for it checked first (ADR 0774).
            IoOp::BufReaderNew => {
                let inner = self.buf_reader_inner_given(&call, next());
                Expr::object(vec![
                    Prop::Field("inner".into(), inner),
                    Prop::Field("buf".into(), plain::filled(Expr::int(8192), Some(Expr::int(0)))),
                    Prop::Field("pos".into(), Expr::int(0)),
                    Prop::Field("filled".into(), Expr::int(0)),
                ])
            }
            IoOp::BufReaderWithCapacity => {
                // Each argument is evaluated, in order, before std asks the
                // room: where the room is asked, an inner made by a call is
                // made first, though the room fails.
                let mut capacity = next();
                if !capacity.reads_same() {
                    capacity = self.spill("capacity", capacity, out);
                }
                let mut check = Vec::new();
                self.room_for((None, capacity.clone()), Expr::int(1), call.span, &mut check);
                let mut inner = self.buf_reader_inner_given(&call, next());
                if !check.is_empty() && !inner.reads_same() {
                    inner = self.spill("inner", inner, out);
                }
                out.extend(check);
                Expr::object(vec![
                    Prop::Field("inner".into(), inner),
                    Prop::Field("buf".into(), plain::filled(capacity, Some(Expr::int(0)))),
                    Prop::Field("pos".into(), Expr::int(0)),
                    Prop::Field("filled".into(), Expr::int(0)),
                ])
            }
            IoOp::BufReaderBuffer => {
                let mut reader = next();
                if !reader.reads_same() {
                    reader = self.spill("reader", reader, out);
                }
                Expr::call_helper(
                    "$part",
                    vec![
                        Expr::member(reader.clone(), "buf"),
                        Expr::member(reader.clone(), "pos"),
                        Expr::member(reader, "filled"),
                    ],
                )
            }
            IoOp::BufReaderCapacity => Expr::member(Expr::member(next(), "buf"), "length"),
            IoOp::Empty => Expr::undefined(),
            IoOp::Repeat => Expr::object(vec![Prop::Field("byte".into(), next())]),
        }))
    }

    /// A `BufReader`'s inner, as `new` and `with_capacity` hold it: of a
    /// `&mut` to an object, generic code's box of it, which std's `impl Read
    /// for &mut R`, `$mutReader`, its `Read`, reads, `Form::Box`'s (ADR 0454,
    /// #1374); any other as it is.
    fn buf_reader_inner_given(&self, call: &Call<'_, 'tcx>, inner: Expr) -> Expr {
        let inputs = self.signature_of(call.def_id).0;
        let declared = *inputs.last().expect("a constructor of its reader");
        let held = self.instantiated(declared, call.generic_args);
        self.form_given(Form::Box, (call.def_id, declared, held), inner)
    }

    /// What a `BufReader`'s `get_ref` and `into_inner` give of its inner,
    /// held as `buf_reader_inner_given` holds it: the `&mut` to an object
    /// out of generic code's box (#1374); its `get_mut`, a `&mut` to that
    /// `&mut`, is refused before it's asked, as a value from its MIR. A
    /// `Cursor`'s inner as it is.
    fn buf_reader_inner_back(&self, call: &Call<'_, 'tcx>, inner: Expr) -> R<Expr> {
        let inputs = self.signature_of(call.def_id).0;
        let ty::Adt(adt, args) = inputs[0].peel_refs().kind() else {
            return Ok(inner);
        };
        if !crate::lower::recognition::is_buf_reader(self.tcx, adt.did()) {
            return Ok(inner);
        }
        let declared = args.type_at(0);
        let held = self.instantiated(declared, call.generic_args);
        let crossing = (call.def_id, declared, held);
        if !self.form_differs(Form::Box, crossing) {
            return Ok(inner);
        }
        Ok(self.form_back(Form::Box, crossing, inner))
    }
}
