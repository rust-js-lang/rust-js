//! [React Native](https://reactnative.dev) for rust-js (ADR 1083): the
//! globals its host installs in the JS runtime, as its C++ bindings define
//! them, for a program's code that runs in React Native, as `webapi` is the
//! browser's and `node` is Node's. Each may be absent, where the program
//! runs elsewhere: `js::is_defined(react_native::native_runtime_scheduler)`
//! is `typeof nativeRuntimeScheduler !== "undefined"`.
//!
//! ```rust,ignore
//! use react_native::{NativeScheduler, native_runtime_scheduler};
//!
//! let now = if js::is_defined(native_runtime_scheduler) {
//!     native_runtime_scheduler.unstable_now()
//! } else {
//!     js::date::now()
//! };
//! ```
//!
//! ```js
//! const now = typeof nativeRuntimeScheduler !== "undefined" ? nativeRuntimeScheduler.unstable_now() : Date.now();
//! ```

// A binding's body never runs: rust-js skips its check of it, but a plain
// rustc, a `cargo check` that runs no rust-js, sees an unread parameter
// (ADR 0440, amended).
#![cfg_attr(not(rust_js), allow(unused_variables))]

pub mod scheduler;

pub use js;
pub use scheduler::{Callback, NativeScheduler, Task, native_runtime_scheduler};
