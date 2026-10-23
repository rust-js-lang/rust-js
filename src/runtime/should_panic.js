// `#[should_panic]` (ADR 0026): the test passes only if it panics, and if
// `expected` is given, only if the panic message contains it, as with
// libtest. A panic is what `$isPanic` says is one (ADR 0660); anything else
// thrown, a `TypeError` say, is the JS going wrong, and fails the test as it
// is.
function $shouldPanic(f, expected) {
  try {
    f();
  } catch (e) {
    if (!$isPanic(e)) {
      throw e;
    }
    const message = e.message;
    if (expected === undefined || message.includes(expected)) {
      return;
    }
    throw new Error("panic did not contain expected string\n      panic message: " + JSON.stringify(message) + "\n expected substring: " + JSON.stringify(expected));
  }
  throw new Error("test did not panic as expected");
}
