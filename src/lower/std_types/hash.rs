//! Hashing (ADR 0463): std's hashers, `DefaultHasher` and `SipHasher`, each
//! a `$SipHasher`; and what std's `Hash` of its own types writes to a
//! hasher, any `Hasher`, as std's impls write it.

use crate::js::{self, Expr, Pattern, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::mut_refs::boxed;
use crate::lower::recognition::{Std, StdItem, is_std_def, std_item};
use crate::lower::{FnCx, R};
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
    /// A reference, a `Box`, an `Rc` or an `Arc`: what it points at.
    Pointer(Ty<'tcx>),
    /// A tuple's parts in order; `()` and `PhantomData`, nothing.
    Tuple(Vec<Ty<'tcx>>),
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
                let hasher = values.next().expect("a hasher");
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
        let held = match !own && self.is_cell_pointee(ty) {
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
        let [value, state, dictionary] = ["value", "state", "H"].map(|name| self.fresh(name));
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
        let items = self.fresh("items");
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
            false => self.spill("value", value, out),
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
            Hashed::Tuple(parts) => {
                let value = self.spilled(value, out);
                for (i, part) in parts.into_iter().enumerate() {
                    self.hash_writes(
                        Expr::index(value.clone(), Expr::int(i as i128)),
                        part,
                        hasher,
                        span,
                        out,
                    )?;
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
                let present = self.present(value, item);
                out.push(StmtKind::If(present, some, Some(none)).at(js::Span::NONE));
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
        let name = self.fresh("item");
        let mut body = Vec::new();
        self.hash_writes(Expr::var(&name), item, hasher, span, &mut body)?;
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
