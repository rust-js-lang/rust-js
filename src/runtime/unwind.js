// Unwinding (ADR 0660): what JS throws, a panic's `Error` or anything else,
// unwinds Rust's frames; `catch_unwind` catches it, its payload a `Box<dyn
// Any + Send>`, the pair `{ value, impl }` (ADR 0331), and `resume_unwind`
// throws one.

// Whether `e`, thrown, is a panic: an `Error` of JS's own constructor, as
// `panic!` throws (ADR 0012), whoever threw it; or a payload that isn't
// text, which `resume_unwind` throws in a `$Unwinding`.
function $isPanic(e) {
  return e instanceof $Unwinding || (e instanceof Error && e.constructor === Error);
}

// A panic's message as a `dyn Any`'s `Any`: a `&str`, which a downcast to a
// `String` finds too, as JS doesn't keep which a panic's was.
const $panicMessage = { typeId: () => "&str" };

// What JS threw, as a `dyn Any`'s `Any`: a `&'static JsError`.
const $jsThrown = { typeId: () => "&js::JsError" };

// A payload that's neither text nor what JS threw, `resume_unwind(Box::new(5))`,
// thrown: an `Error`, so its host shows where, as Rust's hook shows it; and
// what the payload's `&JsError` is, its `error`, as every unwind's payload
// downcasts as what was thrown (ADR 0710).
class $Unwinding extends Error {
  constructor(payload) {
    super("Box<dyn Any>");
    this.payload = payload;
    payload.error = this;
  }
}

// What unwound, of `e`, thrown: a payload thrown as it is; a panic's
// message, and the `Error` it was, what JS threw too (ADR 0692); or what
// JS threw. Each one's `error` is `e`, what its `&JsError` is (ADR 0710).
function $unwound(e) {
  if (e instanceof $Unwinding) {
    return e.payload;
  }
  if ($isPanic(e)) {
    return { value: e.message, impl: $panicMessage, error: e };
  }
  return { value: e, impl: $jsThrown, error: e };
}

// What's thrown to unwind with `payload`: the `$Unwinding` it was thrown
// in, a panic's `Error`, its message as it was, or what JS threw, itself;
// a panic of text, its `Error`; any other, in a `$Unwinding`.
function $unwinding(payload) {
  if (payload.error instanceof $Unwinding) {
    return payload.error;
  }
  if (payload.impl === $panicMessage && payload.value === payload.error.message) {
    return payload.error;
  }
  const id = payload.impl.typeId();
  if (id === "&js::JsError") {
    return payload.value;
  }
  if (id === "&str" || id === "alloc::string::String") {
    return new Error(payload.value);
  }
  return new $Unwinding(payload);
}

// `catch_unwind(f)`: `Ok` of what `f()` gives, or `Err` of what unwound;
// never of an abort, which no `catch_unwind` catches (#847).
function $catchUnwind(f) {
  try {
    return { TAG: "Ok", _0: f() };
  } catch (e) {
    if (e instanceof $Abort) throw e;
    return { TAG: "Err", _0: $unwound(e) };
  }
}

// What a panic runs as it unwinds, Rust's cleanup: the drops of what the
// frames it leaves hold. `run()`, where a throw, a drop's panic while
// another panic unwinds, aborts the process, as Rust's does (#847, ADR 0660
// amended): the one place it's decided, which the lowering's cleanups and
// the runtime's (`$dropsUnwinding`) both call. Once aborted, as in a
// browser, which `$abort` can't end, nothing more runs as it unwinds.
function $cleanup(run) {
  if ($aborted) return;
  try {
    run();
  } catch (second) {
    throw $abort(second);
  }
}

// The process ended by Rust's abort, of `cause`, a drop's panic while
// another unwound: std's line to stderr, then exit 134, as a shell gives a
// SIGABRT, where the host has a process. A browser has none, so what's
// thrown is a `$Abort`, which `$catchUnwind` and the runtime throw on, but a
// `catch_unwind` lowered to JS's own `try` catches (ADR 0660's limit).
let $aborted = false;
class $Abort extends Error {}
function $abort(cause) {
  if (cause instanceof $Abort) return cause;
  $aborted = true;
  $eprint("thread caused non-unwinding panic. aborting.\n");
  globalThis.process?.exit?.(134);
  return new $Abort("panic in a destructor during cleanup", { cause });
}
