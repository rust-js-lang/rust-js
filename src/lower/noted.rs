//! What lowering a body from its MIR notes of the item for what comes
//! after it, tidy and the lowering of the bodies a closure's lowered
//! inside: which locals read alike and are plain data, each closure's
//! forwarding, the literals patterns name, what's tested and falsy, the
//! ranges no step leaves exhausted and each in-place collect's source; and
//! what's asked of them back.

use super::*;
use rustc_middle::thir;

mod made;
pub(in crate::lower) use made::Made;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `name`, a local of the body lowered from its MIR, reads the
    /// same wherever it's read: assigned once and never borrowed.
    pub(in crate::lower) fn alike_local(&self, name: &js::Name) -> bool {
        self.locals.mir.alike.contains(name)
    }

    /// Whether `ty` is plain Rust data, which nothing changes but an
    /// assignment of what holds it: no `&mut`, `Cell`, JS object or counted
    /// `Rc` in it (ADR 0320).
    pub(super) fn plain_type(&self, ty: Ty<'tcx>) -> bool {
        ty.walk().all(|arg| match arg.kind() {
            ty::GenericArgKind::Type(t) => {
                !self.refers_mutably(t)
                    // Inside a binder, a function pointer's parameter, unknown.
                    && !t.has_escaping_bound_vars()
                    && t.is_freeze(self.tcx, self.typing_env)
                    && !self.is_js_object(t)
                    && self.counted_rc(t).is_none()
                    && self.counted_weak(t).is_none()
            }
            _ => true,
        })
    }

    /// Whether `name`, such a local, is plain Rust data, whose fields don't
    /// change either.
    pub(in crate::lower) fn plain_local(&self, name: &js::Name) -> bool {
        self.locals.mir.plain.contains(&js::Path::var(name))
    }

    /// Of each closure of one call the body lowered from its MIR makes so
    /// far, where it's written and what it passes its call on to, which
    /// `passed_on` reads (ADR 0605, amended).
    pub(in crate::lower) fn forwarding(&self) -> &[crate::lower::passed_on::Forwarding] {
        &self.locals.mir.forwarding
    }

    /// A literal a pattern names by a `const`, which tidy names once it's
    /// done (`named_constants`, ADR 0590, amended).
    pub(in crate::lower) fn note_named_text(&mut self, named: crate::lower::named_constants::NamedText) {
        self.locals.mir.named_texts.push(named);
    }

    /// Each literal a pattern names by a `const`, as `note_named_text` was
    /// told, of the body and the closures it's made of so far: each of its
    /// own place, which none of another's is.
    pub(in crate::lower) fn named_texts(&self) -> Vec<crate::lower::named_constants::NamedText> {
        self.locals.mir.named_texts.clone()
    }

    /// `name`, a temporary a `test && value` given what skips what's falsy
    /// tests: read for its truth alone.
    pub(in crate::lower) fn note_tested(&mut self, name: &js::Name) {
        self.locals.mir.tested.insert(name.clone());
    }

    /// `name`, a temporary given to what skips what's falsy, whose `test ?
    /// value : undefined` is given as `test && value` where it's moved to.
    pub(in crate::lower) fn note_falsy(&mut self, name: &js::Name) {
        self.locals.mir.falsy.insert(name.clone());
    }

    /// A body's inclusive ranges no step can leave exhausted, by name
    /// (`fresh_ranges`, #418).
    pub(in crate::lower) fn note_fresh_ranges(&mut self, names: Vec<js::Name>) {
        self.locals.mir.fresh_ranges.extend(names);
    }

    /// Each in-place collect's source `Vec`'s variable, by the call's span
    /// (`in_place_sources`, #1259).
    pub(in crate::lower) fn note_in_place_sources(
        &mut self,
        names: rust_js_program::collections::IndexMap<rustc_span::Span, js::Name>,
    ) {
        self.locals.mir.in_place_sources.extend(names);
    }

    /// The `Vec` the `collect()` at `span` is made of, where std may make it
    /// in place: its variable, or the place in it the chain moved, its
    /// fields projected as the body reads them, whose capacity is the
    /// collect's buffer's (`in_place_sources`, #1259).
    pub(in crate::lower) fn in_place_source(&self, span: rustc_span::Span) -> Option<Expr> {
        let name = self.locals.mir.in_place_sources.get(&span)?;
        let source = self.collect_sources_at(span).next()?;
        let place = (source.fields.iter()).fold(Expr::var(name), |base, &(ty, i)| self.project(base, ty, i));
        Some(place)
    }

    /// Whether the `collect()`s at `span`, two a macro's repetition writes
    /// there, are of sources apart, so none is the call's
    /// (`in_place_source_names`, ADR 1151): its capacity, where it's read,
    /// is refused, naming #1789.
    // helper: MIR state the base's `iterators::mir_steps::collect` asks, a hook at #2299's commit 5
    pub(in crate::lower) fn collect_sources_apart(&self, span: rustc_span::Span) -> bool {
        self.collect_sources_at(span).next().is_some() && !self.locals.mir.in_place_sources.contains_key(&span)
    }

    /// The source of each THIR `collect()` at `span` that has one.
    fn collect_sources_at(
        &self,
        span: rustc_span::Span,
    ) -> impl Iterator<Item = &crate::lower::body_queries::CollectSource<'tcx>> {
        let thir = self.frame.thir;
        (thir.exprs.iter_enumerated())
            .filter(move |(_, e)| matches!(e.kind, thir::ExprKind::Call { fn_span, .. } if fn_span == span))
            .filter_map(|(id, _)| self.frame.facts.in_place_collect_sources.get(&id))
    }

    /// Whether `range` is a variable no step can leave exhausted, a `let`
    /// not `mut` of `a..=b` (`fresh_ranges`, #418).
    pub(in crate::lower) fn never_exhausted(&self, range: &Expr) -> bool {
        matches!(&range.kind, js::ExprKind::Var(name) if self.locals.mir.fresh_ranges.contains(name))
    }
}

