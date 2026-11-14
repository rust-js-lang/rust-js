//! Which of each function's type parameters it writes into a writer a
//! piece at a time through their formatting dictionaries, of its own
//! `write!` into a writer, as a writer shown into one (`Prints::framed`),
//! or by what the functions it calls do with what it gives them, a
//! library's as its manifest says; and the types that reach such a
//! parameter, or a `dyn` written so, each of whose formatting
//! dictionaries then carries `fmtInto(f, x, options)`, which writes the
//! value into the sink `f` a call at a time, as std's `fmt` of it does
//! (#1652, step 5 of ADR 1135). A dictionary nothing writes so carries
//! none, so no other program's JS changes.
//!
//! Where a `dyn` is written so, crate-wide to start: every value the crate
//! makes a `dyn` of, and every impl of the crate's own, whose dictionary a
//! `dyn` std's conversions make, `?`'s, holds. A library's type given such
//! a parameter, or a `dyn` a library may make, is written so only where
//! its contract says its dictionaries carry `fmtInto`; otherwise it's
//! refused, naming #1688.

use super::prints::{Prints, shown_by, shown_into_writers};
use crate::collections::{IndexMap, IndexSet};
use crate::lower::library::Foreign;
use crate::lower::recognition::{Recognition, pointer_target};
use crate::lower::{Body, Std, fn_def};
use rustc_middle::mir;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// Of each of the crate's functions, its type parameters written into a
/// writer through their dictionaries, by index; and each type that
/// reaches one, or a `dyn` written so.
// Flags, not a state: two facts of the crate apart, what it writes and what it makes.
#[derive(Default)]
pub(in crate::lower) struct WrittenParams<'tcx> {
    pub(in crate::lower) params: IndexMap<DefId, IndexSet<u32>>,
    types: IndexSet<Ty<'tcx>>,
    adts: IndexSet<DefId>,
    /// The types given such a parameter, or held by what is
    /// (`given_adt`).
    given_adts: IndexSet<DefId>,
    /// Of a library, its types `prints` published, which its contract
    /// lists, std's of `@rust-js/std`'s build too.
    published: IndexSet<DefId>,
    /// Whether a `dyn` of a formatting trait is written so, by the crate
    /// or a library it gives one to: every impl of the crate's own carries
    /// `fmtInto`.
    pub(in crate::lower) dyn_written: bool,
    /// Of a `dyn` written so, the traits it may be of, each with its
    /// supertraits, `dyn Display`'s `Display`; where any may, none here and
    /// `dyn_any` (a library's given one, or a `dyn Any`).
    dyn_traits: IndexSet<DefId>,
    dyn_any: bool,
    /// Whether the crate makes a `dyn Display` or a `dyn Error`, a pair of
    /// a value and its dictionary, which a library's contract says
    /// (`Library::makes_fmt_dyns`).
    pub(in crate::lower) makes_dyns: bool,
    /// Each function's `fmt::Arguments` parameters it writes into a
    /// writer a piece at a time (step 6).
    pub(in crate::lower) arguments: WrittenArguments,
}

impl<'tcx> WrittenParams<'tcx> {
    /// Does a formatting dictionary made for `ty` carry `fmtInto`: a type
    /// that reaches such a parameter, or a `dyn` written so, or one made of
    /// one?
    pub(in crate::lower) fn of_type(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.types.contains(&tcx.erase_and_anonymize_regions(ty))
    }

    /// Does the dictionary of an impl for `adt`, one of the crate's types,
    /// carry `fmtInto`, as one of its instances reaches such a parameter,
    /// or as a `dyn` is written so?
    pub(in crate::lower) fn of_adt(&self, adt: DefId, tr: DefId) -> bool {
        self.of_dyn(tr) || self.adts.contains(&adt)
    }

    /// May a `dyn` written so be an impl of `tr`: one of `tr`'s, or of a
    /// trait it's a supertrait of, `dyn Error`'s `Debug` (#1712)?
    fn of_dyn(&self, tr: DefId) -> bool {
        self.dyn_any || self.dyn_traits.contains(&tr)
    }

