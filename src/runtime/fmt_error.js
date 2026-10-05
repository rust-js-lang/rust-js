// A writer's `Err(fmt::Error)` (ADR 0187): thrown, with what was written
// before it, which each writer it passes out of puts its own in front of.
// `Ok` is `undefined`, as it was (ADR 0054), and an `Err` caught is the error.
// One no consumer took, of a library's generic code, is `format!`'s panic.
class $FmtError extends Error {
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
// A `fmt::Result` as a value: what `write` returns, `undefined`, or its error.
function $fmtTry(write) {
  try {
    return write();
  } catch (error) {
    if (error instanceof $FmtError) return error;
    throw error;
  }
}
// `?` of one: its error, on.
function $fmtCheck(result) {
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
// `unwrap()` and `expect(..)` of one: `()`, or std's panic.
function $fmtExpect(result, message) {
  if (result !== undefined) throw new Error(message + ": Error");
}
// What a write to a `String` had written before it failed, taken from the
// error, as the string has it now.
function $fmtPartial(error) {
  if (!(error instanceof $FmtError)) return "";
  const written = error.written;
  error.written = "";
  return written;
}
