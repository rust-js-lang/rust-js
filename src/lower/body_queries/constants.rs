//! The constants a body's patterns name each value they test by, where
//! each `match` or `let` tests it (ADRs 0590, 0634).

use super::pattern_text;
use rustc_middle::thir::{self, ExprKind, Pat, PatKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// A value a pattern tests, which a `const` may name (`constant_patterns`):
/// a number's or a `char`'s, by its type and bits; the registry's
/// symbol's, `js::SymbolFor`'s, by its key (ADR 0634); or a string's, by
/// its text, which tells two string patterns apart (#389c, rd112).
#[derive(Clone, PartialEq, Eq, Hash)]
pub(in crate::lower) enum PatternValue<'tcx> {
    Bits(ty::Ty<'tcx>, u128),
    Key(String),
    Text(String),
}

/// A step from the value a `match` or `let` tests to a part a pattern
/// tests of it (`ConstantPatterns`): a field, a variant, an element; MIR's
/// place of it is the same steps, its derefs left out (`pattern_places`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum PatternStep {
    Field(rustc_abi::FieldIdx),
    Variant(rustc_abi::VariantIdx),
    Element,
}

/// Where in what a `match` or `let` matches MIR's `place`, a part of it a
/// test reads, may be (`ConstantPatterns::named`): of each part of `place`
/// that may be what's matched, its type and the steps from it to `place`,
/// its derefs left out, as a pattern's are.
pub(in crate::lower) fn pattern_places<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    place: rustc_middle::mir::Place<'tcx>,
) -> Vec<(ty::Ty<'tcx>, Vec<PatternStep>)> {
    use rustc_middle::mir::ProjectionElem;
    let step = |elem: &ProjectionElem<_, _>| match *elem {
        ProjectionElem::Field(field, _) => Some(PatternStep::Field(field)),
        ProjectionElem::Downcast(_, variant) => Some(PatternStep::Variant(variant)),
        ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } | ProjectionElem::Subslice { .. } => {
            Some(PatternStep::Element)
        }
        _ => None,
    };
    let mut part = rustc_middle::mir::PlaceTy::from_ty(body.local_decls[place.local].ty);
    let mut places = Vec::new();
    for (at, elem) in place.projection.iter().enumerate() {
        places.push((part.ty, at));
        part = part.projection_ty(tcx, elem);
    }
    places.push((part.ty, place.projection.len()));
    (places.into_iter())
        .map(|(ty, at)| (ty, place.projection[at..].iter().filter_map(step).collect()))
        .collect()
}

/// A value a `match` or `let` tests (`ConstantPatterns`): what it is,
/// where in what's matched, and the `const` its pattern names it by.
type Tested<'tcx> = (PatternValue<'tcx>, Vec<PatternStep>, Option<DefId>);

/// What a `match` or `let` tests (`ConstantPatterns`): where it's
/// written, a macro's where it's called, the type of what it matches, and
/// each value it tests.
type Matched<'tcx> = (Span, ty::Ty<'tcx>, Vec<Tested<'tcx>>);

/// Each value a body's patterns test of a number, a `char` or the
/// registry's symbol, and the `const` they name it by, of each `match` and
/// `let` and where in what it matches (ADRs 0590, 0634, amended): two
/// constants of one value, `SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE`
/// and `HOST_COMPONENT`, each name it where it's written.
#[derive(Default)]
pub(in crate::lower) struct ConstantPatterns<'tcx> {
    matched: Vec<Matched<'tcx>>,
}

