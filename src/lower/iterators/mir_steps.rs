//! The iterator half of MIR's iterator calls (#2299, #2294 step 4, commit
//! 1): collecting, consuming, closing, copying and the sources an adapter
//! steps, each `FnCx`'s, which `mir::iter`, bound to `mir`'s `State` and
//! `Value`, asks. Moved from `iterators/mir_steps/` beneath `mir`, so the lowering's
//! base names nothing of MIR's for them.

mod backs;
pub(in crate::lower) use backs::reversed_in_body;
mod closing;
mod collect;
pub(in crate::lower) mod consumers;
mod copies;
pub(in crate::lower) mod sources;

use self::sources::*;
use crate::lower::recognition::{OwnStep, Reach};
use crate::lower::temporary_names::Role;

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::iterators::{collected, is_js_iterator, made_array, stepped_range};
use crate::lower::recognition::{IterMethod, StdItem, std_item};
use crate::lower::representation::Num;
use crate::lower::std_types::heap::HeapOp;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R, bindings};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a call of `def_id` at `span` is a `rev()`, an `enumerate()`
    /// or a `map()` whose adapter a std iterator's method steps where it's
    /// made, `body_queries::in_place_adapters` says: what the recognizers
    /// make of it, not std's compiled one (ADR 0871). It records only the
    /// calls of `HELD_COMPILED`'s makers of a type compiled for this build
    /// (`makes_held_compiled`), each its THIR call, asked of each at the
    /// span (`each_call_at`, #1671 change 3 M5).
    // helper: asked by `mir::iter` alone, the entry it was beneath before #2299 moved it
    pub(in crate::lower) fn adapter_in_place(&self, _def_id: rustc_span::def_id::DefId, span: Span) -> bool {
        self.frame.facts.in_place_adapters.made_at(self.frame.thir, span)
    }
}