    /// Is `adt` given such a parameter itself, or held by what is, `Vec<T>`'s
    /// `T`, or a `dyn` written so: not reached only as a field of the crate's
    /// type, whose writer writes it so only where framing frames its writer
    /// too, `Der`'s private `BytesInner`, which its own `Debug` never shows.
    pub(in crate::lower) fn given_adt(&self, adt: DefId, tr: DefId) -> bool {
        self.of_dyn(tr) || self.given_adts.contains(&adt)
    }

    /// The crate's types whose impls' dictionaries carry `fmtInto`, which a
    /// library's contract lists (`Library::written_into`); every one where
    /// a `dyn` is written so.
    pub(in crate::lower) fn local_adts(&self, tcx: TyCtxt<'tcx>) -> Vec<DefId> {
        match self.dyn_written {
            true => (tcx.hir_crate_items(()).definitions())
                .filter(|&id| {
                    matches!(
                        tcx.def_kind(id),
                        rustc_hir::def::DefKind::Struct
                            | rustc_hir::def::DefKind::Enum
                            | rustc_hir::def::DefKind::Union
                    )
                })
                .map(|id| id.to_def_id())
                .collect(),
            // And of `@rust-js/std`'s build, the std types it compiles a
            // writer of, which `prints` published (#1712).
            false => (self.adts.iter().copied())
                .filter(|&did| did.is_local() || self.published.contains(&did))
                .collect(),
        }
    }

    /// The parameters of `owner`, by index, written so.
    pub(in crate::lower) fn of_fn(&self, owner: DefId) -> impl Iterator<Item = u32> + '_ {
        self.params.get(&owner).into_iter().flatten().copied()
    }

    /// `owner` writes `ty` so: each of its type parameters `ty` holds, and
    /// each type `ty` is made of, reach it.
    fn give(&mut self, tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>) -> bool {
        let mut changed = false;
        // An `impl Display` a library gives out, as the type it hides, a
        // `Wrap<char>`, whose `char`'s dictionary its writer is given (#1743).
        let revealed = crate::lower::mir::revealed(tcx, ty);
        if revealed != ty {
            changed |= self.give(tcx, owner, revealed);
        }
        let given = (ty.walk().filter_map(|part| part.as_type())).filter_map(|part| part.ty_adt_def());
        self.given_adts.extend(given.map(|adt| adt.did()));
        let identity = ty::GenericArgs::identity_for_item(tcx, owner);
        for (index, param) in identity.iter().enumerate() {
            if param.as_type().is_some() && ty.walk().any(|part| part == param) {
                changed |= self.params.entry(owner).or_default().insert(index as u32);
            }
        }
        self.reach(tcx, ty, &mut IndexSet::default());
        changed
    }

    /// `ty`, each type it's made of and each field of the crate's structs
    /// and enums, which their writers may write so in turn, reach it.
    fn reach(&mut self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if !seen.insert(ty) {
            return;
        }
        for part in ty.walk().filter_map(|part| part.as_type()) {
            let part = tcx.erase_and_anonymize_regions(part);
            self.types.insert(part);
            if let ty::Adt(adt, args) = *part.kind() {
                self.adts.insert(adt.did());
                if adt.did().is_local() {
                    for field in adt.all_fields() {
                        self.reach(tcx, field.ty(tcx, args).skip_normalization(), seen);
                    }
                }
            }
        }
    }
}

