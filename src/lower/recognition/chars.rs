//! What a call asks of a string's iterator past steps from its front: a
//! `Chars` (#1186), and a `str`'s search by a pattern that runs code
//! (#1501); std's names of it, which `analysis/chars_asked.rs` asks of each
//! call of a crate's bodies.

use super::Recognition;
use super::std_crates::std_crate;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Ty};

/// A string's iterator whose JS steps only its front where nothing more is
/// asked of it: a `Chars` or `CharIndices`, JS's own string iterator, else
/// `$chars(s)` (#1186); and a `str`'s search by a pattern that runs code,
/// `$lazySplitBy` and the like, what's made of it JS's own, `.map(f)`, else
/// made knowing its back (#1501). One rule asks what's asked of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum TextIter {
    Chars,
    Search,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty` holds a string's iterator of `kind`, a `Chars` or a
    /// `CharIndices` say: is one, or has one among what it's made of, its
    /// type's arguments, a reference's, a tuple's, what an `impl Trait`
    /// hides, another crate's too.
    pub(in crate::lower) fn holds_text_iter(&self, kind: TextIter, ty: Ty<'tcx>) -> bool {
        ty.walk()
            .filter_map(|part| part.as_type())
            .map(|part| self.reveal(part))
            .any(|part| self.is_text_iter(kind, part))
    }

    /// Whether `ty` is a string's iterator of `kind`.
    pub(in crate::lower) fn is_text_iter(&self, kind: TextIter, ty: Ty<'tcx>) -> bool {
        match kind {
            TextIter::Chars => self.is_str_chars(ty) || self.is_char_indices(ty),
            TextIter::Search => self.str_search_runs_code(ty),
        }
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
    ///
    /// Of a search, the same, but its rest, which it has none of.
    pub(in crate::lower) fn asks_whole(&self, kind: TextIter, callee: DefId, args: GenericArgsRef<'tcx>) -> bool {
        let tcx = self.tcx;
        let holds = |ty: Ty<'tcx>| self.holds_text_iter(kind, ty);
        let sig =
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(callee).instantiate(tcx, args).skip_normalization());
        if std_crate(tcx, callee.krate).is_none() {
            return args.types().any(holds) || sig.inputs().iter().any(|&input| holds(input));
        }
        let name = tcx.item_name(callee);
        let name = name.as_str();
        if name == "as_str"
            && kind == TextIter::Chars
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

/// Whether `ty`, a type a library gives out or takes, holds a string's
/// iterator of `kind`, a `Chars` say,
/// through what an `impl Trait` hides and the public fields of the crate's own
/// types, `seen` those already looked through: a consumer may ask it
/// anything (#1186).
pub(in crate::lower) fn reaches_text_iter<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    kind: TextIter,
    ty: Ty<'tcx>,
    seen: &mut Vec<Ty<'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    for part in ty.walk().filter_map(|part| part.as_type()) {
        if seen.contains(&part) {
            continue;
        }
        seen.push(part);
        if recognition.is_text_iter(kind, part) {
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
            .any(|inner| reaches_text_iter(recognition, kind, inner, seen))
        {
            return true;
        }
    }
    false
}
