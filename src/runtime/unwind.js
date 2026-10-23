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
// thrown: an `Error`, so its host shows where, as Rust's hook shows it.
class $Unwinding extends Error {
  constructor(payload) {
    super("Box<dyn Any>");
    this.payload = payload;
  }
}

// What unwound, of `e`, thrown: a payload thrown as it is, a panic's
// message, or what JS threw.
function $unwound(e) {
  if (e instanceof $Unwinding) {
    return e.payload;
  }
  if ($isPanic(e)) {
    return { value: e.message, impl: $panicMessage };
  }
  return { value: e, impl: $jsThrown };
}

// What's thrown to unwind with `payload`: what JS threw, itself; a panic of
// text, its `Error`; any other, in a `$Unwinding`.
function $unwinding(payload) {
  const id = payload.impl.typeId();
  if (id === "&js::JsError") {
    return payload.value;
  }
  if (id === "&str" || id === "alloc::string::String") {
    return new Error(payload.value);
  }
  return new $Unwinding(payload);
}

// `catch_unwind(f)`: `Ok` of what `f()` gives, or `Err` of what unwound.
function $catchUnwind(f) {
  try {
    return { TAG: "Ok", _0: f() };
  } catch (e) {
    return { TAG: "Err", _0: $unwound(e) };
  }
}
