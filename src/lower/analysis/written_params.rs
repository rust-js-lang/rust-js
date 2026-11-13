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

use super::prints::{Prints, shown_by, written_arguments};
use crate::collections::{IndexMap, IndexSet};
use crate::lower::library::Foreign;
use crate::lower::recognition::{Recognition, pointer_target};
use crate::lower::{Body, fn_def};
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
    /// Whether a `dyn` of a formatting trait is written so, by the crate
    /// or a library it gives one to: every impl of the crate's own carries
    /// `fmtInto`.
    pub(in crate::lower) dyn_written: bool,
    /// Whether the crate makes a `dyn Display` or a `dyn Error`, a pair of
    /// a value and its dictionary, which a library's contract says
    /// (`Library::makes_fmt_dyns`).
    pub(in crate::lower) makes_dyns: bool,
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
    pub(in crate::lower) fn of_adt(&self, adt: DefId) -> bool {
        self.dyn_written || self.adts.contains(&adt)
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
            false => self.adts.iter().copied().filter(|did| did.is_local()).collect(),
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
) -> WrittenParams<'tcx> {
    let tcx = recognition.tcx;
    let mut found = WrittenParams::default();
    if !crate::lower::recognition::writes_piecewise() {
        return found;
    }
    let is_dyn = |ty: Ty<'tcx>| {
        ty.walk()
            .any(|part| matches!(part.as_type().map(|t| *t.kind()), Some(ty::Dynamic(..))))
    };
    // A pair `dyn`, a value and its dictionary, of a trait with a principal:
    // `Display`, `Error`, or one of a crate's, which may have `Display` as a
    // supertrait.
    let fmt_dyn = |ty: Ty<'tcx>| {
        ty.walk().any(|part| {
            matches!(part.as_type().map(|t| *t.kind()), Some(ty::Dynamic(traits, ..))
                if traits.principal_def_id().is_some_and(|id| recognition.is_pair_trait(id)))
        })
    };
    found.makes_dyns = all_bodies
        .iter()
        .any(|body| body.thir.exprs.iter().any(|expr| fmt_dyn(expr.ty)));
    // Of each body, what it writes into a writer: a `write!` into one's
    // values, and a writer's shown into one, what it shows; and where a
    // `dyn` is among them, where.
    let mut dyns_written: Vec<Span> = Vec::new();
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>, Span)> = Vec::new();
    for body in all_bodies {
        let owner = tcx.typeck_root_def_id(body.def_id);
        let mut written = Vec::new();
        let writes_into = (body.thir.exprs.iter()).find(|expr| match expr.kind {
            ExprKind::Call { fun, .. } => fn_def(body.thir[fun].ty)
                .is_some_and(|(callee, args)| recognition.writes_arguments_piecewise(callee, args)),
            _ => false,
        });
        if writes_into.is_some() {
            written.extend(written_arguments(recognition, body).unwrap_or_else(|| shown_by(recognition, body)));
        }
        if prints.writes_into_writers(body.def_id) {
            written.extend(shown_by(recognition, body));
        }
        for ty in written {
            if is_dyn(ty) {
                dyns_written.push(writes_into.map_or_else(|| tcx.def_span(body.def_id), |expr| expr.span));
            }
            found.give(tcx, owner, ty);
        }
        for expr in body.thir.exprs.iter() {
            if let ExprKind::Call { fun, .. } = expr.kind
                && let Some((callee, args)) = fn_def(body.thir[fun].ty)
            {
                calls.push((owner, callee, args, expr.span));
            }
        }
    }
    // Where a `dyn` is written so, here or by a library given one, each
    // value the crate makes a `dyn` of: its pair's dictionary writes it.
    found.dyn_written = !dyns_written.is_empty() || foreign.writes_dyns_given();
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
    }
    found
}

/// Refuses `ty`, given at `span` where it's written into a writer through
/// its dictionaries, if it holds a library's type whose dictionaries its
/// contract doesn't say carry `fmtInto` (#1688).
fn refuse_lacked<'tcx>(tcx: TyCtxt<'tcx>, foreign: &Foreign<'_, 'tcx>, ty: Ty<'tcx>, span: Span) {
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
