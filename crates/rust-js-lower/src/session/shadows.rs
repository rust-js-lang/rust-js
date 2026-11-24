//! The shadows a compile records, `--shadow <kind>[,<kind>]=<file>`: of
//! each kind asked, a line for each verdict its analysis gives beside the
//! one the compile takes, no output changed. One flag and one cache for
//! every kind; the driver writes the file, each line `<kind>\t<line>`, the
//! kinds in the order `ShadowKind::ALL` gives.

use std::cell::RefCell;

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::ty::TyCtxt;

/// What a shadow is of.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShadowKind {
    /// Each check MIR's intervals prove, and the effects query's verdicts
    /// beside the walk's forgetting rules (#1672, #2112 B).
    Intervals,
    /// Each `Drop` of the crate's bodies compared with rustc's elaboration
    /// (#2112's change D).
    Drops,
}

impl ShadowKind {
    /// Every kind, in the order the file says them.
    pub const ALL: [ShadowKind; 2] = [ShadowKind::Intervals, ShadowKind::Drops];

    /// The name `--shadow` takes it by, and its lines begin with.
    pub fn name(self) -> &'static str {
        match self {
            ShadowKind::Intervals => "intervals",
            ShadowKind::Drops => "drops",
        }
    }

    /// The kind `name` names.
    pub fn named(name: &str) -> Option<ShadowKind> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// The lines of each kind a session records, each once, in the order first
/// given.
#[derive(Default)]
pub struct Shadows {
    lines: RefCell<IndexMap<ShadowKind, IndexSet<String>>>,
}

impl Shadows {
    /// Recording `kinds`.
    pub(crate) fn of(kinds: &[ShadowKind]) -> Option<Shadows> {
        (!kinds.is_empty()).then(|| Shadows {
            lines: RefCell::new(kinds.iter().map(|&kind| (kind, IndexSet::default())).collect()),
        })
    }
}

/// Whether the session `tcx` is records `kind`'s shadow.
pub fn shadowing(tcx: TyCtxt<'_>, kind: ShadowKind) -> bool {
    (super::caches(tcx).shadows.as_ref()).is_some_and(|shadows| shadows.lines.borrow().contains_key(&kind))
}

/// A line of `kind`'s shadow, where the session records it: `line`, made
/// only then.
pub fn shadow(tcx: TyCtxt<'_>, kind: ShadowKind, line: impl FnOnce() -> String) {
    let caches = super::caches(tcx);
    let Some(shadows) = &caches.shadows else { return };
    if let Some(lines) = shadows.lines.borrow_mut().get_mut(&kind) {
        lines.insert(line());
    }
}

/// What `--shadow` writes: each line of each kind recorded, `<kind>\t<line>`.
pub fn shadow_report(tcx: TyCtxt<'_>) -> String {
    let caches = super::caches(tcx);
    let Some(shadows) = &caches.shadows else {
        return String::new();
    };
    let lines = shadows.lines.borrow();
    let mut report = String::new();
    for kind in ShadowKind::ALL {
        for line in lines.get(&kind).into_iter().flatten() {
            report.push_str(kind.name());
            report.push('\t');
            report.push_str(line.trim_end_matches('\n'));
            report.push('\n');
        }
    }
    report
}
