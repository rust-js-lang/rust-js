//! What a call asks of a string's `Chars` past steps from its front
//! (#1186): std's names of it, which `analysis/chars_asked.rs` asks of each
//! call of a crate's bodies.

use super::Recognition;
use super::std_crates::std_crate;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Ty};

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty` holds a string's `Chars` or `CharIndices`: is one, or has one among
    /// what it's made of, its type's arguments, a reference's, a tuple's,
    /// what an `impl Trait` hides, another crate's too.
    pub(in crate::lower) fn holds_str_chars(&self, ty: Ty<'tcx>) -> bool {
        ty.walk()
            .filter_map(|part| part.as_type())
            .map(|part| self.reveal(part))
            .any(|part| self.is_str_chars(part) || self.is_char_indices(part))
    }

    /// Whether a call of `callee` given `args` asks a `Chars` that a type
    /// it's given holds more than steps from its front, so that its JS is
    /// the runtime's `$chars(s)`, not JS's own string iterator:
    /// - its rest, `as_str()`;
    /// - its hint, `size_hint()` of what holds one, std's `Flatten`'s asking
    ///   each inner's;
    /// - its back, of what holds one, the `Chars` itself or std's `Fuse`
    ///   and the like: a `DoubleEndedIterator` method, `rev()` or
    ///   `rposition()`;
    /// - its `Debug`, a `Debug` bound of a type that holds one, `{:?}`'s
    ///   `Argument::new_debug` too;
    /// - anything, given to a function that isn't std's, as its argument or
    ///   its type's, whose body, of this crate or another, asks what it
    ///   will of it.
    pub(in crate::lower) fn asks_chars_whole(&self, callee: DefId, args: GenericArgsRef<'tcx>) -> bool {
        let tcx = self.tcx;
        let holds = |ty: Ty<'tcx>| self.holds_str_chars(ty);
        let sig =
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(callee).instantiate(tcx, args).skip_normalization());
        if std_crate(tcx, callee.krate).is_none() {
            return args.types().any(holds) || sig.inputs().iter().any(|&input| holds(input));
        }
        let name = tcx.item_name(callee);
        let name = name.as_str();
        if name == "as_str"
            && sig
                .inputs()
                .first()
                .is_some_and(|input| self.is_str_chars(input.peel_refs()))
        {
            return true;
        }
        let of = args.types().next().map(|ty| ty.peel_refs());
        let double_ended = tcx
            .trait_of_assoc(callee)
            .is_some_and(|tr| super::is_double_ended(tcx, tr));
        if of.is_some_and(holds) && (name == "size_hint" || double_ended || matches!(name, "rev" | "rposition")) {
            return true;
        }
        let debug = tcx.get_diagnostic_item(rustc_span::sym::Debug);
        args.types().any(holds)
            && (tcx.clauses_of(callee).instantiate(tcx, args).clauses.iter())
                .filter_map(|clause| clause.as_trait_clause())
                .any(|clause| Some(clause.def_id()) == debug && holds(clause.self_ty().skip_binder()))
    }
}

/// Whether `ty`, a type a library gives out or takes, holds a `Chars`,
/// through what an `impl Trait` hides and the public fields of the crate's own
/// types, `seen` those already looked through: a consumer may ask it
/// anything (#1186).
pub(in crate::lower) fn reaches_str_chars<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    seen: &mut Vec<Ty<'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    for part in ty.walk().filter_map(|part| part.as_type()) {
        if seen.contains(&part) {
            continue;
        }
        seen.push(part);
        if recognition.is_str_chars(part) || recognition.is_char_indices(part) {
            return true;
        }
        let inner: Vec<Ty<'tcx>> = match *part.kind() {
            ty::Alias(_, alias) if let ty::Opaque { def_id } = alias.kind => {
                vec![tcx.type_of(def_id).instantiate(tcx, alias.args).skip_normalization()]
            }
            // Of its public fields: what it keeps to itself, another crate
            // reaches only through its functions, which give or take it.
            ty::Adt(adt, args) if adt.did().is_local() => (adt.all_fields())
                .filter(|field| field.vis.is_public())
                .map(|field| field.ty(tcx, args).skip_normalization())
                .collect(),
            _ => Vec::new(),
        };
        if inner
            .into_iter()
            .any(|inner| reaches_str_chars(recognition, inner, seen))
        {
            return true;
        }
    }
    false
}
