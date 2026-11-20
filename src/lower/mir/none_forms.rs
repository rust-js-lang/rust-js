//! How each of a body's `Option` locals holds its `None`, decided once
//! (#2112's Change C, its second step): whether the body makes it `null`
//! where it's made (`mir_null_nones`, ADRs 0275, 0861), and what each read
//! of it may trust (`mir_nones`, ADR 0298), answered together, so the two
//! can't disagree. They were two analyses of one local, each read alone:
//! over the corpus, 70 locals the body makes `null` were also said to hold
//! only `undefined`, each read stamped `Known::NoneUndefined`, whose test is
//! `x !== undefined`, and true of a stored `null`. Only the first
//! analysis's restrictions, that such a local is read only by a nullable
//! place or a test that reads `null` and `undefined` alike, kept the stamp
//! unread. Here a local made `null` is trusted only to hold `null`, never
//! `undefined` alone, by construction (ADR 1096, amended).

use super::*;
use crate::collections::{IndexMap, IndexSet};

/// Each `Option` local's `None`: the ones the body makes `null`, and what
/// reads may trust of the rest, and of those, `NoneNull` alone.
#[derive(Default)]
pub(super) struct NoneForms {
    made_null: IndexSet<Local>,
    reads: IndexMap<Local, js::Known>,
}

impl NoneForms {
    /// Of `made_null`, the locals the body makes `null` (`mir_null_nones`),
    /// and `reads`, the one `None` each other local's writes give
    /// (`mir_nones`): a local made `null` keeps only `NoneNull`.
    pub(super) fn of(made_null: IndexSet<Local>, mut reads: IndexMap<Local, js::Known>) -> Self {
        reads.retain(|local, none| !made_null.contains(local) || matches!(none, js::Known::NoneNull));
        NoneForms { made_null, reads }
    }

    /// Whether the body makes `local`'s `None` `null` where it's made, `c ?
    /// v : null`, its writes storing so.
    pub(super) fn made_null(&self, local: Local) -> bool {
        self.made_null.contains(&local)
    }

    /// What a read of `local`, or through a reference to it, may trust of
    /// its `None`: `Known::NoneUndefined` or `Known::NoneNull`, where every
    /// write gives that one; `None` where they may give either.
    pub(super) fn read(&self, local: Local) -> Option<js::Known> {
        self.reads.get(&local).copied()
    }
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// Whether `operand`, an `Option` written where `None` is `undefined`
    /// alone, gives no `null` as it is (#2112, Change C's third step), where
    /// the taint says it may hold one (`none_trusted`, which answers a
    /// constant, a value made `Some` of what holds no JS `null`, and a place
    /// it finds none in): a place every `null` of which is a written
    /// `js::null()`, JS's on purpose (`JsNones::holds_written_null`, ADR
    /// 0935), a local a test proved `Some` there (`proved_present`), or a
    /// prop `jsx!` omitted, Rust's `None` (`omitted_prop`). A
    /// call's write of `js::null()`, or of `?`'s `None`, Rust's own, is
    /// `one_none_returned`'s, which knows the call.
    pub(super) fn gives_no_null(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(place) = operand.place().filter(|place| place.projection.is_empty()) else {
            return false;
        };
        let (tcx, body) = (self.tcx, state.body);
        let js_nones = self.krate.nulled_params.js_nones();
        js_nones.holds_written_null(tcx, body, place)
            || self.proved_present(state, place.local)
            || self.omitted_prop(state, place.local)
    }
}

/// What MIR proves of an `Option` where an operand reads it (#2112's
/// Change C, S4): `Some` there, a test of it every path takes first; never
/// `undefined` there, though maybe `null`; or nothing. The one answer of
/// each use, asked where the use is lowered, never carried on its JS.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Presence {
    #[default]
    Unknown,
    Present,
    Defined,
}

