//! What the crate does with its `RefCell`s, found once over every body
//! before any is lowered (ADRs 0362, 0863): which are the object they hold;
//! a field's whose every borrow is held for a moment, the value itself in
//! the field; and each type none of whose borrows is held while code that
//! may borrow it runs, so that no borrow of it can meet another, and none
//! is checked. A library's contract says each to its consumers, which agree
//! or are refused.

use super::*;
use crate::lower::analysis::{Borrow, Cell};

/// What the crate's bodies do with its `RefCell`s, by type, each of its
/// regions erased: those a field holds as its value (ADR 0362), and those
/// some borrow of which is counted, held while code that may borrow it
/// runs, or that may be one (ADR 0863).
#[derive(Default)]
pub(in crate::lower) struct RefCells<'tcx> {
    plain: HashSet<Ty<'tcx>>,
    counted: HashSet<Ty<'tcx>>,
    /// Each the crate's bodies borrow.
    borrowed: HashSet<Ty<'tcx>>,
    /// Each its types' fields hold, and its exported functions take and give.
    handed: HashSet<Ty<'tcx>>,
    /// What each library it's compiled against does with its own (ADR 0863).
    libraries: Vec<(String, crate::library::RefCellFacts)>,
    /// Those of an object that are still `{ value }` (ADR 0863): every one,
    /// where generic code has a `RefCell<T>` of a type parameter's value,
    /// which may be any; and each made a `RefCell<dyn T>`, whose cell is
    /// its pair, and each that may be one of those.
    boxed_all: bool,
    boxed: HashSet<Ty<'tcx>>,
    /// Whether each type's own `Clone`, `PartialEq`, `PartialOrd`, `Ord`
    /// and `Debug`, which hold a borrow of it while what it holds runs its
    /// own, may borrow it, found when it's first asked.
    impls_ask: RefCell<HashMap<Ty<'tcx>, bool>>,
}

impl<'tcx> RefCells<'tcx> {
    /// Whether `ty` is one of the crate's `RefCell`s held as their values
    /// (ADR 0362), of its regions erased: a field's own type says `dyn
    /// FnMut() + 'static`, a body's `'{erased}`.
    pub(in crate::lower) fn is_plain(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.plain.contains(&tcx.erase_and_anonymize_regions(ty)) || self.library_has(tcx, ty, |f| &f.plain).is_some()
    }

    /// The library, if any, whose facts' `kind` names a type `ty` may be
    /// (ADR 0863).
    fn library_has(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
        kind: fn(&crate::library::RefCellFacts) -> &Vec<crate::library::RcShape>,
    ) -> Option<&str> {
        (self.libraries.iter())
            .find(|(_, facts)| {
                kind(facts)
                    .iter()
                    .any(|shape| crate::lower::library::may_be_cell_shape(tcx, shape, ty))
            })
            .map(|(name, _)| name.as_str())
    }

    /// What its contract says of its `RefCell`s (ADR 0863): those of an
    /// object it boxes, those it counts, those it borrows unchecked, those
    /// a field holds as its value, and those it shares with consumers.
    pub(in crate::lower) fn facts(
        &self,
        recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
        borrows: &crate::lower::analysis::CellBorrows<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> crate::library::RefCellFacts {
        let tcx = recognition.tcx;
        let shapes = |tys: &HashSet<Ty<'tcx>>| crate::lower::library::shapes(tcx, tys.iter().copied());
        let unchecked: HashSet<Ty<'tcx>> = (self.borrowed.iter())
            .filter(|&&ty| self.quiet(recognition, borrows, ty, drops))
            .copied()
            .collect();
        crate::library::RefCellFacts {
            boxed: match self.boxed_all {
                true => vec![crate::library::RcShape::default()],
                false => shapes(&self.boxed),
            },
            counted: shapes(&self.counted),
            unchecked: shapes(&unchecked),
            plain: shapes(&self.plain),
            handed: shapes(&self.handed),
        }
    }

    /// Whether a `RefCell` of type `ty` is the object it holds, not a
    /// `{ value }` box (ADR 0863): one of what `holds_itself` says is an
    /// object, but one that's still a box (`boxes`), or a field's that's
    /// the property it holds, whose field's borrows can't meet another
    /// (ADR 0362).
    pub(in crate::lower) fn is_value_cell(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, args) = ty.kind() else { return false };
        crate::lower::recognition::is_std_def(tcx, adt.did(), StdItem::RefCell)
            && holds_itself(tcx, args.type_at(0))
            && !self.boxes(tcx, ty)
            && !self.is_plain(tcx, ty)
    }

