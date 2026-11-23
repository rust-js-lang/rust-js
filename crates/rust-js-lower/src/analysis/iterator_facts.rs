//! The crate's iterator facts, crate-wide, each a question of the whole
//! crate that makes an adapter in the form that knows its answer where it's
//! made: what it steps from its back (`back_stepped.rs`, ADR 0545) and
//! what generic code asks the hint of (`hint_asked.rs`, #1236). One value
//! the lowering is given, as its `iterators`.

pub(crate) mod typed_key;

use super::back_stepped::{self, BackStepped};
use super::hint_asked::{self, Hinted};
use crate::Body;
use crate::generic_facts::GivenADrop;
use crate::recognition::Recognition;
use rustc_middle::ty::{Ty, TyCtxt};

pub struct IteratorFacts<'tcx> {
    /// The map and filter types the crate steps from their back, each made
    /// knowing its back (ADR 0545).
    pub(crate) back_stepped: BackStepped<'tcx>,
    /// The `take(n)`s, `skip(n)`s and `step_by(n)`s whose hint generic
    /// code asks of what its callers give, each made `$taking`, `$skipping`
    /// or `$stepBy` where it's made (`hint_asked.rs`, #1236).
    pub hinted: Hinted<'tcx>,
}

/// Of `all_bodies`, what they step from their back (ADR 0545, #814, #851),
/// of an array's items with a destructor, `drops`, which its native forms
/// don't step; and what generic code asks the hint of, made knowing it
/// where it's made, never stepped from its back for it (#1236, #2209).
pub(super) fn of_crate<'tcx>(
    (tcx, recognition): (TyCtxt<'tcx>, Recognition<'_, 'tcx>),
    (all_bodies, names): (&[&Body<'tcx>], &crate::call_graph::Names<'tcx>),
    (library, drops, keeps_capacity): (bool, impl Fn(Ty<'tcx>) -> bool + Copy, bool),
    given: (bool, Option<&GivenADrop<'_>>),
) -> IteratorFacts<'tcx> {
    let back_stepped = back_stepped::of_crate(tcx, recognition, all_bodies, (library, drops, keeps_capacity), given);
    let hinted = hint_asked::of_crate(tcx, recognition, (all_bodies, names), keeps_capacity);
    IteratorFacts { back_stepped, hinted }
}
