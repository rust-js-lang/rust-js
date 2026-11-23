//! What lowering a body from its MIR notes of the item for what comes
//! after it, tidy and the lowering of the bodies a closure's lowered
//! inside: which locals read alike and are plain data, each closure's
//! forwarding, the literals patterns name, what's tested and falsy, the
//! ranges no step leaves exhausted and each in-place collect's source; and
//! what's asked of them back.

use super::*;
use rustc_middle::thir;

mod made;
pub use made::{Handled, Made};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What the handle `place` is the `value` of holds, of one a variable
    /// holds of a `&mut` to a place that stores its own way, `place.value`
    /// of `const place = { get value() .. }` on `fiber.owner` (#541, #1018);
    /// `None` of any other place.
    pub fn held_handle(&self, place: &Expr) -> Option<&Handled> {
        match &place.kind {
            js::ExprKind::Member(of, key) if key == js::HELD => match &of.kind {
                js::ExprKind::Var(name) => self.locals.mir.made.handles.get(name),
                _ => None,
            },
            _ => None,
        }
    }

    /// `handle`, a handle on a `&mut`'s place, held in a `const` of its
    /// own named for `base`, that variable: of one that stores its own way,
    /// noted (`Made::note_handle`), so what's lent of its `value` is that
    /// handle, a nullable place's (#541) or a flattened state's lent,
    /// `task.state.get_mut()`'s (#2429).
    pub fn hold_handle<'b>(
        &mut self,
        base: impl Into<crate::temporary_names::Base<'b>>,
        handle: Expr,
        out: &mut Vec<js::Stmt>,
    ) -> Expr {
        let made = handle.clone();
        let held = self.spill(base, handle, out);
        if let js::ExprKind::Var(name) = &held.kind {
            self.locals.mir.made.note_handle(name, &made);
        }
        held
    }

    /// Whether `name`, a local of the body lowered from its MIR, reads the
    /// same wherever it's read: assigned once and never borrowed.
    pub fn alike_local(&self, name: &js::Name) -> bool {
        self.locals.mir.alike.contains(name)
    }

    /// Whether `ty` is plain Rust data, which nothing changes but an
    /// assignment of what holds it: no `&mut`, `Cell`, JS object or counted
    /// `Rc` in it (ADR 0320).
    pub fn plain_type(&self, ty: Ty<'tcx>) -> bool {
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
    pub fn plain_local(&self, name: &js::Name) -> bool {
        self.locals.mir.plain.contains(&js::Path::var(name))
    }

    /// Of each closure of one call the body lowered from its MIR makes so
    /// far, where it's written and what it passes its call on to, which
    /// `passed_on` reads (ADR 0605, amended).
    pub fn forwarding(&self) -> &[crate::passed_on::Forwarding] {
        &self.locals.mir.forwarding
    }

    /// A literal a pattern names by a `const`, which tidy names once it's
    /// done (`named_constants`, ADR 0590, amended).
    pub(crate) fn note_named_text(&mut self, named: crate::named_constants::NamedText) {
        self.locals.mir.named_texts.push(named);
    }

    /// Each literal a pattern names by a `const`, as `note_named_text` was
    /// told, of the body and the closures it's made of so far: each of its
    /// own place, which none of another's is.
    pub fn named_texts(&self) -> Vec<crate::named_constants::NamedText> {
        self.locals.mir.named_texts.clone()
    }

    /// `name`, a temporary a `test && value` given what skips what's falsy
    /// tests: read for its truth alone.
    pub(crate) fn note_tested(&mut self, name: &js::Name) {
        self.locals.mir.tested.insert(name.clone());
    }

    /// `name`, a temporary given to what skips what's falsy, whose `test ?
    /// value : undefined` is given as `test && value` where it's moved to.
    pub(crate) fn note_falsy(&mut self, name: &js::Name) {
        self.locals.mir.falsy.insert(name.clone());
    }

    /// A body's inclusive ranges no step can leave exhausted, by name
    /// (`fresh_ranges`, #418).
    pub fn note_fresh_ranges(&mut self, names: Vec<js::Name>) {
        self.locals.mir.fresh_ranges.extend(names);
    }

    /// Each in-place collect's source `Vec`'s variable, by the call's span
    /// (`in_place_sources`, #1259).
    pub fn note_in_place_sources(&mut self, names: rust_js_program::collections::IndexMap<rustc_span::Span, js::Name>) {
        self.locals.mir.in_place_sources.extend(names);
    }

    /// The `Vec` the `collect()` at `span` is made of, where std may make it
    /// in place: its variable, or the place in it the chain moved, its
    /// fields projected as the body reads them, whose capacity is the
    /// collect's buffer's (`in_place_sources`, #1259).
    pub(crate) fn in_place_source(&self, span: rustc_span::Span) -> Option<Expr> {
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
    pub(crate) fn collect_sources_apart(&self, span: rustc_span::Span) -> bool {
        self.collect_sources_at(span).next().is_some() && !self.locals.mir.in_place_sources.contains_key(&span)
    }

    /// The source of each THIR `collect()` at `span` that has one.
    fn collect_sources_at(
        &self,
        span: rustc_span::Span,
    ) -> impl Iterator<Item = &crate::body_queries::CollectSource<'tcx>> {
        let thir = self.frame.thir;
        (thir.exprs.iter_enumerated())
            .filter(move |(_, e)| matches!(e.kind, thir::ExprKind::Call { fn_span, .. } if fn_span == span))
            .filter_map(|(id, _)| self.frame.facts.in_place_collect_sources.get(&id))
    }

    /// Whether `range` is a variable no step can leave exhausted, a `let`
    /// not `mut` of `a..=b` (`fresh_ranges`, #418).
    pub(crate) fn never_exhausted(&self, range: &Expr) -> bool {
        matches!(&range.kind, js::ExprKind::Var(name) if self.locals.mir.fresh_ranges.contains(name))
    }
}