    /// Whether `ty`, a `RefCell` of an object, is still `{ value }` (ADR
    /// 0863): as every one is where generic code has one of a type
    /// parameter's value, and as one made a `RefCell<dyn T>` is, and one
    /// that may be one of those.
    pub(in crate::lower) fn boxes(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.boxes_itself(tcx, ty) || self.library_has(tcx, ty, |f| &f.boxed).is_some()
    }

    /// `boxes`, of what the crate itself has, not its libraries.
    fn boxes_itself(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        let ty = tcx.erase_and_anonymize_regions(ty);
        self.boxed_all || (self.boxed.iter()).any(|&boxed| crate::lower::analysis::may_be_cell(tcx, boxed, ty))
    }

    /// Whether `ty`, a `RefCell` type, is one no borrow of which can meet
    /// another (ADR 0863): none is counted, held while code that may borrow
    /// it runs, of it or of a type that may be its cell, a generic
    /// `RefCell<T>`'s of any, a `RefCell<dyn T>`'s of any it's made of
    /// (`may_be_cell`); and its own impls, which borrow it
    /// while what it holds runs its own, borrow nothing that may be it
    /// (`CellBorrows::impls_ask`), `drops` saying what drops. So none can
    /// find it borrowed, and none is checked. Nor one a library counts; and
    /// a library's consumers count none it doesn't while its code runs.
    pub(in crate::lower) fn quiet(
        &self,
        recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
        borrows: &crate::lower::analysis::CellBorrows<'tcx>,
        ty: Ty<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        let tcx = recognition.tcx;
        let ty = tcx.erase_and_anonymize_regions(ty);
        if (self.counted.iter()).any(|&counted| crate::lower::analysis::may_be_cell(tcx, counted, ty))
            || self.library_has(tcx, ty, |f| &f.counted).is_some()
        {
            return false;
        }
        if let Some(&asks) = self.impls_ask.borrow().get(&ty) {
            return !asks;
        }
        let recognition = crate::lower::recognition::Recognition {
            typing_env: ty::TypingEnv::fully_monomorphized(),
            ..*recognition
        };
        let asks = borrows.impls_ask(&recognition, ty, drops);
        self.impls_ask.borrow_mut().insert(ty, asks);
        !asks
    }
}