/// Each function's type parameters `all_bodies` write into a writer
/// through their dictionaries, and what they give each so, to a fixed point
/// through their calls; a library's function's as its manifest says. Only
/// where writes are followed a piece at a time (ADR 1135's switch); none
/// else. A library's type given one, or a library's `dyn` written so,
/// whose dictionary its contract doesn't say carries `fmtInto`, is refused
/// (#1688).
pub(super) fn written_params<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    prints: &Prints,
    foreign: &Foreign<'_, 'tcx>,
    arguments: WrittenArguments,
) -> WrittenParams<'tcx> {
    let tcx = recognition.tcx;
    let mut found = WrittenParams {
        arguments,
        ..Default::default()
    };
    // Of a library, its types a consumer may write into a writer, which
    // `prints` framed: their dictionaries carry `fmtInto`.
    found.adts.extend(prints.published.iter().copied());
    found.published.extend(prints.published.iter().copied());
    let is_dyn = |ty: Ty<'tcx>| {
        ty.walk()
            .any(|part| matches!(part.as_type().map(|t| *t.kind()), Some(ty::Dynamic(..))))
    };
    // A pair `dyn`, a value and its dictionary, of a trait with a principal:
    // `Display`, `Debug`, `Error`, or one of a crate's, which may have
    // `Display` as a supertrait.
    let fmt_dyn = |ty: Ty<'tcx>| {
        ty.walk().any(|part| {
            matches!(part.as_type().map(|t| *t.kind()), Some(ty::Dynamic(traits, ..))
                if traits.principal_def_id().is_some_and(|id| recognition.is_pair_trait(id)))
        })
    };
    // Not a derived `Debug`'s, whose `&dyn Debug`s of its fields std's
    // `debug_struct_fields_finish` alone is given, which never leave it.
    let derived = |body: &Body<'tcx>| {
        (tcx.opt_parent(body.def_id)).is_some_and(|imp| {
            tcx.def_kind(imp) == rustc_hir::def::DefKind::Impl { of_trait: true } && tcx.is_automatically_derived(imp)
        })
    };
    // What makes a pair `dyn`: a coercion of what isn't one, `Box::new(e)`
    // as a `Box<dyn Error>`, or std's conversion into one, `?`'s
    // `from_residual` or a `From<E> for Box<dyn Error>`; not one passed on,
    // either's `inner.source()`, which what made it made (#1712).
    let makes = |body: &Body<'tcx>, expr: &rustc_middle::thir::Expr<'tcx>| match expr.kind {
        ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } => fmt_dyn(expr.ty) && !fmt_dyn(body.thir[source].ty),
        ExprKind::Call { fun, .. } => fn_def(body.thir[fun].ty).is_some_and(|(callee, args)| {
            let converts = tcx.is_lang_item(callee, rustc_hir::attrs::lang_items::LangItem::TryTraitFromResidual)
                || tcx.trait_of_assoc(callee).is_some_and(|tr| {
                    use crate::lower::recognition::{StdItem, is_std_def};
                    is_std_def(tcx, tr, StdItem::From) || is_std_def(tcx, tr, StdItem::Into)
                });
            converts && args.types().any(fmt_dyn)
        }),
        _ => false,
    };
    found.makes_dyns = all_bodies
        .iter()
        .filter(|body| !derived(body))
        .any(|body| body.thir.exprs.iter().any(|expr| makes(body, expr)));
    // Of each body, what it writes into a writer: a `write!` into one's
    // values, and a writer's shown into one, what it shows; and where a
    // `dyn` is among them, where.
    let mut dyns_written: Vec<Span> = Vec::new();
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>, Span)> = Vec::new();
    for body in all_bodies {
        let owner = tcx.typeck_root_def_id(body.def_id);
        let recognition = &within(recognition, body.def_id);
        let mut written = Vec::new();
        let writes_into = (body.thir.exprs.iter()).find(|expr| match expr.kind {
            ExprKind::Call { fun, ref args, .. } => fn_def(body.thir[fun].ty).is_some_and(|callee| {
                !recognition.formatter_write_fmt(callee.0, callee.1)
                    && (0..args.len()).any(|at| found.arguments.writes(recognition, callee, at))
            }),
            _ => false,
        });
        if let Some(shown) = shown_into_writers(recognition, body, &found.arguments) {
            written.extend(shown.unwrap_or_else(|| shown_by(recognition, body)));
        }
        if prints.writes_into_writers(body.def_id) {
            written.extend(shown_by(recognition, body));
        }
        for ty in written {
            if is_dyn(ty) {
                dyns_written.push(writes_into.map_or_else(|| tcx.def_span(body.def_id), |expr| expr.span));
                for part in ty.walk().filter_map(|part| part.as_type()) {
                    if let ty::Dynamic(traits, ..) = part.kind() {
                        match traits.principal_def_id() {
                            Some(principal) => found
                                .dyn_traits
                                .extend(ty::elaborate::supertrait_def_ids(tcx, principal)),
                            None => found.dyn_any = true,
                        }
                    }
                }
            }
            found.give(tcx, owner, ty);
        }
        // Each function it calls or names, a `fn` pointer of an instance
        // too, `bumped::<Point>`, which writes what it's given so (#1710).
        for (callee, args, span) in crate::lower::instances::calls_of(body) {
            calls.push((owner, callee, args, span));
        }
    }
    // Where a `dyn` is written so, here or by a library given one, each
    // value the crate makes a `dyn` of: its pair's dictionary writes it.
    found.dyn_written = !dyns_written.is_empty() || foreign.writes_dyns_given();
    found.dyn_any |= foreign.writes_dyns_given();
    if found.dyn_written {
        for body in all_bodies {
            let owner = tcx.typeck_root_def_id(body.def_id);
            for expr in body.thir.exprs.iter() {
                if let ExprKind::PointerCoercion {
                    cast: PointerCoercion::Unsize,
                    source,
                    ..
                } = expr.kind
                    && matches!(pointer_target(tcx, expr.ty).kind(), ty::Dynamic(..))
                {
                    let made_of = pointer_target(tcx, body.thir[source].ty);
                    if !matches!(made_of.kind(), ty::Dynamic(..)) {
                        found.give(tcx, owner, made_of);
                        refuse_lacked(tcx, foreign, made_of, expr.span);
                    }
                }
            }
        }
    }
    // A library's `dyn` written so may hold a dictionary of its own without
    // `fmtInto`, which its contract says it may make.
    if let Some(&span) = dyns_written.first()
        && foreign.makes_fmt_dyns()
    {
        let what = "writing a `dyn` into a writer of the crate's own or a generic one where a library it uses makes such a `dyn`";
        let why =
            "std writes it a piece at a time through its dictionary, which the library's doesn't say it can (#1688)";
        crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(what, why));
    }
    // What each call gives a parameter its callee writes so, to a fixed point.
    let mut given: IndexSet<(Ty<'tcx>, Span)> = IndexSet::default();
    loop {
        let mut changed = false;
        for &(owner, callee, args, span) in &calls {
            let written: Vec<u32> = match callee.is_local() {
                true => found.of_fn(callee).collect(),
                false => foreign.written_marks(callee),
            };
            for index in written {
                if let Some(ty) = args.get(index as usize).and_then(|arg| arg.as_type()) {
                    changed |= found.give(tcx, owner, ty);
                    given.insert((ty, span));
                }
            }
        }
        if !changed {
            break;
        }
    }
    for (ty, span) in given {
        refuse_lacked(tcx, foreign, ty, span);
        refuse_arguments(tcx, ty, span);
    }
    found
}

