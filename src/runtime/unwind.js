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
// never of what `$catchable` throws on (#847, #870, #1193).
function $catchUnwind(f) {
  try {
    return { TAG: "Ok", _0: f() };
  } catch (e) {
    $catchable(e);
    return { TAG: "Err", _0: $unwound(e) };
  }
}

// What no `catch_unwind` catches, thrown on, `$catchUnwind`'s and the
// inline one's `catch (error) { $catchable(error); .. }`: an abort (#847),
// the runtime's own bug (#870), what rust-js doesn't support as it runs
// (`$Unsupported`); and a stack overflow, which Rust aborts the process of
// (`$overflowed`, #1193).
function $catchable(e) {
  if (e instanceof RangeError || e?.name === "InternalError") $suspect = e;
  if (e instanceof $Abort) $abortEnds();
  if ($endsTheProgram(e)) throw e;
  if ($isStackOverflow(e)) throw $overflowed(e);
}

// Whether `e`, thrown, ends the program, never unwound as a panic: an
// abort, the runtime's own bug, or what rust-js doesn't support as it runs.
function $endsTheProgram(e) {
  return e instanceof $Abort || e instanceof $InternalError || e instanceof $Unsupported;
}

// `e`, thrown on through a scope whose drops run the crate's code, `try {
// .. } catch (error) { throw $unwindsOn(error); } finally { drops }`: of a
// stack overflow, Rust's abort first, so no drop runs, as Rust's runs none
// (#1193).
//
// It's the first call a stack overflow's deepest frame makes, where there's
// little room: so first, before it calls anything, what may be one is
// kept, `$suspect`, which that frame's `finally` reads (`$live`), as small
// a call, where this one had no room to say more.
let $suspect;
function $unwindsOn(e) {
  if (e instanceof RangeError || e?.name === "InternalError") $suspect = e;
  return $unwindsOnTold(e);
}
function $unwindsOnTold(e) {
  if (e instanceof $Abort) $abortEnds();
  return $isStackOverflow(e) ? $overflowed(e) : e;
}

// What a panic runs as it unwinds, Rust's cleanup: the drops of what the
// frames it leaves hold. `run()`, where a throw, a drop's panic while
// another panic unwinds, aborts the process, as Rust's does (#847, ADR 0660
// amended): the one place it's decided, which the lowering's cleanups and
// the runtime's (`$dropsUnwinding`) both call. Once aborted, as in a
// browser, which `$abort` can't end, nothing more runs as it unwinds. Of
// what unwinds, `e`, where it's given: none of it of what ends the program
// (`$endsTheProgram`), and of a stack overflow, Rust's abort first (#1193).
// A write's `Err`, thrown in JS (ADR 0187), is returned in Rust, not unwound:
// its drops are an ordinary scope's end, `thread::panicking()` false in them,
// and their panic an ordinary one, which goes on in its place (#1566).
function $cleanup(run, e) {
  if (e instanceof $WriteError) {
    run();
    return;
  }
  if (!$live()) return;
  if (e !== undefined) {
    if ($endsTheProgram(e)) return;
    if ($isStackOverflow(e)) throw $overflowed(e);
  }
  $cleanupDepth++;
  try {
    run();
  } catch (second) {
    if (second instanceof $InternalError || second instanceof $Unsupported) throw second;
    throw $abort(second);
  } finally {
    $cleanupDepth--;
  }
}

// How deep this thread is in cleanups a panic runs as it unwinds, and
// `thread::panicking()`, whether it's in one: a drop a panic runs asks it
// (#1285).
let $cleanupDepth = 0;
function $panicking() {
  return $cleanupDepth > 0;
}

// What rust-js doesn't support as the program runs, where Rust's runs on:
// `$unsupported("rust-js does not support ..")`'s, the one form of a
// refusal at run time, which no `catch_unwind` takes for a panic and no
// cleanup runs drops past, as it ends the program (#1132, #1180).
class $Unsupported extends Error {}
function $unsupported(message) {
  return new $Unsupported(message);
}