impl<'tcx> RefCells<'tcx> {
    /// Whether the crate agrees with each library it's compiled against on
    /// its `RefCell`s, an error where it can't (ADR 0863): one a library
    /// shares with it as the object it holds is the object here too, not
    /// a `{ value }` this crate boxes, of its generic code's or made a
    /// `dyn`; and no borrow of one a library borrows unchecked is held
    /// while code rust-js can't see runs, the library's own say, which
    /// doesn't check it.
    fn agrees(&self, tcx: TyCtxt<'tcx>, seen: &HashSet<Ty<'tcx>>, unseen: &[(Ty<'tcx>, rustc_span::Span)]) {
        for &ty in seen {
            let ty::Adt(_, args) = ty.kind() else { continue };
            if holds_itself(tcx, args.type_at(0))
                && self.boxes_itself(tcx, ty)
                && self.library_has(tcx, ty, |f| &f.boxed).is_none()
                && let Some(library) = self.library_has(tcx, ty, |f| &f.handed)
            {
                tcx.dcx().err(format!(
                    "rust-js: this crate has `{ty}` as a `{{ value }}`, of its generic code's `RefCell<T>` of any `T`, or made a `RefCell<dyn T>`, but `{library}`, a rust-js library, shares one with it as the object it holds (ADR 0863)"
                ));
            }
        }
        for &(ty, span) in unseen {
            if let Some(library) = self.library_has(tcx, ty, |f| &f.unchecked) {
                tcx.dcx().span_err(
                    span,
                    format!(
                        "rust-js: this borrow of a `{ty}` is held while code rust-js can't see runs, which may be `{library}`'s, a rust-js library that borrows one unchecked, as no borrow of it there can meet another (ADR 0863): end the borrow before that code runs"
                    ),
                );
            }
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a `RefCell` of type `ty` is the object it holds, not a
    /// `{ value }` box (ADR 0863): a `Vec`'s array, a map's `Map`, a set's
    /// `Set`, a struct's object (`holds_itself`), which its borrows read
    /// and its writes change in place, as a `&mut` to one does (ADR
    /// 0147), and which counts its borrows itself where they're counted.
    /// Not one generic code may have of any value, nor one made a
    /// `RefCell<dyn T>` (`RefCells::boxes`).
    pub(in crate::lower) fn is_value_cell(&self, ty: Ty<'tcx>) -> bool {
        self.ref_cells().is_value_cell(self.tcx, ty)
    }

    /// The `RefCell<T>` of a `held`, `T`.
    pub(in crate::lower) fn ref_cell_of(&self, held: Ty<'tcx>) -> Ty<'tcx> {
        let ref_cell = self
            .tcx
            .adt_def(crate::lower::recognition::std_item(self.tcx, StdItem::RefCell));
        Ty::new_adt(self.tcx, ref_cell, self.tcx.mk_args(&[held.into()]))
    }

    /// The type a method `id`, called of `args`, is of, its impl's: a
    /// `RefCell<T>`'s `new`'s, `RefCell<T>`.
    pub(in crate::lower) fn owner_of(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        match self.tcx.impl_of_assoc(id) {
            Some(imp) => self.tcx.type_of(imp).instantiate(self.tcx, args).skip_normalization(),
            None => self.tcx.types.unit,
        }
    }

    /// What the crate does with its `RefCell`s (`RefCells`), found the first
    /// time it's asked.
    pub(in crate::lower) fn ref_cells(&self) -> &'a RefCells<'tcx> {
        self.krate.ref_cells.get_or_init(|| self.find_ref_cells())
    }

    /// A `RefCell` type a field holds as its value (ADR 0362): each field of
    /// one in the crate is only ever borrowed for a moment, where nothing can
    /// ask, so no borrow of it is counted, its count stays 0, and its checks
    /// can't fail; a library's too, where its contract says so (ADR 0863).
    /// A field's cell lent, bound or moved, as a derive's code does, one of
    /// generic code, and one a library shares as a cell, is a cell.
    pub(in crate::lower) fn plain_ref_cell(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::RefCell) && self.ref_cells().is_plain(self.tcx, ty)
    }

    /// A `RefCell` type no borrow of which can meet another (ADR 0863): none
    /// is counted, held while code that may borrow it runs, of it or of a
    /// type that may be it, and its own impls, which borrow it while what
    /// it holds runs its own, borrow nothing that may be it. So none can
    /// find it borrowed, and none is checked.
    pub(in crate::lower) fn quiet_ref_cell(&self, ty: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        self.is_std_type(ty, StdItem::RefCell)
            && self.ref_cells().quiet(&recognition, self.krate.cell_borrows, ty, &|t| {
                super::drop_runs(
                    recognition,
                    (self.krate.library, self.krate.closure_facts),
                    self.krate.counted,
                    t,
                )
            })
    }

    fn find_ref_cells(&self) -> RefCells<'tcx> {
        let tcx = self.tcx;
        let mut plain = HashSet::new();
        let mut cells: HashMap<Ty<'tcx>, rustc_span::Span> = HashMap::new();
        let mut counted = HashSet::new();
        let mut borrowed = HashSet::new();
        let mut unseen: Vec<(Ty<'tcx>, rustc_span::Span)> = Vec::new();
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
            // Each body's own facts, asked where it is, of its own type
            // parameters, with no dictionaries: a type parameter's drop is
            // then a maybe, and its borrow counted.
            let owner = tcx.typeck_root_def_id(body.def_id);
            let recognition = crate::lower::recognition::Recognition {
                tcx,
                typing_env: ty::TypingEnv::post_analysis(tcx, owner),
                trait_impls: self.krate.trait_impls,
                foreign: self.krate.foreign,
            };
            let state = TypeDrops::default();
            let query = without_dictionaries(
                recognition,
                (false, self.krate.closure_facts),
                self.krate.counted,
                &state,
            );
            let thir = &body.thir;
            let facts = find_facts(&query, self.krate.cell_borrows, thir, body.def_id);
            for found in self.counted_in(&recognition, thir, &facts) {
                borrowed.insert(found.cell);
                if found.counted {
                    counted.insert(found.cell);
                    if !found.seen {
                        unseen.push((found.cell, found.span));
                    }
                }
            }
            // Of THIR's borrows, as MIR's are, whose guards of a field's
            // handle tidy reads as the field (ADR 0364).
            // `x.f` lent to a borrow held for a moment: `x.f.borrow_mut().push(..)`,
            // or to a write that holds it only while it swaps the value,
            // `x.f.replace(v)`, whose old value drops after; `x.f.take()`
            // runs `T::default()` too, which std's runs nothing of. And to
            // `get_mut`, a `&mut` to what it holds, borrowing nothing, the
            // cell lent exclusively (ADR 0863).
            let writes_for_a_moment = |fun: rustc_middle::thir::ExprId| {
                crate::lower::fn_def(thir[fun].ty).is_some_and(|(id, args)| match recognition.classify(id, args) {
                    Some(crate::lower::Std::CellReplace | crate::lower::Std::CellGetMut) => true,
                    Some(crate::lower::Std::CellTake) => {
                        args.types().next().is_some_and(|held| recognition.std_default(held))
                    }
                    _ => false,
                })
            };
            let mut for_a_moment = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && (facts.momentary.contains(&fun) || writes_for_a_moment(fun))
                    && let Some(&lent) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[crate::lower::body_queries::strip(thir, lent)].kind
                {
                    for_a_moment.insert(crate::lower::body_queries::strip(thir, arg));
                }
            }
            for (id, expr) in thir.exprs.iter_enumerated() {
                if let ExprKind::Field { lhs, .. } = expr.kind
                    && self.is_std_type(expr.ty, StdItem::RefCell)
                {
                    // A `#[repr(transparent)]` struct's is the struct itself
                    // (ADR 0584), no property to hold its value: a cell.
                    let transparent = matches!(thir[lhs].ty.kind(),
                        ty::Adt(adt, _) if crate::lower::recognition::transparent_field(tcx, *adt).is_some());
                    match for_a_moment.contains(&id) && !transparent {
                        true => _ = plain.insert(expr.ty),
                        false => _ = cells.entry(expr.ty).or_insert(expr.span),
                    };
                }
            }
            // One bound by a pattern is a value of its own, a cell.
            let mut bound = BoundRefCells {
                recognition: self.recognition(),
                thir,
                found: &mut cells,
            };
            for expr in thir.exprs.iter() {
                if let ExprKind::Let { ref pat, .. } = expr.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
            for stmt in thir.stmts.iter() {
                if let ThirStmt::Let { ref pattern, .. } = stmt.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pattern);
                }
            }
            for arm in thir.arms.iter() {
                rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, &arm.pattern);
            }
            for param in thir.params.iter() {
                if let Some(pat) = &param.pat {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
        }
        // Each of its regions erased, as `is_plain` asks.
        let erased = |set: HashSet<Ty<'tcx>>| -> HashSet<Ty<'tcx>> {
            set.into_iter().map(|ty| tcx.erase_and_anonymize_regions(ty)).collect()
        };
        let counted = erased(counted);
        let borrowed = erased(borrowed);
        let (boxed_all, boxed, seen) = self.boxed_ref_cells();
        let mut found = RefCells {
            counted,
            borrowed,
            handed: self.handed_ref_cells(),
            libraries: self.krate.foreign.ref_cell_facts(),
            boxed_all,
            boxed,
            ..RefCells::default()
        };
        found.agrees(tcx, &seen, &unseen);
        // A library's that a field of its holds as its value, which a field
        // of the crate's lent, bound, held or derived can't be (ADR 0863).
        for (&ty, &span) in &cells {
            if let Some(library) = found.library_has(tcx, ty, |f| &f.plain) {
                tcx.dcx().span_err(
                    span,
                    format!(
                        "rust-js: `{library}`, a rust-js library, has a field's `{ty}` as the value it holds, each only ever borrowed for a moment (ADR 0362); this field is lent, bound, held while code that may borrow it runs, or read by a derive: borrow it for a moment, `x.f.borrow_mut().push(..)`, or replace it, `x.f.replace(v)` (ADR 0863)"
                    ),
                );
            }
        }
        // A generic one may be any of them.
        let generic = cells.keys().any(|ty| ty.has_param());
        let cells = erased(cells.into_keys().collect());
        let mut plain = erased(plain);
        // A struct's field is the value it holds by its type and its
        // declaration, whether or not a body borrows one of its type (ADR
        // 0863, amended): of a type no body lends, binds or derives, which
        // `cells` takes away below.
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if self.is_std_type(ty, StdItem::RefCell) {
                        plain.insert(tcx.erase_and_anonymize_regions(ty));
                    }
                }
            }
        }
        // Not one a library shares with the crate as its cell, which a
        // field of the crate's can't hold as its value then (ADR 0863).
        plain.retain(|&ty| {
            !generic
                && !cells.contains(&ty)
                && !ty.has_param()
                && (found.library_has(tcx, ty, |f| &f.handed).is_none()
                    || found.library_has(tcx, ty, |f| &f.plain).is_some())
        });
        found.plain = plain;
        // A `#[rust_js::nullable]` field's `RefCell` is the value it holds,
        // whose `None` is `null`, where it's plain: one that's a cell of
        // its own holds `{ value }` (ADR 0863). Not one of a `Vec`'s, the
        // array, whose items are nullable (ADR 0586, amended).
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if crate::lower::bindings::is_nullable(tcx, field)
                        && self.is_std_type(ty, StdItem::RefCell)
                        && crate::lower::bindings::nullable_option(tcx, ty).is_some()
                        && !found.is_plain(tcx, ty)
                    {
                        tcx.dcx().span_err(
                            tcx.def_span(field.did),
                            "rust-js: `#[rust_js::nullable]` of a `RefCell` is of one a field holds as the value it holds, each of its fields only ever borrowed for a moment, `x.f.borrow().clone()` or `x.f.replace(v)` (ADR 0362); this one is lent, bound, held while code that may borrow it runs, or read by a derive, so it's a cell of its own (ADR 0863)",
                        );
                    }
                }
            }
        }
        found
    }

    /// The `RefCell` types the crate shares with its consumers (ADR 0863):
    /// each its types' fields hold, and each its exported functions take
    /// and give, each of its regions erased.
    fn handed_ref_cells(&self) -> HashSet<Ty<'tcx>> {
        let tcx = self.tcx;
        let mut tys: Vec<Ty<'tcx>> = Vec::new();
        for id in tcx.hir_crate_items(()).definitions() {
            match tcx.def_kind(id) {
                DefKind::Struct | DefKind::Enum | DefKind::Union => {
                    tys.extend(
                        (tcx.adt_def(id).all_fields())
                            .map(|field| tcx.type_of(field.did).instantiate_identity().skip_normalization()),
                    );
                }
                DefKind::Fn | DefKind::AssocFn if crate::lower::library::reachable(tcx, id.to_def_id()) => {
                    let sig = tcx.instantiate_bound_regions_with_erased(
                        tcx.fn_sig(id).instantiate_identity().skip_normalization(),
                    );
                    tys.extend(sig.inputs_and_output.iter());
                }
                _ => {}
            }
        }
        (tys.iter().flat_map(|ty| ty.walk()).filter_map(|part| part.as_type()))
            .filter(|&part| self.is_std_type(part, StdItem::RefCell))
            .map(|part| tcx.erase_and_anonymize_regions(part))
            .collect()
    }

    /// The `RefCell`s of an object that are still `{ value }` (ADR 0863),
    /// each of its regions erased: every one, where a body or a type of the
    /// crate's has one of what may be any value, a type parameter's, of
    /// which nothing says it's an object; else each made a
    /// `RefCell<dyn T>` or of a slice, whose cell is shared with what it's
    /// made of, and each that may be one of those, to a fixed point, as
    /// generic code's `RefCell<Node<T>>` stands for each `Node`'s.
    fn boxed_ref_cells(&self) -> (bool, HashSet<Ty<'tcx>>, HashSet<Ty<'tcx>>) {
        let tcx = self.tcx;
        let recognition = self.recognition();
        let refcell = |ty: Ty<'tcx>| recognition.is_std_type(ty, StdItem::RefCell);
        // Of a `RefCell`, or a guard of one, of what may be any value.
        let any_value = |ty: Ty<'tcx>| match ty.kind() {
            ty::Adt(_, args)
                if refcell(ty)
                    || [StdItem::RefCellRef, StdItem::RefCellRefMut]
                        .into_iter()
                        .any(|item| recognition.is_std_type(ty, item)) =>
            {
                args.types()
                    .next()
                    .is_some_and(|held| crate::lower::analysis::head(tcx, held).is_none())
            }
            _ => false,
        };
        let mut seen: HashSet<Ty<'tcx>> = HashSet::new();
        let mut boxed: HashSet<Ty<'tcx>> = HashSet::new();
        let mut all = false;
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
            let thir = &body.thir;
            let types = (thir.exprs.iter().map(|e| e.ty)).chain(thir.params.iter().map(|p| p.ty));
            for ty in types {
                for part in ty.walk().filter_map(|part| part.as_type()) {
                    all |= any_value(part);
                    if refcell(part) {
                        seen.insert(tcx.erase_and_anonymize_regions(part));
                    }
                }
            }
            for expr in thir.exprs.iter() {
                if let ExprKind::PointerCoercion {
                    cast: rustc_middle::ty::adjustment::PointerCoercion::Unsize,
                    source,
                    ..
                } = expr.kind
                {
                    let shares = expr
                        .ty
                        .walk()
                        .filter_map(|part| part.as_type())
                        .any(|part| match part.kind() {
                            ty::Adt(_, args) if refcell(part) => {
                                matches!(args.type_at(0).kind(), ty::Dynamic(..) | ty::Slice(_))
                            }
                            _ => false,
                        });
                    if shares {
                        let from = thir[source].ty.walk().filter_map(|part| part.as_type());
                        boxed.extend(
                            from.filter(|&part| refcell(part))
                                .map(|part| tcx.erase_and_anonymize_regions(part)),
                        );
                    }
                }
            }
        }
        // A type of the crate's that holds one of what may be any value,
        // which its derives and drops read as generic code does.
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    all |= ty.walk().filter_map(|part| part.as_type()).any(any_value);
                }
            }
        }
        // An impl of the crate's for one, whose dictionary is generic code.
        for impls in tcx.all_local_trait_impls(()).values() {
            for &imp in impls {
                let ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
                all |= ty.walk().filter_map(|part| part.as_type()).any(any_value);
            }
        }
        if all {
            return (true, HashSet::new(), seen);
        }
        loop {
            let more: Vec<Ty<'tcx>> = (seen.iter())
                .filter(|&&ty| !boxed.contains(&ty))
                .filter(|&&ty| (boxed.iter()).any(|&b| crate::lower::analysis::may_be_cell(tcx, b, ty)))
                .copied()
                .collect();
            if more.is_empty() {
                break;
            }
            boxed.extend(more);
        }
        (false, boxed, seen)
    }

    /// The `RefCell` types a body borrows counted (ADR 0863): a guard held
    /// while code that may borrow its cell runs, not one of `facts`'
    /// momentary ones (ADR 0328); a `try_borrow`'s, whose guard may be kept;
    /// a `swap`'s, which borrows two that may be one; a `replace_with`'s,
    /// held while its function runs, and a `take`'s, while the crate's
    /// `Default` does; a thread-local's `with_borrow`, held while what it's
    /// given runs, where that may borrow it (ADR 0580); and a borrow given
    /// as a function, `map(RefCell::borrow)`, whose guard its callee keeps.
    /// Each of its borrows, of a `RefCell` and whether it's counted, and
    /// where, and whether only code rust-js sees runs while it's held.
    fn counted_in(
        &self,
        recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
        thir: &rustc_middle::thir::Thir<'tcx>,
        facts: &Facts,
    ) -> Vec<Found<'tcx>> {
        use crate::lower::Std;
        let tcx = self.tcx;
        let refcell = |ty: Ty<'tcx>| recognition.is_std_type(ty, StdItem::RefCell);
        // What a borrow's cell is a `RefCell` of, a thread-local's by its key.
        let cell_of = |borrow: Borrow<'tcx>| match borrow.cell {
            Cell::Type(ty) => refcell(ty).then_some(ty),
            Cell::Key(key) => match tcx.type_of(key).instantiate_identity().skip_normalization().kind() {
                ty::Adt(_, args) => args.types().next().filter(|&cell| refcell(cell)),
                _ => None,
            },
        };
        let mut callees = HashSet::new();
        let mut found = Vec::new();
        for (id, expr) in thir.exprs.iter_enumerated() {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            callees.insert(fun);
            let Some((def_id, generic_args)) = crate::lower::fn_def(thir[fun].ty) else {
                continue;
            };
            let receiver = args.first().map(|&a| thir[a].ty.peel_refs());
            let kind = recognition.classify(def_id, generic_args);
            let a_borrow = match kind {
                Some(Std::Borrow { lock: false, .. } | Std::LocalBorrow { .. }) => true,
                Some(
                    Std::TryBorrow { lock: false, .. }
                    | Std::CellSwap
                    | Std::CellReplaceWith
                    | Std::CellTake
                    | Std::CellReplace,
                ) => receiver.is_some_and(refcell),
                _ => false,
            };
            let held = match kind {
                Some(Std::Borrow { lock: false, .. }) => {
                    !facts.momentary.contains(&fun) && !facts.let_held.contains(&fun)
                }
                Some(Std::LocalBorrow { .. }) => {
                    let quiet = crate::lower::analysis::borrowed(recognition, thir, id).is_some_and(|b| {
                        matches!(b.cell, Cell::Key(key) if key.as_local().is_some_and(|k| self.krate.quiet_locals.contains(&k)))
                    });
                    !quiet
                }
                Some(Std::TryBorrow { lock: false, .. } | Std::CellSwap | Std::CellReplaceWith) => {
                    receiver.is_some_and(refcell)
                }
                Some(Std::CellTake) => {
                    receiver.is_some_and(refcell)
                        && !generic_args
                            .types()
                            .next()
                            .is_some_and(|item| recognition.std_default(item))
                }
                _ => false,
            };
            if a_borrow && let Some(cell) = crate::lower::analysis::borrowed(recognition, thir, id).and_then(cell_of) {
                found.push(Found {
                    cell,
                    counted: held,
                    seen: matches!(kind, Some(Std::Borrow { .. })) && facts.held_while_seen.contains(&fun),
                    span: expr.span,
                });
            }
        }
        // A borrow given as a function: of the `RefCell` its type names.
        for (id, expr) in thir.exprs.iter_enumerated() {
            if !callees.contains(&id)
                && let Some((def_id, generic_args)) = crate::lower::fn_def(expr.ty)
                && let Some(Std::Borrow { lock: false, .. } | Std::TryBorrow { lock: false, .. }) =
                    recognition.classify(def_id, generic_args)
                && let Some(held) = generic_args.types().next()
            {
                found.push(Found {
                    cell: Ty::new_adt(
                        tcx,
                        tcx.adt_def(crate::lower::recognition::std_item(tcx, StdItem::RefCell)),
                        tcx.mk_args(&[held.into()]),
                    ),
                    counted: true,
                    seen: false,
                    span: expr.span,
                });
            }
        }
        found
    }
}

