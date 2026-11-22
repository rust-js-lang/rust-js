//! What a body's MIR says, asked of the body alone, never of a lowering's
//! state: what a call's `Result` and `Arguments` are given to, the drops
//! that drop nothing, the constant a promoted body refers to, whether a
//! local is the program's variable, and the types of what MIR proves of an
//! `Option`. The base's since #2299 moved them from `mir`, which asks them
//! and the analyses with it.

pub mod arguments_use;
pub mod drop_flow;
pub mod presence;
pub mod referred;
pub mod result_use;
pub mod user_variable;
