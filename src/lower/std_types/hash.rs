//! Hashing (ADR 0463): std's hashers, `DefaultHasher` and `SipHasher`, each
//! a `$SipHasher`; and what std's `Hash` of its own types writes to a
//! hasher, any `Hasher`, as std's impls write it.

use crate::js::{self, Expr, Pattern, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::mut_refs::boxed;
use crate::lower::recognition::{Std, StdItem, is_std_def, std_item};
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// Which of std's hashers: `DefaultHasher`, SipHash-1-3 of zero keys, or
/// the deprecated `SipHasher`, SipHash-2-4.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum StdHasher {
    Default,
    Sip,
}

/// What's asked of hashing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum HashOp {
    /// `DefaultHasher::new()` or `SipHasher::new()`.
    New(StdHasher),
    /// `SipHasher::new_with_keys(k0, k1)`.
    NewWithKeys,
    /// A std hasher's own `Hasher` method: `h.write_u32(n)`, `h.finish()`.
    Own,
    /// A crate's hasher's provided method its impl doesn't write:
    /// `count.write_i32(n)`, its dictionary's, `$hasher`'s.
    Provided,
    /// `value.hash(state)` of a std type: the writes std's impl makes.
    Hash,
    /// `T::hash_slice(items, state)` of a std type: its items', unprefixed.
    HashSlice,
    /// `ptr::hash(p, state)`: its pointer's address, as a `*const T`'s
    /// `hash` writes it (ADRs 0562, 0864).
    Address,
    /// `RandomState::new()`: `undefined`, as its `Default` is, as it holds
    /// nothing JS sees (#1420).
    RandomState,
    /// `BuildHasherDefault::<H>`'s `build_hasher()`: an `H::default()`
    /// (#1155); `RandomState`'s, a `DefaultHasher` (#1420).
    BuildHasher,
    /// `BuildHasherDefault::<H>`'s `hash_one(x)`, std's provided body: an
    /// `H::default()`, `x`'s `Hash` written into it, its `finish()` (#1155).
    HashOne,
}

