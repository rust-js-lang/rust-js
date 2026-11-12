//! The recognisers a compile leaves out, `--without-recognizer`'s (ADR
//! 1128): a std function named there is one no recogniser takes, in every
//! body and every analysis alike, as `classify` is each one's entry. What's
//! left is what rust-js does of a std function it has no model of: its
//! body compiled from std's, a trait's dictionary's, or a refusal. A test
//! hook, as `--without-known` is: a build names none.

use super::*;

/// `known`, what `id` was recognised as, or none of one whose recogniser
/// the session leaves out; recorded as recognised where the session
/// records them. Of a build, `known` as it is, at the cost of two reads.
pub(super) fn left(tcx: TyCtxt<'_>, id: DefId, known: Option<Std>) -> Option<Std> {
    let known = known?;
    let caches = crate::lower::session::caches(tcx);
    if caches.without_recognizers.is_empty() && caches.recognized.is_none() {
        return Some(known);
    }
    // Only std's functions are named: a binding's or the crate's own a
    // recogniser knows by its identity has no std path, and stays.
    let path = std_path(tcx, id);
    if path.is_empty() {
        return Some(known);
    }
    if caches.without_recognizers.contains(path) {
        return None;
    }
    if let Some(recognized) = &caches.recognized {
        recognized.borrow_mut().insert(path);
    }
    Some(known)
}
