//! A `dyn`: its `{ value, impl }` pair, made by an unsizing, of a cell, an
//! error, or a struct whose last field is one.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `dyn` given for one of `id`'s type parameters that its signature,
    /// or its impl's type, holds in a cell, `RefCell<T>`, or a guard of one,
    /// a function's or an impl's: a cell of a `dyn` is its pair (ADR 0397),
    /// which generic code would read as a cell of a value (ADR 0451).
    pub(super) fn dyn_in_generic_cell(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let tcx = self.tcx;
        let held: Vec<Ty<'tcx>> = match tcx.def_kind(id) {
            DefKind::Impl { .. } => {
                let mut tys = vec![tcx.type_of(id).instantiate_identity().skip_normalization()];
                if let Some(tr) = tcx.impl_opt_trait_ref(id) {
                    tys.extend(tr.instantiate_identity().skip_normalization().args.types());
                }
                tys
            }
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx.fn_sig(id).instantiate_identity().skip_normalization().skip_binder();
                sig.inputs_and_output.iter().collect()
            }
            _ => return None,
        };
        let mut seen = Vec::new();
        held.into_iter().find_map(|ty| self.celled_dyn(ty, args, &mut seen))
    }

    /// `dyn_in_generic_cell`'s, of `ty` and what it's made of, the crate's
    /// structs' fields too, `seen` once each.
    fn celled_dyn(&self, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>, seen: &mut Vec<DefId>) -> Option<Ty<'tcx>> {
        let cells = [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock];
        ty.walk().filter_map(|part| part.as_type()).find_map(|part| {
            let ty::Adt(adt, part_args) = part.kind() else {
                return None;
            };
            let cell = cells.into_iter().any(|item| self.is_std_type(part, item)) || self.recognition().is_guard(part);
            if cell
                && let Some(&ty::Param(param)) = part_args.types().next().map(|t| t.kind())
                && let Some(given) = args.get(param.index as usize).and_then(|a| a.as_type())
                && self.cell_dyn_trait(given).is_some()
            {
                return Some(given);
            }
            if adt.did().is_local() && !seen.contains(&adt.did()) {
                seen.push(adt.did());
                let fields: Vec<Ty<'tcx>> = adt.all_fields().map(|f| self.field_ty(f, part_args)).collect();
                return fields.into_iter().find_map(|field| self.celled_dyn(field, args, seen));
            }
            None
        })
    }

    /// The trait of a `RefCell<dyn T>` that `ty` is or points at: the
    /// crate's own `T` (ADR 0397).
    pub(in crate::lower) fn dyn_cell_trait(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let cell = self.pointee(ty);
        if !self.is_std_type(cell, StdItem::RefCell) {
            return None;
        }
        let ty::Adt(_, args) = cell.kind() else { return None };
        self.cell_dyn_trait(args.type_at(0))
    }

    /// The trait of `held`, a `dyn` whose `RefCell` is its pair, `{ value,
    /// impl }`, its guard too: the crate's own (ADR 0397). A `dyn FnMut`'s
    /// holds the function.
    pub(in crate::lower) fn cell_dyn_trait(&self, held: Ty<'tcx>) -> Option<DefId> {
        match held.kind() {
            ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&id| self.is_rust_trait(id)),
            _ => None,
        }
    }

    /// Another of the crate's traits that `held`, in a cell made a `dyn` of
    /// `trait_id`, has, with a method named as one of `trait_id`'s: neither
    /// the other's supertrait. A cell made a `dyn` of two has one `impl`,
    /// both's methods (ADR 0397), which two of one name can't be.
    fn dyn_cell_clash(&self, held: Ty<'tcx>, trait_id: DefId) -> Option<DefId> {
        let methods = |id: DefId| -> HashSet<Symbol> {
            supertrait_def_ids(self.tcx, id)
                .flat_map(|of| self.tcx.associated_items(of).in_definition_order())
                .filter(|item| item.is_fn())
                .map(|item| item.name())
                .collect()
        };
        let under = |sub: DefId, of: DefId| supertrait_def_ids(self.tcx, sub).any(|id| id == of);
        let named = methods(trait_id);
        let bounds = self.declared_bounds(held, self.typing_env.param_env);
        self.tcx.all_traits_including_private().find(|&other| {
            self.is_rust_trait(other)
                && !under(trait_id, other)
                && !under(other, trait_id)
                && (bounds.contains(&other) || self.has_user_impl(other, held))
                && !methods(other).is_disjoint(&named)
        })
    }

    /// The trait a `dyn` of `ty` is a pair of, `{ value, impl }` (ADR 0049):
    /// one rust-js compiled, or std's `Display` or `Error`, whose
    /// dictionaries it makes (ADR 0141). A `dyn Debug` is a string instead.
    /// Is `ty`, through its pointer, a `dyn` that's a pair, its value and
    /// its dictionary: of a pair trait, or of none, `dyn Send`.
    pub(in crate::lower) fn is_pair_dyn(&self, ty: Ty<'tcx>) -> bool {
        matches!(self.pointee(ty).kind(), ty::Dynamic(predicates, ..) if self.recognition().is_pair_dyn(predicates))
    }

    pub(in crate::lower) fn dynamic_trait(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let inner = self.pointee(ty);
        match inner.kind() {
            ty::Dynamic(predicates, ..) => predicates
                .principal_def_id()
                .filter(|&id| self.recognition().is_pair_trait(id)),
            _ => None,
        }
    }

    /// `{}` or `{:?}` of `pair`, a `dyn` of `ty`, as `writer`, `Display` or
    /// `Debug`, says: through its dictionary, `d.impl.fmt(d.value)`, or a
    /// supertrait's, `e.impl.Display().fmt(e.value)` (ADR 0141), given
    /// `options` (ADR 0058). None if `ty` isn't such a `dyn`.
    pub(in crate::lower) fn dyn_written(
        &self,
        pair: Expr,
        (ty, writer): (Ty<'tcx>, DefId),
        options: Option<Expr>,
        span: Span,
    ) -> R<Option<Expr>> {
        if !matches!(ty.kind(), ty::Dynamic(..)) || self.dynamic_trait(ty).is_none() {
            return Ok(None);
        }
        let from = self.dyn_trait_ref(ty, ty).expect("a principal");
        let to = ty::TraitRef::new(self.tcx, writer, [ty]);
        let Some(dictionary) = self.super_evidence(from, to, Expr::member(pair.clone(), js::DICTIONARY)) else {
            return Ok(None);
        };
        // Read twice: once for its dictionary, once for its value.
        if !pair.parts_read_same() {
            return Err(self.unsupported(span, &format!("showing a `{ty}` made where it's shown")));
        }
        let mut values = vec![Expr::held(pair)];
        values.extend(options);
        Ok(Some(Expr::call(Expr::member(dictionary, "fmt"), values)))
    }

    /// What `From::from` or `Into::into` makes, and from what.
    pub(in crate::lower) fn converted(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let tr = self.tcx.trait_of_assoc(def_id)?;
        let mut types = args.types();
        let (first, second) = (types.next()?, types.next()?);
        if is_std_def(self.tcx, tr, StdItem::From) {
            Some((first, second))
        } else if is_std_def(self.tcx, tr, StdItem::Into) {
            Some((second, first))
        } else {
            None
        }
    }

    /// The dictionary of a `Box<dyn Error>`, `to`, made from a `from` by std's
    /// `From` (ADR 0141): the error's own `Error`, or a message's,
    /// `$stringError()`. None if `to` isn't one.
    pub(in crate::lower) fn dyn_error_from(&mut self, to: Ty<'tcx>, from: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let is_error = |id: DefId| is_std_def(self.tcx, id, StdItem::Error);
        let Some(inner) = to.boxed_ty() else {
            return Ok(None);
        };
        if !matches!(inner.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some_and(is_error)) {
            return Ok(None);
        }
        self.error_dictionary(from, span).map(Some)
    }

    /// The `Error` dictionary of a `from` made a `dyn Error` (ADR 0141): the
    /// error's own `Error`, or a message's, `$stringError()`, a std error's
    /// its own.
    pub(in crate::lower) fn error_dictionary(&mut self, from: Ty<'tcx>, span: Span) -> R<Expr> {
        if self.is_string_like(from) {
            return Ok(Expr::call(Expr::helper("$stringError"), Vec::new()));
        }
        if let Some(dictionary) = self.std_error_dictionary(from) {
            return Ok(dictionary);
        }
        let error = std_item(self.tcx, StdItem::Error);
        let tr = ty::TraitRef::new(self.tcx, error, [from]);
        self.dictionary(tr, span)
    }

    /// The `Error` dictionary of one of std's errors rust-js holds as its own
    /// value, shown as std shows it: a parse error's, its message (ADR
    /// 0063), serde_json's, and an `io::Error`'s (ADR 0462).
    pub(super) fn std_error_dictionary(&self, ty: Ty<'tcx>) -> Option<Expr> {
        if self.is_parse_error(ty)
            && let ty::Adt(adt, _) = ty.kind()
        {
            let name = self.tcx.item_name(adt.did());
            return Some(Expr::call(
                Expr::helper("$parseErrorDyn"),
                vec![Expr::str(name.as_str())],
            ));
        }
        if self.is_json_error(ty) {
            return Some(Expr::call(Expr::helper("$jsonErrorDyn"), Vec::new()));
        }
        if self.recognition().is_io_error(ty) {
            return Some(Expr::call(Expr::helper("$ioErrorDyn"), Vec::new()));
        }
        None
    }

    /// `Error::source`, which a `dyn Error`'s dictionary has (ADR 0141).
    pub(super) fn is_error_source(&self, id: DefId) -> bool {
        is_std_method(self.tcx, id, StdItem::Error, "source")
    }

    pub(super) fn dyn_trait_ref(&self, ty: Ty<'tcx>, self_ty: Ty<'tcx>) -> Option<ty::TraitRef<'tcx>> {
        match self.pointee(ty).kind() {
            // `dyn for<'a> AsStr<'a, 'a>`: its lifetimes erased, as a
            // dictionary is the same for any.
            ty::Dynamic(predicates, ..) => predicates.principal().map(|p| {
                self.tcx
                    .instantiate_bound_regions_with_erased(p.with_self_ty(self.tcx, self_ty))
            }),
            _ => None,
        }
    }

    pub(in crate::lower) fn pointee(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        match ty.kind() {
            ty::Ref(_, inner, _) => *inner,
            _ if let Some((inner, _)) = crate::lower::pointers::raw_pointee(self.tcx, ty) => inner,
            ty::Adt(_, args) if self.is_std_wrapper(ty) => args.type_at(0),
            _ => ty,
        }
    }

    /// `holder`, what holds a `dyn`'s value as its last field, a `RefCell`
    /// or a struct, given `dictionary`, the `impl` of `held`'s `trait_id`,
    /// in place (ADRs 0397, 0409): one made here, `{ value: .. }`, made with
    /// it; another, which may be a `dyn` of another trait already, given
    /// both's methods, unless they share a name.
    fn given_impl(
        &self,
        holder: Expr,
        (held, trait_id): (Ty<'tcx>, Option<DefId>),
        dictionary: Expr,
        target: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        if let js::ExprKind::Object(props, ..) = &holder.kind
            && matches!(props.as_slice(), [Prop::Field(name, _)] if name == js::HELD)
        {
            let mut props = props.clone();
            props.push(Prop::Field(js::DICTIONARY.into(), dictionary));
            return Ok(Expr::object(props));
        }
        if let Some(other) = trait_id.and_then(|trait_id| self.dyn_cell_clash(held, trait_id)) {
            return Err(self.unsupported(
                span,
                &format!(
                    "a `{target}` of a value that may be a `dyn {}` too: its one `impl` can't hold two methods of one name",
                    self.tcx.def_path_str(other)
                ),
            ));
        }
        Ok(Expr::call(Expr::helper("$dynCell"), vec![holder, dictionary]))
    }

    /// Of `source` unsized as `target`, a reference to, or a `Box` of, a
    /// struct of the crate's whose last field, or its last field's, is a
    /// `dyn` that's a pair: the type that field is in `source`, and the `dyn`
    /// (ADRs 0409, 0436). Not a `dyn FnMut`, the JS function: the same struct
    /// either way.
    pub(in crate::lower) fn dyn_tail(&self, source: Ty<'tcx>, target: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let (from, to) = (self.pointee(source), self.pointee(target));
        if !self.holds_a_dyn_tail(target)
            || !matches!(to.kind(), ty::Adt(adt, _) if adt.is_struct() && self.is_rust_adt(adt.did()))
        {
            return None;
        }
        let (concrete, tail) = self.tcx.struct_lockstep_tails_for_codegen(from, to, self.typing_env);
        matches!(tail.kind(), ty::Dynamic(traits, ..) if self.recognition().is_pair_dyn(traits))
            .then_some((concrete, tail))
    }

    /// The fields from a `ty` to the struct its last `dyn` field is in, each
    /// a struct and its index: none where it's `ty`'s own, or its last
    /// field's, `fat.inner`.
    pub(in crate::lower) fn tail_path(&self, mut ty: Ty<'tcx>) -> Vec<(Ty<'tcx>, usize)> {
        let mut path = Vec::new();
        loop {
            let ty::Adt(adt, args) = ty.kind() else { return path };
            let Some(last) = adt.non_enum_variant().fields.iter().last() else {
                return path;
            };
            // A `ManuallyDrop` is the value it holds, a `dyn` the struct holds.
            let field = self.unwrapped(self.field_ty(last, args));
            if matches!(field.kind(), ty::Dynamic(..)) || !matches!(field.kind(), ty::Adt(..)) {
                return path;
            }
            path.push((ty, adt.non_enum_variant().fields.len() - 1));
            ty = field;
        }
    }

    /// Whether a `target` pointer to a struct whose last field is a `dyn`
    /// can hold it (ADR 0409): a reference, a `Box` or an `Rc`, each the
    /// struct, or a counted `Rc` the cell that holds it (ADR 0320).
    fn holds_a_dyn_tail(&self, target: Ty<'tcx>) -> bool {
        target.is_ref() || target.is_box() || self.is_rc(target)
    }

    /// An unsizing JS can't make: of a pointer of the crate's own, or to a
    /// struct whose last field is a `dyn`.
    pub(in crate::lower) fn check_unsize(&self, target: Ty<'tcx>, span: Span) -> R<()> {
        // A pointer of the crate's own, as `#[derive(CoercePointee)]` makes
        // one, would hold a `dyn`'s value and impl where it holds the value.
        if let ty::Adt(adt, _) = target.kind()
            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))
        {
            return Err(self.unsupported(span, &format!("unsizing a `{target}`")));
        }
        // A `RefCell<dyn T>` is its cell with its `impl`, a pair (ADR 0397).
        if self.dyn_cell_trait(target).is_some() {
            return Ok(());
        }
        // `&Fat<Bar>` to `&Fat<dyn ToBar>`: the struct given its last field's
        // `impl` (ADR 0409), behind a reference or a `Box`, which drops its
        // last field through it (ADR 0436); an `Rc`'s, which may be counted,
        // not yet. A `dyn Debug`'s would be the string it shows, which the
        // field isn't. One ending in a slice, or a `dyn FnMut`, the JS
        // function, is the same value either way.
        // A counted `Rc` of a `dyn Debug`: its value would be the string
        // shown, which its count isn't (ADR 0320).
        if self.counted_rc(target).is_some() && self.is_dyn_debug(self.pointee(target)) {
            return Err(self.unsupported(span, &format!("a counted `{target}`")));
        }
        let pointee = self.pointee(target);
        let tail = self.tcx.struct_tail_for_codegen(pointee, self.typing_env);
        if pointee.is_adt()
            && matches!(tail.kind(), ty::Dynamic(traits, ..)
                if self.recognition().is_pair_dyn(traits) || self.is_dyn_debug(tail))
            && (!self.holds_a_dyn_tail(target) || self.is_dyn_debug(tail))
        {
            return Err(self.unsupported(span, &format!("a `{pointee}`, whose last field is a `dyn`")));
        }
        Ok(())
    }

    /// `x as &dyn Trait`: `{ value, impl }`, or for a trait object, the same
    /// value with its supertrait's dictionary. `out` gets a value computed once.
    pub(in crate::lower) fn unsize_trait(
        &mut self,
        source: Ty<'tcx>,
        target: Ty<'tcx>,
        value: Expr,
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Expr> {
        self.check_unsize(target, span)?;
        // A `&dyn Debug` is the string it shows (ADR 0060), and serde's `&dyn
        // Expected` its text (ADR 0459).
        if self.is_dyn_debug(target) && !self.is_dyn_debug(source) {
            return self.dyn_debug_string(value, self.pointee(source), span);
        }
        if crate::lower::recognition::is_dyn_expected(self.tcx, target)
            && !crate::lower::recognition::is_dyn_expected(self.tcx, source)
        {
            return self.expected_text(value, self.pointee(source), span);
        }
        // A `()` as a `dyn` is `undefined`, as it is, or a list of none,
        // `[]`, of a list's: `use_effect(f, Some(&()))`'s (ADR 0775).
        let value = match self.pointee(source).is_unit() {
            true => crate::lower::representation::unit_given(
                self.tcx,
                crate::lower::representation::UnitTo::Dyn(self.pointee(target)),
            )
            .unwrap_or(value),
            false => value,
        };
        // A `RefCell<C>` as a `RefCell<dyn T>`: the same cell, given its
        // `impl`, in place, so an `Rc` of it before is the same (ADR 0397).
        if self.dyn_cell_trait(target).is_some()
            && let (ty::Adt(_, from), ty::Adt(_, to)) = (self.pointee(source).kind(), self.pointee(target).kind())
            && let Some(tr) = self.dyn_trait_ref(to.type_at(0), from.type_at(0))
        {
            // A `dyn` of its supertrait, whose dictionary would be the cell's
            // `impl`'s: not yet.
            if matches!(from.type_at(0).kind(), ty::Dynamic(..)) {
                return Err(self.unsupported(span, &format!("a `{source}` as a `{target}`")));
            }
            let dictionary = self.dictionary(tr, span)?;
            // A `Weak`'s of one, whose `dyn` is a pair around it: not yet.
            if self.counted_weak(target).is_some() {
                return Err(self.unsupported(span, &format!("a counted `{target}`")));
            }
            // A counted `Rc`'s (ADR 0320): its cell given the `impl`, as
            // every owner shares it, and the same `Rc`, its counts theirs.
            if self.counted_rc(target).is_some() {
                let rc = match value.reads_same() {
                    true => value,
                    false => self.spill("rc", value, out),
                };
                let cell = Expr::member(rc.clone(), js::HELD);
                let given = self.given_impl(cell, (from.type_at(0), Some(tr.def_id)), dictionary, target, span)?;
                out.push(StmtKind::Expr(given).at(js::Span::NONE));
                return Ok(rc);
            }
            return self.given_impl(value, (from.type_at(0), Some(tr.def_id)), dictionary, target, span);
        }
        // `&Fat<Bar>` as a `&Fat<dyn ToBar>`: the same struct, the one its
        // last field is in given `Bar`'s `ToBar` as its `impl`, as a
        // `RefCell`'s is (ADR 0409).
        if let Some((concrete, tail)) = self.dyn_tail(source, target) {
            // Of a `dyn` already, the same struct; as another trait's, its
            // supertrait's, not yet.
            if matches!(concrete.kind(), ty::Dynamic(..)) {
                let erased = |ty| self.tcx.erase_and_anonymize_regions(ty);
                if erased(concrete) == erased(tail) {
                    return Ok(value);
                }
                return Err(self.unsupported(span, &format!("a `{source}` as a `{target}`")));
            }
            let trait_id = self.dyn_trait_ref(tail, concrete).map(|tr| tr.def_id);
            let dictionary = self.dyn_dictionary(tail, concrete, span)?;
            // The struct itself holds it: given its `impl` as it's made.
            let path = self.tail_path(self.pointee(target));
            let counted = self.counted_rc(target).is_some();
            if path.is_empty() && !counted {
                return self.given_impl(value, (concrete, trait_id), dictionary, target, span);
            }
            let value = match value.reads_same() {
                true => value,
                false => self.spill(if counted { "rc" } else { "value" }, value, out),
            };
            // A counted `Rc`'s struct, in its cell.
            let root = match counted {
                true => Expr::member(value.clone(), js::HELD),
                false => value.clone(),
            };
            let parent = (path.into_iter()).fold(root, |parent, (ty, i)| self.project(parent, ty, i));
            let given = self.given_impl(parent, (concrete, trait_id), dictionary, target, span)?;
            out.push(StmtKind::Expr(given).at(js::Span::NONE));
            return Ok(value);
        }
        // A function made a `dyn Fn`, holding a value with a destructor: the
        // function carrying its drop, `$dropping(f, drop)`, which the `dyn`'s
        // drop runs, `$dispose(f)` (ADR 0521). One that carries it already,
        // itself.
        if let ty::Dynamic(predicates, ..) = self.pointee(target).kind()
            && crate::lower::recognition::is_fn_dyn(self.tcx, predicates)
        {
            let function = self.pointee(source);
            let carried = matches!(function.kind(), ty::Closure(id, _) if self.krate.closure_facts.carries_its_drop(self.tcx, *id))
                || matches!(function.kind(), ty::Dynamic(..));
            if carried || self.drops(function) == crate::lower::drops::Drops::Nothing {
                return Ok(value);
            }
            let f = self.spill("f", value, out);
            let mut dropped = Vec::new();
            self.drop_value(f.clone(), function, span, &mut dropped)?;
            return Ok(Expr::call_helper(
                "$dropping",
                vec![f, Expr::arrow(Vec::new(), dropped)],
            ));
        }
        if !self.is_pair_dyn(target) {
            return Ok(value);
        }
        self.check_value_ty(target, span)?;
        // A counted `Rc` as an `Rc<dyn T>` is its pair, `{ value, impl,
        // strong, weak }` (ADRs 0320, 0520): of one made here; of one made
        // elsewhere, which its other owners share as what it is, not yet.
        if self.counted_rc(source).is_some() || self.counted_weak(source).is_some() {
            return self.counted_unsized(source, target, value, span, out);
        }
        if self.is_pair_dyn(source) {
            let self_ty = self.pointee(source);
            let from = self.dyn_trait_ref(source, self_ty);
            // The same trait (a `Box<dyn T>` to a `Box<dyn T>`), or none, a
            // `dyn T + Send` as a `dyn Send`, whose dictionary is its drop,
            // which the first's has: the same pair.
            let Some(to) = self.dyn_trait_ref(target, self_ty) else {
                return Ok(value);
            };
            let from = from.expect("a `dyn` of no trait made one of a trait");
            if self.tcx.erase_and_anonymize_regions(from) == self.tcx.erase_and_anonymize_regions(to) {
                return Ok(value);
            }
            // Read twice, so one with effects goes in a `const` first.
            let pair = if value.has_effects() {
                self.spill("receiver", value, out)
            } else {
                value
            };
            let dictionary = self
                .super_evidence(from, to, Expr::member(pair.clone(), js::DICTIONARY))
                .ok_or_else(|| self.unsupported(span, "this trait upcast"))?;
            // A `&mut dyn Sub` as a `&mut dyn Super`: a pair on the first's
            // `value`, which its `&mut self` methods write (ADR 0099).
            if matches!(target.kind(), ty::Ref(_, _, Mutability::Mut)) {
                return Ok(Expr::pair(Expr::held(pair), dictionary));
            }
            return Ok(Expr::object(vec![
                Prop::Field(js::HELD.into(), Expr::held(pair)),
                Prop::Field(js::DICTIONARY.into(), dictionary),
            ]));
        }
        let dictionary = self.dyn_dictionary(target, self.pointee(source), span)?;
        // `&mut n as &mut dyn Trait` of a number: the pair reads and writes
        // the place the cell does (ADR 0099). A temporary's box is the pair's.
        if self.is_cell(source) {
            return Ok(match value.kind {
                js::ExprKind::Handle(place, _) => Expr::pair(*place, dictionary),
                js::ExprKind::Object(mut props, ..) if matches!(props.as_slice(), [Prop::Field(name, _)] if name == js::HELD) =>
                {
                    props.push(Prop::Field(js::DICTIONARY.into(), dictionary));
                    Expr::object(props)
                }
                _ => {
                    let cell = if value.reads_same() {
                        value
                    } else {
                        self.spill("cell", value, out)
                    };
                    Expr::pair(Expr::held(cell), dictionary)
                }
            });
        }
        Ok(Expr::object(vec![
            Prop::Field(js::HELD.into(), value),
            Prop::Field(js::DICTIONARY.into(), dictionary),
        ]))
    }

    /// The dictionary a `self_ty` made a `target`, a `dyn`, is given: its
    /// trait's, or of a `dyn` of no trait, `dyn Send`, its drop alone,
    /// `{ $drop }`, or nothing, `{}`.
    fn dyn_dictionary(&mut self, target: Ty<'tcx>, self_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.dyn_trait_ref(target, self_ty) {
            Some(tr) => self.dictionary(tr, span),
            None => Ok(Expr::object(self.dictionary_drop(self_ty, span)?.into_iter().collect())),
        }
    }

    /// A counted `Rc` made here, `{ value: x, strong: 1, weak: 0 }`, as an
    /// `Rc<dyn T>`: the pair itself, `{ value: x, impl, strong: 1, weak: 0
    /// }`, as `counted_pointee` reads it (ADRs 0320, 0520); one of the same
    /// `dyn` already, itself.
    /// One of a `dyn Sub` as an `Rc<dyn Super>`: `$rcUpcast` of it, its
    /// pair of the supertrait's dictionary whose value and counts are its
    /// own, shared (#186). Another made elsewhere, of another `dyn`, or a
    /// `Weak`, is refused: its other owners share it as the type it is.
    fn counted_unsized(
        &mut self,
        source: Ty<'tcx>,
        target: Ty<'tcx>,
        value: Expr,
        span: Span,
        out: &mut Vec<js::Stmt>,
    ) -> R<Expr> {
        // Of the same `dyn` already, a slice's unsized: itself.
        if self.tcx.erase_and_anonymize_regions(source) == self.tcx.erase_and_anonymize_regions(target) {
            return Ok(value);
        }
        if let Some(pointee) = self.counted_rc(source)
            && self.is_pair_dyn(pointee)
            && let (Some(from), Some(to)) = (self.dyn_trait_ref(source, pointee), self.dyn_trait_ref(target, pointee))
        {
            // Of its own trait, an auto trait left out, `dyn Sub + Send` as
            // `dyn Sub`: itself.
            if from == to {
                return Ok(value);
            }
            // Read twice, its dictionary and itself.
            let rc = if value.reads_same() {
                value
            } else {
                self.spill("rc", value, out)
            };
            let dictionary = self
                .super_evidence(from, to, Expr::member(rc.clone(), js::DICTIONARY))
                .ok_or_else(|| self.unsupported(span, "this trait upcast"))?;
            let key = Expr::str(self.tcx.def_path_str(to.def_id));
            return Ok(Expr::call_helper("$rcUpcast", vec![rc, key, dictionary]));
        }
        let what = format!("a counted `{source}` made elsewhere as a `{target}`");
        let made_here = match (self.counted_rc(source), value.kind) {
            (Some(pointee), js::ExprKind::Object(props, ..)) => Some((pointee, props)),
            _ => None,
        };
        let Some((pointee, mut props)) = made_here else {
            return Err(self.unsupported(span, &what));
        };
        let dictionary = self.dyn_dictionary(target, pointee, span)?;
        let Some(at) = props
            .iter()
            .position(|p| matches!(p, Prop::Field(name, _) if name == js::HELD))
        else {
            return Err(self.unsupported(span, &what));
        };
        props.insert(at + 1, Prop::Field(js::DICTIONARY.into(), dictionary));
        Ok(Expr::object(props))
    }

    /// Can a `dyn` of the trait `id` be a pair (ADR 0049): its items are
    /// methods, and neither it nor a supertrait has type parameters. A `&mut
    /// self` method is given the pair, whose `value` a box's is (ADR 0099).
    pub(in crate::lower) fn dyn_supported(&self, id: DefId) -> bool {
        self.tcx
            .associated_items(id)
            .in_definition_order()
            .all(|item| match self.tcx.def_kind(item.def_id) {
                DefKind::AssocFn => true,
                // A `dyn Source<Item = u32>` says what it is (ADR 0106).
                DefKind::AssocTy => self.tcx.generics_of(item.def_id).own_params.is_empty(),
                _ => false,
            })
            && self
                .tcx
                .explicit_super_clauses_of(id)
                .iter_identity_copied()
                .map(|item| item.skip_normalization())
                .all(|(clause, _)| match clause.kind().skip_binder() {
                    ty::ClauseKind::Trait(p) if operational(self.tcx, self.krate.foreign, p.trait_ref.def_id) => {
                        self.dyn_supported(p.trait_ref.def_id)
                    }
                    _ => true,
                })
    }
}
