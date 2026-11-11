//! A cast of or to a raw pointer (ADR 0407): another pointer to the same
//! place, the same handle, where JS has what it points to there, an
//! array's as its first item's and its first item's as the array's,
//! checked as it's made, a `MaybeUninit`'s as its value's (ADR 0332), a
//! struct's as its field's at offset 0; `0 as *const T`, `null`. What
//! reads one type's memory as another's, or an address, is refused.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A cast of or to a raw pointer, `kind`'s, of `value`: another pointer
    /// to the same place, the same handle, where what it points to stays
    /// what it is or is `()`; `0 as *const T`, `null`. An address, a
    /// pointer made a number or one made of a number, isn't JS's: refused.
    /// `None` of a cast of no pointer. What it reads twice is spilled into
    /// `out` first.
    pub(in crate::lower) fn pointer_cast(
        &mut self,
        kind: mir::CastKind,
        value: Expr,
        (from, to): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        match kind {
            // A slice's as one of what's at the same place, `*mut
            // [MaybeUninit<T>]` as `*mut [T]` (ADR 0332): the slice itself
            // (ADR 0482).
            mir::CastKind::PtrToPtr
                if let (Some((a, _)), Some((b, _))) = (raw_pointee(self.tcx, from), raw_pointee(self.tcx, to))
                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())
                    && self.same_place(*a, *b) =>
            {
                Ok(Some(value))
            }
            // A slice's as an array's of its items, ppv-lite86's `input as
            // *const [u8] as *const [u8; 16]`: its first item's as the whole
            // array's, `recast`'s, as of a pointer into an array (#1402).
            mir::CastKind::PtrToPtr
                if let (Some((ty::Slice(item), _)), Some((b, _))) = (
                    raw_pointee(self.tcx, from).map(|(a, m)| (*a.kind(), m)),
                    raw_pointee(self.tcx, to),
                ) && matches!(b.kind(), ty::Array(of, _) if self.same_place(item, *of)) =>
            {
                let first = Expr::call(Expr::helper("$ptr"), vec![value, Expr::int(0)]);
                Ok(Some(self.recast(first, item, b, span)?))
            }
            // A slice's as one of another item's, rand_core's `results as
            // *const [u64] as *const [u32]`, its length kept: a view of as
            // many of them from where its first item is, that item's pointer
            // `recast`, as `slice::from_raw_parts_mut` of it is (#1419). Read
            // once for each, so a slice a variable holds; one a call makes,
            // generic code's `self.results.as_ref()` of an `R::Results:
            // AsRef<[u64]>`, BlockRng64's own, spilled first (#1420).
            mir::CastKind::PtrToPtr
                if let (Some((ty::Slice(a), _)), Some((ty::Slice(b), _))) = (
                    raw_pointee(self.tcx, from).map(|(a, m)| (*a.kind(), m)),
                    raw_pointee(self.tcx, to).map(|(b, m)| (*b.kind(), m)),
                ) && !self.same_place(a, b) =>
            {
                let value = match value.reads_same() {
                    true => value,
                    false => self.spill("items", value, out),
                };
                let first = Expr::call(Expr::helper("$ptr"), vec![value.clone(), Expr::int(0)]);
                let retyped = self.recast(first, a, b, span)?;
                let length = Expr::member(value, "length");
                Ok(Some(Expr::call(Expr::helper("$ptrSliceMut"), vec![retyped, length])))
            }
            // A slice's, which is the slice, as its first item's: `$ptr(items, 0)`.
            mir::CastKind::PtrToPtr
                if let (Some((ty::Slice(item), _)), Some((b, _))) = (
                    raw_pointee(self.tcx, from).map(|(a, m)| (*a.kind(), m)),
                    raw_pointee(self.tcx, to),
                ) && self.same_place(item, b) =>
            {
                Ok(Some(Expr::call(Expr::helper("$ptr"), vec![value, Expr::int(0)])))
            }
            // A pair `dyn`'s pointer as its value's, `z as *const A`: a
            // pointer to the pair's value (ADR 0562).
            mir::CastKind::PtrToPtr
                if let Some((a, _)) = raw_pointee(self.tcx, from)
                    && matches!(a.kind(), ty::Dynamic(..))
                    && self.pointee_is_itself(a)
                    && let Some((b, _)) = raw_pointee(self.tcx, to)
                    && self.is_handle_pointer(to) =>
            {
                Ok(Some(self.pointer_to(
                    Expr::member(value, js::HELD),
                    b,
                    Offset::Unsaid,
                    span,
                )?))
            }
            mir::CastKind::PtrToPtr if self.is_handle_pointer(from) && self.is_handle_pointer(to) => {
                let (ty::RawPtr(a, _), ty::RawPtr(b, _)) = (from.kind(), to.kind()) else {
                    unreachable!("pointers")
                };
                Ok(Some(self.recast(value, *a, *b, span)?))
            }
            // A reference to a value of no size as a pointer, `&() as *const
            // ()`: the dangling address (ADR 0562).
            mir::CastKind::PtrToPtr
                if let Some((pointee, _)) = raw_pointee(self.tcx, to)
                    && let Some(dangling) = self.dangling_pointer(pointee, span)? =>
            {
                Ok(Some(dangling))
            }
            // A function as a pointer, `f as *const u8`: the function, an
            // allocation of its own, whose address is its (ADR 0562).
            mir::CastKind::FnPtrToPtr if self.is_handle_pointer(to) => Ok(Some(value)),
            // `n as *const T`: `null` of 0, else the place at `n`, or a
            // pointer with no place (ADR 0562); of any integer, through
            // `usize`, a number, which a 64-bit one's BigInt isn't.
            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {
                Some(0) => Ok(Some(Expr::null())),
                _ => {
                    let (pointee, _) = raw_pointee(self.tcx, to).expect("a pointer");
                    let addr = self.cast(value, from, self.tcx.types.usize, span)?;
                    Ok(Some(Expr::call(
                        Expr::helper("$ptrFrom"),
                        self.at_address(addr, pointee, span)?,
                    )))
                }
            },
            // `p as usize`: its address; as another integer, `usize`'s.
            mir::CastKind::PointerExposeProvenance if self.is_handle_pointer(from) => {
                let (pointee, _) = raw_pointee(self.tcx, from).expect("a pointer");
                let size = self.pointee_size(pointee, span)?;
                let addr = Expr::call(Expr::helper("$addr"), vec![value, size]);
                Ok(Some(self.cast(addr, self.tcx.types.usize, to, span)?))
            }
            _ => Ok(None),
        }
    }

    /// A pointer to an `a` as one to a `b`, where JS has it at the same
    /// place: the same handle, of the same type, `()`, or a `MaybeUninit`
    /// of it, which is its value (ADR 0332), an array or a slice of them
    /// too, or a reference to one (#1283); an
    /// array's, its first item's, arrayvec's `self.xs.as_ptr() as *const
    /// T`; and its first item's, the whole array's, checked as it's made,
    /// `$ptrWhole`. In generic code, where whether it is depends on the
    /// type, arrayvec's `ptr.cast::<u8>()` of a type with no size: checked
    /// where it's made, `$ptrRecast`. Else it reads one type's memory as
    /// another's, which is refused.
    pub(in crate::lower) fn recast(&mut self, pointer: Expr, a: Ty<'tcx>, b: Ty<'tcx>, span: Span) -> R<Expr> {
        if self.same_place(a, b) || a.is_unit() || b.is_unit() {
            return Ok(pointer);
        }
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let value = |t: Ty<'tcx>| self.recognition().uninit_of(t).unwrap_or(t);
        let (a, b) = (value(a), value(b));
        // Bytes are `u8`s: an `i8`'s items are numbers of their sign, whose
        // bytes are their codec's, as any other integer's are (#1253).
        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8));
        let integer = |t: Ty<'tcx>| Num::of(t).is_some_and(|n| !n.float());
        match (a.kind(), b.kind()) {
            (ty::Array(item, _), _) if self.same_place(*item, b) => Ok(self.first_item_pointer(pointer)),
            (_, ty::Array(item, length)) if self.same_place(a, *item) && !is_bytes(a) => {
                Ok(helper("$ptrWhole", vec![pointer, self.const_arg(*length, span)?]))
            }
            // From bytes: an integer, an `i8` too, or a `[u8; N]`, read
            // where they are; another type, what was cast to bytes.
            (_, _) if is_bytes(a) && integer(b) => {
                let width = self.pointee_size(b, span)?;
                let signed = Expr::bool(b.is_signed());
                Ok(helper("$ptrRetyped", vec![pointer, width, signed]))
            }
            (_, ty::Array(item, length)) if is_bytes(a) && is_bytes(*item) => {
                let width = self.const_arg(*length, span)?;
                Ok(helper("$ptrRetyped", vec![pointer, width, Expr::str("bytes")]))
            }
            (_, _) if is_bytes(a) => Ok(helper("$ptrTyped", vec![pointer])),
            // To bytes: plain data's, a struct's or an integer's, an `i8`'s
            // too, as wasm32 holds them, through its codec (ADR 0562); of
            // what has none, an allocation's slots given to the allocator
            // (ADR 0417), cast back only.
            (_, _) if is_bytes(b) => Ok(match self.plain_codec(a, span)? {
                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),
                None => helper("$ptrBytes", vec![pointer]),
            }),
            // A struct's pointer as its first field's, anyhow's `*const
            // ErrorImpl` as `*const &'static ErrorVTable`: the handle on the
            // field rustc lays out at offset 0 (ADR 0562).
            (ty::Adt(adt, args), _)
                if adt.is_struct()
                    && let Some(field) = self.field_at_zero(a, args, b) =>
            {
                let object = self.pointee_place(pointer);
                let key = crate::lower::bindings::field_key(self.tcx, field);
                self.pointer_to(self.field_member(object, &key), b, Offset::At(0), span)
            }
            // Two layouts of one `#[repr(C)]` struct, anyhow's `ErrorImpl<E>`
            // as its prefix `ErrorImpl<()>` and back: the same handle, its
            // fields read by name (ADR 0562).
            (ty::Adt(x, _), ty::Adt(y, _)) if x.did() == y.did() && x.is_struct() && x.repr().c() => Ok(pointer),
            // Between integers of two widths or two signs, an `i8`'s and a
            // `u8`'s too, and plain data read as an integer or as bytes:
            // through the bytes.
            (_, _)
                if (integer(a) || self.plain_codec(a, span)?.is_some())
                    && (integer(b) || matches!(b.kind(), ty::Array(item, _) if is_bytes(*item))) =>
            {
                let bytes = self.recast(pointer, a, self.tcx.types.u8, span)?;
                self.recast(bytes, self.tcx.types.u8, b, span)
            }
            // A whole array's pointer as its first item's, of a type each of
            // whose impls of the item's trait is an array of it, smallvec's
            // `*const A` as `*const A::Item` (ADR 0417).
            _ if self.is_array_item_of(b, a) => Ok(helper("$ptrFirst", vec![pointer])),
            _ if a.has_param() || b.has_param() => Ok(helper("$ptrRecast", vec![pointer])),
            _ => Err(refuse(
                self.tcx,
                span,
                Refused::Memory("a pointer read as another type's"),
            )),
        }
    }

    /// Whether `item` is `<of as Tr>::X`, and each impl of `Tr` is of an array
    /// whose item is its `X`: `A::Item` of smallvec's `A: Array`.
    fn is_array_item_of(&self, item: Ty<'tcx>, of: Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) = *item.kind()
        else {
            return false;
        };
        let erased = |t: Ty<'tcx>| tcx.erase_and_anonymize_regions(t);
        tcx.trait_of_assoc(def_id).is_some_and(|trait_id| {
            erased(alias.self_ty()) == erased(of) && impls_are_arrays(tcx, trait_id, Some(def_id))
        })
    }

    /// Whether `ty` is a type parameter bound by a trait each impl of which
    /// is of an array, smallvec's `A: Array`: what it is is an array, whose
    /// slots, uninit, are written an item at a time (ADR 0417).
    pub(in crate::lower) fn is_array_param(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
            && self.typing_env.param_env.caller_bounds().iter().any(|clause| {
                clause.as_trait_clause().is_some_and(|bound| {
                    bound.self_ty().skip_binder() == ty && impls_are_arrays(self.tcx, bound.def_id(), None)
                })
            })
    }

    /// The field of a struct `of` that rustc's layout puts at its offset 0
    /// and that is a `target` in JS, `same_place`: `None` of a struct whose
    /// layout isn't known here, or that has none.
    fn field_at_zero(
        &self,
        of: Ty<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
        target: Ty<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let ty::Adt(adt, _) = of.kind() else {
            return None;
        };
        let layout = self.layout(of)?;
        adt.non_enum_variant()
            .fields
            .iter_enumerated()
            .find_map(|(index, field)| {
                let placed = layout.fields.offset(index.index()).bytes() == 0;
                (placed && self.same_place(self.field_ty(field, args), target)).then_some(field)
            })
    }

    /// Whether what an `a` and a `b` are is one value in JS: the same type,
    /// or a `MaybeUninit` of it, which is its value (ADR 0332), or arrays of
    /// as many of those.
    fn same_place(&self, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
        let value = |t: Ty<'tcx>| {
            self.tcx
                .erase_and_anonymize_regions(self.recognition().uninit_of(t).unwrap_or(t))
        };
        let (a, b) = (value(a), value(b));
        match (a.kind(), b.kind()) {
            _ if a == b => true,
            (ty::Array(x, n), ty::Array(y, m)) => n == m && self.same_place(*x, *y),
            // Of a slice of them, or a reference to one, the same view, httparse's
            // `&mut [T]` as a `&mut [MaybeUninit<T>]` (#1283).
            (ty::Slice(x), ty::Slice(y)) => self.same_place(*x, *y),
            (ty::Ref(_, x, p), ty::Ref(_, y, q)) => p == q && self.same_place(*x, *y),
            _ => false,
        }
    }

    /// An array's pointer as its first item's, `arr.as_ptr()` or `&arr as
    /// *const T`: `$ptr(arr, 0)`.
    pub(in crate::lower) fn first_item_pointer(&self, pointer: Expr) -> Expr {
        let array = self.pointee_place(pointer);
        Expr::call(Expr::helper("$ptr"), vec![array, Expr::int(0)])
    }
}