/// Refuses `ty`, given at `span` to generic code that writes it into a
/// writer through its dictionaries, if it holds an `fmt::Arguments`, which
/// is the function that writes into the sink it claims only where a write
/// of it is followed, or it's made a `dyn` (#1652, `given_to_a_write`).
fn refuse_arguments<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, span: Span) {
    use rustc_hir::attrs::lang_items::LangItem;
    let arguments = (ty.walk().filter_map(|part| part.as_type()))
        .any(|part| matches!(part.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArguments)));
    if arguments {
        let what = format!("an `fmt::Arguments` given to generic code that writes a `{ty}` into a writer");
        let why = "std writes it a piece at a time, which isn't followed for one given so (#1745)";
        crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(&what, why));
    }
}

/// Refuses `ty`, given at `span` where it's written into a writer through
/// its dictionaries, if it holds a library's type whose dictionaries its
/// contract doesn't say carry `fmtInto` (#1688).
fn refuse_lacked<'tcx>(tcx: TyCtxt<'tcx>, foreign: &Foreign<'_, 'tcx>, ty: Ty<'tcx>, span: Span) {
    // An `impl Display` a library gives out, as the type it hides (#1743).
    let ty = crate::lower::mir::revealed(tcx, ty);
    let lacked = (ty.walk().filter_map(|part| part.as_type()))
        .find(|part| part.ty_adt_def().is_some_and(|adt| foreign.lacks_fmt_into(adt.did())));
    if let Some(lacked) = lacked {
        let what = format!(
            "writing a `{lacked}` into a writer of the crate's own or a generic one through generic code or a `dyn`"
        );
        let why =
            "std writes it a piece at a time through its dictionary, which its library doesn't say it can (#1688)";
        crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(&what, why));
    }
}

