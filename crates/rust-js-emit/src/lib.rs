//! The compiler's printing (ADR 1181): the program owned output holds
//! (`rust_js_program`), written as JS and its source maps by oxc, its
//! declarations by TypeScript, laid out by the app's Prettier, and
//! published. It knows nothing of the front end.

pub mod format;
pub mod hooks;
pub mod node_session;
pub mod output;
pub mod prettier;
pub mod publish;
pub mod to_oxc;
pub mod typescript;
