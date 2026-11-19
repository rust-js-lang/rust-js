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
