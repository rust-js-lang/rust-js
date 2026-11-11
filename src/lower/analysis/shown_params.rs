//! Which of each function's type parameters it shows with a `Formatter`'s
//! options, `{:#?}` pretty or `{:x?}` in hex, of its own placeholders and
//! of what the functions it calls show of what it gives them, a library's
//! as its manifest says; and the types the crate gives such a parameter,
//! and those their `Debug`s show, whose writers then take those options
//! (#1331). A writer of a type nothing gives one takes none.

use super::debug::format_calls;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::format_args::Piece;
use crate::lower::recognition::{StdItem, is_debug_argument, pointer_target, points_at_dyn_debug, std_item};
use crate::lower::{Body, fn_def, strip};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// What a value is shown with: whether pretty, `{:#?}`, and in hex,
/// `{:x?}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::lower) struct Shown {
    pub(in crate::lower) pretty: bool,
    pub(in crate::lower) hex: bool,
}

impl Shown {
    pub(in crate::lower) fn any(self) -> bool {
        self.pretty || self.hex
    }

    pub(in crate::lower) fn with(self, other: Shown) -> Shown {
        Shown {
            pretty: self.pretty || other.pretty,
            hex: self.hex || other.hex,
        }
    }
}

/// Of each of the crate's functions, its type parameters shown with
/// options, by index; and what's shown so of each type it gives one.
#[derive(Default)]
pub(in crate::lower) struct ShownParams<'tcx> {
    pub(in crate::lower) params: IndexMap<DefId, IndexMap<u32, Shown>>,
    types: IndexMap<Ty<'tcx>, Shown>,
    adts: IndexMap<DefId, Shown>,
    /// What the libraries the crate uses show of a `dyn Debug` they're
    /// given, as their manifests say (#1335).
    dyn_debug_given: Shown,
}

impl<'tcx> ShownParams<'tcx> {
    /// What's shown of any type the crate gives a parameter shown so.
    pub(in crate::lower) fn reached(&self) -> Shown {
        self.types
            .values()
            .fold(Shown::default(), |all, &shown| all.with(shown))
    }

    /// What the crate shows of a `dyn Debug` it's given, and the libraries
    /// it uses do, which its manifest says (#1335).
    pub(in crate::lower) fn dyn_debug(&self, tcx: TyCtxt<'tcx>) -> Shown {
        let debug = std_item(tcx, StdItem::Debug);
        (self.types.iter())
            .filter(|(ty, _)| matches!(ty.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id() == Some(debug)))
            .fold(self.dyn_debug_given, |all, (_, &shown)| all.with(shown))
    }

    /// What's shown of what `writer`, an impl's method, writes: its type's,
    /// by its definition, as one writer writes each of its instances.
    pub(in crate::lower) fn of_writer(&self, tcx: TyCtxt<'tcx>, writer: DefId) -> Shown {
        let Some(imp) = tcx.impl_of_assoc(writer) else {
            return Shown::default();
        };
        match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
            ty::Adt(adt, _) => self.adts.get(&adt.did()).copied().unwrap_or_default(),
            _ => Shown::default(),
        }
    }

    /// What's shown of `ty`, a type a dictionary's writer is made for.
    pub(in crate::lower) fn of_type(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Shown {
        let ty = tcx.erase_and_anonymize_regions(ty);
        self.types.get(&ty).copied().unwrap_or_default()
    }

    /// `owner` shows `ty` so: each of its type parameters `ty` holds, and
    /// each type `ty` is made of.
    fn give(&mut self, tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>, shown: Shown) -> bool {
        let mut changed = false;
        let identity = ty::GenericArgs::identity_for_item(tcx, owner);
        for (index, param) in identity.iter().enumerate() {
            if param.as_type().is_some() && ty.walk().any(|part| part == param) {
                let at = self.params.entry(owner).or_default().entry(index as u32).or_default();
                changed |= *at != at.with(shown);
                *at = at.with(shown);
            }
        }
        self.reach(tcx, ty, shown, &mut IndexSet::default());
        changed
    }

    /// `ty` and what its `Debug` shows of it, each type it's made of and
    /// each field of the crate's structs and enums, are shown so.
    fn reach(&mut self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, shown: Shown, seen: &mut IndexSet<Ty<'tcx>>) {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if !seen.insert(ty) {
            return;
        }
        for part in ty.walk().filter_map(|part| part.as_type()) {
            let part = tcx.erase_and_anonymize_regions(part);
            let at = self.types.entry(part).or_default();
            *at = at.with(shown);
            if let ty::Adt(adt, args) = *part.kind()
                && adt.did().is_local()
            {
                let at = self.adts.entry(adt.did()).or_default();
                *at = at.with(shown);
                for field in adt.all_fields() {
                    self.reach(tcx, field.ty(tcx, args).skip_normalization(), shown, seen);
                }
            }
        }
    }
}

