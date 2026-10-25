//! What the crate does with its `RefCell`s, found once over every body
//! before any is lowered (ADRs 0362, 0863): which are the object they hold;
//! a struct's field's, the value it holds by its declaration, and of those
//! each no borrow of which can meet another, its own uses all borrows of
//! it for a moment; and each type none of whose borrows is held while code
//! that may borrow it runs, so that no borrow of it can meet another, and
//! none is checked. A library's contract says each to its consumers, which
//! agree or are refused.

use super::*;
use crate::lower::analysis::{Borrow, Cell};

/// What the crate's bodies do with its `RefCell`s: by type, each of its
/// regions erased, those some borrow of which is counted, held while code
/// that may borrow it runs, or that may be one (ADR 0863); and by field,
/// each of a struct's no borrow of which can meet another (ADR 0863,
/// amended), a field being a place of its own whatever its type.
#[derive(Default)]
pub(in crate::lower) struct RefCells<'tcx> {
    /// Each `RefCell` field of the crate's structs that's never lent, bound
    /// or moved, and whose every borrow is held only where nothing that may
    /// borrow it runs: none of its borrows can meet another.
    quiet_fields: HashSet<DefId>,
    /// Each a library's contract says is quiet, by its key, and the
    /// library that says so (ADR 0906).
    library_quiet: HashMap<String, String>,
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
    /// Whether `field` is a struct's `RefCell` field that holds its cell's
    /// value as the property, by its declaration alone, whatever its uses
    /// (ADR 0863, amended): of a struct not `#[repr(transparent)]`, which
    /// is its field, the cell (ADR 0584); and of what isn't an object the
    /// cell is (`is_value_cell` of the type it's declared, so each of a
    /// generic struct's is one or none), whose field holds that object.
    /// The same in a library and its consumers, as each reads it of its
    /// declaration.
    pub(in crate::lower) fn holds_value(&self, tcx: TyCtxt<'tcx>, field: &ty::FieldDef) -> bool {
        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_std_def(tcx, adt.did(), StdItem::RefCell))
            && struct_field(tcx, field.did)
            && !self.is_value_cell(tcx, ty)
    }

    /// Whether `field`, a struct's `RefCell` field, is one no borrow of
    /// which can meet another (ADR 0863, amended): never lent, bound or
    /// moved, each of its borrows held only where nothing that may borrow
    /// it runs, the crate's or, of a library's, as its contract says.
    pub(in crate::lower) fn quiet_field(&self, tcx: TyCtxt<'tcx>, field: DefId) -> bool {
        match field.as_local() {
            Some(_) => self.quiet_fields.contains(&field),
            None => (self.library_quiet).contains_key(&crate::lower::library::item_key(tcx, field)),
        }
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
    /// object it boxes, those it counts, those it borrows unchecked, its
    /// fields no borrow of which can meet another, and those it shares
    /// with consumers.
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
            handed: shapes(&self.handed),
        }
    }

    /// What its contract says of its structs' fields (ADRs 0863, amended,
    /// 0906): each quiet one.
    pub(in crate::lower) fn fields(&self, tcx: TyCtxt<'tcx>) -> Vec<crate::library::Field> {
        crate::lower::library::fields(
            tcx,
            self.quiet_fields.iter().copied(),
            &[crate::library::FieldFact::Quiet],
        )
    }

    /// Whether a `RefCell` of type `ty` is the object it holds, not a
    /// `{ value }` box (ADR 0863): one of what `holds_itself` says is an
    /// object, but one that's still a box (`boxes`). By its type alone,
    /// wherever it is: a struct's field that holds one holds the object.
    pub(in crate::lower) fn is_value_cell(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, args) = ty.kind() else { return false };
        crate::lower::recognition::is_std_def(tcx, adt.did(), StdItem::RefCell)
            && holds_itself(tcx, args.type_at(0))
            && !self.boxes(tcx, ty)
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

    /// Whether `field`, a struct's, holds its `RefCell`'s value as the
    /// property, by its declaration (`RefCells::holds_value`, ADR 0863,
    /// amended).
    pub(in crate::lower) fn holds_value(&self, field: &ty::FieldDef) -> bool {
        self.ref_cells().holds_value(self.tcx, field)
    }

    /// Whether `field`, a struct's `RefCell` field, is one no borrow of
    /// which can meet another (`RefCells::quiet_field`), so none is checked
    /// (ADR 0863, amended).
    pub(in crate::lower) fn quiet_field(&self, field: DefId) -> bool {
        self.ref_cells().quiet_field(self.tcx, field)
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
        // Each struct's `RefCell` field a body lends, binds or moves, where
        // first; and each a borrow of which is counted (ADR 0863, amended).
        let mut lent: HashMap<DefId, rustc_span::Span> = HashMap::new();
        let mut counted_fields: HashMap<DefId, rustc_span::Span> = HashMap::new();
        // Each a derive rust-js writes itself reads, `Clone`'s, `Debug`'s,
        // which borrows it while what it holds runs its own.
        let mut derived: HashMap<DefId, rustc_span::Span> = HashMap::new();
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
                    if let Some(field) = found.field {
                        counted_fields.entry(field).or_insert(found.span);
                    }
                    if !found.seen {
                        unseen.push((found.cell, found.span));
                    }
                }
            }
            // A struct's field each of whose uses is a borrow of it, or a
            // write that holds it only while it swaps what it holds: `x.f`
            // of `x.f.borrow_mut()`, `x.f.replace(v)`, `x.f.take()` or
            // `x.f.get_mut()`, whose borrows `counted_in` says are counted
            // or not. Any other use, lent, `&x.f`, or moved, makes a
            // reference to it or a cell of it elsewhere.
            let mut borrowed_there = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && crate::lower::fn_def(thir[fun].ty)
                        .is_some_and(|(id, args)| borrows_its_cell(&recognition, id, args))
                    && let Some(&lent) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[crate::lower::body_queries::strip(thir, lent)].kind
                {
                    borrowed_there.insert(crate::lower::body_queries::strip(thir, arg));
                }
            }
            let derive = (tcx.trait_impl_of_assoc(body.def_id))
                .is_some_and(|imp| crate::lower::recognition::known_derive(tcx, imp));
            for (id, expr) in thir.exprs.iter_enumerated() {
                if let ExprKind::Field { .. } = expr.kind
                    && self.is_std_type(expr.ty, StdItem::RefCell)
                    && let Some(field) = field_of(thir, id)
                    && !borrowed_there.contains(&id)
                {
                    let uses = match derive {
                        true => &mut derived,
                        false => &mut lent,
                    };
                    uses.entry(field.did).or_insert(expr.span);
                }
            }
            // One bound by a pattern is a value of its own, lent.
            let mut bound = BoundRefCells {
                recognition: self.recognition(),
                thir,
                found: &mut lent,
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
        // A derive rust-js writes itself, of which no body is lowered, a
        // `Clone`'s, a `PartialEq`'s: it reads each `RefCell` field of its
        // struct through the `RefCell`'s own impl. Not a `Default`'s, which
        // makes them.
        for impls in tcx.all_local_trait_impls(()).values() {
            for &imp in impls {
                let of = tcx
                    .impl_trait_ref(imp)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id;
                let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
                if crate::lower::recognition::known_derive(tcx, imp.to_def_id())
                    && !crate::lower::recognition::is_std_def(tcx, of, StdItem::Default)
                    && let ty::Adt(adt, _) = self_ty.kind()
                    && adt.is_struct()
                {
                    for field in adt.all_fields() {
                        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                        if self.is_std_type(ty, StdItem::RefCell) {
                            derived.entry(field.did).or_insert(tcx.def_span(imp));
                        }
                    }
                }
            }
        }
        // Each of its regions erased, as `quiet` asks.
        let erased = |set: HashSet<Ty<'tcx>>| -> HashSet<Ty<'tcx>> {
            set.into_iter().map(|ty| tcx.erase_and_anonymize_regions(ty)).collect()
        };
        let counted = erased(counted);
        let borrowed = erased(borrowed);
        // Whether a struct's field may be one some borrow of which meets
        // another: not one no body lends, binds, moves or holds a borrow of,
        // nor a library's its contract says is quiet.
        let library_quiet = self.krate.foreign.fields_saying(crate::library::FieldFact::Quiet);
        let loud = |field: DefId| match field.as_local() {
            Some(_) => lent.contains_key(&field) || counted_fields.contains_key(&field) || derived.contains_key(&field),
            None => !library_quiet.contains_key(&crate::lower::library::item_key(tcx, field)),
        };
        let (boxed_all, boxed, seen) = self.boxed_ref_cells(&loud);
        let mut found = RefCells {
            counted,
            borrowed,
            handed: self.handed_ref_cells(),
            libraries: self.krate.foreign.ref_cell_facts(),
            library_quiet,
            boxed_all,
            boxed,
            ..RefCells::default()
        };
        found.agrees(tcx, &seen, &unseen);
        // A derive's read borrows it for a moment, while what it holds runs
        // its own impl, as `RefCell`'s does: of a type no borrow of which
        // can meet another, one more such borrow; else one that may meet
        // the borrow what it runs takes, counted (ADR 0863, amended).
        let recognition = self.recognition();
        for (field, span) in derived {
            let ty = tcx.type_of(field).instantiate_identity().skip_normalization();
            let quiet = found.quiet(&recognition, self.krate.cell_borrows, ty, &|t| {
                super::drop_runs(
                    recognition,
                    (self.krate.library, self.krate.closure_facts),
                    self.krate.counted,
                    t,
                )
            });
            if !quiet {
                lent.entry(field).or_insert(span);
            }
        }
        // A library's field none of whose borrows its code checks, which
        // this crate can't lend, or hold a borrow of while code that may
        // borrow it runs (ADR 0863, amended).
        for (&field, &span) in lent.iter().chain(&counted_fields) {
            if !field.is_local()
                && let Some(library) = found.library_quiet.get(&crate::lower::library::item_key(tcx, field))
            {
                tcx.dcx().span_err(
                    span,
                    format!(
                        "rust-js: `{library}`, a rust-js library, borrows `{}` unchecked, as it only ever borrows it for a moment (ADR 0863); this lends it, binds it, or holds a borrow of it while code that may borrow it runs: borrow it for a moment, `x.f.borrow_mut().push(..)`, or replace it, `x.f.replace(v)`",
                        tcx.def_path_str(field)
                    ),
                );
            }
        }
        // Each field of the crate's structs that's a `RefCell` none of whose
        // borrows can meet another: never lent, bound or moved, and never
        // borrowed while code that may borrow it runs.
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if self.is_std_type(ty, StdItem::RefCell)
                        && struct_field(tcx, field.did)
                        && !lent.contains_key(&field.did)
                        && !counted_fields.contains_key(&field.did)
                    {
                        found.quiet_fields.insert(field.did);
                    }
                }
            }
        }
        // A `#[rust_js::nullable]` field's `RefCell` is the value it holds,
        // whose `None` is `null`: not one of a `#[repr(transparent)]`
        // struct, which is its field, the cell (ADR 0584). Not one of a
        // `Vec`'s, the array, whose items are nullable (ADR 0586, amended).
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if crate::lower::bindings::is_nullable(tcx, field)
                        && self.is_std_type(ty, StdItem::RefCell)
                        && crate::lower::bindings::nullable_option(tcx, ty).is_some()
                        && !found.holds_value(tcx, field)
                    {
                        tcx.dcx().span_err(
                            tcx.def_span(field.did),
                            "rust-js: `#[rust_js::nullable]` of a `RefCell` is of a field that holds the value it holds, as a struct's does (ADR 0863); this one is a `#[repr(transparent)]` struct's, which is its field, the cell (ADR 0584)",
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
    /// generic code's `RefCell<Node<T>>` stands for each `Node`'s. Not a
    /// generic struct's field that's a `RefCell<T>`, which holds its value
    /// by its declaration (ADR 0863, amended): of each it's made of an
    /// object, where it's `loud`, some borrow of it may meet another, so
    /// that its generic code's `$fieldCell` is the cell each of its reads
    /// borrows; of one none can, the object itself.
    fn boxed_ref_cells(&self, loud: &dyn Fn(DefId) -> bool) -> (bool, HashSet<Ty<'tcx>>, HashSet<Ty<'tcx>>) {
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
        // Those a body has or a field is declared, which may be one boxed.
        let mut candidates: HashSet<Ty<'tcx>> = HashSet::new();
        let mut boxed: HashSet<Ty<'tcx>> = HashSet::new();
        let mut all = false;
        // Of a type made of a generic struct, `Cursor<Vec<u8>>`, each
        // `RefCell` of an object its loud generic field is made.
        let of_objects = |ty: Ty<'tcx>, boxed: &mut HashSet<Ty<'tcx>>| {
            for part in ty.walk().filter_map(|part| part.as_type()) {
                let ty::Adt(adt, args) = part.kind() else { continue };
                if !adt.is_struct() {
                    continue;
                }
                for field in adt.all_fields() {
                    let declared = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    if refcell(declared) && declared.has_param() && struct_field(tcx, field.did) && loud(field.did) {
                        let made = field.ty(tcx, args).skip_normalization();
                        if let ty::Adt(_, held) = made.kind()
                            && holds_itself(tcx, held.type_at(0))
                            && !made.has_param()
                        {
                            boxed.insert(tcx.erase_and_anonymize_regions(made));
                        }
                    }
                }
            }
        };
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
            let thir = &body.thir;
            // A read of a generic struct's field that holds its value, which
            // reads it as `{ value }` of any type, and what's made of it
            // there, its guard: not one of any value, which code given one
            // reads, nor one of what may be boxed.
            let reads = property_reads(tcx, thir);
            let types = (thir.exprs.iter_enumerated())
                .map(|(id, e)| (e.ty, reads.contains(&id)))
                .chain(thir.params.iter().map(|p| (p.ty, false)));
            for (ty, read) in types {
                of_objects(ty, &mut boxed);
                for part in ty.walk().filter_map(|part| part.as_type()) {
                    all |= !read && any_value(part);
                    if refcell(part) {
                        seen.insert(tcx.erase_and_anonymize_regions(part));
                        if !read {
                            candidates.insert(tcx.erase_and_anonymize_regions(part));
                        }
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
        // which its derives and drops read as generic code does; not a
        // struct's field that's one, which holds the value (ADR 0863,
        // amended), whatever it is. And each its fields are declared, of
        // which a field's is one or none, as each it may be.
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
                for field in tcx.adt_def(id).all_fields() {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    of_objects(ty, &mut boxed);
                    let holds_value = refcell(ty) && struct_field(tcx, field.did);
                    for part in ty.walk().filter_map(|part| part.as_type()) {
                        let its_own = holds_value && part == ty;
                        all |= !its_own && any_value(part);
                        if refcell(part) && !(its_own && ty.has_param()) {
                            candidates.insert(tcx.erase_and_anonymize_regions(part));
                        }
                    }
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
            let more: Vec<Ty<'tcx>> = (candidates.iter())
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
                // Of a struct's field, `x.f` of `x.f.borrow()`.
                let field =
                    args.first()
                        .and_then(|&lent| match thir[crate::lower::body_queries::strip(thir, lent)].kind {
                            ExprKind::Borrow { arg, .. } => {
                                field_of(thir, crate::lower::body_queries::strip(thir, arg))
                            }
                            _ => None,
                        });
                found.push(Found {
                    cell,
                    field: field.map(|field| field.did),
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
                    field: None,
                    counted: true,
                    seen: false,
                    span: expr.span,
                });
            }
        }
        found
    }
}

/// A borrow of a `RefCell` of type `cell`, at `span`, a struct's `field`'s
/// where it's one: whether it's counted, and whether only code rust-js sees
/// runs while it's held.
struct Found<'tcx> {
    cell: Ty<'tcx>,
    field: Option<DefId>,
    counted: bool,
    seen: bool,
    span: rustc_span::Span,
}

/// Each struct's `RefCell` field a pattern binds, not as `_` (ADR 0362).
struct BoundRefCells<'a, 'b, 'tcx> {
    recognition: crate::lower::recognition::Recognition<'a, 'tcx>,
    thir: &'b rustc_middle::thir::Thir<'tcx>,
    found: &'b mut HashMap<DefId, rustc_span::Span>,
}

impl<'b, 'tcx> rustc_middle::thir::visit::Visitor<'b, 'tcx> for BoundRefCells<'_, 'b, 'tcx> {
    fn thir(&self) -> &'b rustc_middle::thir::Thir<'tcx> {
        self.thir
    }

    fn visit_pat(&mut self, pat: &'b Pat<'tcx>) {
        if let rustc_middle::thir::PatKind::Leaf { ref subpatterns } = pat.kind
            && let ty::Adt(adt, _) = pat.ty.kind()
            && adt.is_struct()
        {
            for field in subpatterns {
                if !matches!(field.pattern.kind, rustc_middle::thir::PatKind::Wild)
                    && self.recognition.is_std_type(field.pattern.ty, StdItem::RefCell)
                {
                    let did = adt.non_enum_variant().fields[field.field].did;
                    self.found.entry(did).or_insert(field.pattern.span);
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

/// Whether `field` is a struct's, not `#[repr(transparent)]`, which is its
/// field (ADR 0584): one that's the property it holds, where a `RefCell`
/// field holds its value (ADR 0863, amended). Not an enum's variant's, read
/// by the pattern that binds it.
pub(in crate::lower) fn struct_field(tcx: TyCtxt<'_>, field: DefId) -> bool {
    let owner = tcx.parent(field);
    matches!(tcx.def_kind(owner), DefKind::Struct)
        && crate::lower::recognition::transparent_field(tcx, tcx.adt_def(owner)).is_none()
}

/// The struct's field `expr` reads, `x.f`, if it's one.
fn field_of<'tcx>(
    thir: &rustc_middle::thir::Thir<'tcx>,
    expr: rustc_middle::thir::ExprId,
) -> Option<&'tcx ty::FieldDef> {
    let ExprKind::Field {
        lhs,
        variant_index,
        name,
    } = thir[expr].kind
    else {
        return None;
    };
    match thir[lhs].ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(&adt.variant(variant_index).fields[name]),
        _ => None,
    }
}

/// Whether a call of `id` is one of a `RefCell`'s that borrows it, or
/// holds it only while it swaps what it holds, whose borrow `counted_in`
/// counts where it may meet another: what a field's use may be and the
/// field still have no reference to it elsewhere (ADR 0863, amended).
fn borrows_its_cell<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    use crate::lower::Std;
    matches!(
        recognition.classify(id, args),
        Some(
            Std::Borrow { lock: false, .. }
                | Std::TryBorrow { lock: false, .. }
                | Std::CellReplace
                | Std::CellTake
                | Std::CellGetMut
                | Std::CellReplaceWith
        )
    )
}

/// Of a body, each read of a generic struct's `RefCell<T>` field, which
/// holds its value by its declaration whatever it is (`RefCells::holds_value`),
/// and what's made of one there, a borrow of it, its guard, a call given it:
/// a box of the property, whatever its type, not a `RefCell<T>` of any value
/// that generic code is given (ADR 0863, amended).
fn property_reads<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &rustc_middle::thir::Thir<'tcx>,
) -> HashSet<rustc_middle::thir::ExprId> {
    use crate::lower::body_queries::strip;
    let mut reads = HashSet::new();
    for _ in 0..2 {
        for (id, expr) in thir.exprs.iter_enumerated() {
            let read = match expr.kind {
                ExprKind::Field { .. } => field_of(thir, id).is_some_and(|field| {
                    let declared = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    let ty::Adt(cell, args) = declared.kind() else {
                        return false;
                    };
                    crate::lower::recognition::is_std_def(tcx, cell.did(), StdItem::RefCell)
                        && struct_field(tcx, field.did)
                        && crate::lower::analysis::head(tcx, args.type_at(0)).is_none()
                }),
                ExprKind::Borrow { arg, .. } | ExprKind::RawBorrow { arg, .. } | ExprKind::Deref { arg } => {
                    reads.contains(&strip(thir, arg))
                }
                ExprKind::Call { fun, ref args, .. } => {
                    let read = args.first().is_some_and(|&first| reads.contains(&strip(thir, first)));
                    // And what it calls, `Deref::deref::<Ref<T>>`.
                    if read {
                        reads.insert(fun);
                    }
                    read
                }
                // A scope of one, as `strip` reads through.
                _ => strip(thir, id) != id && reads.contains(&strip(thir, id)),
            };
            if read {
                reads.insert(id);
            }
        }
    }
    reads
}