/// What lowering a body from its MIR found of its variables, which goes
/// with them where a body inside it is lowered (`Locals`): each read and
/// written here alone, and asked of by the rest, `alike_local`.
#[derive(Default)]
pub struct Found {
    /// Of a body lowered from its MIR (ADR 0364): the names of its locals
    /// that read the same wherever they're read, assigned once and never
    /// borrowed, and of those of plain Rust data, whose fields don't change
    /// either, which `reads_alike` and `plain_field` ask of.
    /// And of the others' arrays, their items, `...items`.
    pub alike: IndexSet<js::Name>,
    pub plain: IndexSet<js::Path>,
    /// Of those of plain data, each path to what they hold whose fields a
    /// getter may answer, each read anew, `p.style` (`given_paths`, #1908).
    pub given: IndexSet<js::Path>,
    /// And those of plain Rust data written again, a `mut` parameter
    /// assigned after what it's read for: each the same until it's written
    /// (`written_before_read`, #121).
    pub settled: IndexSet<js::Name>,
    /// And the places that hold its cells, `&mut`s to what JS can't change
    /// in place: a box or a handle, whose `value` is what it points at,
    /// unlike a struct's field of that name. Its locals', by name, and a
    /// place a reborrow found one at, `o.value` of `&mut **u`.
    pub cells: Vec<Expr>,
    /// Of a body lowered from its MIR, its inclusive ranges no step can
    /// leave exhausted, by name: each a `let` not `mut`, so never lent as a
    /// `&mut`, made only by `a..=b`, which `range_state` reads as it is
    /// (#418).
    pub(crate) fresh_ranges: IndexSet<js::Name>,
    /// Of a body lowered from its MIR, the `Vec` each `collect()` std may
    /// make in place is made of, by the call's span: its variable's name,
    /// `in_place_collect_sources`'s (#1259).
    pub(crate) in_place_sources: rust_js_program::collections::IndexMap<rustc_span::Span, js::Name>,
    /// Of a body lowered from its MIR, the temporaries a function that skips
    /// what's falsy is given, whose `test ? value : undefined` is given as
    /// `test && value` (`#[rust_js::skips_falsy]`) where it's moved to.
    pub falsy: IndexSet<js::Name>,
    /// And the temporaries such a `test && value` tests: read for their
    /// truth alone.
    pub tested: IndexSet<js::Name>,
    /// Of a body lowered from its MIR, the `const`s it made that tidy
    /// reads in place where it can, as its temporaries.
    pub made: Made,
    /// Each name `spill_subject` gave a subject after its call, `nth`: another
    /// of it is `nth$1`, which tidy writes `nth` where their scopes are apart.
    pub(crate) subjects: IndexSet<String>,

