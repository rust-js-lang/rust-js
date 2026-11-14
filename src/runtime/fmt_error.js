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
// What `f` had written before `error` came out of what it called. Of a
// panic part-way through a line, each line that text ended, what it printed
// as it formatted, written already, as Rust's stdout writes a line at its
// newline; the rest of the line is lost as the program ends (#1761, the
// guide's difference 57).
function $fmtWritten(error, f) {
  if (error instanceof $FmtError) error.written = f + error.written;
  else if ($printFrames.at(-1) === $out && f !== "") {
    $captures.push({ text: f });
    $flushAhead();
    $captures.pop();
  }
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
// A write's result given where generic code has a `Result` of its type
// parameters, `{ TAG }` (#1408): `Ok`, or `Err` of its error.
function $writeTagged(result) {
  return result === undefined ? { TAG: "Ok" } : { TAG: "Err", _0: result };
}
// A generic `Result` of a write's error, as a write's result is a value:
// `undefined`, or its error; a `fmt::Error` generic code made, which holds
// nothing, `undefined`, a write's error of its own (#1408).
function $writeOf(result) {
  if (result.TAG === "Ok") return undefined;
  return result._0 ?? new $FmtError();
}
// What a function that gives a value's `Result` caught: a write's error,
// its `Err`'s; anything else thrown on (ADR 0462).
function $writeCaught(error) {
  if (error instanceof $WriteError) return error;
  throw error;
}
// A `write!`'s text through generic code's writer, `make()`'s: where a
// value fails, what was made before it given to `written`, the writer's
// `write_str`, as std's `write_fmt` writes each piece before it, then the
// error thrown on (#1696). Till ADR 1135's last step writes each piece.
function $fmtTextOr(make, written) {
  try {
    return make();
  } catch (error) {
    if (error instanceof $FmtError) written($fmtPartial(error));
    throw error;
  }
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
  // In a line formatted through stdout's, after what's written there (#1490).
  if (!stderr && $printFrames.length > 0) $out(text + error.written);
  else $write(text + error.written, stderr);
  throw new Error("a formatting trait implementation returned an error when the underlying stream did not");
}
// `unwrap()` and `expect(..)` of one: `()`, or std's panic, of the error's
// `{:?}`.
function $writeExpect(result, message) {
  if (result === undefined) return;
  throw new Error(message + ": " + (result instanceof $IoError ? $ioErrorDebug(result) : "Error"));
}
// `unwrap_err()` and `expect_err(..)` of one: its error, or std's panic, of
// `Ok`'s `()`.
function $writeExpectErr(result, message) {
  if (result === undefined) throw new Error(message + ": ()");
  return result;
}
// What a write to a `String` had written before it failed, taken from the
// error, as the string has it now.
function $fmtPartial(error) {
  if (!(error instanceof $FmtError)) return "";
  const written = error.written;
  error.written = "";
  return written;
}
