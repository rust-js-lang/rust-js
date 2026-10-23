//! What `@types/node` adds to `Error`, the js crate's [`JsError`]: V8's
//! [stack trace API](https://v8.dev/docs/stack-trace-api), the hook that
//! writes an error's `stack` and how many frames it's given. Chrome's V8
//! has it too, where React's client reads its owner stacks (ADR 0648).
//!
//! ```rust,ignore
//! let prev_prepare_stack_trace = js_error::prepare_stack_trace();
//! js_error::set_prepare_stack_trace(DEFAULT_PREPARE_STACK_TRACE);
//! let stack = error.stack();
//! js_error::set_prepare_stack_trace(prev_prepare_stack_trace);
//! ```
//!
//! ```js
//! const prevPrepareStackTrace = Error.prepareStackTrace;
//! Error.prepareStackTrace = DefaultPrepareStackTrace;
//! const stack = error.stack;
//! Error.prepareStackTrace = prevPrepareStackTrace;
//! ```

use js::{JsError, Unknown};

use crate::CallSite;

/// [`Error.prepareStackTrace`](https://v8.dev/docs/stack-trace-api#customizing-stack-traces):
/// the hook that writes an error's `stack`, given the error and its
/// frames, or `None`, `undefined`, as it is until a program sets one, when
/// V8 writes its own. What it gives is of any shape, `any`.
#[cfg_attr(rust_js, rust_js::link_name = "get Error.prepareStackTrace")]
pub fn prepare_stack_trace() -> Option<&'static dyn Fn(&JsError, &[&CallSite]) -> Option<&'static Unknown>> {
    unreachable!()
}

/// `Error.prepareStackTrace = hook`: the hook from now on, what
/// [`prepare_stack_trace`] gave, a Rust function, `Some(&prepare)`, or
/// `None`, `undefined`, V8's own again. It may give any value, `any`: a
/// `None` of no hook says of what, `set_prepare_stack_trace::<String>(None)`.
#[cfg_attr(rust_js, rust_js::link_name = "set Error.prepareStackTrace")]
pub fn set_prepare_stack_trace<R>(hook: Option<&'static dyn Fn(&JsError, &[&CallSite]) -> R>) {
    unreachable!()
}

/// [`Error.stackTraceLimit`](https://v8.dev/docs/stack-trace-api#stack-trace-collection-for-custom-exceptions):
/// how many frames a stack is given, 10 until it's set.
#[cfg_attr(rust_js, rust_js::link_name = "get Error.stackTraceLimit")]
pub fn stack_trace_limit() -> f64 {
    unreachable!()
}

/// `Error.stackTraceLimit = limit`: of each stack made from now on.
#[cfg_attr(rust_js, rust_js::link_name = "set Error.stackTraceLimit")]
pub fn set_stack_trace_limit(limit: f64) {
    unreachable!()
}
