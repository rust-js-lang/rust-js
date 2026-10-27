//! A number's call where its type is generic: num-traits' methods, a type
//! fact's value, a generic's name, and a layout's bytes.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A number's conversion, comparison, operator or size (ADRs 0011, 0057): `None` if `known` is another.
    pub(in crate::lower) fn numeric_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            Std::Clock { op, instant } => self.clock_values(op, instant, values, out)?,
            Std::Duration(op) => {
                let nanos = |n: i64| Expr::bigint(i128::from(n));
                let helper = |name: &'static str, args: Vec<Expr>| Expr::call(Expr::helper(name), args);
                match op {
                    DurationOp::New => {
                        let (secs, subsec) = (arg(), arg());
                        helper("$durationNew", vec![secs, subsec])
                    }
                    DurationOp::From(1) | DurationOp::As(1) => arg(),
                    DurationOp::From(unit) => Expr::bin(Op::Mul, arg(), nanos(unit)),
                    DurationOp::As(unit) => Expr::bin(Op::Div, arg(), nanos(unit)),
                    DurationOp::Subsec(unit) => {
                        let part = Expr::bin(Op::Rem, arg(), nanos(1_000_000_000));
                        let part = match unit {
                            1 => part,
                            _ => Expr::bin(Op::Div, part, nanos(unit)),
                        };
                        Expr::call(Expr::var("Number"), vec![part])
                    }
                    DurationOp::IsZero => Expr::bin(Op::Eq, arg(), nanos(0)),
                    DurationOp::Add => {
                        let (a, b) = (arg(), arg());
                        helper("$durationAdd", vec![a, b])
                    }
                    DurationOp::Sub => {
                        let (a, b) = (arg(), arg());
                        helper("$durationSub", vec![a, b])
                    }
                    DurationOp::Checked { add: true } => {
                        let (a, b) = (arg(), arg());
                        helper("$durationChecked", vec![Expr::bin(Op::Add, a, b)])
                    }
                    DurationOp::Checked { add: false } => {
                        let (a, b) = (arg(), arg());
                        let a = if a.reads_same() { a } else { self.spill("a", a, out) };
                        let b = if b.reads_same() { b } else { self.spill("b", b, out) };
                        let less = Expr::bin(Op::Lt, a.clone(), b.clone());
                        Expr::cond(less, Expr::undefined(), Expr::bin(Op::Sub, a, b))
                    }
                }
            }
            Std::SliceToArray { into, owned } => {
                let target = if into {
                    generic_args.type_at(1)
                } else {
                    generic_args.type_at(0)
                };
                let ty::Array(item, len) = *target.peel_refs().kind() else {
                    unreachable!("recognized as an array")
                };
                let len = len
                    .try_to_target_usize(self.tcx)
                    .ok_or_else(|| self.unsupported(span, "an array of a generic length here"))?;
                let slice = arg();
                // An array of its own, `[T; N]`, is a copy, as a write to the
                // slice's after doesn't change it; of an array made here, a
                // part's, `$slice`'s or `slice`'s copy already (ADR 0806). A
                // `&[T; N]` is the slice's.
                let fresh = crate::lower::iterators::made_array(&slice);
                let slice = if slice.reads_same() {
                    slice
                } else {
                    self.spill("slice", slice, out)
                };
                let fits = Expr::bin(Op::Eq, Expr::member(slice.clone(), "length"), Expr::int(len as i128));
                // A `Vec`'s, moved, is itself, and given back as it's refused.
                // Its items copies too, where something changes one in place.
                let value = match target.is_ref() || owned || (fresh && !self.needs_clone(item)) {
                    true => slice.clone(),
                    false => self.clone_items(slice.clone(), item, span)?,
                };
                let refused = match owned {
                    true => slice.clone(),
                    false => Expr::undefined(),
                };
                Expr::cond(fits, result::ok(value), result::err(refused))
            }
            Std::ToBig => Expr::call(Expr::var("BigInt"), vec![arg()]),
            Std::Cast { into } => {
                let (from, to) = match into {
                    true => (generic_args.type_at(0), generic_args.type_at(1)),
                    false => (generic_args.type_at(1), generic_args.type_at(0)),
                };
                let value = arg();
                self.number_from(value, from, to, span)?
            }
            Std::TryFromInt { into } => {
                let target = if into {
                    generic_args.type_at(1)
                } else {
                    generic_args.type_at(0)
                };
                let num = self.num(target, span)?;
                let (lo, hi) = num.range();
                let tried = Expr::call(
                    Expr::helper("$tryFromInt"),
                    vec![arg(), num.literal(lo), num.literal(hi as i128)],
                );
                // A `NonZero`'s, an error of `0` (ADR 0177), its message as the
                // others' is.
                match crate::lower::recognition::is_non_zero_ty(target) {
                    true => Expr::call(
                        Expr::helper("$nonZeroOk"),
                        vec![tried, Expr::str("number would be zero for non-zero type")],
                    ),
                    false => tried,
                }
            }
            Std::TryFromChar { to_char } => {
                let name = if to_char { "$charTryFrom" } else { "$u8TryFromChar" };
                Expr::call(Expr::helper(name), vec![arg()])
            }
            Std::FromDigit => Expr::call(Expr::helper("$fromDigit"), vec![arg(), arg()]),
            Std::FromU32 => Expr::call(Expr::helper("$fromU32"), vec![arg()]),
            Std::Cmp => Expr::call(Expr::helper("$cmp"), vec![arg(), arg()]),
            Std::MaxOf(max) => {
                let num = Num::of(tys[0].peel_refs());
                let callee = if num.is_some_and(Num::float) {
                    Expr::helper(if max { "$f64Max" } else { "$f64Min" })
                } else if num.is_some_and(Num::big) {
                    // `Math.max` takes numbers only.
                    Expr::helper(if max { "$bigMax" } else { "$bigMin" })
                } else {
                    Expr::member(Expr::var("Math"), if max { "max" } else { "min" })
                };
                // Of one of `Math`'s of another, `a.min(b).min(c)`, one call,
                // `Math.min(a, b, c)`: the same order, and the same number.
                let math = callee.helper_name().is_none();
                let args = [arg(), arg()].into_iter().flat_map(|a| match a.kind {
                    js::ExprKind::Call(inner, args) if math && js::same_path(&inner, &callee) => args,
                    kind => vec![Expr { kind, ..a }],
                });
                Expr::call(callee.clone(), args.collect())
            }
            // An `Ordering` is -1, 0 or 1: `Equal` is the one that's falsy.
            Std::Operator(op) => {
                let ty = generic_args
                    .types()
                    .next()
                    .expect("an operator's trait has a type")
                    .peel_refs();
                let (l, r) = (arg(), arg());
                // `a << &n` of an `i64` `n`: its type, the trait's `Rhs`.
                let r = match generic_args.types().nth(1) {
                    Some(rhs) => crate::lower::std_types::number::shift_amount_of(op, r, ty, rhs),
                    None => r,
                };
                self.binary(op, l, r, None, ty, span)?
            }
            Std::UnaryOperator(op) => {
                let ty = generic_args
                    .types()
                    .next()
                    .expect("an operator's trait has a type")
                    .peel_refs();
                let a = arg();
                self.unary(op, a, ty, span)?
            }
            Std::Reverse => Expr::unary(UnaryOp::Neg, arg()),
            // The size rustc works out for the wasm32 target, which rust-js
            // checks programs for, as a `const` of it has (ADR 0090). A
            // generic function is one JS function for every type, so a type
            // parameter's is given by its caller (ADR 0145).
            Std::NeedsDrop => {
                let of = generic_args.types().next().expect("`needs_drop`'s type argument");
                self.type_fact_value(of, TypeFact::NeedsDrop, span)?
            }
            Std::Layout { fact, of_val } => {
                let of = generic_args.types().next().expect("a size's type argument");
                // A type's, its own fact; a value's of a type rustc knows, its
                // bytes, which may be the value's own.
                let bytes = match of_val && !matches!(of.kind(), ty::Param(_)) {
                    true => Expr::int(self.layout_bytes((fact, of_val), of, span)?),
                    false => self.type_fact_value(of, fact, span)?,
                };
                // What's measured still runs, if it does anything.
                if of_val {
                    let measured = arg();
                    if measured.has_effects() {
                        out.push(StmtKind::Expr(measured).at(js_span));
                    }
                }
                bytes
            }
            _ => return Ok(None),
        }))
    }

    /// A fact of `of` (ADR 0145): its size or alignment, as rustc works it out
    /// for the wasm32 target, or its name; of a type parameter, the one this
    /// function was given for it.
    pub(in crate::lower) fn type_fact_value(&mut self, of: Ty<'tcx>, fact: TypeFact, span: Span) -> R<Expr> {
        let what = match fact {
            TypeFact::Size => "size_of",
            TypeFact::Align => "align_of",
            TypeFact::Name => "type_name",
            TypeFact::Id => "the `TypeId`",
            TypeFact::NeedsDrop => "needs_drop",
            TypeFact::Map | TypeFact::Set | TypeFact::TreeMap | TypeFact::TreeSet => "a map's class",
            TypeFact::Sort => "an unstable sort",
            TypeFact::Fused => "whether it's a `FusedIterator`",
            TypeFact::ConstSize => "its constant size",
            TypeFact::OneShot => "whether it's a `OneShot`",
            TypeFact::RandomAccess => "whether it's a `TrustedRandomAccess`",
            TypeFact::RandomAccessNoCoerce => "whether it's a `TrustedRandomAccessNoCoerce`",
            TypeFact::TrustedLen => "whether it's a `TrustedLen`",
        };
        // An associated type's, `<A as Array>::Item`'s: its trait's
        // dictionary's (ADR 0145).
        if let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) = of.kind()
            && self
                .krate
                .type_facts
                .get(def_id)
                .is_some_and(|facts| facts.iter().any(|&(_, f)| f == fact))
            && let Some(dictionary) = self.evidence_for(alias.trait_ref(self.tcx))
        {
            return Ok(Expr::member(
                dictionary,
                crate::lower::traits::item_fact_key(self.tcx, *def_id, fact),
            ));
        }
        // A std associated type's of a type parameter, `IItemMap` (ADR 0466).
        if let ty::Alias(..) = of.kind()
            && let Some(given) = self.given_fact_of(of, fact)
        {
            return Ok(given);
        }
        if let ty::Param(param) = of.kind() {
            // Whether it's a `FusedIterator`, its caller's, or not where no
            // caller says (ADR 0543).
            if fact == TypeFact::Fused {
                return Ok(self.fused_fact(param.index).unwrap_or_else(|| Expr::bool(false)));
            }
            // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
            if fact == TypeFact::Id
                && self.given_type_fact(param.index, fact).is_none()
                && let Some(any) = any_trait(self.tcx)
                && let Some(dictionary) = self.evidence_for(ty::TraitRef::new(self.tcx, any, [of]))
            {
                let type_id = crate::lower::bindings::trait_method_name(self.tcx, any, "type_id");
                return Ok(Expr::call(Expr::member(dictionary, type_id), Vec::new()));
            }
            return self
                .given_type_fact(param.index, fact)
                .ok_or_else(|| self.unsupported(span, &format!("`{what}` of a type parameter")));
        }
        if fact == TypeFact::NeedsDrop {
            return Ok(Expr::bool(of.needs_drop(self.tcx, self.typing_env)));
        }
        // The class of a map keyed by it, as one made where its type's known
        // is (ADR 0466).
        if let Some((set, sorted)) = fact.map_kind() {
            return self.map_class(set, sorted, Some(of), span);
        }
        // Whether its `Fuse` keeps it as it ends (ADR 0543).
        if fact == TypeFact::Fused {
            return Ok(Expr::bool(self.recognition().is_fused_iterator(of)));
        }
        // Of a type whose fact is the facts of what it holds, `TrustedLen` of
        // a `&mut I`, `I`'s, each's (ADR 0975).
        if let [(held, _), ..] = crate::lower::recognition::held_facts(self.tcx, self.typing_env, of, fact)[..]
            && held != of
        {
            let mut holds: Option<Expr> = None;
            for (held, fact) in crate::lower::recognition::held_facts(self.tcx, self.typing_env, of, fact) {
                let value = self.type_fact_value(held, fact, span)?;
                holds = Some(match holds {
                    Some(both) => Expr::bin(crate::js::Op::And, both, value),
                    None => value,
                });
            }
            return Ok(holds.expect("one held"));
        }
        // Of a type only a caller knows, its caller's, which none gave; but a
        // trait rustc can say holds of it here, as of any `T` (ADR 0975).
        let decided = crate::lower::recognition::fact_trait(self.tcx, fact)
            .and_then(|fact_trait| self.recognition().trait_holds(of, fact_trait))
            .is_some();
        if !matches!(fact, TypeFact::Fused)
            && !decided
            && (fact == TypeFact::ConstSize || crate::lower::recognition::fact_trait(self.tcx, fact).is_some())
            && (of.has_param() || matches!(of.kind(), ty::Alias(..)))
        {
            return Err(self.unsupported(span, &format!("{what} of `{of}`, which no caller gives")));
        }
        // Whether it has the trait the fact is, a `OneShot`, whose `flatten()`
        // std specializes (ADR 0899), or a `TrustedRandomAccess`, by which
        // std's zip reads by index (ADR 0975).
        if let Some(fact_trait) = crate::lower::recognition::fact_trait(self.tcx, fact) {
            let holds = self.recognition().implements_trait(of, fact_trait);
            // One std's zip would read by index that rust-js can't read so:
            // refused, never read as another is.
            if holds
                && matches!(fact, TypeFact::RandomAccess | TypeFact::RandomAccessNoCoerce)
                && self.index_reader(of, span)?.is_none()
            {
                return Err(self.unsupported(span, &format!("reading a `{of}` by its index, as std's zip of it does")));
            }
            return Ok(Expr::bool(holds));
        }
        // `N` of an array of `N` items, or a reference to one, else none
        // (ADR 0899).
        if fact == TypeFact::ConstSize {
            let array = match of.peel_refs().kind() {
                ty::Array(_, len) => len.try_to_target_usize(self.tcx),
                _ => None,
            };
            return Ok(array.map_or_else(Expr::undefined, |n| Expr::int(n.into())));
        }
        // What std's unstable sort of it picks by its layout (ADR 0471).
        if fact == TypeFact::Sort {
            return Ok(Expr::str(self.sort_kind(of, span)?));
        }
        if !matches!(fact, TypeFact::Name | TypeFact::Id) {
            return Ok(Expr::int(self.layout_bytes((fact, false), of, span)?));
        }
        // The name of one that holds a type parameter, `Cow<'_, T>`: rustc's,
        // with what its caller gives of each parameter's in its place (ADR
        // 0145).
        if fact == TypeFact::Name && of.has_param() {
            return self.name_of_generic(of, span);
        }
        if of.has_param() {
            return Err(self.unsupported(span, &format!("{what} of a type with a type parameter")));
        }
        let of = self
            .tcx
            .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(of));
        Ok(Expr::str(match fact {
            TypeFact::Id => self.type_id_key(self.tcx.erase_and_anonymize_regions(of)),
            _ => rustc_const_eval::util::type_name(self.tcx, of),
        }))
    }

    /// The name of `of`, a type that holds type parameters, `Cow<'_, T>`:
    /// rustc's, which names a parameter `_`, so each is put in as a type no
    /// program names, `[!; 4000000000]` and up, by its index, and the name
    /// given for it, `TName`, put where its text is.
    fn name_of_generic(&self, of: Ty<'tcx>, span: Span) -> R<Expr> {
        const BASE: u64 = 4_000_000_000;
        let tcx = self.tcx;
        let marked = of.fold_with(&mut ty::BottomUpFolder {
            tcx,
            ty_op: |t: Ty<'tcx>| match t.kind() {
                ty::Param(param) => Ty::new_array(tcx, tcx.types.never, BASE + u64::from(param.index)),
                _ => t,
            },
            lt_op: |r| r,
            ct_op: |c| c,
        });
        let marked = tcx.normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(marked));
        let name = rustc_const_eval::util::type_name(tcx, marked);
        let mut text = name.as_str();
        let mut parts = Vec::new();
        while let Some(at) = text.find("[!; ") {
            let rest = &text[at + 4..];
            let digits = rest.find(']').expect("an array's length");
            let index = rest[..digits].parse::<u64>().expect("a marked parameter") - BASE;
            parts.push(Expr::str(&text[..at]));
            let name = self.given_type_fact(index as u32, TypeFact::Name);
            parts.push(name.ok_or_else(|| self.unsupported(span, "`type_name` of a type parameter"))?);
            text = &rest[digits + 1..];
        }
        parts.push(Expr::str(text));
        Ok(crate::lower::display::join(parts))
    }

    /// The bytes `size_of`, `align_of`, `size_of_val` or `align_of_val`
    /// gives for `of`: its size or alignment, and whether of a value's.
    pub(in crate::lower) fn layout_bytes(&self, (fact, of_val): (TypeFact, bool), of: Ty<'tcx>, span: Span) -> R<i128> {
        let name = match (fact, of_val) {
            (TypeFact::Align, false) => "align_of",
            (TypeFact::Align, true) => "align_of_val",
            (_, false) => "size_of",
            (_, true) => "size_of_val",
        };
        if of.has_param() {
            return Err(self.unsupported(span, &format!("`{name}` of a type parameter")));
        }
        if !of.is_sized(self.tcx, self.typing_env) {
            return Err(self.unsupported(span, &format!("`{name}` of a value without one size")));
        }
        let layout = self
            .layout(of)
            .ok_or_else(|| self.unsupported(span, &format!("`{name}` of this type")))?;
        let bytes = if fact == TypeFact::Align {
            layout.align.abi.bytes()
        } else {
            layout.size.bytes()
        };
        Ok(bytes as i128)
    }
}

/// `x.ilog10()`, `$ilog(x, 10)`, a number's or a BigInt's, of `base`;
/// `checked_ilog10()`'s, `None` of what isn't positive.
pub(super) fn ilog(x: Expr, base: u32, checked: bool) -> Expr {
    let mut given = vec![x, Expr::int(base.into())];
    if checked {
        given.push(Expr::bool(true));
    }
    Expr::call_helper("$ilog", given)
}
