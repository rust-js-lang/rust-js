//! What a function takes after its own parameters, and in which order: the
//! one list its body names (`evidence_params`) and each call gives
//! (`evidence_args_with`, `method_evidence`, `own_evidence`), each slot of
//! it given by that consumer's own rule of its value.

use super::*;
use rustc_span::def_id::DefId;

/// One of what a function takes after its own parameters.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Slot<'tcx> {
    /// Where it's called, of one that tracks its caller (ADR 0427).
    Location,
    /// A const parameter's value, `N`, by its index (ADR 0107).
    Const(u32),
    /// A bound's dictionary, `IIterator`, of the function's own terms (ADR
    /// 0049).
    Bound(ty::TraitRef<'tcx>),
    /// A fact it asks of a type parameter, `TSize`, by its slot (ADR 0145).
    Fact(u32, TypeFact),
    /// A drop function of a type parameter or an associated type, `dropT`,
    /// by its slot (ADR 0098): given last, so one with nothing to drop is
    /// left out at the end.
    Drop(u32),
}

/// `{ entry: helper }`, a default dictionary (`default_dictionary`).
pub(in crate::lower) fn plain_dictionary((entry, helper): (&str, &'static str)) -> Expr {
    Expr::object(vec![js::Prop::Field(entry.to_string(), Expr::helper(helper))])
}

/// Whether `e` is `plain_dictionary(default)` as written.
pub(in crate::lower) fn is_plain_dictionary(e: &Expr, (entry, helper): (&str, &str)) -> bool {
    matches!(&e.kind, js::ExprKind::Object(props, _)
        if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name() == Some(helper)))
}

/// Whose evidence: the whole function's, or a trait's generic method's own,
/// given through a dictionary, whose impl's and trait's the dictionary has
/// (ADR 0106).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum Scope {
    Whole,
    Own,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The dictionary `id`, a std body compiled into `@rust-js/std`, takes
    /// for its bound `tr` where it's given none: a JS iterator's own, `{
    /// next: $next }` of an `Iterator` bound, `{ next_back: $nextBack }` of a
    /// `DoubleEndedIterator` one, as its entry and its helper. A call whose
    /// last argument is it leaves it out (ADR 0995): the one rule, of its
    /// parameter and of its calls.
    pub(in crate::lower) fn default_dictionary(&self, id: DefId, tr: DefId) -> Option<(&'static str, &'static str)> {
        if !crate::lower::recognition::is_compiled_std(self.tcx, id) {
            return None;
        }
        if is_std_def(self.tcx, tr, StdItem::Iterator) {
            Some(("next", "$next"))
        } else if self.recognition().double_ended_iterator() == Some(tr) {
            Some(("next_back", "$nextBack"))
        } else {
            None
        }
    }

    /// What `id` takes after its own parameters, of `scope`, in order: its
    /// caller's location, its const parameters, its bounds' dictionaries,
    /// the facts it asks, then its drops. The one order.
    pub(in crate::lower) fn evidence_slots(&self, id: DefId, scope: Scope) -> Vec<Slot<'tcx>> {
        let tcx = self.tcx;
        let mut slots = Vec::new();
        if self.takes_caller_location(id) {
            slots.push(Slot::Location);
        }
        let consts = match scope {
            Scope::Whole => const_params(tcx, id).into_iter().map(|param| param.index).collect(),
            Scope::Own => own_const_params(tcx, id),
        };
        slots.extend(consts.into_iter().map(Slot::Const));
        let bounds = match scope {
            Scope::Whole => bounds(tcx, self.krate.foreign, self.krate.given, id),
            Scope::Own => own_bounds(tcx, self.krate.foreign, self.krate.given, id),
        };
        slots.extend(bounds.into_iter().map(Slot::Bound));
        // Of a method's own, what it asks of its own type parameters (ADR
        // 0466).
        let parent = tcx.generics_of(id).parent_count as u32;
        let facts = self.facts_asked(id).into_iter();
        slots.extend(
            facts
                .filter(|&(index, _)| scope == Scope::Whole || index >= parent)
                .map(|(i, f)| Slot::Fact(i, f)),
        );
        // Of a method's own, a drop of each of its own type parameters the
        // trait declares one for (ADR 0163).
        let drops = match scope {
            Scope::Whole => self.drops_taken(id),
            Scope::Own => own_drop_params(tcx, self.krate.foreign, id),
        };
        slots.extend(drops.into_iter().map(Slot::Drop));
        slots
    }
}
