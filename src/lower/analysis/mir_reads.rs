//! What a body's MIR says, asked of the body alone, never of a lowering's
//! state: what a call's `Result` and `Arguments` are given to, the drops
//! that drop nothing, the constant a promoted body refers to, whether a
//! local is the program's variable, and the types of what MIR proves of an
//! `Option`. The base's since #2299 moved them from `mir`, which asks them
//! and the analyses with it.

pub(in crate::lower) mod arguments_use;
pub(in crate::lower) mod drop_flow;
pub(in crate::lower) mod presence;
pub(in crate::lower) mod referred;
pub(in crate::lower) mod result_use;
pub(in crate::lower) mod user_variable;