/// A borrow of a `RefCell` of type `cell`, at `span`: whether it's
/// counted, and whether only code rust-js sees runs while it's held.
struct Found<'tcx> {
    cell: Ty<'tcx>,
    counted: bool,
    seen: bool,
    span: rustc_span::Span,
}

/// Each `RefCell` a pattern binds of a field, not as `_` (ADR 0362).
struct BoundRefCells<'a, 'b, 'tcx> {
    recognition: crate::lower::recognition::Recognition<'a, 'tcx>,
    thir: &'b rustc_middle::thir::Thir<'tcx>,
    found: &'b mut HashMap<Ty<'tcx>, rustc_span::Span>,
}

impl<'b, 'tcx> rustc_middle::thir::visit::Visitor<'b, 'tcx> for BoundRefCells<'_, 'b, 'tcx> {
    fn thir(&self) -> &'b rustc_middle::thir::Thir<'tcx> {
        self.thir
    }

    fn visit_pat(&mut self, pat: &'b Pat<'tcx>) {
        if let rustc_middle::thir::PatKind::Variant { ref subpatterns, .. }
        | rustc_middle::thir::PatKind::Leaf { ref subpatterns } = pat.kind
        {
            for field in subpatterns {
                if !matches!(field.pattern.kind, rustc_middle::thir::PatKind::Wild)
                    && self.recognition.is_std_type(field.pattern.ty, StdItem::RefCell)
                {
                    self.found.entry(field.pattern.ty).or_insert(field.pattern.span);
                }
            }
        }
        rustc_middle::thir::visit::walk_pat(self, pat);
    }
}

