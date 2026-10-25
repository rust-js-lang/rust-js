//! What's before a `switch`'s case (ADR 0629): `// Fallthrough` where the
//! one before falls through to it (ADR 0905), and the comments written
//! before the `match` arm it's of, before its labels (ADR 0905, amended).

use super::*;

/// What a case that falls through to the next says, before it, as React
/// writes it, and ESLint's `no-fallthrough` asks (ADR 0905).
pub(super) const FALLTHROUGH: &str = "// Fallthrough";

/// Where each case a case falls through to starts, of `module`'s
/// `switch`es: its first value's, which the case is printed at
/// (`fallen_to`), its comment `FALLTHROUGH` before it.
pub(super) fn fallthrough_starts(module: &Module) -> Vec<u32> {
    let mut starts = Vec::new();
    each_switch(module, &mut |cases| {
        starts.extend((0..cases.len()).filter_map(|k| fallen_to(cases, k)));
    });
    starts
}

/// Where the case `cases[k]` is printed, where the one before falls
/// through to it: its first value's start, `FALLTHROUGH` before it.
pub(crate) fn fallen_to(cases: &[js::Case], k: usize) -> Option<u32> {
    let before = cases.get(k.checked_sub(1)?)?;
    let first = cases.get(k)?.values.first()?;
    (before.falls && !first.span.is_none()).then_some(first.span.lo)
}

/// Where each case the one before falls through to starts, in order: a
/// start a comment right before the arm it's of is before, only its
/// pattern's other constants between, with its `FALLTHROUGH`, as React
/// writes one (ADR 0905, amended). Another case's is its body's, as the
/// scheduler's are (ADR 0590).
pub(super) fn case_starts(module: &Module) -> Vec<u32> {
    let mut starts = fallthrough_starts(module);
    starts.sort_unstable();
    starts
}

/// Each `switch` of `module`'s bodies, its cases.
fn each_switch(module: &Module, f: &mut dyn FnMut(&[js::Case])) {
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Switch { cases, .. } = &stmt.kind {
                    f(cases);
                }
            }
        });
    }
}