/// Of each of the crate's functions, its `fmt::Arguments` value
/// parameters it writes into a writer a piece at a time, by index, its
/// `self` first: given by its own `write_fmt` or `fmt::write` into a writer
/// std writes so, a `W: fmt::Write`'s and a `Formatter`'s too, or given on
/// to a function's parameter written so, to a fixed point; a library's as
/// its contract says (`Calling.written_arguments`). Each `format_args!`
/// given one is the function that makes its text by writing into the sink
/// it claims, `const f = $sink(); ..; return f.done();`, which a write of
/// it arms with its writer's, `$writingInto(f, args)`, and which, called
/// plainly, gives its text (#1652, step 6 of ADR 1135).
///
/// A trait's method is marked too where an impl of the crate's writes its
/// parameter so, so a call through a dictionary or a `dyn`, which may run
/// that impl's, gives what it writes.
#[derive(Default)]
pub(in crate::lower) struct WrittenArguments {
    params: IndexMap<DefId, IndexSet<u32>>,
    /// Whether anything of the program may write an `Arguments` into a
    /// writer's sink (`arms_sinks`): where nothing does, each one's only use
    /// is its text, and each is its template, `() => `<${value}>``.
    pub(in crate::lower) arms: bool,
}

impl WrittenArguments {
    /// The parameters of `owner`, by index, written so.
    pub(in crate::lower) fn of_fn(&self, owner: DefId) -> impl Iterator<Item = u32> + '_ {
        self.params.get(&owner).into_iter().flatten().copied()
    }

    /// Whether a call of `callee`, of `generic_args`, writes the
    /// `Arguments` it's given at `index` into a writer a piece at a time:
    /// std's `write_fmt` into a writer std writes so, or `fmt::write`, of
    /// its second; a `Formatter`'s `write_fmt`; or a function's parameter
    /// written so, of the impl a trait's method resolves to, else of the
    /// trait's, a library's as its contract says.
    pub(in crate::lower) fn writes<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (callee, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        index: usize,
    ) -> bool {
        if index == 1
            && (recognition.writes_arguments_piecewise(callee, generic_args)
                || recognition.formatter_write_fmt(callee, generic_args))
        {
            return true;
        }
        let callee = implementor(recognition, callee, generic_args).unwrap_or(callee);
        self.marks(recognition, callee).contains(&(index as u32))
    }

    /// Each of the crate's traits' methods an impl of the crate's writes so,
    /// and its parameters, which its contract lists, not being items
    /// (`Library::written_trait_arguments`).
    pub(in crate::lower) fn trait_methods(&self, tcx: TyCtxt<'_>) -> Vec<(DefId, Vec<u32>)> {
        (self.params.iter())
            .filter(|(id, _)| id.is_local() && tcx.trait_of_assoc(**id).is_some())
            .map(|(&id, params)| (id, params.iter().copied().collect()))
            .collect()
    }

    /// The parameters of `id`, by index, written so: as found here, of the
    /// crate's function or a library's it compiles itself, an impl's
    /// method, which its contract doesn't list; and of a library's, as its
    /// contract says.
    pub(in crate::lower) fn marks(&self, recognition: &Recognition<'_, '_>, id: DefId) -> Vec<u32> {
        let mut marks: Vec<u32> = self.of_fn(id).collect();
        if !id.is_local() {
            marks.extend(recognition.foreign.written_arguments(id));
        }
        marks
    }

    /// `owner` writes its parameter `index` so; and, of an impl's method of
    /// the crate's trait, its trait's method does.
    fn mark(&mut self, tcx: TyCtxt<'_>, owner: DefId, index: u32) -> bool {
        let mut changed = self.params.entry(owner).or_default().insert(index);
        if let Some(declared) = tcx.trait_item_of(owner)
            && declared.is_local()
        {
            changed |= self.params.entry(declared).or_default().insert(index);
        }
        changed
    }
}

/// The method of an impl, the crate's or a library's, that a call of
/// `callee`, a trait's method, of `generic_args` runs; `None` of another
/// call, or of one only a caller's types say.
fn implementor<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    callee: DefId,
    generic_args: ty::GenericArgsRef<'tcx>,
) -> Option<DefId> {
    use rustc_middle::ty::TypeVisitableExt;
    let tcx = recognition.tcx;
    let trait_ = tcx.trait_of_assoc(callee)?;
    let tr = ty::TraitRef::from_assoc(tcx, trait_, generic_args);
    if tr.has_non_region_param() {
        return None;
    }
    let imp = recognition.user_impl(tr)?;
    tcx.impl_item_implementor_ids(imp).get(&callee).copied()
}

