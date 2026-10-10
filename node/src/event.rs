//! Node's events, each a name of its own (ADR 0361), a string in JS:
//! [`Exit`] is `"exit"`. An emitter says which it has, and its listener's
//! type, by [`Emits`](crate::events::Emits).

/// `"beforeExit"`: the event loop is empty, and the process would end.
#[cfg_attr(rust_js, rust_js::name = "beforeExit")]
pub struct BeforeExit;

/// `"disconnect"`: an IPC channel closed.
#[cfg_attr(rust_js, rust_js::name = "disconnect")]
pub struct Disconnect;

/// `"exit"`: the process ends, now.
#[cfg_attr(rust_js, rust_js::name = "exit")]
pub struct Exit;

/// `"rejectionHandled"`: a promise rejected unhandled was handled later.
#[cfg_attr(rust_js, rust_js::name = "rejectionHandled")]
pub struct RejectionHandled;

/// `"uncaughtException"`: nothing caught what was thrown.
#[cfg_attr(rust_js, rust_js::name = "uncaughtException")]
pub struct UncaughtException;

/// `"uncaughtExceptionMonitor"`: [`UncaughtException`], before its listeners.
#[cfg_attr(rust_js, rust_js::name = "uncaughtExceptionMonitor")]
pub struct UncaughtExceptionMonitor;

/// `"unhandledRejection"`: nothing handled a promise's rejection.
#[cfg_attr(rust_js, rust_js::name = "unhandledRejection")]
pub struct UnhandledRejection;

/// `"warning"`: a warning, `process.emitWarning`'s say.
#[cfg_attr(rust_js, rust_js::name = "warning")]
pub struct Warning;

/// `"workerMessage"`: a message a worker posted to the process.
#[cfg_attr(rust_js, rust_js::name = "workerMessage")]
pub struct WorkerMessage;