/// What lowering a body from its MIR found of its variables, which goes
/// with them where a body inside it is lowered (`Locals`): each read and
/// written here alone, and asked of by the rest, `alike_local`.
#[derive(Default)]
pub(in crate::lower) struct Found {
    /// Of a body lowered from its MIR (ADR 0364): the names of its locals
    /// that read the same wherever they're read, assigned once and never
    /// borrowed, and of those of plain Rust data, whose fields don't change
    /// either, which `reads_alike` and `plain_field` ask of.
    /// And of the others' arrays, their items, `...items`.
    pub(in crate::lower) alike: IndexSet<js::Name>,
    pub(in crate::lower) plain: IndexSet<js::Path>,
    /// Of those of plain data, each path to what they hold whose fields a
    /// getter may answer, each read anew, `p.style` (`given_paths`, #1908).
    pub(in crate::lower) given: IndexSet<js::Path>,
    /// And those of plain Rust data written again, a `mut` parameter
    /// assigned after what it's read for: each the same until it's written
    /// (`written_before_read`, #121).
    pub(in crate::lower) settled: IndexSet<js::Name>,
    /// And the places that hold its cells, `&mut`s to what JS can't change
    /// in place: a box or a handle, whose `value` is what it points at,
    /// unlike a struct's field of that name. Its locals', by name, and a
    /// place a reborrow found one at, `o.value` of `&mut **u`.
    pub(in crate::lower) cells: Vec<Expr>,
    /// Of a body lowered from its MIR, its inclusive ranges no step can
    /// leave exhausted, by name: each a `let` not `mut`, so never lent as a
    /// `&mut`, made only by `a..=b`, which `range_state` reads as it is
    /// (#418).
    pub(in crate::lower) fresh_ranges: IndexSet<js::Name>,
    /// Of a body lowered from its MIR, the `Vec` each `collect()` std may
    /// make in place is made of, by the call's span: its variable's name,
    /// `in_place_collect_sources`'s (#1259).
    pub(in crate::lower) in_place_sources: rust_js_program::collections::IndexMap<rustc_span::Span, js::Name>,
    /// Of a body lowered from its MIR, the temporaries a function that skips
    /// what's falsy is given, whose `test ? value : undefined` is given as
    /// `test && value` (`#[rust_js::skips_falsy]`) where it's moved to.
    pub(in crate::lower) falsy: IndexSet<js::Name>,
    /// And the temporaries such a `test && value` tests: read for their
    /// truth alone.
    pub(in crate::lower) tested: IndexSet<js::Name>,
    /// Of a body lowered from its MIR, the `const`s it made that tidy
    /// reads in place where it can, as its temporaries.
    pub(in crate::lower) made: Made,
    /// Each name `spill_subject` gave a subject after its call, `nth`: another
    /// of it is `nth$1`, which tidy writes `nth` where their scopes are apart.
    pub(in crate::lower) subjects: IndexSet<String>,

    /// Of a body lowered from its MIR, each literal its patterns name by
    /// a `const`, named once tidy's done (`FnCx::named_text`, ADR 0590,
    /// amended).
    pub(in crate::lower) named_texts: Vec<crate::lower::named_constants::NamedText>,
    /// Of a body lowered from its MIR, each dictionary it made of a type
    /// known whole that holds a function, and the name it would have: one
    /// written twice or more is declared once at the body's top and named
    /// where it was (ADR 0979).
    pub(in crate::lower) dictionaries: Vec<(String, Expr)>,
    /// Whether a body from its MIR is being lowered, whose top they go at.
    pub(in crate::lower) declaring: bool,
    /// Of a body lowered from its MIR, each closure it makes of one call:
    /// where it's written, and what it passes its call on to, which says
    /// where it may be that function (ADR 0605, amended).
    pub(in crate::lower) forwarding: Vec<crate::lower::passed_on::Forwarding>,
    /// And where each is written of one call whose JS gives `undefined`
    /// anyway, and how: a handler's, or one of its value, `() => f()` (ADRs
    /// 0040, 0883).
    pub(in crate::lower) concise: Vec<(js::Span, crate::lower::jsx::OneCall)>,
    /// Of a body lowered from its MIR, each field path it reads,
    /// `element.editable`, and whether it's a `bool` each time.
    pub(in crate::lower) paths: IndexMap<js::Path, bool>,
}