impl<'tcx> ConstantPatterns<'tcx> {
    /// The `const` that names `value` where the `match` or `let` written
    /// around `at`, its test's, tests it: `IMMEDIATE_PRIORITY` where each of
    /// its patterns of it there is that constant, none where one is written
    /// another way, a literal, another constant or a trait's. `places` is
    /// where in what's matched the test may be (`pattern_places`); of none
    /// given, a test of the whole value, its patterns' anywhere.
    pub(in crate::lower) fn named(
        &self,
        tcx: TyCtxt<'tcx>,
        value: &PatternValue<'tcx>,
        at: Span,
        places: Option<&[(ty::Ty<'tcx>, Vec<PatternStep>)]>,
    ) -> Option<DefId> {
        // Where the program writes it, of a macro's, `matches!`'s, its call.
        let at = at.source_callsite();
        let (_, matched, tested) = (self.matched.iter())
            .filter(|(written, ..)| written.contains(at))
            .min_by_key(|(written, ..)| written.hi() - written.lo())?;
        let alike = |a: ty::Ty<'tcx>, b: ty::Ty<'tcx>| {
            tcx.erase_and_anonymize_regions(a.peel_refs()) == tcx.erase_and_anonymize_regions(b.peel_refs())
        };
        let mut named = (tested.iter())
            .filter(|(tested, path, _)| {
                tested == value
                    && places
                        .is_none_or(|places| places.iter().any(|(ty, steps)| alike(*ty, *matched) && steps == path))
            })
            .map(|&(_, _, named)| named);
        let first = named.next()??;
        named.all(|named| named == Some(first)).then_some(first)
    }
}

/// Of each `match` and `let` of `thir`, what it tests (`ConstantPatterns`).
pub(super) fn constant_patterns<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> ConstantPatterns<'tcx> {
    let of = |span: Span, patterns: &[&Pat<'tcx>]| -> Option<Matched<'tcx>> {
        let mut tested = Vec::new();
        (patterns.iter()).for_each(|pattern| tested_constants(tcx, pattern, &mut Vec::new(), &mut tested));
        Some((span.source_callsite(), patterns.first()?.ty, tested))
    };
    let mut found = ConstantPatterns::default();
    for expr in thir.exprs.iter() {
        match expr.kind {
            ExprKind::Match { ref arms, .. } => {
                let patterns: Vec<&Pat<'tcx>> = arms.iter().map(|&arm| &*thir[arm].pattern).collect();
                found.matched.extend(of(expr.span, &patterns));
            }
            ExprKind::Let { ref pat, .. } => found.matched.extend(of(expr.span, &[pat])),
            _ => {}
        }
    }
    for stmt in thir.stmts.iter() {
        if let thir::StmtKind::Let { ref pattern, span, .. } = stmt.kind {
            found.matched.extend(of(span, &[pattern]));
        }
    }
    found
}

/// Each value `pattern`, at `at` in what's matched, tests
/// (`ConstantPatterns`), by the constant it's written as: one of no
/// generics; a trait's, `<u8 as Level>::HIGH`, is of a type the pattern
/// doesn't keep, so it's its value.
fn tested_constants<'tcx>(
    tcx: TyCtxt<'tcx>,
    pattern: &Pat<'tcx>,
    at: &mut Vec<PatternStep>,
    tested: &mut Vec<Tested<'tcx>>,
) {
    if let Some(value) = pattern_value(tcx, pattern) {
        let named = (pattern.extra.as_ref().and_then(|extra| extra.expanded_const))
            .filter(|&d| tcx.generics_of(d).count() == 0);
        // Of a `#[repr(transparent)]` struct, its field's, where MIR tests it.
        let mut path = at.clone();
        let mut leaf = pattern;
        while let Some(field) = transparent_subpattern(tcx, leaf)
            && let ty::Adt(adt, _) = leaf.ty.kind()
            && let Some(index) = crate::lower::recognition::transparent_field(tcx, *adt)
        {
            path.push(PatternStep::Field(rustc_abi::FieldIdx::from_usize(index)));
            leaf = field;
        }
        tested.push((value, path, named));
        return;
    }
    let mut each = |step: Option<PatternStep>, pattern: &Pat<'tcx>, at: &mut Vec<PatternStep>| {
        at.extend(step);
        tested_constants(tcx, pattern, at, tested);
        if step.is_some() {
            at.pop();
        }
    };
    match &pattern.kind {
        PatKind::Binding {
            subpattern: Some(subpattern),
            ..
        }
        | PatKind::Deref { subpattern, .. }
        | PatKind::Guard { subpattern, .. } => each(None, subpattern, at),
        PatKind::Or { pats } => pats.iter().for_each(|pat| each(None, pat, at)),
        PatKind::Leaf { subpatterns } => {
            (subpatterns.iter()).for_each(|field| each(Some(PatternStep::Field(field.field)), &field.pattern, at))
        }
        PatKind::Variant {
            variant_index,
            subpatterns,
            ..
        } => {
            at.push(PatternStep::Variant(*variant_index));
            (subpatterns.iter()).for_each(|field| each(Some(PatternStep::Field(field.field)), &field.pattern, at));
            at.pop();
        }
        PatKind::DerefPattern { subpattern, .. } => each(Some(PatternStep::Element), subpattern, at),
        PatKind::Slice { prefix, slice, suffix } | PatKind::Array { prefix, slice, suffix } => {
            (prefix.iter().chain(slice.as_deref()).chain(suffix.iter()))
                .for_each(|pat| each(Some(PatternStep::Element), pat, at))
        }
        _ => {}
    }
}

/// The value `pat` tests, of a number, a `char`, a string or the
/// registry's symbol, itself, not what's in it: the one answer to which value a constant
/// pattern is (`PatternValue`).
pub(in crate::lower) fn pattern_value<'tcx>(tcx: TyCtxt<'tcx>, pat: &Pat<'tcx>) -> Option<PatternValue<'tcx>> {
    match &pat.kind {
        PatKind::Constant { value } if pat.ty.is_integral() || pat.ty.is_char() => value
            .valtree
            .try_to_leaf()
            .map(|leaf| PatternValue::Bits(pat.ty, leaf.to_bits_unchecked())),
        // A `#[repr(transparent)]` struct's, which is its field in JS (ADR
        // 0584): its field's, `EventPriority(2)`'s `2`.
        PatKind::Leaf { .. } if let Some(field) = transparent_subpattern(tcx, pat) => pattern_value(tcx, field),
        // Its key's pattern, a string's, inside it.
        PatKind::Leaf { subpatterns }
            if matches!(pat.ty.kind(), ty::Adt(adt, _)
            if crate::lower::recognition::is_symbol_for_def(tcx, adt.did())) =>
        {
            match &subpatterns[..] {
                [key] => pattern_text(&key.pattern).map(PatternValue::Key),
                _ => None,
            }
        }
        // A string's, `"keypress"`, by its text.
        _ => pattern_text(pat).map(PatternValue::Text),
    }
}

/// The pattern of the field a `#[repr(transparent)]` struct's pattern is in
/// JS, as the struct is its field (ADR 0584): a constant's, `EventPriority(2)`
/// of `DISCRETE_EVENT_PRIORITY =>`, its `2`.
pub(in crate::lower) fn transparent_subpattern<'p, 'tcx>(
    tcx: TyCtxt<'tcx>,
    pat: &'p Pat<'tcx>,
) -> Option<&'p Pat<'tcx>> {
    let (PatKind::Leaf { subpatterns }, ty::Adt(adt, _)) = (&pat.kind, pat.ty.kind()) else {
        return None;
    };
    let at = crate::lower::recognition::transparent_field(tcx, *adt)?;
    (subpatterns.iter())
        .find(|field| field.field.as_usize() == at)
        .map(|field| &field.pattern)
}