/// What std's `Hash` of a type writes, by the impl's shape.
pub(in crate::lower) enum Hashed<'tcx> {
    /// An integer written as one of `ty`, `write_u32(n)` by its name: an
    /// integer's own, a `NonZero`'s number's, an `Ordering`'s `i8`.
    Number(Ty<'tcx>),
    /// `write_u8` of `0` or `1`.
    Bool,
    /// `write_u32` of its code point.
    Char,
    /// `str` and `String`: `write_str`.
    Text,
    /// A `Duration`, an `Instant` or a `SystemTime`, its nanoseconds, a
    /// BigInt (ADRs 0188, 0447): its seconds, a `u64`, then the nanoseconds
    /// under one, a `u32`, as std's derived `Hash` of a `Duration` writes,
    /// and of an `Instant` and a `SystemTime` of wasm32's, which hold one or
    /// a timespec; of a `SystemTime`, `before_epoch`, a timespec's, its
    /// seconds floored, an `i64`, its nanoseconds never negative (#1447).
    Nanoseconds { before_epoch: bool },
    /// An `OsStr` or an `OsString`, its text in JS (ADR 0173): its encoded
    /// bytes, UTF-8's, as a `[u8]`, a length then the bytes, as std's
    /// `self.as_encoded_bytes().hash(state)` (#1447).
    OsText,
    /// A `Path` or a `PathBuf`, its text: std's component-wise `Hash`,
    /// `$hashPath(path, state, H)` (#1447).
    PathText,
    /// A `Layout`, `{ size, align }` (ADR 0417): its size, then its
    /// alignment, each a `usize`, as std's derived `Hash` writes (#1447).
    Layout,
    /// A reference, a `Box`, an `Rc` or an `Arc`: what it points at.
    Pointer(Ty<'tcx>),
    /// One of std's that's in JS the value it holds, a `ManuallyDrop`, the
    /// `MaybeDangling` in it, a `Pin` its pointer: what it holds, of the
    /// same value, as std's `Hash` of it writes, a `Pin`'s its target's
    /// (#1289).
    Held(Ty<'tcx>),
    /// A `Pin` of the crate's own pointer, of that pointer's type and its
    /// target's: its target, read by the pointer's own `Deref::deref`, as
    /// std's `Ptr::Target::hash(self)` reads it (#1435).
    Target(Ty<'tcx>, Ty<'tcx>),
    /// A `Cow`: what it borrows or owns, read through its `Deref`
    /// (`cow_deref`), of its borrowed type, as std's `Hash` of one writes
    /// (#1289).
    Borrowed(Ty<'tcx>),
    /// A tuple's parts in order; `()` and `PhantomData`, nothing.
    Tuple(Vec<Ty<'tcx>>),
    /// One of std's enums whose `Hash` is derived, `Bound`, `Poll` or
    /// `ControlFlow`: its discriminant, of its repr's integer, where it has
    /// more than one variant, then its variant's fields in order, as
    /// `#[derive(Hash)]` writes (#1289).
    Variants(ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>),
    /// A slice, an array or a `Vec`: its length, then its items, an integer
    /// slice's bytes in one write.
    Slice(Ty<'tcx>),
    /// A `VecDeque`: its length, then each item.
    Items(Ty<'tcx>),
    /// An `Option`'s discriminant, an `isize`, then what `Some` holds.
    Option(Ty<'tcx>),
    /// A `Result`'s discriminant, then what it holds.
    Result(Ty<'tcx>, Ty<'tcx>),
    /// A raw pointer's address, a `usize`, of a pointer to the type (ADR 0562).
    Address(Ty<'tcx>),
    /// One of std's ranges, of bounds of its type, `None` of `..`: its
    /// derived `Hash`, each bound by its field's order, then of `a..=b`
    /// whether it's exhausted, a `bool` (#1361).
    Range(crate::lower::std_types::range::RangeKind, Option<Ty<'tcx>>),
}

/// A hasher, as hashing writes to it: given to a type's `hash<H>` as
/// `state`, a box of it or the cell generic code holds, with `dictionary`,
/// its `Hasher`'s; its methods called through it, `HHasher.write_u32(state,
/// n)`, or a std hasher's, `own`, on it, `hasher.write_u32(n)`.
struct Hasher {
    state: Expr,
    dictionary: Expr,
    own: Option<Expr>,
}

impl Hasher {
    fn call(&self, method: &str, args: Vec<Expr>) -> Expr {
        match &self.own {
            Some(hasher) => Expr::call(Expr::member(hasher.clone(), method), args),
            None => {
                let mut given = vec![self.state.clone()];
                given.extend(args);
                Expr::call(Expr::member(self.dictionary.clone(), method), given)
            }
        }
    }

    /// `call` as a statement: a write.
    fn write(&self, method: &str, args: Vec<Expr>) -> Stmt {
        StmtKind::Expr(self.call(method, args)).at(js::Span::NONE)
    }
}

/// A std hasher made, of `keys`, `[k0, k1]`, or zero ones: `new $SipHasher(1,
/// 3)`, `DefaultHasher`'s rounds, or `SipHasher`'s, `(2, 4)`. Its `new` and
/// `Default`'s.
pub(in crate::lower) fn new_hasher(which: StdHasher, keys: Vec<Expr>) -> Expr {
    let rounds = match which {
        StdHasher::Default => [1, 3],
        StdHasher::Sip => [2, 4],
    };
    let mut args = vec![Expr::int(rounds[0]), Expr::int(rounds[1])];
    args.extend(keys);
    Expr::new_(Expr::helper("$SipHasher"), args)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What std's `BuildHasher` `built` builds, and its type: of a
    /// `BuildHasherDefault<H>`, an `H::default()` (#1155); of a
    /// `RandomState`, a `DefaultHasher`, whose keys are random in Rust and
    /// fixed here, which no program can tell apart (#1420, ADR 0262's
    /// "none" row).
    fn built_hasher(&mut self, call: &Call<'_, 'tcx>) -> R<(Expr, Ty<'tcx>)> {
        let built = call.generic_args.type_at(0);
        let ty::Adt(adt, of) = built.kind() else {
            unreachable!("a `BuildHasherDefault` or a `RandomState`")
        };
        if is_std_def(self.tcx, adt.did(), StdItem::RandomState) {
            // Its `BuildHasher::Hasher`, std's `DefaultHasher`.
            let build_hasher = self.tcx.trait_of_assoc(call.def_id).expect("`BuildHasher`'s");
            let hasher = (self.tcx.associated_items(build_hasher).in_definition_order())
                .find(|item| matches!(item.kind, ty::AssocKind::Type { .. }) && item.name().as_str() == "Hasher")
                .expect("`BuildHasher::Hasher`");
            let projected = Ty::new_projection(self.tcx, ty::IsRigid::No, hasher.def_id, [built]);
            let hasher = (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projected))
                .unwrap_or(projected);
            return Ok((new_hasher(StdHasher::Default, Vec::new()), hasher));
        }
        Ok((self.default_value(of.type_at(0), call.span)?, of.type_at(0)))
    }

    /// Hashing's function: `None` if `known` is another.
    pub(in crate::lower) fn hash_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Hash(op) = known else {
            return Ok(None);
        };
        Ok(Some(match op {
            HashOp::New(which) => new_hasher(which, Vec::new()),
            HashOp::NewWithKeys => new_hasher(StdHasher::Sip, values.collect()),
            HashOp::Own => {
                let mut hasher = values.next().expect("a hasher");
                // The hasher its receiver, `&self` or `&mut self`, points at,
                // through each `&mut` to it, of std's `impl Hasher for &mut
                // H`, read where it's a cell, as each forwards to it (#1488).
                let sig = self.tcx.fn_sig(call.def_id).instantiate(self.tcx, call.generic_args);
                let mut pointer = sig.skip_binder().inputs()[0];
                if let ty::Ref(_, inner, ty::Mutability::Not) = *pointer.kind() {
                    pointer = inner;
                }
                while let ty::Ref(_, pointee, ty::Mutability::Mut) = *pointer.kind() {
                    if !self.mut_ref_form(pointee).read_is_itself() {
                        hasher = Expr::member(hasher, js::HELD);
                    }
                    pointer = pointee;
                }
                let method = crate::lower::bindings::fn_name(self.tcx, call.def_id);
                Expr::call(Expr::member(hasher, method), values.collect())
            }
            HashOp::Provided => {
                let state = values.next().expect("a hasher");
                let hasher = self.hasher(state, call.generic_args.type_at(0), call.span, out)?;
                hasher.call(
                    &crate::lower::bindings::fn_name(self.tcx, call.def_id),
                    values.collect(),
                )
            }
            HashOp::RandomState => Expr::undefined(),
            HashOp::BuildHasher => self.built_hasher(&call)?.0,
            HashOp::HashOne => {
                let (made, hasher_ty) = self.built_hasher(&call)?;
                let _built = values.next();
                let value = values.next().expect("what's hashed");
                let state = self.spill(Role::Hasher, made, out);
                let hasher = self.hasher(state, hasher_ty, call.span, out)?;
                self.hash_writes(value, call.generic_args.type_at(1), &hasher, call.span, out)?;
                hasher.call("finish", Vec::new())
            }
            HashOp::Hash | HashOp::HashSlice | HashOp::Address => {
                let value = values.next().expect("what's hashed");
                let state = values.next().expect("a hasher");
                let hasher = self.hasher(state, call.generic_args.type_at(1), call.span, out)?;
                let hashed = call.generic_args.type_at(0);
                match op {
                    HashOp::Hash => self.hash_writes(value, hashed, &hasher, call.span, out)?,
                    HashOp::Address => {
                        let pointer = ty::Ty::new_imm_ptr(self.tcx, hashed);
                        self.hash_writes(value, pointer, &hasher, call.span, out)?
                    }
                    _ => {
                        let value = self.spilled(value, out);
                        self.item_writes(value, hashed, &hasher, call.span, out)?;
                    }
                }
                Expr::undefined()
            }
        }))
    }

    /// The hasher a `&mut H`, `state`, is, of an `H` of std's, the crate's or
    /// a type parameter: a std one's methods its own, else its dictionary's,
    /// given what generic code holds of it, the cell it's given, or a box of
    /// the object.
    fn hasher(&mut self, state: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Hasher> {
        let state = self.spilled(state, out);
        let own = self.recognition().std_hasher(ty).is_some();
        let held = match !own && self.mut_ref_form(ty).is_cell() {
            true => state.clone(),
            false => boxed(state.clone()),
        };
        let hasher_trait = self.recognition().hasher_trait().expect("std has `Hasher`");
        let dictionary = self.dictionary(ty::TraitRef::new(self.tcx, hasher_trait, [ty]), span)?;
        Ok(Hasher {
            state: held,
            dictionary,
            own: own.then_some(state),
        })
    }

    /// Does `hash` of a `ty` run a hand-written `Hash`, the crate's or a
    /// library's, whose calls a program can see, a print or a panic (ADR
    /// 1120)? std's and a derived one's, all through, run none, nor does a
    /// type's equal only to itself, its address's (ADR 0864).
    pub(in crate::lower) fn hash_runs_code(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        if let Some(hashed) = self.recognition().hashed_as(ty) {
            return match hashed {
                Hashed::Pointer(held)
                | Hashed::Held(held)
                | Hashed::Borrowed(held)
                | Hashed::Target(_, held)
                | Hashed::Slice(held)
                | Hashed::Items(held)
                | Hashed::Option(held) => self.hash_runs_code(held, seen),
                Hashed::Tuple(parts) => parts.into_iter().any(|part| self.hash_runs_code(part, seen)),
                Hashed::Variants(adt, args) => {
                    (adt.all_fields()).any(|field| self.hash_runs_code(self.field_ty(field, args), seen))
                }
                Hashed::Result(ok, err) => self.hash_runs_code(ok, seen) || self.hash_runs_code(err, seen),
                Hashed::Range(_, bound) => bound.is_some_and(|bound| self.hash_runs_code(bound, seen)),
                Hashed::Number(_)
                | Hashed::Bool
                | Hashed::Char
                | Hashed::Text
                | Hashed::Nanoseconds { .. }
                | Hashed::OsText
                | Hashed::PathText
                | Hashed::Layout
                | Hashed::Address(_) => false,
            };
        }
        match ty.kind() {
            ty::Adt(..) if self.equal_only_to_itself(ty) => false,
            // std's impl's, which hashes what its bounds say it does:
            // `Reverse<T>`'s `T`, never `Discriminant<T>`'s.
            ty::Adt(adt, _) if self.is_std(adt.did()) => {
                let hash = std_item(self.tcx, StdItem::Hash);
                let tr = self
                    .tcx
                    .erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, hash, [ty]));
                let Ok(ImplSource::UserDefined(imp)) = crate::lower::traits::select(self.tcx, self.typing_env, tr)
                else {
                    return false;
                };
                let bounds = self.tcx.param_env(imp.impl_def_id).caller_bounds();
                bounds
                    .filter_map(|clause| clause.as_trait_clause())
                    .filter(|bound| bound.def_id() == hash)
                    .map(|bound| {
                        ty::EarlyBinder::bind(self.tcx, bound.skip_binder().self_ty())
                            .instantiate(self.tcx, imp.args)
                            .skip_normalization()
                    })
                    .any(|held| self.hash_runs_code(held, seen))
            }
            ty::Adt(adt, args) if self.recognition().derives(std_item(self.tcx, StdItem::Hash), ty) => adt
                .all_fields()
                .any(|field| self.hash_runs_code(self.field_ty(field, args), seen)),
            _ => true,
        }
    }

    /// A std hasher's `Hasher`, `$sipHasher`, its own methods; and a std
    /// type's `Hash`, `{ hash: (value, state, H) => .. }`, the writes std's
    /// impl makes (ADR 0463). `None` of another.
    pub(in crate::lower) fn std_hash_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Option<Expr>> {
        let ty = tr.self_ty();
        if self.recognition().hasher_trait() == Some(tr.def_id) {
            if self.recognition().std_hasher(ty).is_some() {
                return Ok(Some(Expr::helper("$sipHasher")));
            }
            // std's `impl Hasher for &mut H` and `Box<H>`, which call what
            // they point at: an object's, a `dyn`'s pair too, which its `&mut`
            // is.
            let pointee = match *ty.kind() {
                ty::Ref(_, pointee, ty::Mutability::Mut) => pointee,
                ty::Adt(_, args) if ty.is_box() => args.type_at(0),
                _ => return Ok(None),
            };
            if !self.is_object(pointee) {
                return Ok(None);
            }
            return Ok(Some(
                self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [pointee]), span)?,
            ));
        }
        if !is_std_def(self.tcx, tr.def_id, StdItem::Hash) || self.recognition().hashed_as(ty).is_none() {
            return Ok(None);
        }
        let [value, state, dictionary] = [Role::Value, Role::State, Role::H].map(|name| self.fresh(name));
        let hasher = Hasher {
            state: Expr::var(&state),
            dictionary: Expr::var(&dictionary),
            own: None,
        };
        let mut body = Vec::new();
        self.hash_writes(Expr::var(&value), ty, &hasher, span, &mut body)?;
        let hash = Expr::arrow(
            vec![value.into(), state.clone().into(), dictionary.clone().into()],
            body,
        );
        let items = self.fresh(Role::Items);
        let mut body = Vec::new();
        self.item_writes(Expr::var(&items), ty, &hasher, span, &mut body)?;
        let hash_slice = Expr::arrow(vec![items.into(), state.into(), dictionary.into()], body);
        Ok(Some(Expr::object(vec![
            js::Prop::Field("hash".into(), hash),
            js::Prop::Field(
                crate::lower::bindings::trait_method_name(self.tcx, tr.def_id, "hash_slice"),
                hash_slice,
            ),
        ])))
    }

    /// `value`, as one const where it's read more than once and doesn't read
    /// the same.
    fn spilled(&mut self, value: Expr, out: &mut Vec<Stmt>) -> Expr {
        match value.reads_same() {
            true => value,
            false => self.spill(Role::Value, value, out),
        }
    }

    /// What `Hasher`'s method `name` is called in JS, a std hasher's and a
    /// dictionary's alike: `writeU32` of `write_u32`.
    fn hasher_key(&self, name: &str) -> String {
        let hasher = self.recognition().hasher_trait().expect("std has `Hasher`");
        crate::lower::bindings::trait_method_name(self.tcx, hasher, name)
    }

    /// The writes of `value.hash(hasher)`, of a `ty`: std's impl's, or a
    /// crate's type's or a type parameter's `hash`, its dictionary's.
    fn hash_writes(&mut self, value: Expr, ty: Ty<'tcx>, hasher: &Hasher, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let Some(hashed) = self.recognition().hashed_as(ty) else {
            return self.dictionary_writes("hash", value, ty, hasher, span, out);
        };
        match hashed {
            Hashed::Number(written) => {
                out.push(hasher.write(&self.hasher_key(&format!("write_{written}")), vec![value]))
            }
            Hashed::Bool => out.push(hasher.write(
                &self.hasher_key("write_u8"),
                vec![Expr::call(Expr::var("Number"), vec![value])],
            )),
            Hashed::Char => {
                let code = Expr::call(Expr::member(value, "codePointAt"), vec![Expr::int(0)]);
                out.push(hasher.write(&self.hasher_key("write_u32"), vec![code]));
            }
            Hashed::Text => out.push(hasher.write(&self.hasher_key("write_str"), vec![value])),
            Hashed::Layout => {
                let value = self.spilled(value, out);
                for field in ["size", "align"] {
                    out.push(hasher.write(
                        &self.hasher_key("write_usize"),
                        vec![Expr::member(value.clone(), field)],
                    ));
                }
            }
            Hashed::PathText => {
                // A std hasher's own, its dictionary's, `$sipHasher` of its cell.
                let (state, dictionary) = match &hasher.own {
                    Some(own) => (
                        Expr::object(vec![js::Prop::Field(js::HELD.into(), own.clone())]),
                        Expr::helper("$sipHasher"),
                    ),
                    None => (hasher.state.clone(), hasher.dictionary.clone()),
                };
                let call = Expr::call(Expr::helper("$hashPath"), vec![value, state, dictionary]);
                out.push(StmtKind::Expr(call).at(js::Span::NONE));
            }
            Hashed::OsText => {
                let bytes = Expr::call(Expr::helper("$utf8Bytes"), vec![value]);
                let slice = Ty::new_imm_ref(
                    self.tcx,
                    self.tcx.lifetimes.re_erased,
                    Ty::new_slice(self.tcx, self.tcx.types.u8),
                );
                self.hash_writes(bytes, slice, hasher, span, out)?;
            }
            Hashed::Nanoseconds { before_epoch } => {
                let value = self.spilled(value, out);
                let second = || Expr::bigint(1_000_000_000);
                let (seconds, nanos) = match before_epoch {
                    false => (
                        Expr::bin(js::Op::Div, value.clone(), second()),
                        Expr::bin(js::Op::Rem, value, second()),
                    ),
                    true => {
                        let under = Expr::bin(js::Op::Rem, value.clone(), second());
                        let wrapped = Expr::bin(js::Op::Add, under, second());
                        let nanos = self.spill(Role::Nanos, Expr::bin(js::Op::Rem, wrapped, second()), out);
                        let whole = Expr::bin(js::Op::Sub, value, nanos.clone());
                        (Expr::bin(js::Op::Div, whole, second()), nanos)
                    }
                };
                let seconds_write = if before_epoch { "write_i64" } else { "write_u64" };
                out.push(hasher.write(&self.hasher_key(seconds_write), vec![seconds]));
                let nanos = Expr::call(Expr::var("Number"), vec![nanos]);
                out.push(hasher.write(&self.hasher_key("write_u32"), vec![nanos]));
            }
            // A raw pointer's address, as `usize` (ADR 0562).
            Hashed::Address(pointee) => {
                let size = self.pointee_size(pointee, span)?;
                let addr = Expr::call(Expr::helper("$addr"), vec![value, size]);
                out.push(hasher.write(&self.hasher_key("write_usize"), vec![addr]));
            }
            Hashed::Pointer(pointee) => {
                let (value, _) = self.through_refs(value, ty);
                let value = match self.counted_rc(ty) {
                    Some(pointee) => self.counted_pointee(value, pointee),
                    None => value,
                };
                self.hash_writes(value, pointee, hasher, span, out)?;
            }
            Hashed::Held(held) => self.hash_writes(value, held, hasher, span, out)?,
            Hashed::Target(pointer, target) => {
                let read = self.own_deref(value, pointer, span, out)?;
                self.hash_writes(read, target, hasher, span, out)?;
            }
            Hashed::Borrowed(borrowed) => {
                let (read, _) = self.cow_deref(&value, ty).expect("a Cow's parts");
                self.hash_writes(read, borrowed, hasher, span, out)?;
            }
            Hashed::Variants(adt, args) => {
                let value = self.spilled(value, out);
                let discriminant = rustc_middle::ty::util::IntTypeExt::to_ty(&adt.repr().discr_type(), self.tcx);
                let write = self.hasher_key(&format!("write_{discriminant}"));
                let mut chain: Option<Vec<Stmt>> = None;
                for variant in adt.variants().indices().rev() {
                    let mut arm = Vec::new();
                    if adt.variants().len() > 1 {
                        // Its value, of its signed integer sign-extended, `Less`'s `-1`.
                        let bits = adt.discriminant_for_variant(self.tcx, variant).val;
                        let at = match discriminant.int_size_and_signed(self.tcx) {
                            (size, true) => size.sign_extend(bits),
                            (_, false) => bits as i128,
                        };
                        arm.push(hasher.write(&write, vec![Expr::int(at)]));
                    }
                    for (i, field) in adt.variant(variant).fields.iter().enumerate() {
                        let read = self.field_of_variant(value.clone(), ty, variant, i.into());
                        self.hash_writes(read, self.field_ty(field, args), hasher, span, &mut arm)?;
                    }
                    chain = Some(match chain {
                        None => arm,
                        Some(rest) => {
                            let test = self.variant_test(value.clone(), ty, adt, variant, false);
                            vec![StmtKind::If(test, arm, Some(rest)).at(js::Span::NONE)]
                        }
                    });
                }
                out.extend(chain.unwrap_or_default());
            }
            // Of an array made here that does nothing, `Wrapping(x)`'s `[x]`,
            // each part as it's written, never the array held to be read.
            Hashed::Tuple(parts) => {
                let made = match &value.kind {
                    js::ExprKind::Array(items) if items.len() == parts.len() && !value.has_effects() => {
                        Some(items.clone())
                    }
                    _ => None,
                };
                let value = match made {
                    Some(_) => value,
                    None => self.spilled(value, out),
                };
                for (i, part) in parts.into_iter().enumerate() {
                    let read = match &made {
                        Some(items) => items[i].clone(),
                        None => Expr::index(value.clone(), Expr::int(i as i128)),
                    };
                    self.hash_writes(read, part, hasher, span, out)?;
                }
            }
            Hashed::Slice(item) | Hashed::Items(item) => {
                let value = self.spilled(value, out);
                out.push(hasher.write(
                    &self.hasher_key("write_length_prefix"),
                    vec![Expr::member(value.clone(), "length")],
                ));
                match matches!(hashed, Hashed::Slice(_)) {
                    true => self.item_writes(value, item, hasher, span, out)?,
                    false => self.each_item_writes(value, item, hasher, span, out)?,
                }
            }
            Hashed::Option(item) => {
                let value = self.spilled(value, out);
                let mut some = vec![hasher.write(&self.hasher_key("write_isize"), vec![Expr::int(1)])];
                let held = match self.boxed_payload(item) {
                    true => self.some_value(value.clone()),
                    false => value.clone(),
                };
                self.hash_writes(held, item, hasher, span, &mut some)?;
                let none = vec![hasher.write(&self.hasher_key("write_isize"), vec![Expr::int(0)])];
                let present = Expr::presence(value, true);
                out.push(StmtKind::If(present, some, Some(none)).at(js::Span::NONE));
            }
            // Its state as Rust's is, `range_state`'s: its bounds, then of
            // `a..=b` its `exhausted`, which one never stepped isn't.
            Hashed::Range(kind, bound) => {
                let (bounds, exhausted) = self.range_state(value, ty, out);
                if let Some(bound) = bound {
                    for each in bounds {
                        self.hash_writes(each, bound, hasher, span, out)?;
                    }
                }
                if kind == crate::lower::std_types::range::RangeKind::Inclusive {
                    let exhausted = exhausted.unwrap_or_else(|| Expr::bool(false));
                    out.push(hasher.write(
                        &self.hasher_key("write_u8"),
                        vec![Expr::call(Expr::var("Number"), vec![exhausted])],
                    ));
                }
            }
            Hashed::Result(ok, err) => {
                let value = self.spilled(value, out);
                let mut arms = Vec::new();
                for (discriminant, held) in [(0, ok), (1, err)] {
                    let mut arm = vec![hasher.write(&self.hasher_key("write_isize"), vec![Expr::int(discriminant)])];
                    self.hash_writes(crate::js::result::held(value.clone()), held, hasher, span, &mut arm)?;
                    arms.push(arm);
                }
                let is_ok = crate::js::result::is(value, true);
                let err = arms.pop();
                out.push(StmtKind::If(is_ok, arms.pop().expect("`Ok`'s"), err).at(js::Span::NONE));
            }
        }
        Ok(())
    }

    /// The writes of `method`, `hash` or `hash_slice`, of a crate's type, or
    /// a type parameter, `ty`: its `Hash`'s dictionary's, given the hasher as
    /// generic code holds it, and its `Hasher`'s.
    fn dictionary_writes(
        &mut self,
        method: &str,
        value: Expr,
        ty: Ty<'tcx>,
        hasher: &Hasher,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let tr = ty::TraitRef::new(self.tcx, std_item(self.tcx, StdItem::Hash), [ty]);
        let dictionary = self.dictionary(tr, span)?;
        let call = Expr::call(
            Expr::member(
                dictionary,
                crate::lower::bindings::trait_method_name(self.tcx, tr.def_id, method),
            ),
            vec![value, hasher.state.clone(), hasher.dictionary.clone()],
        );
        out.push(StmtKind::Expr(call).at(js::Span::NONE));
        Ok(())
    }

    /// The writes of a slice's items, `items`, of `item`s, as `hash_slice`
    /// makes them: std's of an integer, its bytes in one `write`, and of
    /// another std type, each item's; a crate's type's or a parameter's, its
    /// dictionary's.
    fn item_writes(&mut self, items: Expr, item: Ty<'tcx>, hasher: &Hasher, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        // A crate's type's, or a type parameter's, which its impl may write.
        if self.recognition().hashed_as(item).is_none() {
            return self.dictionary_writes("hash_slice", items, item, hasher, span, out);
        }
        let Some(num) = (matches!(item.kind(), ty::Int(_) | ty::Uint(_)))
            .then(|| super::super::representation::Num::of(item))
            .flatten()
        else {
            return self.each_item_writes(items, item, hasher, span, out);
        };
        let bytes = match num.bits() {
            8 if !num.signed() => items,
            bits => {
                let to_bytes = Expr::call(
                    Expr::helper("$toBytes"),
                    vec![Expr::var("n"), Expr::int(i128::from(bits / 8)), Expr::bool(true)],
                );
                let each = Expr::arrow(
                    vec!["n".into()],
                    vec![StmtKind::Return(Some(to_bytes)).at(js::Span::NONE)],
                );
                Expr::call(Expr::member(items, "flatMap"), vec![each])
            }
        };
        out.push(hasher.write(&self.hasher_key("write"), vec![bytes]));
        Ok(())
    }

    /// Each of `items`' writes, in a loop.
    fn each_item_writes(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        hasher: &Hasher,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let name = self.fresh(Role::Item);
        let mut body = Vec::new();
        self.hash_writes(Expr::var(&name), item, hasher, span, &mut body)?;
        // Of items that write nothing, a `PhantomData`'s, no loop.
        if body.is_empty() {
            return Ok(());
        }
        out.push(
            StmtKind::ForOf {
                label: None,
                pattern: Pattern::Name(name),
                mutable: false,
                iterable: items,
                body,
                keys: false,
            }
            .at(js::Span::NONE),
        );
        Ok(())
    }
}