/// Whether a `RefCell<T>`'s `T` is an object the cell is (ADR 0863), by
/// what `T` is alone, so that each of generic code's `RefCell<Node<T>>`
/// is one or none: a `Vec`'s or a `VecDeque`'s array, a map's `Map`, a
/// set's `Set`, a struct's object or a tuple struct's array, of fields.
/// Not a reference, which a write points at another; nor a class's
/// instance, a JS object's binding or one of markers alone. A
/// `#[repr(transparent)]` struct is its field's answer (ADR 0584). One that's `Copy` is copied where it's changed in
/// place, as any is (ADR 0623).
pub(in crate::lower) fn holds_itself(tcx: TyCtxt<'_>, held: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = held.kind() else { return false };
    let std = |item: StdItem| crate::lower::recognition::is_std_def(tcx, adt.did(), item);
    if crate::lower::recognition::is_vec_like_def(tcx, adt.did())
        || [StdItem::HashMap, StdItem::HashSet, StdItem::BTreeMap, StdItem::BTreeSet]
            .into_iter()
            .any(std)
    {
        return true;
    }
    if !adt.is_struct() || crate::lower::recognition::is_std_crate(tcx, adt.did()) {
        return false;
    }
    let fields = &adt.non_enum_variant().fields;
    let markers = fields.iter().all(|field| {
        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        matches!(ty.kind(), ty::Adt(of, _) if of.is_phantom_data())
    });
    if let Some(at) = crate::lower::recognition::transparent_field(tcx, *adt) {
        // A `#[repr(transparent)]` struct is its field (ADR 0584): an
        // object where that is, a `Vec`'s newtype its array; of its own
        // type parameters, so that a generic one is one or none too.
        let field = &fields[rustc_abi::FieldIdx::from_usize(at)];
        return holds_itself(tcx, tcx.type_of(field.did).instantiate_identity().skip_normalization());
    }
    !markers && !crate::lower::classes::is_class(tcx, adt.did())
}