// Whether `e`, thrown, is the engine's stack overflow, which Rust's would
// be an abort of (#1193): what it threw of its own when the runtime once
// overflowed its stack on purpose, of the same constructor and message,
// asked first of a `RangeError`, V8's and JavaScriptCore's, or an
// `InternalError`, SpiderMonkey's, never of a panic; another `RangeError`,
// an array's length, is another message.
let $overflowSeen;
function $isStackOverflow(e) {
  if (!(e instanceof RangeError || e?.name === "InternalError")) return false;
  if ($overflowSeen === undefined) {
    const deep = () => deep() + 1;
    try {
      deep();
    } catch (overflow) {
      $overflowSeen = { constructor: overflow?.constructor, message: overflow?.message };
    }
  }
  return e.constructor === $overflowSeen?.constructor && e.message === $overflowSeen?.message;
}

// The process ended by Rust's abort of a stack overflow, `cause`: std's
// lines to stderr, then exit 134, as a shell gives a SIGABRT (#1193).
function $overflowed(cause) {
  return $abort(cause, "\nthread 'main' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n", "stack overflow");
}

// The process ended by Rust's abort, of `cause`, a drop's panic while
// another unwound: std's line to stderr, then exit 134, as a shell gives a
// SIGABRT, where the host has a process. A browser has none, so what's
// thrown is a `$Abort`, which `$catchUnwind` and the runtime throw on, but a
// `catch_unwind` lowered to JS's own `try` catches (ADR 0660's limit).
// What it says, then the exit, are tried again as it unwinds, by each
// guard it passes (`$abortEnds`), where the stack had no room for them
// where it began, a stack overflow's deepest frame (#1193).
let $aborted = false;
let $abortSaid;
class $Abort extends Error {}
// A panic out of a function whose ABI can't unwind, `extern "C"`'s: the
// process aborts, as Rust's does, past any `catch_unwind` (#1582). What
// ends the program on its own, an exit or a refusal, goes on as it is.
function $cannotUnwind(e) {
  if ($endsTheProgram(e) || e instanceof $Unsupported) return e;
  return $abort(e, undefined, "panic in a function that cannot unwind");
}

function $abort(cause, said = "thread caused non-unwinding panic. aborting.\n", why = "panic in a destructor during cleanup") {
  if (cause instanceof $Abort) {
    $abortEnds();
    return cause;
  }
  // One abort's: another as it unwinds only ends it.
  if ($aborted) {
    $abortEnds();
    return new $Abort(why, { cause });
  }
  $aborted = true;
  $abortSaid = said;
  $abortEnds();
  return new $Abort(why, { cause });
}

// An abort's line said, once, then the exit: where the stack has no room
// for them yet, as it unwinds, a guard further out tries again.
function $abortEnds() {
  if ($abortSaid === undefined) return;
  try {
    if ($abortSaid !== "") {
      $eprint($abortSaid);
      $abortSaid = "";
    }
    $abortExit(134);
    // A host with no process to end, a browser's: ended as far as it goes.
    $abortSaid = undefined;
  } catch {
    // No room yet: a frame further out ends it.
  }
}

// Whether the program goes on, not aborted: a scope's drops a program sees
// run only so, in a `finally` that runs as an abort unwinds, where the
// host has no process to end, a browser, or the stack had no room to end
// it yet (#1193).
//
// Of a stack overflow that `$unwindsOn` kept, `$suspect`, where it had no
// room to abort, the abort, here or further out (#1193).
function $live() {
  if (!$aborted && $suspect === undefined) return true;
  return $liveTold();
}
function $liveTold() {
  if ($suspect !== undefined) {
    const suspect = $suspect;
    if (!$isStackOverflow(suspect)) {
      $suspect = undefined;
      return !$aborted;
    }
    $overflowed(suspect);
  }
  $abortEnds();
  return false;
}

// The runtime's own bug, which no Rust program makes: thrown as it is, never
// a panic (`$isPanic`), caught by no `catch_unwind` and no cleanup's abort,
// so a compiler's bug is never taken for a program's (#870).
class $InternalError extends Error {
  constructor(message) {
    super(`rust-js internal error: ${message}`);
  }
}
