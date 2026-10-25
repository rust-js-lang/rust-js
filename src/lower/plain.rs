//! Plain data (ADR 0408): a value of numbers, `bool`s and `char`s alone, in
//! tuples, arrays and structs, each at the offset rustc lays it out at for
//! wasm32. What `mem::zeroed` makes, all of them zero, and what `transmute`
//! reads one type's bytes as another's: the part at the same offset, of the
//! same size, retyped, else read from the bytes of the whole, little-endian
//! as wasm32 is. JS has no bytes to address, but plain data's are its
//! numbers', which `$toBytes` and `$fromBytes` write and read (ADR 0156).

use rustc_abi::VariantIdx;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::calls::Call;
use super::recognition::Std;
use super::refusal::{Refused, refuse};
use super::representation::Num;
use super::{FnCx, R};
use crate::js::{self, Expr, Op, Stmt, StmtKind};

/// A plain value's layout: a number, a `bool` or a `char`, or its parts,
/// each at its offset in it.
enum Plain<'tcx> {
    Scalar(Ty<'tcx>),
    /// A tuple's, an array's or a struct's fields, in order.
    Parts(Ty<'tcx>, Vec<(u64, Plain<'tcx>)>),
}

/// One of a plain value's scalars: where it is, its type, and what reads it.
struct Read<'tcx> {
    offset: u64,
    ty: Ty<'tcx>,
    value: Expr,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `ty`'s layout for wasm32, the target rust-js checks programs for (ADR
    /// 0090): what `size_of`, `align_of` and plain data's offsets are. Of a
    /// type without parameters, as codegen asks: 1.98 finds an `async fn`'s
    /// future too generic to lay out otherwise (ADR 0109).
    /// None of a type with a type parameter, which its caller lays out.
    pub(in crate::lower) fn layout(&self, ty: Ty<'tcx>) -> Option<TyAndLayout<'tcx>> {
        if ty.has_param() {
            return None;
        }
        self.tcx
            .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
            .ok()
    }

    /// `std::mem::transmute::<A, B>(a)`, `transmute_unchecked`: `None` if
    /// `known` is another.
    pub(in crate::lower) fn transmute_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        if known != Std::Transmute {
            return Ok(None);
        }
        let mut types = call.generic_args.types();
        let (from, to) = (
            types.next().expect("transmute's source"),
            types.next().expect("its target"),
        );
        let value = values.next().expect("rustc checked the arguments");
        self.transmute(value, (from, to), call.span, out).map(Some)
    }

    /// Plain data's codec, for a pointer to its bytes (ADR 0562): its size,
    /// `(value) => bytes` and `(bytes) => value`, as `transmute` writes
    /// each way. `None` of what isn't plain data.
    pub(super) fn plain_codec(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<(Expr, Expr, Expr)>> {
        let Some(layout) = self.layout(ty).filter(|_| self.plain(ty).is_some()) else {
            return Ok(None);
        };
        let size = layout.size.bytes();
        let bytes = Ty::new_array(self.tcx, self.tcx.types.u8, size);
        let arrow = |cx: &mut Self, name: &str, from: Ty<'tcx>, to: Ty<'tcx>| -> R<Expr> {
            let mut body = Vec::new();
            let made = cx.transmute(Expr::var(name), (from, to), span, &mut body)?;
            body.push(StmtKind::Return(Some(made)).at(js::Span::NONE));
            Ok(Expr::arrow(vec![name.into()], body))
        };
        let encode = arrow(self, "value", ty, bytes)?;
        let decode = arrow(self, "bytes", bytes, ty)?;
        Ok(Some((Expr::int(i128::from(size)), encode, decode)))
    }

    /// A union's field of `held` read as `read` (ADR 0562): its bytes as
    /// `transmute` reads them, `(value) => ..`; `None` where either isn't
    /// plain data.
    pub(super) fn plain_decoder(&mut self, held: Ty<'tcx>, read: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if self.plain(held).is_none() || self.plain(read).is_none() {
            return Ok(None);
        }
        // A field read past the one held reads bytes Rust leaves undefined:
        // none, so the read is rust-js's error (ADR 0406).
        if let (Some(held), Some(read)) = (self.layout(held), self.layout(read))
            && held.size.bytes() < read.size.bytes()
        {
            return Ok(None);
        }
        let mut body = Vec::new();
        let made = self.transmute(Expr::var("value"), (held, read), span, &mut body)?;
        body.push(StmtKind::Return(Some(made)).at(js::Span::NONE));
        Ok(Some(Expr::arrow(vec!["value".into()], body)))
    }

    /// `value`, a `from`, as a `to` of the same bytes: itself, of a type
    /// that differs only in its lifetimes, else of plain data, its parts;
    /// bytes as text, `Box<[u8]>` as `Box<str>`, decoded (ADR 0562).
    fn transmute(&mut self, value: Expr, (from, to): (Ty<'tcx>, Ty<'tcx>), span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let erased = |ty| self.tcx.erase_and_anonymize_regions(ty);
        if erased(from) == erased(to) {
            return Ok(value);
        }
        let pointee = |ty: Ty<'tcx>| match ty.kind() {
            ty::Ref(_, inner, _) => Some(*inner),
            ty::Adt(_, args) if ty.is_box() => Some(args.type_at(0)),
            _ => None,
        };
        if let (Some(bytes), Some(text)) = (pointee(from), pointee(to))
            && matches!(bytes.kind(), ty::Slice(item) if *item == self.tcx.types.u8)
            && text.is_str()
        {
            return Ok(Expr::call(Expr::helper("$utf8Decode"), vec![value]));
        }
        if let Some(value) = self.transparent_transmute(value.clone(), (from, to)) {
            return Ok(value);
        }
        let (Some(source), Some(target)) = (self.plain(from), self.plain(to)) else {
            let what = format!("a `transmute` of `{from}` as `{to}`, which aren't numbers alone");
            return Err(refuse(self.tcx, span, Refused::Memory(&what)));
        };
        let value = match value.reads_same() {
            true => value,
            false => self.spill("value", value, out),
        };
        let mut reads = Vec::new();
        self.plain_reads(value.clone(), &source, 0, &mut reads);
        // A reference to a type parameter's object is a scalar with no layout
        // of its own (ADR 0562).
        let size = match self.layout(from) {
            Some(layout) => layout.size.bytes(),
            None => self.scalar_bytes(from),
        };
        // A `[u8; N]` is the bytes themselves.
        if self.is_bytes(to) {
            return self.bytes_of(&value, &reads, (from, size), span);
        }
        // The bytes of the whole, made once, where a part's aren't another's.
        let mut bytes: Option<Expr> = None;
        self.plain_made(
            &target,
            0,
            &mut |cx, offset, ty| {
                let bytes_of = cx.scalar_bytes(ty);
                if let Some(read) = reads
                    .iter()
                    .find(|r| r.offset == offset && cx.scalar_bytes(r.ty) == bytes_of)
                {
                    return cx.retyped(read.value.clone(), (read.ty, ty), span);
                }
                let all = match bytes.clone() {
                    Some(all) => all,
                    None => {
                        let made = cx.bytes_of(&value, &reads, (from, size), span)?;
                        let all = match made.reads_same() {
                            true => made,
                            false => cx.spill("bytes", made, out),
                        };
                        bytes = Some(all.clone());
                        all
                    }
                };
                cx.read_bytes(&all, (offset, size), ty, span)
            },
            span,
        )
    }

    /// `value`, a `from`, as a `to`, where one is a `#[repr(transparent)]`
    /// struct of the other, through as many as wrap it: the value itself,
    /// which each struct is (ADR 0584). By value, or through a reference of
    /// either kind; not a pointer.
    fn transparent_transmute(&self, value: Expr, (from, to): (Ty<'tcx>, Ty<'tcx>)) -> Option<Expr> {
        let (from, to) = match (from.kind(), to.kind()) {
            (ty::Ref(_, from, a), ty::Ref(_, to, b)) if a == b => (*from, *to),
            (ty::Ref(..), _) | (_, ty::Ref(..)) => return None,
            _ if [from, to]
                .iter()
                .any(|&ty| super::pointers::raw_pointee(self.tcx, ty).is_some()) =>
            {
                return None;
            }
            _ => (from, to),
        };
        let held = |mut ty: Ty<'tcx>| {
            while let ty::Adt(adt, _) = ty.kind()
                && adt.is_struct()
                && adt.repr().transparent()
                && let Some(field) = self.transparent(ty)
            {
                ty = field;
            }
            self.tcx.erase_and_anonymize_regions(ty)
        };
        (held(from) == held(to)).then_some(value)
    }

    /// `ty`'s parts, if it's plain data: a number, a `bool`, a `char`, or a
    /// tuple, an array of a known length, or a struct of the crate's or a
    /// dependency's, of them. None of any other, as std's structs are JS's
    /// own values, or of a type parameter, which isn't laid out here.
    fn plain(&self, ty: Ty<'tcx>) -> Option<Plain<'tcx>> {
        // Rust's own, not std's structs JS has as numbers, as a `Duration`'s
        // nanoseconds, whose bytes aren't its value's.
        let number = matches!(ty.kind(), ty::Int(_) | ty::Uint(_) | ty::Float(_)) && Num::of(ty).is_some();
        // A thin pointer is 4 bytes, its address's; a reference to an object
        // too (ADR 0562).
        let sized = |pointee: Ty<'tcx>| pointee.is_sized(self.tcx, self.typing_env);
        let pointer = super::pointers::raw_pointee(self.tcx, ty).is_some_and(|(pointee, _)| sized(pointee))
            || matches!(ty.kind(), ty::Ref(_, pointee, _)
                if sized(*pointee) && (self.is_object(*pointee) && !self.is_cell_pointee(*pointee) || self.is_unknown(*pointee)));
        if number || ty.is_bool() || ty.is_char() || pointer {
            return Some(Plain::Scalar(ty));
        }
        // A `MaybeUninit<T>` is its `T`, laid out alike (ADR 0332).
        if let Some(held) = self.recognition().uninit_of(ty) {
            return self.plain(held);
        }
        let layout = self.layout(ty)?;
        let fields: Vec<Ty<'tcx>> = match ty.kind() {
            ty::Tuple(items) => items.to_vec(),
            ty::Array(item, len) => vec![*item; len.try_to_target_usize(self.tcx)? as usize],
            ty::Adt(adt, args) if adt.is_struct() && self.is_rust_adt(adt.did()) => {
                adt.all_fields().map(|f| self.field_ty(f, args)).collect()
            }
            _ => return None,
        };
        let parts = fields
            .into_iter()
            .enumerate()
            .map(|(i, field)| Some((layout.fields.offset(i).bytes(), self.plain(field)?)))
            .collect::<Option<Vec<_>>>()?;
        Some(Plain::Parts(ty, parts))
    }

    /// Whether a `ty` is plain data whose scalars fill its `size` bytes,
    /// with no padding between them: a value of it holds every byte, so a
    /// union's field of it can be held in place of another of that size,
    /// and lose none (ADR 0562).
    pub(super) fn fills(&self, ty: Ty<'tcx>, size: u64) -> bool {
        fn scalars<'tcx>(cx: &FnCx<'_, 'tcx>, plain: &Plain<'tcx>) -> Option<u64> {
            match plain {
                Plain::Scalar(ty) => Some(cx.layout(*ty)?.size.bytes()),
                Plain::Parts(_, parts) => parts.iter().map(|(_, part)| scalars(cx, part)).sum(),
            }
        }
        self.layout(ty).is_some_and(|layout| layout.size.bytes() == size)
            && self.plain(ty).and_then(|plain| scalars(self, &plain)) == Some(size)
    }

    /// A `ty` of all zero bytes, where that's one: a number's `0`, `false`,
    /// `'\0'`, and a tuple, an array or a struct of them.
    pub(in crate::lower) fn zero(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // An array of a length generic code is given, `[MaybeUninit<u8>;
        // CAP]`, arrayvec's: as many zeros.
        if let ty::Array(item, len) = *ty.kind()
            && len.try_to_target_usize(self.tcx).is_none()
        {
            let n = self.const_arg(len, span)?;
            let zero = self.zero(item, span)?;
            return Ok(filled(n, Some(zero)));
        }
        let Some(plain) = self.plain(ty) else {
            return Err(self.unsupported(span, &format!("a zeroed `{ty}`, which may be no value of it")));
        };
        self.plain_made(&plain, 0, &mut |cx, _, ty| Ok(cx.zero_scalar(ty)), span)
    }

    /// A scalar's zero: `0`, `0n`, `false`, `"\0"`.
    fn zero_scalar(&self, ty: Ty<'tcx>) -> Expr {
        match Num::of(ty) {
            Some(num) => num.literal(0),
            None if ty.is_bool() => Expr::bool(false),
            None if super::pointers::raw_pointee(self.tcx, ty).is_some() => Expr::null(),
            None => Expr::str("\0"),
        }
    }

    /// A value of `plain`'s type, at `offset` in a whole, of each scalar's
    /// value, `scalar(cx, offset, ty)`, made in order.
    fn plain_made(
        &mut self,
        plain: &Plain<'tcx>,
        offset: u64,
        scalar: &mut dyn FnMut(&mut Self, u64, Ty<'tcx>) -> R<Expr>,
        span: Span,
    ) -> R<Expr> {
        let (ty, parts) = match plain {
            Plain::Scalar(ty) => return scalar(self, offset, *ty),
            Plain::Parts(ty, parts) => (*ty, parts),
        };
        let values = parts
            .iter()
            .map(|(at, part)| self.plain_made(part, offset + at, scalar, span))
            .collect::<R<Vec<_>>>()?;
        Ok(match ty.kind() {
            ty::Tuple(items) if items.is_empty() => Expr::undefined(),
            ty::Tuple(_) | ty::Array(..) => Expr::array(values),
            _ => self.adt_value(ty, VariantIdx::ZERO, values, &[], None, span)?,
        })
    }

    /// Each of `value`'s scalars, read where it is, at `offset` in a whole.
    fn plain_reads(&self, value: Expr, plain: &Plain<'tcx>, offset: u64, reads: &mut Vec<Read<'tcx>>) {
        let (ty, parts) = match plain {
            Plain::Scalar(ty) => return reads.push(Read { offset, ty: *ty, value }),
            Plain::Parts(ty, parts) => (*ty, parts),
        };
        for (i, (at, part)) in parts.iter().enumerate() {
            let field = match ty.kind() {
                ty::Array(..) => Expr::index(value.clone(), Expr::int(i as i128)),
                _ => self.project(value.clone(), ty, i),
            };
            self.plain_reads(field, part, offset + at, reads);
        }
    }

    /// Whether `ty` is bytes, a `[u8; N]`.
    fn is_bytes(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Array(item, _) if *item == self.tcx.types.u8)
    }

    /// A scalar's bytes: 1 of a `bool`, 4 of a `char`, a number's own.
    fn scalar_bytes(&self, ty: Ty<'tcx>) -> u64 {
        match Num::of(ty) {
            Some(num) => u64::from(num.bits() / 8),
            None if ty.is_bool() => 1,
            // A `char`'s, and a thin pointer's, wasm32's (ADR 0562).
            None => 4,
        }
    }

    /// The unsigned integer of `ty`'s bytes, what holds its bits.
    fn bits_type(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        let tcx = self.tcx;
        match self.scalar_bytes(ty) {
            1 => tcx.types.u8,
            2 => tcx.types.u16,
            4 => tcx.types.u32,
            8 => tcx.types.u64,
            _ => tcx.types.u128,
        }
    }

    /// `value`, a `from`, as a `to` of its bytes: through its bits, an
    /// unsigned integer's, which of an integer is `as`.
    fn retyped(&mut self, value: Expr, (from, to): (Ty<'tcx>, Ty<'tcx>), span: Span) -> R<Expr> {
        if from == to {
            return Ok(value);
        }
        let bits = self.scalar_bits(value, from, span)?;
        self.scalar_of_bits(bits, to, span)
    }

    /// A scalar's bits, an unsigned integer of its bytes: a float's
    /// `to_bits()`, `true` as 1, a `char`'s code point.
    fn scalar_bits(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let bits = self.bits_type(ty);
        match Num::of(ty) {
            Some(num) if num.float() => Ok(Expr::call(
                Expr::helper("$floatToBits"),
                vec![value, Expr::int(i128::from(num.bits() / 8))],
            )),
            // A pointer's bits are its address, a reference's to an object
            // the object's (ADR 0562).
            None if let Some(pointee) = self.address_of(ty) => {
                let size = self.pointee_size(pointee, span)?;
                Ok(Expr::call(Expr::helper("$addr"), vec![value, size]))
            }
            _ => self.cast(value, ty, bits, span),
        }
    }

    /// What a scalar `ty` whose bits are an address points to: a raw
    /// pointer's pointee, or a reference's object (ADR 0562).
    fn address_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Ref(_, pointee, _) => Some(*pointee),
            _ => super::pointers::raw_pointee(self.tcx, ty).map(|(pointee, _)| pointee),
        }
    }

    /// The scalar `ty` of `bits`, an unsigned integer of its bytes.
    fn scalar_of_bits(&mut self, bits: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        Ok(match Num::of(ty) {
            Some(num) if num.float() => Expr::call(
                Expr::helper("$floatFromBits"),
                vec![bits, Expr::int(i128::from(num.bits() / 8))],
            ),
            Some(_) => self.cast(bits, self.bits_type(ty), ty, span)?,
            None if ty.is_bool() => Expr::bin(Op::Ne, bits, Expr::int(0)),
            // A pointer of an address's bits, a reference what's there (ADR
            // 0562).
            None if let Some(pointee) = self.address_of(ty) => {
                let pointer = Expr::call(Expr::helper("$ptrFrom"), self.at_address(bits, pointee, span)?);
                match ty.kind() {
                    ty::Ref(..) => Expr::held(pointer),
                    _ => pointer,
                }
            }
            None => Expr::call(Expr::member(Expr::var("String"), "fromCodePoint"), vec![bits]),
        })
    }

    /// The bytes of `value`, a `from` of `size` bytes, `reads` its scalars:
    /// itself, of a `[u8; N]`, else each scalar's bytes in order, and zeros
    /// for the padding between them. Padding after them nothing reads: a
    /// read of padding is undefined behavior.
    fn bytes_of(&mut self, value: &Expr, reads: &[Read<'tcx>], (from, size): (Ty<'tcx>, u64), span: Span) -> R<Expr> {
        if self.is_bytes(from) {
            return Ok(value.clone());
        }
        let mut sorted: Vec<&Read<'tcx>> = reads.iter().collect();
        sorted.sort_by_key(|r| r.offset);
        // A whole of one scalar's bytes is its `$toBytes`.
        let to_bytes = |bits, n: u64| {
            Expr::call(
                Expr::helper("$toBytes"),
                vec![bits, Expr::int(i128::from(n)), Expr::bool(true)],
            )
        };
        if let [read] = sorted.as_slice()
            && self.scalar_bytes(read.ty) == size
            && size > 1
        {
            let bits = self.scalar_bits(read.value.clone(), read.ty, span)?;
            return Ok(to_bytes(bits, size));
        }
        let mut items = Vec::new();
        let mut at = 0;
        for read in sorted {
            items.extend((at..read.offset).map(|_| Expr::int(0)));
            let n = self.scalar_bytes(read.ty);
            let bits = self.scalar_bits(read.value.clone(), read.ty, span)?;
            items.push(match n {
                1 => bits,
                _ => Expr::spread(to_bytes(bits, n)),
            });
            at = read.offset + n;
        }
        Ok(Expr::array(items))
    }

    /// The scalar `ty` at `offset` in `bytes`, of a whole of `size` bytes.
    fn read_bytes(&mut self, bytes: &Expr, (offset, size): (u64, u64), ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let n = self.scalar_bytes(ty);
        if n == 1 {
            let byte = Expr::index(bytes.clone(), Expr::int(i128::from(offset)));
            return self.scalar_of_bits(byte, ty, span);
        }
        let mine = match (offset, n) == (0, size) {
            true => bytes.clone(),
            false => Expr::call(
                Expr::member(bytes.clone(), "slice"),
                vec![Expr::int(i128::from(offset)), Expr::int(i128::from(offset + n))],
            ),
        };
        // An integer's, signed or not, as `from_le_bytes` reads it.
        let signed = Num::of(ty).is_some_and(|num| !num.float() && num.signed());
        let bits = Expr::call(
            Expr::helper("$fromBytes"),
            vec![mine, Expr::int(i128::from(n)), Expr::bool(true), Expr::bool(signed)],
        );
        match Num::of(ty) {
            Some(num) if !num.float() => Ok(bits),
            _ => self.scalar_of_bits(bits, ty, span),
        }
    }
}

/// `n` items, each `zero` or none: `new Array(n).fill(0)` of a zero that's
/// a constant, `Array.from({ length: n }, () => zero)` of an object made for
/// each, `Array.from({ length: n })` of none.
pub(in crate::lower) fn filled(n: Expr, zero: Option<Expr>) -> Expr {
    if let Some(zero) = zero.as_ref().filter(|zero| zero.is_constant()) {
        let made = Expr::new_(Expr::var("Array"), vec![n]);
        return Expr::call(Expr::member(made, "fill"), vec![zero.clone()]);
    }
    let mut given = vec![Expr::object(vec![js::Prop::Field("length".into(), n)])];
    given.extend(zero.map(|zero| Expr::arrow(Vec::new(), vec![js::StmtKind::Return(Some(zero)).at(js::Span::NONE)])));
    Expr::call(Expr::member(Expr::var("Array"), "from"), given)
}
