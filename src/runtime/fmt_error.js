// A write's error: a `fmt::Error` (ADR 0187), or an `io::Error` (ADR 0462),
// thrown where its `fmt::Result` or `io::Result<()>` is an `Err`. `Ok` is
// `undefined`, as it was (ADRs 0054, 0132), and an `Err` caught is the error.
class $WriteError extends Error {}
// A writer's `Err(fmt::Error)`: thrown, with what was written before it,
// which each writer it passes out of puts its own in front of. One no
// consumer took, of a library's generic code, is `format!`'s panic.
class $FmtError extends $WriteError {
  written = "";
  constructor() {
    super("a formatting trait implementation returned an error when the underlying stream did not: Error");
  }
}
function $fmtError() {
  throw new $FmtError();
}
// What `f` had written before `error` came out of what it called.
function $fmtWritten(error, f) {
  if (error instanceof $FmtError) error.written = f + error.written;
  return error;
}
// A write's result as a value: what `write` returns, `undefined`, or its error.
function $writeTry(write) {
  try {
    return write();
  } catch (error) {
    if (error instanceof $WriteError) return error;
    throw error;
  }
}
// One where generic code has a `Result` (ADR 0461): `Ok`, or `Err` of the
// error `write` threw.
function $writeResult(write) {
  try {
    write();
    return { TAG: "Ok" };
  } catch (error) {
    if (error instanceof $WriteError) return { TAG: "Err", _0: error };
    throw error;
  }
}
// A generic `Result` of a write's error, as a write's result is a value:
// `undefined`, or its error.
function $writeOf(result) {
  return result.TAG === "Ok" ? undefined : result._0;
}
// What a function that gives a value's `Result` caught: a write's error,
// its `Err`'s; anything else thrown on (ADR 0462).
function $writeCaught(error) {
  if (error instanceof $WriteError) return error;
  throw error;
}
// `?` of one: its error, on.
function $writeCheck(result) {
  if (result !== undefined) throw result;
}
// `to_string()` and `format!` of what fails: std's panic, `message`.
function $fmtOrPanic(write, message) {
  try {
    return write();
  } catch (error) {
    if (error instanceof $FmtError) throw new Error(message);
    throw error;
  }
}
// `print!` of what fails: the text written before it, then std's panic.
function $fmtPrintFailed(error, text, stderr) {
  if (!(error instanceof $FmtError)) throw error;
  $write(text + error.written, stderr);
  throw new Error("a formatting trait implementation returned an error when the underlying stream did not");
}
// `unwrap()` and `expect(..)` of one: `()`, or std's panic, of the error's
// `{:?}`.
function $writeExpect(result, message) {
  if (result === undefined) return;
  throw new Error(message + ": " + (result instanceof $IoError ? $ioErrorDebug(result) : "Error"));
}
// What a write to a `String` had written before it failed, taken from the
// error, as the string has it now.
function $fmtPartial(error) {
  if (!(error instanceof $FmtError)) return "";
  const written = error.written;
  error.written = "";
  return written;
}