/// What MIR proves of an argument where a call reads it, which a call of
/// each count of a function's trailing `Option`s asks of it
/// (`optional_ladder`, #2178): that it's never `undefined` there,
/// `Present` or `Defined` (`presence`); and that its `None` is `null`
/// alone (`place_none`), which JS is given as it is, never left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
// Flags, not a state: what's proved of its presence and of its `None` are
// apart, a value or `null` both.
pub(in crate::lower) struct ArgProof {
    pub(in crate::lower) present: bool,
    pub(in crate::lower) null: bool,
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// What MIR proves of `operand`, an `Option`, where it's read: of a
    /// variable a test proved (`proved_present`), what a closure's `with`
    /// lends of a thread-local `Some` wherever it runs (`present_item_read`),
    /// a capture a test in the closure proved (`capture_read_proved`), a
    /// struct's `RefCell` field read where a test of it proved it
    /// (`field_read_proved`): `Present`; of one a test proved defined
    /// (`proved_defined`): `Defined`.
    pub(in crate::lower::mir) fn presence(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> Presence {
        self.presence_of(state, (operand, state.at), 0)
    }

    /// The one `None` `place`, an `Option`, can be where it's read, of
    /// `undefined` and `null`: a local's or what it refers to, `read`'s; a
    /// closure's capture, or a borrow of one, what its maker gave it
    /// (`captured_none`); a
    /// field's, the trust its read may have (`read_none_trust`). What a test
    /// of it's made of (`Expr::presence`, #2112 C S5).
    pub(in crate::lower::mir) fn place_none(
        &self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
    ) -> Option<js::OnlyNone> {
        if let Some(none) = self.capture_none(state, place) {
            return none;
        }
        match place.projection.as_slice() {
            [] => state.facts.options().none_forms.read(place.local),
            // What a reference refers to: its writes', or, of a borrow of a
            // field, the field's trust.
            [PlaceElem::Deref] => (state.facts.options().none_forms.read(place.local))
                .or_else(|| self.read_none_trust(state.body, place, 0)),
            _ => self.read_none_trust(state.body, place, 0),
        }
        .and_then(js::OnlyNone::of)
    }

    /// What MIR proves of each of `operands`, read where the call being
    /// lowered is (`presence`), which a call of each count of a function's
    /// trailing `Option`s asks (`optional_ladder`, #1876) and what gives one
    /// JS may have given as it is (`never_null`).
    pub(in crate::lower::mir) fn presences(
        &self,
        state: &State<'_, 'tcx>,
        operands: &[&Operand<'tcx>],
    ) -> Vec<Presence> {
        operands.iter().map(|operand| self.presence(state, operand)).collect()
    }

    /// `presences` of the items of the tuple the call being lowered gives a
    /// closure, each its field.
    pub(in crate::lower::mir) fn presences_in_a_tuple(&self, state: &State<'_, 'tcx>) -> Vec<Presence> {
        let fields = self.tuple_given(state);
        self.presences(state, &fields.iter().collect::<Vec<_>>())
    }

    /// What MIR proves of each of `operands` where the call being lowered
    /// reads it (`ArgProof`).
    pub(in crate::lower::mir) fn arg_proofs(
        &self,
        state: &State<'_, 'tcx>,
        operands: &[&Operand<'tcx>],
    ) -> Vec<ArgProof> {
        (operands.iter())
            .map(|operand| ArgProof {
                present: self.presence(state, operand) != Presence::Unknown,
                null: operand.place().and_then(|place| self.place_none(state, place)) == Some(js::OnlyNone::Null),
            })
            .collect()
    }

    /// Whether the two arguments of the call being lowered, references to
    /// `Option`s, `a == b`'s, are each one whose `None` MIR says is the same
    /// one alone (`place_none`), which `===` compares (rd117).
    // helper: of the call being lowered, `trait_call_given`'s `same_none`
    pub(in crate::lower::mir) fn same_nones(&self, state: &State<'_, 'tcx>) -> bool {
        let mir::TerminatorKind::Call { args, .. } = &state.body[state.at].terminator().kind else {
            return false;
        };
        let none = |i: usize| {
            let place = args.get(i)?.node.place()?;
            let of = match place.ty(state.body, self.tcx).ty.is_ref() {
                true => place.project_deeper(&[PlaceElem::Deref], self.tcx),
                false => place,
            };
            self.place_none(state, of)
        };
        args.len() == 2 && none(0).is_some() && none(0) == none(1)
    }

    /// `arg_proofs` of the items of the tuple the call being lowered gives
    /// a closure, each its field, of `presences`, theirs
    /// (`presences_in_a_tuple`), each beside its `None`'s `null`.
    pub(in crate::lower::mir) fn arg_proofs_in_a_tuple(
        &self,
        state: &State<'_, 'tcx>,
        presences: &[Presence],
    ) -> Vec<ArgProof> {
        (presences.iter().zip(self.tuple_given(state)))
            .map(|(presence, field)| ArgProof {
                present: *presence != Presence::Unknown,
                null: field.place().and_then(|place| self.place_none(state, place)) == Some(js::OnlyNone::Null),
            })
            .collect()
    }

    /// The items of the tuple the call being lowered gives a closure, each
    /// its field.
    fn tuple_given(&self, state: &State<'_, 'tcx>) -> Vec<Operand<'tcx>> {
        let mir::TerminatorKind::Call { args, .. } = &state.body[state.at].terminator().kind else {
            return Vec::new();
        };
        let Some(tuple) = args.get(1).and_then(|tuple| tuple.node.place()) else {
            return Vec::new();
        };
        let ty::Tuple(tys) = tuple.ty(state.body, self.tcx).ty.kind() else {
            return Vec::new();
        };
        (tys.iter().enumerate())
            .map(|(i, ty)| {
                Operand::Copy(
                    tuple.project_deeper(&[PlaceElem::Field(rustc_abi::FieldIdx::from_usize(i), ty)], self.tcx),
                )
            })
            .collect()
    }

    /// What MIR proves of the `Option` a call of `func` on `args` gives
    /// `local`, where the call's lowered: of one that gives its `Option`
    /// through (`passes_through`), `js::cast(found)`'s, what's proved of it.
    pub(in crate::lower::mir) fn call_presence(
        &self,
        state: &State<'_, 'tcx>,
        local: Local,
        (func, args): (&Operand<'tcx>, &[rustc_span::Spanned<Operand<'tcx>>]),
    ) -> Presence {
        match self.passes_through(state, local, func, args) {
            Some(from) => self.presence_of(state, (&Operand::Copy(from), state.at), 0),
            None => Presence::Unknown,
        }
    }

    /// `presence`, of a temporary followed to what it copies, a few deep
    /// (`copied_from`): what's proved where it's lowered of what it copies.
    fn presence_of(
        &self,
        state: &State<'_, 'tcx>,
        (operand, block): (&Operand<'tcx>, BasicBlock),
        depth: usize,
    ) -> Presence {
        let Some(place) = operand.place() else {
            return Presence::Unknown;
        };
        if self.option_of(operand.ty(&state.body.local_decls, self.tcx)).is_none() {
            return Presence::Unknown;
        }
        // A local's whole read; a capture's or a thread-local's through
        // the place they're read by.
        let whole = place.projection.is_empty();
        if (whole && self.proved_present(state, place.local))
            || self.made_some(state, operand)
            || self.present_item_read(state, place)
            || self.capture_read_proved(state, place)
            || (whole && self.field_read_proved(state, place.local))
        {
            return Presence::Present;
        }
        if whole && self.proved_defined(state, place.local) {
            return Presence::Defined;
        }
        if depth >= 4 || !place.projection.is_empty() {
            return Presence::Unknown;
        }
        let Some((from, at)) = self.copied_from(state, (place.local, block)) else {
            return Presence::Unknown;
        };
        self.presence_of(state, (&Operand::Copy(from), at), depth + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::NoneForms;
    use crate::collections::{IndexMap, IndexSet};
    use crate::js::Known;
    use rustc_middle::mir::Local;

    // A local made `null`, which `mir_nones` says holds only `undefined`, as
    // 70 of the corpus's did: its reads trust nothing of its `None`; one it
    // says holds `null`, that; another, what `mir_nones` says.
    #[test]
    fn a_local_made_null_is_never_read_as_undefined_alone() {
        let (made, other, nulled) = (Local::from_u32(1), Local::from_u32(2), Local::from_u32(3));
        let forms = NoneForms::of(
            IndexSet::from_iter([made, nulled]),
            IndexMap::from_iter([
                (made, Known::NoneUndefined),
                (other, Known::NoneUndefined),
                (nulled, Known::NoneNull),
            ]),
        );
        assert!(forms.made_null(made) && forms.made_null(nulled) && !forms.made_null(other));
        assert_eq!(forms.read(made), None);
        assert_eq!(forms.read(other), Some(Known::NoneUndefined));
        assert_eq!(forms.read(nulled), Some(Known::NoneNull));
    }
}