/// Each function's `fmt::Arguments` parameters `all_bodies` write into a
/// writer a piece at a time, to a fixed point through their calls
/// (`WrittenArguments`), by their MIR, as the lowering reads it: a
/// library's a consumer compiles itself too, whose contract says nothing of
/// an impl's method. Only where writes are followed so (ADR 1135's switch);
/// none else.
pub(super) fn written_arguments<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> WrittenArguments {
    use rustc_hir::attrs::lang_items::LangItem;
    let tcx = recognition.tcx;
    let mut found = WrittenArguments::default();
    let is_arguments = |ty: Ty<'tcx>| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArguments));
    // Of each function, its `Arguments` parameters.
    let params: Vec<(&Body<'tcx>, &mir::Body<'tcx>, Vec<mir::Local>)> = (all_bodies.iter().copied())
        .filter(|body| !tcx.is_closure_like(body.def_id))
        .filter_map(|body| {
            let mir = &body.mir.as_ref()?.body;
            let params: Vec<mir::Local> = mir
                .args_iter()
                .filter(|&arg| is_arguments(mir.local_decls[arg].ty))
                .collect();
            (!params.is_empty()).then_some((body, mir, params))
        })
        .collect();
    loop {
        let mut changed = false;
        for &(body, mir, ref params) in &params {
            let recognition = within(recognition, body.def_id);
            for &param in params {
                if given_to_a_write(&recognition, &found, mir, param) {
                    changed |= found.mark(tcx, body.def_id, param.as_u32() - 1);
                }
            }
        }
        if !changed {
            break;
        }
    }
    found
}

/// Whether `local`, an `Arguments` of `body`, is given, as it is or through
/// what it's copied to, to a parameter a call writes into a writer a piece
/// at a time (`WrittenArguments::writes`); or is kept where a write may
/// find it later, past what's followed: a field, a closure's capture, a
/// value returned, or a generic function's parameter, `Some(args)`,
/// `Box::new(args)`. What's read of a place, of a call's result, is then
/// the function that writes into the sink it claims (`sink_form_given`),
/// so a `Display::fmt(&self.0, f)` writes it a piece at a time.
pub(in crate::lower) fn given_to_a_write<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    marks: &WrittenArguments,
    body: &mir::Body<'tcx>,
    local: mir::Local,
) -> bool {
    use rustc_middle::mir::{Operand, Rvalue, StatementKind, TerminatorKind};
    let tcx = recognition.tcx;
    // Made as what's returned, a function's or a constant's.
    if local == mir::RETURN_PLACE {
        return true;
    }
    let mut holders = vec![local];
    let mut at = 0;
    while let Some(&holder) = holders.get(at) {
        at += 1;
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                // Copied, moved, lent, `&args` of a `Display::fmt`, or lent
                // again, `&*args`.
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) | Rvalue::Ref(_, _, from) =
                        &assign.1
                    && from.local == holder
                    && (from.projection.is_empty()
                        || matches!(assign.1, Rvalue::Ref(..)) && from.projection[..] == [mir::ProjectionElem::Deref])
                    && let Some(to) = assign.0.as_local()
                    && !holders.contains(&to)
                {
                    holders.push(to);
                }
                // Kept: in a field, a capture, or what's returned; or made a
                // `&dyn Display`, which a writer may be given, itertools'
                // `format_with`'s `f(&format_args!(..))` (#1712).
                if let StatementKind::Assign(assign) = &statement.kind
                    && match &assign.1 {
                        Rvalue::Aggregate(_, operands) => operands.iter().any(|operand| is_holder(operand, holder)),
                        Rvalue::Use(operand, _) => {
                            is_holder(operand, holder) && (assign.0.as_local()).is_none_or(|to| to == mir::RETURN_PLACE)
                        }
                        Rvalue::Cast(mir::CastKind::PointerCoercion(PointerCoercion::Unsize, _), operand, _) => {
                            is_holder(operand, holder)
                        }
                        _ => false,
                    }
                {
                    return true;
                }
            }
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some(callee) = fn_def(func.ty(&body.local_decls, tcx))
                && (args.iter().enumerate()).any(|(i, arg)| {
                    is_holder(&arg.node, holder)
                        && (marks.writes(recognition, callee, i) || kept_by_a_generic(recognition, callee, i))
                })
            {
                return true;
            }
        }
    }
    false
}