    /// Of a body lowered from its MIR, each literal its patterns name by
    /// a `const`, named once tidy's done (`FnCx::named_text`, ADR 0590,
    /// amended).
    pub(crate) named_texts: Vec<crate::named_constants::NamedText>,
    /// Of a body lowered from its MIR, each dictionary it made of a type
    /// known whole that holds a function, and the name it would have: one
    /// written twice or more is declared once at the body's top and named
    /// where it was (ADR 0979).
    pub dictionaries: Vec<(String, Expr)>,
    /// Whether a body from its MIR is being lowered, whose top they go at.
    pub declaring: bool,
    /// Of a body lowered from its MIR, each closure it makes of one call:
    /// where it's written, and what it passes its call on to, which says
    /// where it may be that function (ADR 0605, amended).
    pub forwarding: Vec<crate::passed_on::Forwarding>,
    /// And where each is written of one call whose JS gives `undefined`
    /// anyway, and how: a handler's, or one of its value, `() => f()` (ADRs
    /// 0040, 0883).
    pub concise: Vec<(js::Span, crate::jsx::OneCall)>,
    /// Of a body lowered from its MIR, each field path it reads,
    /// `element.editable`, and whether it's a `bool` each time.
    pub paths: IndexMap<js::Path, bool>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Keep `made`, a dictionary of a type known whole, of the body being
    /// lowered from its MIR, as `name`, or `shared` where another type's is
    /// the same: one it writes twice or more is declared once at its top
    /// (`declare_repeated`, ADR 0979). None of another body.
    pub(super) fn keep_dictionary(&mut self, made: &Expr, name: String, shared: String) {
        if !self.locals.mir.declaring {
            return;
        }
        match (self.locals.mir.dictionaries.iter_mut()).find(|(_, kept)| js::scopes::alpha_eq(kept, made)) {
            Some((kept, _)) if *kept != name => *kept = shared,
            Some(_) => {}
            None => self.locals.mir.dictionaries.push((name, made.clone())),
        }
    }

    /// Of `name`, a variable, each path to what it holds whose fields a
    /// getter may answer (`given_paths`).
    pub(crate) fn given_of(&self, name: &js::Name) -> rust_js_program::collections::IndexSet<js::Path> {
        let mut found: rust_js_program::collections::IndexSet<js::Path> = (self.locals.mir.given.iter())
            .filter(|path| matches!(&path.root, js::PathRoot::Var(root) if root == name))
            .cloned()
            .collect();
        for (id, var) in &self.locals.vars {
            if matches!(&var.place.kind, js::ExprKind::Var(n) if n == name) {
                let ty = self.tcx.typeck(id.0.owner.def_id).node_type(id.0);
                self.given_paths(js::Path::var(name), ty, &mut Default::default(), &mut found);
            }
        }
        found
    }

    /// Of a value of `ty` at `path`, a variable's name, each path to what
    /// it is or holds whose fields a getter may answer, a struct JS may give
    /// or make (`fields_read_once`, #1908): `p.style` of a component's
    /// props `p`, whose own fields are plain. `seen`, the structs on the
    /// path, ends a recursive type's walk.
    pub fn given_paths(
        &self,
        path: js::Path,
        ty: Ty<'tcx>,
        seen: &mut IndexSet<rustc_hir::def_id::DefId>,
        found: &mut IndexSet<js::Path>,
    ) {
        let ty = ty.peel_refs();
        if self.fields_read_once(ty) {
            found.insert(path);
            return;
        }
        if let ty::Adt(adt, _) = *ty.kind()
            && adt.is_struct()
            && seen.insert(adt.did())
            && let Shape::Object(fields) = self.shape(ty)
        {
            for (field, ty) in fields {
                if let Some(name) = field.name() {
                    self.given_paths(path.clone().field(name), ty, seen, found);
                }
            }
            seen.swap_remove(&adt.did());
        }
    }
}
