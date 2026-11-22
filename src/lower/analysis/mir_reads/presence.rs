//! What MIR proves of an `Option` where it's read, `Presence`, and of an
//! argument where a call reads it, `ArgProof` (#2112, #2178): the types,
//! the base's since #2299 moved them from `mir::none_forms`, which answers
//! them.

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
/// `Present` or `Defined` (`presence`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::lower) struct ArgProof {
    pub(in crate::lower) present: bool,
}