/// Whether `operand` is `holder` itself, copied or moved.
fn is_holder(operand: &mir::Operand<'_>, holder: mir::Local) -> bool {
    operand
        .place()
        .is_some_and(|p| p.local == holder && p.projection.is_empty())
}

/// Whether `callee` may keep what it's given at `index`, or write it: a
/// formatting trait's `fmt` of it, `Display::fmt(&args, f)`, std's
/// `fmt::write` of it into the `Formatter`'s writer, strum's; or a generic
/// function whose parameter's type parameter is in what it gives back or in
/// another parameter, `Box::new(args)`, `v.push(args)`, which may write it
/// later. Not one that only reads it, `args.to_string()`, whose text is
/// made at once, its template's (#1652); nor of `Arguments` itself,
/// `_print(args)`, a function of the crate's whose writes
/// `WrittenArguments` follows; nor a placeholder's `Argument`,
/// `Argument::new_display(&args)`, which keeps it only to format it there.
fn kept_by_a_generic<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (callee, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
    index: usize,
) -> bool {
    let tcx = recognition.tcx;
    let placeholder = matches!(
        recognition.classify(callee, generic_args),
        Some(Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer)
    );
    if placeholder || crate::lower::mir::arguments_use::std_formats_param(recognition, callee, index) {
        return false;
    }
    if index == 0 && crate::lower::recognition::is_fmt_method(tcx, callee) {
        return true;
    }
    // Of the call's own types: an `Arguments` in what it gives back or in
    // another parameter, `Box<Arguments>`, `&mut Vec<Arguments>`.
    use rustc_hir::attrs::lang_items::LangItem;
    let sig = (tcx.fn_sig(callee).instantiate(tcx, generic_args))
        .skip_normalization()
        .skip_binder();
    let holds_arguments = |ty: Ty<'tcx>| {
        (ty.walk().filter_map(|part| part.as_type())).any(
            |part| matches!(part.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArguments)),
        )
    };
    (sig.inputs().iter().enumerate()).any(|(at, &input)| at != index && holds_arguments(input))
        || holds_arguments(sig.output())
}

/// Whether anything may write an `Arguments` into a writer's sink, armed
/// for it (`$writingInto`): a library, whose consumers may; a writer
/// shown into a writer, `framed`, whose `Formatter`'s writes are a sink's;
/// or a call that writes one it's given into a writer a piece at a time,
/// `w.write_fmt(args)` of a writer of the crate's own or a generic one,
/// `fmt::write`, a function of the crate's or a library's that does
/// (`WrittenArguments::writes`). A `Formatter`'s `write_fmt`, or a writer
/// function's given its `Formatter`, of a writer not framed writes its text.
pub(super) fn arms_sinks<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    (library, framed): (bool, bool),
    marks: &WrittenArguments,
) -> bool {
    use rustc_hir::attrs::lang_items::LangItem;
    use rustc_middle::mir::TerminatorKind;
    let tcx = recognition.tcx;
    let is_arguments = |ty: Ty<'tcx>| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArguments));
    library
        || framed
        || all_bodies.iter().any(|body| {
            let Some(mir) = body.mir.as_ref().map(|mir| &mir.body) else {
                return false;
            };
            let recognition = within(recognition, body.def_id);
            mir.basic_blocks.iter().any(|data| match &data.terminator().kind {
                TerminatorKind::Call { func, args, .. } => {
                    let Some(callee) = fn_def(func.ty(&mir.local_decls, tcx)) else {
                        return false;
                    };
                    (args.iter().enumerate()).any(|(i, arg)| {
                        is_arguments(arg.node.ty(&mir.local_decls, tcx))
                            && !(i == 1 && recognition.formatter_write_fmt(callee.0, callee.1))
                            && recognition.formatter_param(callee.0).is_none()
                            && marks.writes(&recognition, callee, i)
                    })
                }
                _ => false,
            })
        })
}

/// `recognition` asked within `body`, its generic parameters its own, which
/// a call of `body`'s may name, `<Small<CAP> as Write>::write_fmt`.
pub(super) fn within<'r, 'tcx>(recognition: &Recognition<'r, 'tcx>, body: DefId) -> Recognition<'r, 'tcx> {
    let tcx = recognition.tcx;
    Recognition {
        typing_env: ty::TypingEnv::non_body_analysis(tcx, tcx.typeck_root_def_id(body)),
        ..*recognition
    }
}
