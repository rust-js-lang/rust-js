//! A dictionary for a bound, built where it's given: a std trait's, an
//! impl's, a `dyn`'s, and its associated types' facts.

use super::*;
use crate::lower::temporary_names::Role;

mod conversions;
mod declared;
mod references;
use declared::{holds_a_function, is_impl_call, reads_only_at_top};

/// Whether a bound's dictionary is always given, so what's read of it may
/// be read anywhere in its function: every bound's but a `Copy`'s, which
/// is `undefined` where each value is its own copy (ADR 0289), read only
/// behind the test of it, `TCopy ? TCopy.copy(x) : x` (#1355).
pub(super) fn always_given(tcx: TyCtxt<'_>, tr: ty::TraitRef<'_>) -> bool {
    !tcx.is_lang_item(tr.def_id, LangItem::Copy)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std collection's `Extend`, `{ extend(collection, items) }`, what
    /// generic code calls, `c.extend(items)` of a `C: Extend<A>` (ADR 0445):
    /// what a known one's call is, of its items as they're given, any
    /// iterable, and of the box generic code's `&mut` is (ADR 0180). None of
    /// another type, the crate's, whose impl's dictionary is its own.
    fn extend_dictionary(&mut self, ty: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let collection = Expr::member(Expr::var("collection"), js::HELD);
        let known = self.recognition().std_extend(ty);
        // What's stepped of what it's given: a collection's or a table's
        // iterator held, `it`, as std's `extend` holds it, stepped by
        // `next`, or of a list, a hash table or a B-tree by its `fold`,
        // `$folded(it)` (#1511); a string's, the items as they are.
        let by_fold = self.extends_by_fold(ty);
        let stepped = match known {
            Some(Std::Comb(_) | Std::Map(_)) if by_fold => Expr::call_helper("$folded", vec![Expr::var("it")]),
            Some(Std::Comb(_) | Std::Map(_)) => Expr::var("it"),
            _ => Expr::var("items"),
        };
        // Its hint asked first where the collection reserves by it, a
        // `String`'s by `char`s and a hash table's (#849).
        let items = self.dictionary_reserved(stepped, ty, item, collection.clone());
        let mut body = Vec::new();
        // Grown as std's grows it, where capacities are kept (#1180).
        let items = self.dictionary_grown(collection.clone(), items, (ty, item), span, &mut body)?;
        // The items' type said as the collection's, which no range is, so
        // what's given is iterated as it is.
        let given = [ty, ty];
        let none = ty::List::empty();
        let value = match known {
            // Each piece appended as it's stepped, as std's `for_each`
            // pushes each, so a panic part-way leaves what came before (#1127).
            Some(Std::PushStr) => {
                let piece = self.fresh(Role::Piece);
                let added = Expr::bin(Op::Add, collection.clone(), Expr::var(&piece));
                let push = vec![StmtKind::Assign(collection.clone(), added).at(js::Span::NONE)];
                let stepped = crate::lower::std_types::text::extend_text(items.clone(), &piece, push);
                body.push(StmtKind::Expr(stepped).at(js::Span::NONE));
                None
            }
            // Stepped as std's `extend` of it steps it, by `next`, or of a
            // list, a hash table or a B-tree by its `fold`, `$folded`'s; then
            // the iterator it was given dropped, `dropIter`, as std drops it
            // at its end and as a step's panic unwinds, but where its own
            // `fold` took it, which drops it itself (#1511).
            Some(known @ (Std::Comb(_) | Std::Map(_))) => {
                let it = Expr::var("it");
                let extended = match known {
                    Std::Comb(comb) => self.comb_values(
                        comb,
                        (vec![collection.clone(), items], &given),
                        false,
                        none,
                        span,
                        &mut body,
                    )?,
                    Std::Map(op) => {
                        self.map_values(op, (vec![collection.clone(), items], &given), none, span, &mut body)?
                    }
                    _ => unreachable!("matched above"),
                };
                // `dropIter`, or of a list's or a table's, taken by its fold,
                // none where that's its own, which drops it.
                let drop_it = match by_fold {
                    true => Expr::var("dropIt"),
                    false => Expr::var("dropIter"),
                };
                let made = Expr::call_helper("$intoIter", vec![Expr::var("items")]);
                body.insert(0, StmtKind::Const("it".into(), made).at(js::Span::NONE));
                if by_fold {
                    let own = Expr::cond(
                        Expr::member(it.clone(), "ownFold"),
                        Expr::undefined(),
                        Expr::var("dropIter"),
                    );
                    body.insert(1, StmtKind::Const("dropIt".into(), own).at(js::Span::NONE));
                }
                let dropped = || Expr::optional_call(drop_it.clone(), vec![it.clone()]);
                let run = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(extended)).at(js::Span::NONE)]);
                let unwound = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dropped())).at(js::Span::NONE)]);
                body.push(StmtKind::Expr(Expr::call_helper("$unwinds", vec![run, unwound])).at(js::Span::NONE));
                body.push(StmtKind::Expr(dropped()).at(js::Span::NONE));
                let params = ["collection", "items", "inPlace", "dropItems", "dropIter"];
                let extend = Expr::arrow(params.into_iter().map(Into::into).collect(), body);
                return Ok(Some(Expr::object(vec![Prop::Field("extend".into(), extend)])));
            }
            _ => return Ok(None),
        };
        body.extend(value.map(|value| StmtKind::Expr(value).at(js::Span::NONE)));
        let extend = Expr::arrow(vec!["collection".into(), "items".into()], body);
        Ok(Some(Expr::object(vec![Prop::Field("extend".into(), extend)])))
    }

    /// `tr`'s dictionary, written where it's asked. Of a type known whole,
    /// one that holds a function, a compiled method given its sources'
    /// dictionaries, is kept: one its body writes twice or more is declared
    /// once at its top (`declare_repeated`, ADR 0979). So is a generic
    /// impl's, a call of its function made of what the function was given,
    /// `holderTouch(TTouch)`, a copied default's of its own impl among them,
    /// named as a bound's dictionary is, `HolderTouch` (ADR 0979, amended,
    /// #691). An entry of a given one, called, is the body's own
    /// (`given_entries`).
    pub(in crate::lower) fn dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let made = self.dictionary_made(tr, span)?;
        let named = if is_impl_call(&made) {
            let word = bound_word(self.tcx, tr);
            Some((word.clone(), word))
        } else if !rustc_middle::ty::TypeVisitableExt::has_non_region_param(&tr)
            && !self.in_copied_default()
            && holds_a_function(&made)
        {
            let shared = crate::lower::lower_first(self.tcx.item_name(tr.def_id).as_str());
            Some((self.dictionary_name(tr), shared))
        } else {
            None
        };
        if let Some((name, shared)) = named
            && reads_only_at_top(&made, &self.given_names())
        {
            self.keep_dictionary(&made, name, shared);
        }
        Ok(made)
    }

    /// `io::Read`'s, `BufRead`'s or `Seek`'s of one of std's byte sources
    /// (#1156).
    fn byte_source_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        use crate::lower::recognition::ByteSource;
        // A `&mut` to a reader taken by value, `R = &mut c`, std's `impl
        // Read for &mut R`, given in a box as a `&mut` writer is (ADR 0454):
        // its own dictionary, through the box, as `$mutWriter`'s.
        if let ty::Ref(_, inner, ty::Mutability::Mut) = *tr.self_ty().kind() {
            let reader = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span)?;
            return Ok(Expr::call_helper("$mutReader", vec![reader]));
        }
        // A `dyn`'s, given the pair: through the pair's own, `$dynReader`.
        if matches!(tr.self_ty().kind(), ty::Dynamic(..)) {
            return Ok(Expr::helper("$dynReader"));
        }
        // std's `impl Read for Box<R>`, through it: what it holds is a
        // `Box`'s JS, so its reader's own, a `dyn`'s too.
        if let Some(inner) = tr.self_ty().boxed_ty() {
            return self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span);
        }
        match self.recognition().byte_source(tr.self_ty()) {
            Some(ByteSource::Slice) => Ok(Expr::helper("$sliceReader")),
            Some(ByteSource::Empty) => Ok(Expr::helper("$emptyReader")),
            Some(ByteSource::Repeat) => Ok(Expr::helper("$repeatReader")),
            Some(ByteSource::File) => Ok(Expr::helper("$fileReader")),
            Some(ByteSource::Cursor(inner)) => {
                let as_ref = crate::lower::recognition::std_item(self.tcx, StdItem::AsRef);
                let bytes = ty::Ty::new_slice(self.tcx, self.tcx.types.u8);
                // A `&mut` to an object, which the cursor made here holds as
                // the object, where a dictionary of the `&mut` reads generic
                // code's box of it (#1414): the object's, which std's `AsRef`
                // of the `&mut` passes it on to.
                let inner = match *inner.kind() {
                    ty::Ref(_, object, Mutability::Mut)
                        if self.in_generic_box(inner) && !self.mut_ref_form(object).is_cell() =>
                    {
                        object
                    }
                    _ => inner,
                };
                let of = self.dictionary(ty::TraitRef::new(self.tcx, as_ref, [inner, bytes]), span)?;
                Ok(Expr::call_helper("$cursorReader", vec![of]))
            }
            // std's bodies over its buffer, given its inner's `Read`, or
            // `Seek`: what its impls ask of `R`.
            Some(ByteSource::BufReader(inner)) => {
                let asked = match is_std_def(self.tcx, tr.def_id, StdItem::IoSeek) {
                    true => StdItem::IoSeek,
                    false => StdItem::IoRead,
                };
                let of = crate::lower::recognition::std_item(self.tcx, asked);
                let reader = self.dictionary(ty::TraitRef::new(self.tcx, of, [inner]), span)?;
                Ok(Expr::call_helper("$bufReader", vec![reader]))
            }
            // Its `read` and `fill_buf` read its front slice alone, which
            // a JS array has none of: a dictionary would hold them (#1302).
            Some(ByteSource::Deque) => Err(self.unsupported(
                span,
                &format!(
                    "a `VecDeque<u8>` as generic code's `io::{}`, whose `read` and `fill_buf` read its front slice alone (#1302)",
                    self.tcx.item_name(tr.def_id)
                ),
            )),
            // A `Chain`'s or a `Take`'s, its impl's in `@rust-js/std`, and of
            // its `Read`, what that doesn't write std's defaults over its
            // `read`, `$readerDefaults`, alloc's `default_read_to_end` ported,
            // which a reader of the crate's own among its sources sees as
            // std's (ADR 0425, amended, #1340, #1297).
            None if crate::lower::recognition::is_std_reader(self.tcx, tr.self_ty()) => {
                let made = (self.impl_dictionary(tr, span)?).ok_or_else(|| self.no_evidence(span, tr))?;
                if !is_std_def(self.tcx, tr.def_id, StdItem::IoRead) {
                    return Ok(made);
                }
                // And its `read_buf`, of its sources' `Read`, which std's
                // `default_read_to_end` reads into: `$takeReadBuf(TRead)` or
                // `$followedReadBuf(TRead, URead)` (#1340).
                let (chain, sources) = crate::lower::recognition::std_reader_sources(self.tcx, tr.self_ty())
                    .expect("a `Chain` or a `Take`, `is_std_reader`'s");
                let read_bufs = (sources.into_iter())
                    .map(|source| self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [source]), span))
                    .collect::<R<Vec<_>>>()?;
                let read_buf = Expr::call_helper(if chain { "$followedReadBuf" } else { "$takeReadBuf" }, read_bufs);
                Ok(Expr::call_helper("$readerDefaults", vec![made, read_buf]))
            }
            // A reader of the crate's own, its impl's dictionary: of its
            // `Read`, what that doesn't write std's defaults over its `read`,
            // `$readerDefaults` its impl wraps it in (#1297); of its `BufRead`
            // and `Seek`, its own and std's defaults over them (#1489).
            None => (self.impl_dictionary(tr, span)?).ok_or_else(|| self.no_evidence(span, tr)),
        }
    }

    fn dictionary_made(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        // `<Words as Source>::Item: Debug` of a bound instantiated: `String: Debug`;
        // in a copied default, where the impl's bounds and its own hold, in
        // the impl's terms: `K: Hash` of `<Map<T, F> as Iterator>::Item`.
        let tr = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))
            .unwrap_or(tr);
        // serde's: the function that writes or reads the type (ADR 0081).
        match crate::lower::serde::serde_trait(self.tcx, tr.def_id) {
            Some(true) => return self.json_writer(tr.self_ty(), span),
            Some(false) => return self.json_reader(tr.self_ty(), span),
            None => {}
        }
        if let Some(found) = self.evidence_for(tr) {
            return Ok(found);
        }
        // std's byte sources' `io::Read`, `BufRead` and `Seek` (#1156): the
        // runtime's dictionary of each, a `Cursor<T>`'s made of `T`'s
        // `AsRef<[u8]>`. A reader of the crate's own is its impl's, wrapped in
        // `$readerDefaults` (#1297); its `BufRead` and `Seek` are never
        // lowered (ADR 0425, #1489).
        if crate::lower::recognition::is_io_reading(self.tcx, tr.def_id) {
            return self.byte_source_dictionary(tr, span);
        }
        // One of std's errors' `Error`, as its `dyn`'s is.
        if is_std_def(self.tcx, tr.def_id, StdItem::Error)
            && let Some(dictionary) = self.std_error_dictionary(tr.self_ty())
        {
            return Ok(dictionary);
        }
        // A `&mut` generic code holds as its box (ADR 0454), of a trait std
        // passes on from a `&mut T` to its `T`, `impl<T: Display> Display
        // for &mut T`: `T`'s dictionary, each entry given what each box
        // holds (#1414). Not a writer's or `AsMut`'s, whose own read
        // through it, `$mutWriter`'s and `as_mut`'s, nor an iterator's, a
        // `&mut I` of which is given as it is (ADR 0061).
        if self.in_generic_box(tr.self_ty())
            && !rustc_middle::ty::elaborate::supertraits(self.tcx, ty::Binder::dummy(tr))
                .any(|tr| crate::lower::recognition::is_iterator_trait(self.tcx, tr.def_id()))
            && !is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite)
            && !is_std_def(self.tcx, tr.def_id, StdItem::IoWrite)
            && !is_std_def(self.tcx, tr.def_id, StdItem::AsMut)
            && let Some(referent) = self.forwarded_to_referent(tr, self.typing_env)
        {
            return self.through_box(tr, referent, span);
        }
        // A std hasher's `Hasher`, and a std type's `Hash` (ADR 0463).
        if let Some(dictionary) = self.std_hash_dictionary(tr, span)? {
            return Ok(dictionary);
        }
        // An `Iterator`'s, of a type that isn't a parameter given one (ADR
        // 0411). One given as it is, which a JS iterator's `next` can't step,
        // that the analysis found nothing here steps: rust-js's own error.
        if is_std_def(self.tcx, tr.def_id, StdItem::Iterator) {
            // A `&mut I`'s, of what a `&mut` of an iterator is, the iterator:
            // `it.by_ref().take(2)` of a range, or of a type parameter's, by
            // the dictionary it was given (ADR 0912).
            if let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind() {
                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;
                return Ok(self.mut_ref_iterator_dictionary(of));
            }
            if let ty::Param(param) = tr.self_ty().kind()
                && self.param_given_as_it_is(*param)
            {
                return Err(self.unsupported(
                    span,
                    &format!("stepping `{}`, which rust-js found nothing here steps", tr.self_ty()),
                ));
            }
            return self.iterator_dictionary(tr.self_ty(), span);
        }
        // Std's `Try` of what its generic code is given (ADR 0913), and its
        // supertrait `FromResidual`'s, the same dictionary.
        if crate::lower::recognition::is_try_trait(self.tcx, tr.def_id) {
            use crate::lower::recognition::TryKind;
            return match crate::lower::recognition::try_kind(self.tcx, tr.self_ty()) {
                Some(TryKind::ControlFlow) => Ok(Expr::helper("$controlFlowTry")),
                Some(TryKind::NeverShortCircuit) => Ok(Expr::helper("$neverShortCircuitTry")),
                None => Err(self.unsupported(
                    span,
                    &format!("the `Try` of a `{}` given to std's generic code", tr.self_ty()),
                )),
            };
        }
        // An iterator's extension's, of one given as it is (ADR 0487); a
        // `&mut` to a type only its bounds know, of the dictionary it was
        // given, as std's `&mut I` passes on to its `I` (ADR 0937).
        if crate::lower::recognition::is_iterator_extension(self.tcx, tr.def_id) {
            if let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind()
                && self.is_unknown(*inner)
            {
                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;
                return Ok(self.mut_ref_extension_dictionary(tr.def_id, of));
            }
            return self.iterator_extension_dictionary(tr.def_id, tr.self_ty(), span);
        }
        // `Any`'s: its type's key, `{ type_id: () => "i32" }` (ADR 0331).
        if any_trait(self.tcx) == Some(tr.def_id) {
            let key = self.type_fact_value(tr.self_ty(), TypeFact::Id, span)?;
            let type_id = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(key)).at(js::Span::NONE)]);
            let mut props = vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "type_id"),
                type_id,
            )];
            props.extend(self.dictionary_drop(tr.self_ty(), span)?);
            return Ok(Expr::object(props));
        }
        // A marker's of a type a bound gave none for, as none passes one: a
        // type parameter made a `dyn`, `Box::new(value)` of a `T: Marker`.
        // What its impl's would have, its drop and its supertraits', made here.
        if tr.self_ty().has_param() && is_marker(self.tcx, self.krate.foreign, tr.def_id) {
            let mut props = self.supertrait_entries(tr, span)?;
            props.extend(self.dictionary_drop(tr.self_ty(), span)?);
            return Ok(Expr::object(props));
        }
        // Derived and std impls of `Default` and `Clone` (ADR 0052).
        let ty = tr.self_ty();
        // A range's `RangeBounds`, of a range of std's or a pair of `Bound`s:
        // each bound, a `Bound` of what its field holds, `{ TAG: "Included",
        // _0: range.start }`, as any of its values is.
        if is_std_def(self.tcx, tr.def_id, StdItem::RangeBounds)
            && crate::lower::recognition::range_bounds(self.tcx, ty).is_some()
        {
            let start_bound = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "start_bound");
            let bound = self.tcx.instantiate_and_normalize_erasing_regions(
                tr.args,
                self.typing_env,
                self.tcx.fn_sig(start_bound).map_bound(|sig| sig.skip_binder().output()),
            );
            let mut made = Vec::new();
            let values = self.range_bound_values(Expr::var("range"), ty, bound, span, &mut made)?;
            let values = values.expect("a range's bounds, as `range_bounds` found them");
            let props = ["start_bound", "end_bound"]
                .into_iter()
                .zip(values)
                .map(|(name, value)| {
                    let body = vec![StmtKind::Return(Some(value)).at(js::Span::NONE)];
                    let name = bindings::trait_method_name(self.tcx, tr.def_id, name);
                    Prop::Field(name, Expr::arrow(vec!["range".into()], body))
                });
            return Ok(Expr::object(props.collect()));
        }
        // A std collection's `FromIterator`, `{ from_iter(items) }`: what
        // `collect` into it makes, of the items given, any iterable (ADR 0445).
        // Not the crate's own impl, `impl FromIterator<C> for Req`, of the
        // `A` it's given: its `from_iter` is its entry (#794).
        if is_std_def(self.tcx, tr.def_id, StdItem::FromIterator) && !self.is_user_impl(tr) {
            let from_iter = self.collecting_entry(ty, tr.args.type_at(1), span)?;
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "from_iter"),
                from_iter,
            )]));
        }
        // std's `Sum` or `Product` of a number, `{ sum(items) }`: what
        // `sum()` of them makes (ADR 0476).
        if let Some(name) = crate::lower::recognition::accumulation(self.tcx, tr.def_id)
            && let Some(num) = Num::of(ty)
            && !self.has_user_impl(tr.def_id, ty)
        {
            let items = self.js_iterator(Expr::var("items"));
            let made = self.accumulated_numbers(items, num, name == "sum");
            let body = vec![StmtKind::Return(Some(made)).at(js::Span::NONE)];
            return Ok(Expr::object(vec![Prop::Field(
                name.into(),
                Expr::arrow(vec!["items".into()], body),
            )]));
        }
        // std's `Sum` or `Product` of a `Wrapping` or a `Saturating`, of
        // itself or of a reference to it, `{ sum(items) }`: its own `+` or
        // `*` folded from its `[0]` or `[1]`, as std's `iter.fold(Wrapping(0),
        // |a, b| a + b)` is, wrapping or saturating each step (#1445).
        if let Some(name) = crate::lower::recognition::accumulation(self.tcx, tr.def_id)
            && let Some(newtype) = self.recognition().int_newtype(ty)
            && let Some(num) = Num::of(newtype.0)
            && !self.has_user_impl(tr.def_id, ty)
        {
            let (op, start) = match name == "sum" {
                true => (rustc_middle::mir::BinOp::Add, 0),
                false => (rustc_middle::mir::BinOp::Mul, 1),
            };
            let (a, b) = (self.fresh(Role::A), self.fresh(Role::B));
            let mut step = Vec::new();
            let item = Some((Expr::index(Expr::var(&b), Expr::int(0)), newtype.0));
            let total = self.newtype_result(Ok(op), Expr::var(&a), item, newtype, span, &mut step)?;
            step.push(StmtKind::Return(Some(total)).at(js::Span::NONE));
            let f = Expr::arrow(vec![a.into(), b.into()], step);
            let items = self.js_iterator(Expr::var("items"));
            let made = Expr::array(vec![Expr::call(
                Expr::member(items, "reduce"),
                vec![f, num.literal(start)],
            )]);
            let body = vec![StmtKind::Return(Some(made)).at(js::Span::NONE)];
            return Ok(Expr::object(vec![Prop::Field(
                name.into(),
                Expr::arrow(vec!["items".into()], body),
            )]));
        }
        // A std collection's `Extend`, `{ extend(collection, items) }` (ADR 0445).
        if crate::lower::recognition::is_extend(self.tcx, tr.def_id)
            && let Some(dictionary) = self.extend_dictionary(ty, tr.args.type_at(1), span)?
        {
            return Ok(dictionary);
        }
        // std's `Index<usize>` of a slice, an array or a `Vec`, `{ index(items,
        // i) }`: its item, checked as `v[i]` is, rand's `SliceChooseIter` of an
        // `S: Index<usize>` (#1420).
        if self.tcx.is_lang_item(tr.def_id, LangItem::Index)
            && !self.has_user_impl(tr.def_id, ty)
            && tr.args.type_at(1).is_usize()
            && (ty.is_slice() || ty.is_array() || self.is_vec_like(ty))
        {
            let item = self.index_checked(Expr::var("items"), Expr::var("i"), ty);
            let index = Expr::arrow(
                vec!["items".into(), "i".into()],
                vec![StmtKind::Return(Some(item)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field("index".into(), index)]));
        }
        // `usize`'s or a range's `SliceIndex` of a slice (ADR 0418): the
        // runtime's, of each kind, `$sliceIndexes.range`.
        if is_std_def(self.tcx, tr.def_id, StdItem::SliceIndex) {
            use crate::lower::recognition::RangeBound::{Field, Unbounded};
            let kind = match ty.is_usize() {
                true => Some("item"),
                false => match crate::lower::recognition::range_bounds(self.tcx, ty) {
                    Some((Field("Included", _), Field("Excluded", _))) => Some("range"),
                    Some((Field("Included", _), Unbounded)) => Some("from"),
                    Some((Unbounded, Field("Excluded", _))) => Some("to"),
                    Some((Field("Included", _), Field("Included", _))) => Some("inclusive"),
                    Some((Unbounded, Field("Included", _))) => Some("toInclusive"),
                    Some((Unbounded, Unbounded)) => Some("full"),
                    _ => None,
                },
            };
            let kind = kind.ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::member(Expr::helper("$sliceIndexes"), kind));
        }
        // std's `LowerHex` and the like of an integer: its digits, given the
        // options its caller is, as std's `Debug` of one is (ADR 0185).
        if let Some(num) = Num::of(ty).filter(|n| !n.float())
            && let Some(radix) = self.radix_trait(tr.def_id)
            && !self.has_user_impl(tr.def_id, ty)
        {
            let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() {
                true => (
                    vec!["value".into(), "options".into()],
                    Pretty::When(Expr::var("options")),
                ),
                false => (vec!["value".into()], Pretty::Plain),
            };
            let text = self.radix_text(Expr::var("value"), num, radix, &pretty);
            let fmt = Expr::arrow(params, vec![StmtKind::Return(Some(text)).at(js::Span::NONE)]);
            return Ok(Expr::object(vec![Prop::Field("fmt".into(), fmt)]));
        }
        let default = is_std_def(self.tcx, tr.def_id, StdItem::Default);
        let clone = self.tcx.is_lang_item(tr.def_id, LangItem::Clone);
        let eq = self.tcx.is_lang_item(tr.def_id, LangItem::PartialEq);
        let display = tr.def_id == self.display_trait();
        let to_string = Some(tr.def_id) == self.to_string_trait();
        let debug = tr.def_id == self.debug_trait();
        let ord = tr.def_id == self.ord_trait();
        let partial_ord = tr.def_id == self.partial_ord_trait();
        // `PartialEq<u32>` or `PartialOrd<u32>` of another type than itself: its
        // impl's, never a comparison of it with itself.
        let of_other = (eq || partial_ord)
            && tr
                .args
                .types()
                .nth(1)
                .is_some_and(|rhs| rhs.peel_refs() != ty.peel_refs());
        // A class's `Display` is its `toString`, which JS's `String` calls (#358).
        let shown = (display || to_string) && self.is_shown_class(ty);
        if (default || clone || eq || display || to_string || debug || ord || partial_ord)
            && !of_other
            && (!self.has_user_impl(tr.def_id, ty) || shown)
        {
            if debug {
                // Given a `Formatter`'s options, where the crate's writers are
                // (ADRs 0058, 0137), or a library's function shows it with
                // them (#1331).
                let shown_so = self.krate.shown.of_type(self.tcx, ty);
                let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() || shown_so.any() {
                    true => (
                        vec!["value".into(), "options".into()],
                        crate::lower::display::Pretty::When(Expr::var("options")),
                    ),
                    false => (vec!["value".into()], crate::lower::display::Pretty::Plain),
                };
                let mut body = Vec::new();
                let shown = self.with_shown(shown_so, |cx| {
                    cx.debug_string_with(Expr::var("value"), ty, span, &pretty)
                })?;
                body.push(StmtKind::Return(Some(shown)).at(js::Span::NONE));
                let fmt = Expr::arrow(params, body);
                let mut props = vec![Prop::Field("fmt".into(), fmt)];
                // Its type's drop, as an impl's dictionary has it: a `Box<dyn
                // Debug>` of a `Vec<Point>` drops its points (#1578).
                props.extend(self.dictionary_drop(ty, span)?);
                return Ok(Expr::object(props));
            }
            // Each with its supertraits', as an impl's is, which generic code
            // reaches by rustc's route, `TOrd.PartialOrd().lt` (#944): std's
            // quiet orders' shared ones, `quiet_dictionary`'s; a
            // `PartialOrd`'s `lt` and the rest, std's, `$partialOrd`'s, and
            // an `Ord`'s `max`, `min` and `clamp`, `$ord`'s (#962).
            if ord || partial_ord {
                if let Some(shared) = self.quiet_dictionary(ty, ord) {
                    return Ok(Expr::helper(shared));
                }
                // Of a type its module knows, its module's `const`, made once
                // and read wherever it's given, `pointPartialOrd`, as the
                // crate's own impls' are.
                let make = |cx: &mut Self| -> R<Expr> {
                    let (name, compare) = if ord {
                        ("cmp", cx.cmp_fn(ty, false, span)?)
                    } else {
                        ("partial_cmp", cx.cmp_fn(ty, true, span)?)
                    };
                    let mut props = cx.supertrait_entries(tr, span)?;
                    props.push(Prop::Field(
                        bindings::trait_method_name(cx.tcx, tr.def_id, name),
                        compare,
                    ));
                    let maker = if ord { "$ord" } else { "$partialOrd" };
                    Ok(Expr::call(Expr::helper(maker), vec![Expr::object(props)]))
                };
                if rustc_middle::ty::TypeVisitableExt::has_non_region_param(&tr)
                    || ty.has_aliases()
                    || self.in_copied_default()
                {
                    return make(self);
                }
                let base = self.dictionary_name(tr);
                return self.module_dictionary((tr.def_id, ty, &base), make);
            }
            // Each with its type's drop, as an impl's dictionary has it: a
            // `Box<dyn ToString>` of a `Point` drops it (#1578).
            if display {
                let fmt = self.display_fn(ty, span)?;
                let mut props = vec![Prop::Field("fmt".into(), fmt)];
                props.extend(self.dictionary_drop(ty, span)?);
                return Ok(Expr::object(props));
            }
            // `ToString` is std's of a `Display`: what that shows.
            if to_string {
                let fmt = self.display_fn(ty, span)?;
                let mut props = vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "to_string"),
                    fmt,
                )];
                props.extend(self.dictionary_drop(ty, span)?);
                return Ok(Expr::object(props));
            }
            // `$partialEq`'s, which gives std's `ne`, `!eq`, but of what
            // asks its parts' `!=`, a tuple's, its own (#962).
            if eq {
                if self.eq_is_strict(ty) {
                    return Ok(Expr::helper("$strictlyEqual"));
                }
                let mut props = vec![Prop::Field("eq".into(), self.eq_fn(ty, span)?)];
                if self.own_ne(ty) {
                    props.push(Prop::Field("ne".into(), self.ne_fn(ty, span)?));
                }
                return Ok(Expr::call(Expr::helper("$partialEq"), vec![Expr::object(props)]));
            }
            if default {
                let value = self.default_value(ty, span)?;
                return Ok(Expr::object(vec![Prop::Field(
                    "default".into(),
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
                )]));
            }
            if clone {
                let clone = self.clone_fn("value", ty, span)?;
                let mut props = vec![Prop::Field("clone".into(), clone)];
                // And its `clone_from`, where std's is its own, which generic
                // code asks first (#928).
                if let Some(clone_from) = self.dictionary_clone_from(ty, span)? {
                    let name = bindings::trait_method_name(self.tcx, tr.def_id, "clone_from");
                    props.push(Prop::Field(name, clone_from));
                }
                return Ok(Expr::object(props));
            }
        }
        // std's `AsRef` of text as a `str`, or of a `Vec`, an array or a slice
        // as a slice: each is the same JS value (ADR 0162).
        // A `String`'s, or a `Formatter`'s, given in a box of its text: what's
        // written is added to it (ADR 0180).
        if is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) {
            let text =
                |ty: Ty<'tcx>| self.is_lang_adt(ty, LangItem::String) || self.recognition().is_formatter_type(ty);
            if text(ty) {
                return Ok(Expr::helper("$stringWriter"));
            }
        }
        // A `&mut` to a writer taken by value, `W = &mut s`, given in a box
        // (ADR 0454), a generic one's too: its own dictionary, through the
        // box, of a `fmt::Write` and an `io::Write` alike.
        // Not a `&mut [u8]`, whose `io::Write` is its own, std's, its slice
        // re-pointed, `$sliceWriter` (#1282).
        if (is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) || is_std_def(self.tcx, tr.def_id, StdItem::IoWrite))
            && let ty::Ref(_, inner, Mutability::Mut) = *ty.kind()
            && !(is_std_def(self.tcx, tr.def_id, StdItem::IoWrite) && crate::lower::recognition::is_byte_slice(inner))
        {
            let writer = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [inner]), span)?;
            return Ok(Expr::call_helper("$mutWriter", vec![writer]));
        }
        // A JS object type's `Deref` to the class it extends, which is never
        // lowered: the object itself, `(x) => x`, as a call of it is (ADR
        // 0024); its `DerefMut`'s, given the cell its `&mut self` is, that
        // cell, as of any JS object (ADR 0454) (#403).
        let deref = self.tcx.is_lang_item(tr.def_id, LangItem::Deref);
        if (deref || self.tcx.is_lang_item(tr.def_id, LangItem::DerefMut))
            && let Some(imp) = self.recognition().user_impl(tr)
            && crate::lower::recognition::is_js_object_deref(self.tcx, imp)
        {
            let (method, param) = if deref { ("deref", "x") } else { ("deref_mut", "cell") };
            let method = crate::lower::recognition::trait_method(self.tcx, tr.def_id, method);
            let f = Expr::arrow(
                vec![param.into()],
                vec![StmtKind::Return(Some(Expr::var(param))).at(js::Span::NONE)],
            );
            let mut props = match deref {
                true => Vec::new(),
                false => self.supertrait_entries(tr, span)?,
            };
            props.push(Prop::Field(bindings::fn_name(self.tcx, method), f));
            return Ok(Expr::object(props));
        }
        // std's `Deref` of a `P: Deref` in generic code: what std's `deref`
        // of the type is, as a function, `(x) => x` of a reference, a `Box`
        // or text, which are their values (ADR 0394). The crate's own is its
        // impl's.
        if self.tcx.is_lang_item(tr.def_id, LangItem::Deref) && !self.has_user_impl(tr.def_id, ty) {
            // A `&mut`'s, which generic code is given in a box: what's in it
            // (ADR 0454).
            if let ty::Ref(_, _, Mutability::Mut) = ty.kind() {
                let value = Expr::held(Expr::var("x"));
                let f = Expr::arrow(vec!["x".into()], vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]);
                return Ok(Expr::object(vec![Prop::Field("deref".into(), f)]));
            }
            let deref = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "deref");
            let function = Ty::new_fn_def(self.tcx, deref, ty::Binder::dummy(tr.args));
            if let Some(known) = self.recognition().classify(deref, tr.args)
                && let Some(f) = self.std_fn_value(known, function, span)?
            {
                return Ok(Expr::object(vec![Prop::Field("deref".into(), f)]));
            }
        }
        // std's `DerefMut` of a `P: DerefMut` in generic code, given the cell
        // its `&mut self` is and giving the one its `&mut Self::Target` is
        // (ADR 0454): of a pointer whose target is the same JS value, a
        // `Box`, text, a `Vec` or a JS object, that cell; of a `&mut` or a
        // guard, which holds a cell of its target, what's in it (ADR 0394).
        if self.tcx.is_lang_item(tr.def_id, LangItem::DerefMut) && !self.has_user_impl(tr.def_id, ty) {
            let deref_mut = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "deref_mut");
            let target = match self.recognition().classify(deref_mut, tr.args) {
                _ if matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut)) => Some(Expr::held(Expr::var("cell"))),
                Some(Std::GuardValue { .. }) => Some(Expr::held(Expr::var("cell"))),
                Some(Std::Same) => Some(Expr::var("cell")),
                _ if ty.is_box() => Some(Expr::var("cell")),
                _ => None,
            };
            if let Some(target) = target {
                let f = Expr::arrow(
                    vec!["cell".into()],
                    vec![StmtKind::Return(Some(target)).at(js::Span::NONE)],
                );
                let mut props = self.supertrait_entries(tr, span)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, deref_mut), f));
                return Ok(Expr::object(props));
            }
        }
        // std's `IntoIterator` of a `&C` or a `&mut C` a bound gives generic
        // code (ADR 0457): the JS iterator `into_iter` of it is where its type
        // is known, given what generic code has, a `&mut` to an object in a
        // box (ADR 0454): a handle on each item of a `&mut`, as `for x in &mut
        // v` makes, or std's own of what it's given.
        if self.ref_into_iter_of_collection(tr) {
            return self.ref_into_iter_dictionary(tr, self.held_in_box(ty), span);
        }
        // std's `IntoIterator` of one of the function's own type parameters
        // it holds as the iterable its caller gave, an `impl IntoIterator`
        // (ADR 0551), given to std's body compiled, `zip`'s `U`: the JS
        // iterator of it, as a `for` of it steps, stepped as one (ADR 0976).
        if is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && let ty::Param(param) = *ty.kind()
            && !self.param_given_as_it_is(param)
        {
            let items = crate::lower::iterators::given_into_iter(Expr::var("items"));
            let itself = Expr::arrow(
                vec!["items".into()],
                vec![StmtKind::Return(Some(items)).at(js::Span::NONE)],
            );
            let into_iter = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
            let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), itself)];
            props.extend(self.made_steps(tr, false, span)?);
            return Ok(Expr::object(props));
        }
        // std's `IntoIterator` of a type known here, given where what it
        // makes is held as it is (ADR 0552): `into_iter` of it as concrete
        // code has it, an iterator's, a range's too, itself, an array's
        // `$iter`; and the `Iterator` of what it makes, which steps that as
        // it is, `IntoIterIterator`, as the trait declares its `IntoIter`.
        // Of one only bounds know, an iterator's, std's for every iterator:
        // itself, stepped by the dictionary its bounds give.
        if is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && (!matches!(ty.kind(), ty::Ref(..)) || self.recognition().iterates_as_itself(ty))
            && (!crate::lower::recognition::known_by_bounds(ty) || self.recognition().iterates_as_itself(ty))
            && !self.has_user_impl(tr.def_id, ty)
        {
            let itself = Expr::arrow(
                vec!["items".into()],
                vec![StmtKind::Return(Some(Expr::var("items"))).at(js::Span::NONE)],
            );
            let steps = self.made_steps(tr, true, span)?;
            if self.recognition().iterates_as_itself(ty) {
                let mut props = vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "into_iter"),
                    itself,
                )];
                props.extend(steps);
                return Ok(Expr::object(props));
            }
            let into_iter = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
            let known = self
                .recognition()
                .classify(into_iter, tr.args)
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let into_iter_ty = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "IntoIter");
            let output = Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter_ty, tr.args);
            let output = (self.tcx)
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
                .unwrap_or(output);
            let mut body = Vec::new();
            let items = self.std_by_values(
                known,
                into_iter,
                tr.args,
                (&[ty], Default::default()),
                vec![Expr::var("items")],
                output,
                span,
                &mut body,
            )?;
            let made = self.std_iterator(items, output);
            body.push(StmtKind::Return(Some(made)).at(js::Span::NONE));
            let f = Expr::arrow(vec!["items".into()], body);
            let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
            props.extend(steps);
            return Ok(Expr::object(props));
        }
        // An `io::Write`'s bytes, of a `Vec<u8>`, added to it, or of a
        // standard stream, written to it (ADR 0392).
        if is_std_def(self.tcx, tr.def_id, StdItem::IoWrite)
            && let Some(writer) = self.recognition().io_writer(ty)
        {
            return Ok(Expr::helper(writer));
        }
        // std's `AsMut` of text as a `str`, of a sequence as a slice, or of a
        // `Box` as what it holds (`as_mut_is_itself`), given
        // the cell generic code's `&mut self` is, and giving the one its `&mut
        // T` is (ADR 0454): that cell, as a `&mut str` is its text's (ADR
        // 0334) and a `&mut [T]`'s holds its array. Of a `&mut` to one, std's
        // `impl AsMut<U> for &mut T`, that one's, through the cell the `&mut`
        // is: one `value` further for each.
        if is_std_def(self.tcx, tr.def_id, StdItem::AsMut) && !self.has_user_impl(tr.def_id, ty) {
            let mut cell = Expr::var("cell");
            let mut at = ty;
            while let ty::Ref(_, inner, Mutability::Mut) = *at.kind() {
                cell = Expr::held(cell);
                at = inner;
            }
            if self.recognition().as_mut_is_itself(ty, tr.args.type_at(1)) {
                let f = Expr::arrow(
                    vec!["cell".into()],
                    vec![StmtKind::Return(Some(cell)).at(js::Span::NONE)],
                );
                return Ok(Expr::object(vec![Prop::Field(
                    bindings::trait_method_name(self.tcx, tr.def_id, "as_mut"),
                    f,
                )]));
            }
        }
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().as_ref_is_itself(ty, tr.args.type_at(1))
        {
            // Through a counted `Rc`, what it points at, as far as what it's
            // asked as (#1503, #1518).
            let inside = self.through_boxes_to(Expr::var("value"), ty.peel_refs(), tr.args.type_at(1));
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(inside)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                same,
            )]));
        }
        // A path's `Component`'s, as a `Path` or an `OsStr`: its text (ADR 0716).
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && self.recognition().as_ref_is_component_text(ty, tr.args.type_at(1))
        {
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                Expr::helper("$componentText"),
            )]));
        }
        // std's `AsRef<[u8]>` of text: its UTF-8 bytes, lent, as `as_bytes()`
        // gives them (ADRs 0162, 0126).
        if is_std_def(self.tcx, tr.def_id, StdItem::AsRef)
            && !self.has_user_impl(tr.def_id, ty)
            && self.is_string_like(ty)
            && matches!(tr.args.type_at(1).kind(), ty::Slice(item) if *item == self.tcx.types.u8)
        {
            let bytes = Expr::call(Expr::helper("$utf8Bytes"), vec![Expr::var("value")]);
            let as_ref = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(bytes)).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "as_ref"),
                as_ref,
            )]));
        }
        // std's `Borrow` where it's the same JS value: a value as itself, text
        // as a `str`, a `Vec` as a slice (ADR 0167). Of a `&mut`, which
        // generic code holds as its box, `$cellOf(parser)` or a number's
        // cell, what the box holds (`Form::Box`, #1414); through a counted
        // `Rc`, what it points at, as far as what it's borrowed as (#1503,
        // #1518).
        let in_box = |value: Expr| {
            self.through_boxes_to(self.held_in_generic_box(ty)(value), ty.peel_refs(), tr.args.type_at(1))
        };
        if is_std_def(self.tcx, tr.def_id, StdItem::Borrow)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().borrows_as_itself(ty, tr.args.type_at(1))
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(in_box(Expr::var("value")))).at(js::Span::NONE)],
            );
            return Ok(Expr::object(vec![Prop::Field("borrow".into(), same)]));
        }
        // And its `BorrowMut` of what's an object, a `Vec` as a slice, which a
        // `&mut` to is it: the same array, and its `Borrow`'s entry (ADR 0483);
        // of a `&mut`, generic code's box of what it's to, the one it holds,
        // a number's too (#1414).
        if is_std_def(self.tcx, tr.def_id, StdItem::BorrowMut)
            && !self.has_user_impl(tr.def_id, ty)
            && self.recognition().borrows_as_itself(ty, tr.args.type_at(1))
            && (self.is_object(tr.args.type_at(1)) || ty.is_ref())
        {
            let same = Expr::arrow(
                vec!["value".into()],
                vec![StmtKind::Return(Some(in_box(Expr::var("value")))).at(js::Span::NONE)],
            );
            let mut props = vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "borrow_mut"),
                same,
            )];
            props.extend(self.supertrait_entries(tr, span)?);
            return Ok(Expr::object(props));
        }
        // std's `FromStr` of a number, a `bool`, a `char` or a `String`: what
        // `s.parse()` of it is (ADR 0161).
        let parsed_by_std = crate::lower::representation::Num::of(ty).is_some()
            || ty.is_bool()
            || ty.is_char()
            || self.is_lang_adt(ty, LangItem::String);
        if crate::lower::recognition::is_from_str(self.tcx, tr.def_id) && parsed_by_std {
            let mut body = Vec::new();
            let parsed = self.parse_as(Expr::var("s"), ty, span, &mut body)?;
            body.push(StmtKind::Return(Some(parsed)).at(js::Span::NONE));
            let from_str = Expr::arrow(vec!["s".into()], body);
            return Ok(Expr::object(vec![Prop::Field(
                bindings::trait_method_name(self.tcx, tr.def_id, "from_str"),
                from_str,
            )]));
        }
        // `std::num::Wrapping`'s and `Saturating`'s, its number's in its `[x]`
        // (ADR 0175, #1434).
        if let Some(op) = crate::lower::recognition::value_operator(self.tcx, tr.def_id)
            && let Some(newtype) = self.recognition().int_newtype(ty.peel_refs())
        {
            let a = Expr::index(Expr::var("a"), Expr::int(0));
            let mut body = Vec::new();
            let (params, rhs) = match op {
                Ok(_) => (vec!["a".into(), "b".into()], Some((Expr::var("b"), tr.args.type_at(1)))),
                Err(_) => (vec!["a".into()], None),
            };
            let value = self.newtype_result(op, a, rhs, newtype, span, &mut body)?;
            body.push(StmtKind::Return(Some(Expr::array(vec![value]))).at(js::Span::NONE));
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(params, body),
            )]));
        }
        // A number's `+` or `-`, as `a + b` of one is (ADR 0108), of a
        // number on each side: `impl Add<Meters> for f64` is the crate's.
        if let Some(op) = crate::lower::recognition::value_operator(self.tcx, tr.def_id)
            && self.primitive_operands(tr)
        {
            let params: Vec<js::Pattern> = match op {
                Ok(_) => vec!["a".into(), "b".into()],
                Err(_) => vec!["value".into()],
            };
            let (a, b) = match op {
                Ok(_) => (Expr::var("a"), Some(Expr::var("b"))),
                Err(_) => (Expr::var("value"), None),
            };
            let value = self.number_operator(op, a, b, tr, span)?;
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(params, vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
            )]));
        }
        // A number's `+=`, of a number on each side, and a `String`'s of
        // text, through the box a generic `&mut` is given in (ADR 0180): its
        // `value` assigned `a + b` (ADR 0396).
        let text = |t: Ty<'tcx>| self.is_lang_adt(t, LangItem::String) || t.peel_refs().is_str();
        if let Some(op) = crate::lower::recognition::assign_operator(self.tcx, tr.def_id)
            && (self.primitive_operands(tr) || (op == BinOp::Add && tr.args.types().all(text)))
        {
            let place = Expr::member(Expr::var("cell"), js::HELD);
            let value = match self.primitive_operands(tr) {
                true => self.number_operator(Ok(op), place.clone(), Some(Expr::var("b")), tr, span)?,
                false => Expr::bin(Op::Add, place.clone(), Expr::var("b")),
            };
            return Ok(Expr::object(vec![Prop::Field(
                self.operator_entry(tr.def_id),
                Expr::arrow(
                    vec!["cell".into(), "b".into()],
                    vec![StmtKind::Assign(place, value).at(js::Span::NONE)],
                ),
            )]));
        }
        if let Some(made) = self.conversion_dictionary(tr, ty, span)? {
            return Ok(made);
        }
        if self.tcx.is_lang_item(tr.def_id, LangItem::Copy) {
            let ty = tr.self_ty();
            // A type parameter whose `Copy` bound takes no copy function, as
            // each value its callers give is its own copy (ADR 0289): none,
            // as one of a type known so is given, to std's compiled
            // `Copied` too (ADR 0938).
            let item = self.tcx.typeck_root_def_id(self.item);
            if let ty::Param(param) = ty.kind()
                && crate::lower::instances::seen_whole(
                    self.tcx,
                    item,
                    crate::lower::instances::Fact::CopiedApart(self.krate.foreign),
                )
                && !self.krate.given.copied.contains(&(item, param.index))
            {
                return Ok(Expr::undefined());
            }
            if self.is_unknown(ty) {
                return Err(self.unsupported(span, "Copy without representation evidence"));
            }
            // What a `Copy` bound lets be copied, as each call of a `Copy`
            // `FnOnce` is (ADR 0246): a closure that changes what it
            // captured is a `Clone` bound's too, so the crate clones it
            // apart, and its copy is its clone (#1211).
            let value = Expr::var("value");
            let copy = self.copy(value.clone(), ty);
            // Its supertrait `Clone`'s, `Clone()`, where the type's clone
            // runs code, a hand-written `Clone` of a `Copy` type, which
            // generic code that clones through the `Copy` bound calls as
            // std's does (ADR 0623, amended, #1451).
            let clone = match self.copy_clone_runs_code(ty) {
                true => Some(self.dictionary(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]), span)?),
                false => None,
            };
            // A value that's its own copy is given none (ADR 0289, amended).
            if js::same_path(&copy, &value) && clone.is_none() {
                return Ok(Expr::undefined());
            }
            let mut props = vec![Prop::Field(
                "copy".into(),
                Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)],
                ),
            )];
            if let Some(clone) = clone {
                props.push(Prop::Field(
                    "Clone".into(),
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(clone)).at(js::Span::NONE)]),
                ));
            }
            return Ok(Expr::object(props));
        }
        if let Some(made) = self.impl_dictionary(tr, span)? {
            return Ok(made);
        }
        let typing_env = self.typing_env;
        // A trait object's: Rust's built-in `impl Trait for dyn Trait`.
        let selected = crate::lower::traits::select(self.tcx, self.typing_env, tr);
        if let Ok(ImplSource::Builtin(BuiltinImplSource::Object(_), _)) = selected {
            return self.object_dictionary(tr, span);
        }
        // A reference's, by std's impl that passes it on to what it refers
        // to, `impl<T: AsRef<U>> AsRef<U> for &T`: that one's, as a reference
        // is the value it refers to in JS.
        if let Some(referent) = self.forwarded_to_referent(tr, typing_env) {
            return self.dictionary(referent, span);
        }
        Err(self.no_evidence(span, tr))
    }

    /// The crate's own impl's accessor, or one a library exports (ADR 0100),
    /// or one of a library's its JS doesn't, which the crate compiles (ADR
    /// 0914, amended), std's in `@rust-js/std` among them: in a copied
    /// default, of the impl's terms, where its bounds and the default's
    /// hold, `Map<T, F>: Counting` of a blanket impl's. `None` of another.
    fn impl_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some((impl_id, impl_args)) = evidence_impl(self.tcx, self.typing_env, tr)
            .filter(|&(id, _)| self.krate.trait_impls.contains(&id) || self.krate.foreign.compiled(id))
        else {
            return Ok(None);
        };
        // A `&mut` to an object its generic code gives within what a bound
        // trait's method takes, which `T`'s dictionary's entry gives its
        // impl as generic code's box where it doesn't adapt it, a `Vec<&mut
        // Self>`'s (#685, #686): of the crate's impl, or a library's it
        // compiles itself.
        if let Some(here) = self.impl_nested_mut_object(impl_id, impl_args) {
            let what = format!("a `{here}` inside what a generic impl gives a method of a trait it's bound by (#1421)");
            return Err(self.unsupported(span, &what));
        }
        let callee = self.fn_ref(impl_id);
        let args = self.evidence_args(impl_id, impl_args, span)?;
        // One that takes nothing, its module's `const`, `circleShape` (ADR 0592).
        Ok(Some(match self.takes_evidence(impl_id) {
            true => Expr::call(callee, args),
            false => callee,
        }))
    }

    /// An `IntoIterator` dictionary's `Iterator` of what it makes,
    /// `IntoIterIterator`, which steps that as it is, as the trait declares
    /// its `IntoIter` (ADRs 0552, 0566): every one has it, a reference's too,
    /// as generic code that holds what it makes steps it (#278). And its
    /// associated types' drops, `$dropIntoIter`, as every impl's dictionary
    /// has them (`item_drops`, ADR 0178): what generic code holding what it
    /// makes drops it by, std's `Flatten` replacing its inner one (#1510):
    /// of the value itself, where it gives what it makes `as_it_is`.
    fn made_steps(&mut self, tr: ty::TraitRef<'tcx>, as_it_is: bool, span: Span) -> R<Vec<Prop>> {
        let mut steps = Vec::new();
        for (name, bound) in item_bounds(self.tcx, tr.def_id, tr.args) {
            if is_std_def(self.tcx, bound.def_id, StdItem::Iterator) {
                let of = self.tcx.erase_and_anonymize_regions(bound.self_ty());
                let of = (self.tcx)
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(of))
                    .unwrap_or(of);
                // A type parameter's given as it is, its own, as what an
                // `I: Iterator` makes is itself (ADR 0920).
                let dictionary = match self.iterator_evidence(of, bound.def_id) {
                    Some(given) => given,
                    None => self.iterator_dictionary(of, span)?,
                };
                steps.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
        steps.extend(self.item_drops(tr, as_it_is, span)?);
        Ok(steps)
    }

    /// Whether `tr`, std's `IntoIterator` of a reference, is a collection's,
    /// `&C`'s or `&mut C`'s, which `ref_into_iter_dictionary` makes: not a
    /// `&mut I` of an iterator, which std's impl for every iterator takes as
    /// it is, as rustc's selection says (`iterates_as_itself`, #742), nor
    /// one the crate implements, nor one a bound of this function's gives,
    /// `&'a I: IntoIterator`'s, its dictionary (#1264). The one answer, of a
    /// bound's dictionary and of a type parameter given one, `zip`'s `U`.
    pub(in crate::lower) fn ref_into_iter_of_collection(&self, tr: ty::TraitRef<'tcx>) -> bool {
        let ty = tr.self_ty();
        is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && matches!(ty.kind(), ty::Ref(..))
            && !self.recognition().iterates_as_itself(ty)
            && !self.has_user_impl(tr.def_id, ty)
            && self.evidence_for(tr).is_none()
    }

    /// std's `IntoIterator` of a `&C` or a `&mut C`, `tr`, as generic code
    /// holds it: in the box a `&mut` to an object is, `boxed`, as a `c: &mut
    /// C` of an unknown `C` holds it (ADR 0454); or as it is, as a `U:
    /// IntoIterator` given a `&mut [T]` holds it, std's `zip`'s and
    /// `chain`'s, which a bound by `IntoIterator` never boxes (ADR 0976).
    pub(in crate::lower) fn ref_into_iter_dictionary(
        &mut self,
        tr: ty::TraitRef<'tcx>,
        boxed: bool,
        span: Span,
    ) -> R<Expr> {
        let ty = tr.self_ty();
        let ty::Ref(_, pointee, mutability) = *ty.kind() else {
            return Err(self.no_evidence(span, tr));
        };
        let into_iter = crate::lower::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
        // A `&mut`'s, the cell generic code has, a box of an object's.
        let name = match mutability {
            Mutability::Mut => Role::Cell.word(),
            Mutability::Not => self.parameter_name(pointee).word(),
        };
        let given = match boxed {
            true => Expr::held(Expr::var(name)),
            false => Expr::var(name),
        };
        let mut body = Vec::new();
        let items = (self.ref_into_iter(given, ty, span, &mut body)?).ok_or_else(|| self.no_evidence(span, tr))?;
        body.push(StmtKind::Return(Some(items)).at(js::Span::NONE));
        let f = Expr::arrow(vec![name.into()], body);
        let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
        props.extend(self.made_steps(tr, false, span)?);
        Ok(Expr::object(props))
    }

    /// A dictionary's facts of its associated types, what generic code asks
    /// of `<A as Array>::Item` (ADR 0145): `ItemSize`, its type's for `tr`.
    pub(super) fn item_facts(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (item, item_ty) in self.item_types(tr) {
            for &(_, fact) in self.krate.type_facts.get(&item).into_iter().flatten() {
                let value = self.type_fact_value(item_ty, fact, span)?;
                props.push(Prop::Field(item_fact_key(self.tcx, item, fact), value));
            }
        }
        // And, of a library's trait, what it carries of a std supertrait's
        // associated type of its `Self`, `IteratorItemSize` of `<Self as
        // Iterator>::Item`, its type's for `tr`, which the library's defaults
        // compiled over `Self` read (#990). The crate's own trait's defaults
        // are copied into each of its impls, given it as a slot.
        let carried = match tr.def_id.is_local() {
            true => Vec::new(),
            false => self.carried_super(tr.def_id),
        };
        for (assoc, fact) in carried {
            let item_ty =
                crate::lower::analysis::type_facts::super_item_of(self.tcx, self.typing_env, assoc, tr.self_ty());
            let value = self.type_fact_value(item_ty, fact, span)?;
            props.push(Prop::Field(
                crate::lower::traits::super_item_fact_key(self.tcx, assoc, fact),
                value,
            ));
        }
        Ok(props)
    }

    /// Each associated type of `tr`'s trait, plain, and what it is for `tr`.
    pub(super) fn item_types(&self, tr: ty::TraitRef<'tcx>) -> Vec<(DefId, Ty<'tcx>)> {
        let items = self.tcx.associated_items(tr.def_id).in_definition_order();
        items
            .filter(|item| {
                // A trait method's `impl Trait` too, of a method of no
                // generics of its own and no other (#1296).
                self.tcx.def_kind(item.def_id) == DefKind::AssocTy
                    && (!item.is_impl_trait_in_trait() || super::words::one_impl_trait_of(self.tcx, item.def_id))
                    && self.tcx.generics_of(item.def_id).own_params.is_empty()
            })
            .filter_map(|item| {
                let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, item.def_id, tr.args);
                let normalized = self
                    .tcx
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection));
                Some((item.def_id, normalized.ok()?))
            })
            .collect()
    }

    fn object_dictionary(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let object = Expr::var("object");
        let principal = self
            .dyn_trait_ref(tr.self_ty(), tr.self_ty())
            .ok_or_else(|| self.no_evidence(span, tr))?;
        let mut props = self.supertrait_entries(tr, span)?;
        props.extend(self.item_bound_entries(tr, span)?);
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            // What a `dyn` can't be called with, and what a std trait's
            // dictionary hasn't got, as `lower_dictionary` leaves out.
            if !item.is_fn()
                || self.tcx.generics_require_sized_self(item.def_id)
                || (!self.krate.foreign.compiled_by_rust_js(tr.def_id)
                    && self.tcx.defaultness(item.def_id).has_value()
                    && !in_std_dictionary(self.tcx, item.def_id))
            {
                continue;
            }
            let inputs = self
                .tcx
                .fn_sig(item.def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs();
            // Without a `Formatter`, which isn't a JS parameter (ADR 0054), and
            // with a writer's options, where the crate's take them (ADRs 0058, 0137).
            let writer = self.formatter_param(item.def_id).is_some();
            // Or a type's a library's function shows with them, whose entry
            // takes them, passed on where it does (#1331).
            let options = self.writers_take_options() || self.krate.shown.reached().any();
            let count = inputs.len() - usize::from(writer) + usize::from(options && writer);
            // A `&mut self` method is given a box of the pair, as generic code
            // has a `&mut T` (ADR 0099), and gives the pair, as a call on the
            // `dyn` does: `object.value.impl.scale(object.value, k)`.
            let mutable = matches!(inputs[0].kind(), ty::Ref(_, _, Mutability::Mut));
            let pair = match mutable {
                true => Expr::held(object.clone()),
                false => object.clone(),
            };
            let dictionary = self
                .super_evidence(principal, tr, Expr::member(pair.clone(), js::DICTIONARY))
                .ok_or_else(|| self.no_evidence(span, tr))?;
            let this = match mutable {
                true => pair,
                false => Expr::held(pair),
            };
            let mut params: Vec<js::Pattern> = vec!["object".into()];
            let mut values = vec![this];
            for i in 1..count {
                params.push(format!("arg{i}").into());
                values.push(Expr::var(&format!("arg{i}")));
            }
            let name = bindings::fn_name(self.tcx, item.def_id);
            let call = Expr::call(Expr::member(dictionary.clone(), name.clone()), values);
            props.push(Prop::Field(
                name,
                Expr::arrow(params, vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),
            ));
        }
        props.extend(self.item_drops(tr, false, span)?);
        Ok(Expr::object(props))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std collection's `from_iter`, of `output` of `item`s, as generic
    /// code's `collect()` into a `B: FromIterator<A>` calls it,
    /// `fromIter(items, ..facts)`: the collect of an iterator generic code is
    /// given, `I: Iterator<Item = A>`, lowered as generic code's own collect
    /// of one is, the one lowering of it (#1239). A type parameter stands
    /// for `I`, bound so here alone, and each fact `from_iter`'s declaration
    /// carries of what it's given (`std_method_facts`), its `IInPlace`, is
    /// the entry's parameter after the items, as generic code's call gives
    /// it, so the call keeps its shape.
    fn collecting_entry(&mut self, output: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        let iterator = std_item(tcx, StdItem::Iterator);
        let from_iterator = std_item(tcx, StdItem::FromIterator);
        let from_iter = crate::lower::recognition::trait_method(tcx, from_iterator, "from_iter");
        // Numbered past any body's own, so it's no parameter of this one.
        let param = Ty::new_param(tcx, 1 << 20, rustc_span::Symbol::intern("I"));
        let item_def = crate::lower::recognition::trait_method(tcx, iterator, "Item");
        let bounds: [ty::Clause<'tcx>; 2] = [
            ty::TraitRef::new(tcx, iterator, [param]).upcast(tcx),
            ty::ProjectionPredicate {
                projection_term: match *Ty::new_projection(tcx, ty::IsRigid::No, item_def, [param]).kind() {
                    ty::Alias(_, alias) => alias.into(),
                    _ => unreachable!("a projection is an alias"),
                },
                term: item.into(),
            }
            .upcast(tcx),
        ];
        let clauses = self.typing_env.param_env.caller_bounds().iter().chain(bounds);
        let typing_env = ty::TypingEnv::new(
            ty::ParamEnv::new(tcx.mk_clauses_from_iter(clauses)),
            self.typing_env.typing_mode(),
        );
        let facts = crate::lower::traits::std_method_facts(tcx, self.krate.foreign, from_iter);
        let mut params: Vec<String> = vec!["items".into()];
        let given: Vec<(Ty<'tcx>, crate::lower::recognition::TypeFact, Expr)> = (facts.iter())
            .map(|&(_, fact)| {
                let name = format!("I{}", fact.word());
                params.push(name.clone());
                (param, fact, Expr::var(&name))
            })
            .collect();
        let made = self.with_given_facts(typing_env, given, |cx| {
            cx.collected_into((Expr::var("items"), None), output, Some(item), Some(param), span)
        });
        let made = made?;
        // What's left of what it's given closed where collecting it panics,
        // as std drops the iterator its collect holds as it unwinds (#1141):
        // the type parameter says nothing of what it holds, so it's closed,
        // `$collecting`'s.
        let made = Expr::call_helper(
            "$collecting",
            vec![
                Expr::var("items"),
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]),
            ],
        );
        Ok(Expr::arrow(
            params.into_iter().map(Into::into).collect(),
            vec![StmtKind::Return(Some(made)).at(js::Span::NONE)],
        ))
    }
}
