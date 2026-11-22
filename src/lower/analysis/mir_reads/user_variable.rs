//! Whether a body's local is a variable the program named, the one way it's
//! asked (#1281), the base's since #2299 moved it from `mir::locals`.
use rustc_middle::mir::{self};

/// Whether `decl` is a variable the program named: none of a body whose
/// locals' kinds rustc cleared, a constant's it evaluated, a `const fn`
/// given a const generic's argument among them (#1281), or another crate's,
/// whose `LocalDecl::is_user_variable` would panic. The one way a body's
/// local is asked so (#1281).
pub(in crate::lower) fn user_variable(decl: &mir::LocalDecl<'_>) -> bool {
    matches!(decl.local_info, mir::ClearCrossCrate::Set(_)) && decl.is_user_variable()
}