/// Whether `id` is a writer, a function given a `&mut Formatter` (ADR
/// 0054), a `Debug`'s or a `Display`'s `fmt` among them.
fn writes_to_formatter(tcx: TyCtxt<'_>, id: DefId) -> bool {
    if !matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn) {
        return false;
    }
    let formatter = std_item(tcx, StdItem::Formatter);
    let sig = tcx.fn_sig(id).skip_binder().skip_binder();
    sig.inputs().iter().any(|input| {
        matches!(input.kind(), ty::Ref(_, inner, ty::Mutability::Mut)
            if matches!(inner.kind(), ty::Adt(adt, _) if adt.did() == formatter))
    })
}

/// Each function's type parameters `all_bodies` show with options, and
/// what they give each so, to a fixed point through their calls; a
/// library's function's, `marks`', as its manifest says.
pub(super) fn shown_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    marks: &dyn Fn(DefId) -> Vec<(u32, Shown)>,
    dyn_debug_given: Shown,
) -> ShownParams<'tcx> {
    let mut found = ShownParams {
        dyn_debug_given,
        ..ShownParams::default()
    };
    // Of each body, what its placeholders show so, and its calls.
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>)> = Vec::new();
    for body in all_bodies {
        let thir = &body.thir;
        let owner = tcx.typeck_root_def_id(body.def_id);
        for (_, pieces, fields) in format_calls(tcx, thir) {
            let Some(fields) = fields else { continue };
            for piece in &pieces {
                let Piece::Argument(index, spec) = piece else { continue };
                let shown = Shown {
                    pretty: spec.alternate,
                    hex: spec.debug_hex.is_some(),
                };
                if !shown.any() {
                    continue;
                }
                let Some(&field) = fields.get(*index) else { continue };
                if let ExprKind::Call { fun, ref args, .. } = thir[field].kind
                    && fn_def(thir[fun].ty).is_some_and(|(made_by, _)| is_debug_argument(tcx, made_by))
                    && let Some(&shown_value) = args.first()
                {
                    let ty = thir[strip(thir, shown_value)].ty.peel_refs();
                    found.give(tcx, owner, ty, shown);
                }
            }
        }
        // A value made a `dyn Debug` outside a writer, stored or passed,
        // where a library the crate uses shows one it's given pretty or in
        // hex, as its manifest says: its type's writer takes those options,
        // so only a type that may be shown so pays for them (the user's
        // decision, #1335). A writer's, a derived `Debug`'s fields, it
        // shows where it's made. One a placeholder of the body shows,
        // `{:#?}` of `&x as &dyn Debug`, which `format_args!` binds first,
        // is counted too.
        if found.dyn_debug_given.any() && !writes_to_formatter(tcx, body.def_id) {
            for expr in thir.exprs.iter() {
                if let ExprKind::PointerCoercion {
                    cast: PointerCoercion::Unsize,
                    source,
                    ..
                } = expr.kind
                    && points_at_dyn_debug(tcx, expr.ty)
                {
                    let made_of = pointer_target(tcx, thir[source].ty);
                    if !matches!(made_of.kind(), ty::Dynamic(..)) {
                        found.give(tcx, owner, made_of, found.dyn_debug_given);
                    }
                }
            }
        }
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, .. } = expr.kind
                && let Some((callee, args)) = fn_def(thir[fun].ty)
            {
                calls.push((owner, callee, args));
            }
        }
    }
    // What each call gives a parameter its callee shows so, to a fixed point.
    loop {
        let mut changed = false;
        for &(owner, callee, args) in &calls {
            let shown: Vec<(u32, Shown)> = match callee.is_local() {
                true => (found.params.get(&callee).into_iter().flatten())
                    .map(|(&index, &shown)| (index, shown))
                    .collect(),
                false => marks(callee),
            };
            for (index, shown) in shown {
                if let Some(ty) = args.get(index as usize).and_then(|arg| arg.as_type()) {
                    changed |= found.give(tcx, owner, ty, shown);
                }
            }
        }
        if !changed {
            break found;
        }
    }
}
